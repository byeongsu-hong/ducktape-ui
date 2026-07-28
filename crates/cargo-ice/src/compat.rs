use crate::schema::{
    ACCESSKIT_UNIX_VERSION, ACCESSKIT_VERSION, ACCESSKIT_WINDOWS_VERSION, ICED_VERSION,
    ICED_WIDGET_VERSION, UI_LANG_BUILD_VERSION, UI_LANG_RUNTIME_VERSION,
};
use std::fs;
use std::path::Path;

const DIRECT_DEPENDENCIES: &str = "[dependencies]";
const BUILD_DEPENDENCIES: &str = "[build-dependencies]";
const LINUX_TARGET_DEPENDENCIES: &str = r#"[target.'cfg(target_os = "linux")'.dependencies]"#;
const WINDOWS_TARGET_DEPENDENCIES: &str = r#"[target.'cfg(target_os = "windows")'.dependencies]"#;

pub fn verify(root: &Path) -> Result<(), String> {
    verify_lock(&root.join("Cargo.lock"))?;
    verify_dependency(
        &root.join("examples/iced-app/Cargo.toml"),
        "iced",
        &format!("={ICED_VERSION}"),
        None,
        DIRECT_DEPENDENCIES,
    )?;
    verify_dependency(
        &root.join("examples/iced-app/Cargo.toml"),
        "ui-lang-build",
        &format!("={UI_LANG_BUILD_VERSION}"),
        Some(&root.join("crates/ui-lang-build")),
        BUILD_DEPENDENCIES,
    )?;
    verify_dependency(
        &root.join("examples/iced-app/Cargo.toml"),
        "ui-lang-runtime",
        &format!("={UI_LANG_RUNTIME_VERSION}"),
        Some(&root.join("crates/ui-lang-runtime")),
        DIRECT_DEPENDENCIES,
    )?;
    let runtime = root.join("crates/ui-lang-runtime/Cargo.toml");
    verify_dependency(
        &runtime,
        "iced",
        &format!("={ICED_VERSION}"),
        None,
        DIRECT_DEPENDENCIES,
    )?;
    verify_dependency(
        &runtime,
        "accesskit",
        &format!("={ACCESSKIT_VERSION}"),
        None,
        DIRECT_DEPENDENCIES,
    )?;
    verify_dependency(
        &runtime,
        "accesskit_unix",
        &format!("={ACCESSKIT_UNIX_VERSION}"),
        None,
        LINUX_TARGET_DEPENDENCIES,
    )?;
    verify_dependency(
        &runtime,
        "accesskit_windows",
        &format!("={ACCESSKIT_WINDOWS_VERSION}"),
        None,
        WINDOWS_TARGET_DEPENDENCIES,
    )?;
    println!(
        "compatibility baseline: iced {ICED_VERSION}, iced_widget {ICED_WIDGET_VERSION}, ui-lang-build {UI_LANG_BUILD_VERSION}, ui-lang-runtime {UI_LANG_RUNTIME_VERSION}, accesskit {ACCESSKIT_VERSION}, accesskit_unix {ACCESSKIT_UNIX_VERSION} (linux), accesskit_windows {ACCESSKIT_WINDOWS_VERSION} (windows)"
    );
    Ok(())
}

pub fn verify_lock(path: &Path) -> Result<(), String> {
    let lock = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    verify_lock_contents(&lock)
}

fn verify_lock_contents(lock: &str) -> Result<(), String> {
    for (name, expected, unique) in [
        ("iced", ICED_VERSION, true),
        ("iced_widget", ICED_WIDGET_VERSION, true),
        ("ui-lang-build", UI_LANG_BUILD_VERSION, true),
        ("ui-lang-runtime", UI_LANG_RUNTIME_VERSION, true),
        ("accesskit", ACCESSKIT_VERSION, false),
        ("accesskit_unix", ACCESSKIT_UNIX_VERSION, false),
        ("accesskit_windows", ACCESSKIT_WINDOWS_VERSION, false),
    ] {
        let actual = locked_versions(lock, name);
        match actual.as_slice() {
            [] => return Err(format!("Cargo.lock does not resolve `{name}`")),
            [actual] if *actual == expected => {}
            [actual] if unique => {
                return Err(format!(
                    "Cargo.lock resolves `{name}` {actual}; schema requires {expected}"
                ));
            }
            actual if unique => {
                return Err(format!(
                    "Cargo.lock resolves `{name}` more than once ({actual:?}); schema requires exactly {expected}"
                ));
            }
            actual if actual.contains(&expected) => {}
            actual => {
                return Err(format!(
                    "Cargo.lock resolves `{name}` as {actual:?}; runtime requires {expected}"
                ));
            }
        }
    }
    Ok(())
}

fn verify_dependency(
    manifest_path: &Path,
    name: &str,
    expected_version: &str,
    expected_path: Option<&Path>,
    dependency_section: &str,
) -> Result<(), String> {
    let manifest = fs::read_to_string(manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    let value = direct_dependency(&manifest, name, dependency_section).ok_or_else(|| {
        format!(
            "{} must list `{name}` in {dependency_section}",
            manifest_path.display()
        )
    })?;
    let actual_version = dependency_version(value);
    if actual_version != Some(expected_version) {
        return Err(format!(
            "{} requires `{name}` version {actual_version:?}; compatibility requires \"{expected_version}\"",
            manifest_path.display()
        ));
    }
    if let Some(expected_path) = expected_path {
        let relative = quoted_field(value, "path").ok_or_else(|| {
            format!(
                "{} must use the local `{name}` path",
                manifest_path.display()
            )
        })?;
        let parent = manifest_path.parent().unwrap_or_else(|| Path::new("."));
        let actual = parent.join(relative).canonicalize().map_err(|error| {
            format!(
                "cannot resolve `{name}` path `{relative}` from {}: {error}",
                manifest_path.display()
            )
        })?;
        let expected = expected_path.canonicalize().map_err(|error| {
            format!(
                "cannot resolve expected `{name}` path {}: {error}",
                expected_path.display()
            )
        })?;
        if actual != expected {
            return Err(format!(
                "{} points `{name}` at {}; compatibility requires {}",
                manifest_path.display(),
                actual.display(),
                expected.display()
            ));
        }
    }
    Ok(())
}

fn dependency_version(source: &str) -> Option<&str> {
    source
        .strip_prefix('"')
        .and_then(|value| value.split_once('"').map(|(value, _)| value))
        .or_else(|| quoted_field(source, "version"))
}

fn direct_dependency<'a>(
    manifest: &'a str,
    package: &str,
    dependency_section: &str,
) -> Option<&'a str> {
    let mut section = "";
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line;
            continue;
        }
        if section != dependency_section {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim().trim_matches(['\'', '"']) == package {
            return Some(value.trim());
        }
    }
    None
}

fn quoted_field<'a>(source: &'a str, field: &str) -> Option<&'a str> {
    let key_byte = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.');
    let mut string = false;
    let mut escaped = false;
    for (start, ch) in source.char_indices() {
        if string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                string = false;
            }
            continue;
        }
        if ch == '"' {
            string = true;
            continue;
        }
        if ch == '#' {
            break;
        }
        if !source[start..].starts_with(field) {
            continue;
        }
        let end = start + field.len();
        let boundary = (start == 0 || !key_byte(source.as_bytes()[start - 1]))
            && source
                .as_bytes()
                .get(end)
                .is_none_or(|byte| !key_byte(*byte));
        let after = &source[start + field.len()..];
        if boundary
            && let Some(value) = after.trim_start().strip_prefix('=')
            && let Some(value) = value.trim_start().strip_prefix('"')
            && let Some((value, _)) = value.split_once('"')
        {
            return Some(value);
        }
    }
    None
}

fn locked_versions<'a>(lock: &'a str, package: &str) -> Vec<&'a str> {
    lock.split("[[package]]")
        .filter_map(|block| {
            let mut name = None;
            let mut version = None;
            for line in block.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                let Some(value) = value
                    .trim()
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                else {
                    continue;
                };
                match key.trim() {
                    "name" => name = Some(value),
                    "version" => version = Some(value),
                    _ => {}
                }
            }
            (name == Some(package)).then_some(version).flatten()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        BUILD_DEPENDENCIES, DIRECT_DEPENDENCIES, LINUX_TARGET_DEPENDENCIES,
        WINDOWS_TARGET_DEPENDENCIES, dependency_version, direct_dependency, locked_versions,
        quoted_field, verify_lock_contents,
    };

    #[test]
    fn reads_exact_package_versions_from_a_lockfile() {
        let lock = r#"
[[package]]
name = "iced"
version = "0.14.0"
dependencies = [
 "iced_widget",
]

[[package]]
name = "iced_widget"
version = "0.14.2"

[[package]]
name = "ui-lang-build"
version = "0.1.0"

[[package]]
name = "ui-lang-runtime"
version = "0.1.0"

[[package]]
name = "accesskit"
version = "0.21.0"

[[package]]
name = "accesskit"
version = "0.24.1"

[[package]]
name = "accesskit_unix"
version = "0.22.1"

[[package]]
name = "accesskit_windows"
version = "0.32.0"
"#;

        assert_eq!(locked_versions(lock, "iced"), ["0.14.0"]);
        assert_eq!(locked_versions(lock, "iced_widget"), ["0.14.2"]);
        assert_eq!(locked_versions(lock, "accesskit_windows"), ["0.32.0"]);
        assert!(locked_versions(lock, "missing").is_empty());
        assert_eq!(verify_lock_contents(lock), Ok(()));

        let error = verify_lock_contents(&lock.replace(
            "name = \"accesskit_windows\"\nversion = \"0.32.0\"",
            "name = \"accesskit_windows\"\nversion = \"0.31.0\"",
        ))
        .unwrap_err();
        assert!(error.contains("`accesskit_windows`"), "{error}");
        assert!(error.contains("0.32.0"), "{error}");
    }

    #[test]
    fn rejects_missing_mismatched_and_duplicate_baselines() {
        let missing = r#"
[[package]]
name = "iced"
version = "0.14.0"
"#;
        assert_eq!(
            verify_lock_contents(missing).unwrap_err(),
            "Cargo.lock does not resolve `iced_widget`"
        );

        let mismatched = r#"
[[package]]
name = "iced"
version = "0.13.0"

[[package]]
name = "iced_widget"
version = "0.14.2"
"#;
        assert_eq!(
            verify_lock_contents(mismatched).unwrap_err(),
            "Cargo.lock resolves `iced` 0.13.0; schema requires 0.14.0"
        );

        let duplicate = r#"
[[package]]
name = "iced"
version = "0.14.0"

[[package]]
name = "iced"
version = "0.13.0"

[[package]]
name = "iced_widget"
version = "0.14.2"
"#;
        let error = verify_lock_contents(duplicate).unwrap_err();
        assert!(error.contains("resolves `iced` more than once"), "{error}");
        assert!(error.contains("0.14.0"), "{error}");
        assert!(error.contains("0.13.0"), "{error}");
    }

    #[test]
    fn reads_exact_direct_and_target_dependency_requirements() {
        let manifest = r#"
[dependencies]
# comments and blank lines do not end the dependency section

iced = { version = "=0.14.0", features = ["advanced", "canvas"] }
ui-lang-runtime = { path = "../../crates/ui-lang-runtime", version = "=0.1.0" }

[build-dependencies]
ui-lang-build = { path = "../../crates/ui-lang-build", version = "=0.1.0" }

[target.'cfg(target_os = "linux")'.dependencies]
accesskit_unix = "=0.22.1"

[target.'cfg(target_os = "windows")'.dependencies]
accesskit_windows = "=0.32.0"
"#;
        let runtime = direct_dependency(manifest, "ui-lang-runtime", DIRECT_DEPENDENCIES).unwrap();
        let build = direct_dependency(manifest, "ui-lang-build", BUILD_DEPENDENCIES).unwrap();
        let unix =
            direct_dependency(manifest, "accesskit_unix", LINUX_TARGET_DEPENDENCIES).unwrap();
        let windows =
            direct_dependency(manifest, "accesskit_windows", WINDOWS_TARGET_DEPENDENCIES).unwrap();

        assert_eq!(quoted_field(runtime, "version"), Some("=0.1.0"));
        assert_eq!(quoted_field(build, "version"), Some("=0.1.0"));
        assert_eq!(
            quoted_field(build, "path"),
            Some("../../crates/ui-lang-build")
        );
        assert_eq!(
            quoted_field(runtime, "path"),
            Some("../../crates/ui-lang-runtime")
        );
        assert_eq!(dependency_version(unix), Some("=0.22.1"));
        assert_eq!(dependency_version(windows), Some("=0.32.0"));
        assert_eq!(
            quoted_field(
                r#"{ package-version = "wrong", note = "version = wrong", version = "right" }"#,
                "version"
            ),
            Some("right")
        );
        assert_eq!(
            quoted_field(
                r#"{ package-version = "wrong" } # version = "wrong""#,
                "version"
            ),
            None
        );
        assert!(direct_dependency(manifest, "accesskit_unix", DIRECT_DEPENDENCIES).is_none());
        assert!(
            direct_dependency(manifest, "accesskit_windows", LINUX_TARGET_DEPENDENCIES).is_none()
        );

        let not_linux = r#"
[target.'cfg(not(target_os = "linux"))'.dependencies]
accesskit_unix = "=0.22.1"
"#;
        assert!(
            direct_dependency(not_linux, "accesskit_unix", LINUX_TARGET_DEPENDENCIES).is_none()
        );

        let not_windows = r#"
[target.'cfg(not(target_os = "windows"))'.dependencies]
accesskit_windows = "=0.32.0"
"#;
        assert!(
            direct_dependency(
                not_windows,
                "accesskit_windows",
                WINDOWS_TARGET_DEPENDENCIES
            )
            .is_none()
        );
    }
}

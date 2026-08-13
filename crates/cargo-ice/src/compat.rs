use crate::schema::{
    ACCESSKIT_UNIX_VERSION, ACCESSKIT_VERSION, ACCESSKIT_WINDOWS_VERSION, ICED_VERSION,
    ICED_WIDGET_VERSION, UI_LANG_BUILD_VERSION, UI_LANG_RUNTIME_VERSION,
};
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item};

const DIRECT_DEPENDENCIES: &str = "[dependencies]";
const BUILD_DEPENDENCIES: &str = "[build-dependencies]";
const LINUX_TARGET_DEPENDENCIES: &str = r#"[target.'cfg(target_os = "linux")'.dependencies]"#;
const WINDOWS_TARGET_DEPENDENCIES: &str = r#"[target.'cfg(target_os = "windows")'.dependencies]"#;

pub fn verify(root: &Path) -> Result<(), String> {
    verify_lock(&root.join("Cargo.lock"))?;
    let workspace_manifest_path = root.join("Cargo.toml");
    let workspace_manifest = read_manifest(&workspace_manifest_path)?;
    verify_dependency(
        &workspace_manifest,
        root,
        &root.join("examples/showcase/Cargo.toml"),
        "iced",
        &format!("={ICED_VERSION}"),
        None,
        DIRECT_DEPENDENCIES,
    )?;
    verify_dependency(
        &workspace_manifest,
        root,
        &root.join("examples/showcase/Cargo.toml"),
        "ui-lang-build",
        &format!("={UI_LANG_BUILD_VERSION}"),
        Some(&root.join("crates/ui-lang-build")),
        BUILD_DEPENDENCIES,
    )?;
    verify_dependency(
        &workspace_manifest,
        root,
        &root.join("examples/showcase/Cargo.toml"),
        "ui-lang-runtime",
        &format!("={UI_LANG_RUNTIME_VERSION}"),
        Some(&root.join("crates/ui-lang-runtime")),
        DIRECT_DEPENDENCIES,
    )?;
    let runtime = root.join("crates/ui-lang-runtime/Cargo.toml");
    verify_dependency(
        &workspace_manifest,
        root,
        &runtime,
        "iced",
        &format!("={ICED_VERSION}"),
        None,
        DIRECT_DEPENDENCIES,
    )?;
    verify_dependency(
        &workspace_manifest,
        root,
        &runtime,
        "accesskit",
        &format!("={ACCESSKIT_VERSION}"),
        None,
        DIRECT_DEPENDENCIES,
    )?;
    verify_dependency(
        &workspace_manifest,
        root,
        &runtime,
        "accesskit_unix",
        &format!("={ACCESSKIT_UNIX_VERSION}"),
        None,
        LINUX_TARGET_DEPENDENCIES,
    )?;
    verify_dependency(
        &workspace_manifest,
        root,
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
        let mut actual = locked_versions(lock, name);
        let Some(first) = actual.next() else {
            return Err(format!("Cargo.lock does not resolve `{name}`"));
        };
        if unique {
            if let Some(second) = actual.next() {
                let actual = std::iter::once(first)
                    .chain(std::iter::once(second))
                    .chain(actual)
                    .collect::<Vec<_>>();
                return Err(format!(
                    "Cargo.lock resolves `{name}` more than once ({actual:?}); schema requires exactly {expected}"
                ));
            }
            if first != expected {
                return Err(format!(
                    "Cargo.lock resolves `{name}` {first}; schema requires {expected}"
                ));
            }
        } else if first != expected && !actual.clone().any(|version| version == expected) {
            let actual = std::iter::once(first).chain(actual).collect::<Vec<_>>();
            return Err(format!(
                "Cargo.lock resolves `{name}` as {actual:?}; runtime requires {expected}"
            ));
        }
    }
    Ok(())
}

fn verify_dependency(
    workspace_manifest: &DocumentMut,
    workspace_root: &Path,
    manifest_path: &Path,
    name: &str,
    expected_version: &str,
    expected_path: Option<&Path>,
    dependency_section: &str,
) -> Result<(), String> {
    let manifest = read_manifest(manifest_path)?;
    let declared = dependency(&manifest, name, dependency_section).ok_or_else(|| {
        format!(
            "{} must list `{name}` in {dependency_section}",
            manifest_path.display()
        )
    })?;
    let (resolved, path_base) = if declared.workspace {
        let resolved =
            dependency(workspace_manifest, name, "[workspace.dependencies]").ok_or_else(|| {
                format!(
                    "{} inherits `{name}` from [workspace.dependencies], but the workspace does not define it",
                    manifest_path.display()
                )
            })?;
        if resolved.workspace {
            return Err(format!(
                "[workspace.dependencies] cannot inherit `{name}` from itself"
            ));
        }
        (resolved, workspace_root)
    } else {
        let parent = manifest_path.parent().unwrap_or_else(|| Path::new("."));
        (declared, parent)
    };
    let actual_version = resolved.version;
    if actual_version != Some(expected_version) {
        return Err(format!(
            "{} requires `{name}` version {actual_version:?}; compatibility requires \"{expected_version}\"",
            manifest_path.display()
        ));
    }
    if let Some(expected_path) = expected_path {
        let relative = resolved.path.ok_or_else(|| {
            format!(
                "{} must use the local `{name}` path",
                manifest_path.display()
            )
        })?;
        let actual = path_base.join(relative).canonicalize().map_err(|error| {
            format!(
                "cannot resolve `{name}` path `{relative}` from {}: {error}",
                path_base.display()
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

fn read_manifest(path: &Path) -> Result<DocumentMut, String> {
    let manifest = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    manifest
        .parse()
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Dependency<'a> {
    version: Option<&'a str>,
    path: Option<&'a str>,
    workspace: bool,
}

fn dependency<'a>(
    manifest: &'a DocumentMut,
    package: &str,
    dependency_section: &str,
) -> Option<Dependency<'a>> {
    let item = dependency_section_item(manifest, dependency_section)?.get(package)?;
    Some(Dependency {
        version: item
            .as_str()
            .or_else(|| item.get("version").and_then(Item::as_str)),
        path: item.get("path").and_then(Item::as_str),
        workspace: item
            .get("workspace")
            .and_then(Item::as_bool)
            .unwrap_or(false),
    })
}

fn dependency_section_item<'a>(
    manifest: &'a DocumentMut,
    dependency_section: &str,
) -> Option<&'a Item> {
    let root = manifest.as_item();
    match dependency_section {
        DIRECT_DEPENDENCIES => root.get("dependencies"),
        BUILD_DEPENDENCIES => root.get("build-dependencies"),
        LINUX_TARGET_DEPENDENCIES => root
            .get("target")?
            .get(r#"cfg(target_os = "linux")"#)?
            .get("dependencies"),
        WINDOWS_TARGET_DEPENDENCIES => root
            .get("target")?
            .get(r#"cfg(target_os = "windows")"#)?
            .get("dependencies"),
        "[workspace.dependencies]" => root.get("workspace")?.get("dependencies"),
        _ => None,
    }
}

fn locked_versions<'a>(lock: &'a str, package: &'a str) -> impl Iterator<Item = &'a str> + Clone {
    lock.split("[[package]]").filter_map(move |block| {
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
}

#[cfg(test)]
mod tests {
    use super::{
        BUILD_DEPENDENCIES, DIRECT_DEPENDENCIES, Dependency, LINUX_TARGET_DEPENDENCIES,
        WINDOWS_TARGET_DEPENDENCIES, dependency, locked_versions, read_manifest, verify_dependency,
        verify_lock_contents,
    };
    use std::fs;
    use tempfile::tempdir;
    use toml_edit::DocumentMut;

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

        assert_eq!(
            locked_versions(lock, "iced").collect::<Vec<_>>(),
            ["0.14.0"]
        );
        assert_eq!(
            locked_versions(lock, "iced_widget").collect::<Vec<_>>(),
            ["0.14.2"]
        );
        assert_eq!(
            locked_versions(lock, "accesskit_windows").collect::<Vec<_>>(),
            ["0.32.0"]
        );
        assert!(locked_versions(lock, "missing").next().is_none());
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
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_lock_verification_does_not_allocate_on_success() {
        const SCANS: u64 = 100;
        let lock = include_str!("../../../Cargo.lock");
        assert_eq!(verify_lock_contents(lock), Ok(()));

        let _profiler = dhat::Profiler::builder().testing().build();
        for _ in 0..SCANS {
            std::hint::black_box(verify_lock_contents(std::hint::black_box(lock))).unwrap();
        }
        let stats = dhat::HeapStats::get();

        assert_eq!(stats.total_blocks, 0, "{stats:?}");
        assert_eq!(stats.total_bytes, 0, "{stats:?}");
        eprintln!(
            "{SCANS} successful compatibility lock scans: {} heap blocks / {} bytes",
            stats.total_blocks, stats.total_bytes
        );
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
"#
        .parse::<DocumentMut>()
        .unwrap();
        let runtime = dependency(&manifest, "ui-lang-runtime", DIRECT_DEPENDENCIES).unwrap();
        let build = dependency(&manifest, "ui-lang-build", BUILD_DEPENDENCIES).unwrap();
        let unix = dependency(&manifest, "accesskit_unix", LINUX_TARGET_DEPENDENCIES).unwrap();
        let windows =
            dependency(&manifest, "accesskit_windows", WINDOWS_TARGET_DEPENDENCIES).unwrap();

        assert_eq!(runtime.version, Some("=0.1.0"));
        assert_eq!(build.version, Some("=0.1.0"));
        assert_eq!(build.path, Some("../../crates/ui-lang-build"));
        assert_eq!(runtime.path, Some("../../crates/ui-lang-runtime"));
        assert_eq!(unix.version, Some("=0.22.1"));
        assert_eq!(windows.version, Some("=0.32.0"));
        assert!(dependency(&manifest, "accesskit_unix", DIRECT_DEPENDENCIES).is_none());
        assert!(dependency(&manifest, "accesskit_windows", LINUX_TARGET_DEPENDENCIES).is_none());

        let not_linux = r#"
[target.'cfg(not(target_os = "linux"))'.dependencies]
accesskit_unix = "=0.22.1"
"#
        .parse::<DocumentMut>()
        .unwrap();
        assert!(dependency(&not_linux, "accesskit_unix", LINUX_TARGET_DEPENDENCIES).is_none());

        let not_windows = r#"
[target.'cfg(not(target_os = "windows"))'.dependencies]
accesskit_windows = "=0.32.0"
"#
        .parse::<DocumentMut>()
        .unwrap();
        assert!(
            dependency(
                &not_windows,
                "accesskit_windows",
                WINDOWS_TARGET_DEPENDENCIES
            )
            .is_none()
        );
    }

    #[test]
    fn reads_all_cargo_workspace_inheritance_forms() {
        let manifest = r#"
[dependencies]
dotted.workspace = true
inline = { workspace = true }

[dependencies.table]
workspace = true
"#
        .parse::<DocumentMut>()
        .unwrap();

        for name in ["dotted", "inline", "table"] {
            assert_eq!(
                dependency(&manifest, name, DIRECT_DEPENDENCIES),
                Some(Dependency {
                    version: None,
                    path: None,
                    workspace: true,
                })
            );
        }
    }

    #[test]
    fn resolves_inherited_version_and_path_from_the_workspace_root() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let member_dir = root.join("members/app");
        let dependency_dir = root.join("crates/example");
        fs::create_dir_all(&member_dir).unwrap();
        fs::create_dir_all(&dependency_dir).unwrap();
        let workspace_path = root.join("Cargo.toml");
        let member_path = member_dir.join("Cargo.toml");
        fs::write(
            &workspace_path,
            r#"
[workspace.dependencies]
example = { path = "crates/example", version = "=1.2.3" }
"#,
        )
        .unwrap();
        fs::write(
            &member_path,
            r#"
[dependencies]
example.workspace = true
"#,
        )
        .unwrap();
        let workspace = read_manifest(&workspace_path).unwrap();

        assert_eq!(
            verify_dependency(
                &workspace,
                root,
                &member_path,
                "example",
                "=1.2.3",
                Some(&dependency_dir),
                DIRECT_DEPENDENCIES,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_missing_or_mismatched_workspace_dependencies() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let member_dir = root.join("member");
        fs::create_dir_all(&member_dir).unwrap();
        let workspace_path = root.join("Cargo.toml");
        let member_path = member_dir.join("Cargo.toml");
        fs::write(
            &workspace_path,
            r#"
[workspace.dependencies]
example = "=1.2.2"
"#,
        )
        .unwrap();
        fs::write(
            &member_path,
            r#"
[dependencies]
example = { workspace = true }
missing.workspace = true
"#,
        )
        .unwrap();
        let workspace = read_manifest(&workspace_path).unwrap();

        let mismatch = verify_dependency(
            &workspace,
            root,
            &member_path,
            "example",
            "=1.2.3",
            None,
            DIRECT_DEPENDENCIES,
        )
        .unwrap_err();
        assert!(mismatch.contains("Some(\"=1.2.2\")"), "{mismatch}");
        let missing = verify_dependency(
            &workspace,
            root,
            &member_path,
            "missing",
            "=1.2.3",
            None,
            DIRECT_DEPENDENCIES,
        )
        .unwrap_err();
        assert!(
            missing.contains("workspace does not define it"),
            "{missing}"
        );
    }
}

//! Offline installer for source-owned iced components.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const CONFIG_FILE: &str = "ducktape-ui.json";
const MANAGED_START: &str = "// ducktape-ui:managed:start";
const MANAGED_END: &str = "// ducktape-ui:managed:end";
const HELP: &str = "ducktape-ui — copy editable iced components into your project\n\n\
Usage:\n  ducktape-ui init [--ui <path>]\n  ducktape-ui add <component>... [--dry-run] [--overwrite]\n  ducktape-ui list\n  ducktape-ui view <component>\n  ducktape-ui diff <component>\n";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Config {
    ui: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: PathBuf::from("src/ui"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Registry {
    name: String,
    version: u32,
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    description: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default, rename = "cargoDependencies")]
    cargo_dependencies: BTreeMap<String, String>,
    files: Vec<RegistryFile>,
}

#[derive(Debug, Deserialize)]
struct RegistryFile {
    source: String,
    target: String,
}

/// Executes a CLI command relative to `root` and returns printable output.
pub fn execute<I, S>(args: I, root: &Path) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args = args
        .into_iter()
        .map(|arg| {
            arg.into()
                .into_string()
                .map_err(|_| "arguments must be UTF-8".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    match args.first().map(String::as_str) {
        None | Some("help" | "--help" | "-h") => Ok(HELP.into()),
        Some("init") => init(root, &args[1..]),
        Some("add") => add(root, &args[1..]),
        Some("list") if args.len() == 1 => list(),
        Some("view") if args.len() == 2 => view(&args[1]),
        Some("diff") if args.len() == 2 => diff(root, &args[1]),
        Some(command) => Err(format!("unknown or invalid command `{command}`\n\n{HELP}")),
    }
}

fn registry() -> Result<Registry, String> {
    let registry: Registry = serde_json::from_str(include_str!("../registry/registry.json"))
        .map_err(|error| format!("invalid embedded registry: {error}"))?;
    if registry.version != 1 {
        return Err(format!("unsupported registry version {}", registry.version));
    }
    Ok(registry)
}

fn init(root: &Path, args: &[String]) -> Result<String, String> {
    require_cargo_project(root)?;

    let mut config = Config::default();
    match args {
        [] => {}
        [flag, path] if flag == "--ui" => config.ui = PathBuf::from(path),
        _ => return Err("usage: ducktape-ui init [--ui <path>]".into()),
    }
    validate_relative(&config.ui)?;
    reject_symlink_path(root, &root.join(&config.ui))?;

    let path = root.join(CONFIG_FILE);
    if path.exists() {
        let existing = load_config(root)?;
        return Ok(format!(
            "already initialized: {} ({})\n",
            path.display(),
            existing.ui.display()
        ));
    }

    let encoded = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    fs::create_dir_all(root.join(&config.ui)).map_err(io_error)?;
    fs::write(&path, format!("{encoded}\n")).map_err(io_error)?;

    Ok(format!(
        "initialized {}\ncomponents will be copied to {}\n",
        path.display(),
        config.ui.display()
    ))
}

fn add(root: &Path, args: &[String]) -> Result<String, String> {
    require_cargo_project(root)?;
    let config = load_config(root)?;
    let mut dry_run = false;
    let mut overwrite = false;
    let mut requested = Vec::new();

    for arg in args {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--overwrite" => overwrite = true,
            value if value.starts_with('-') => return Err(format!("unknown flag `{value}`")),
            value => requested.push(value.to_string()),
        }
    }
    if requested.is_empty() {
        return Err("usage: ducktape-ui add <component>... [--dry-run] [--overwrite]".into());
    }

    let registry = registry()?;
    let order = resolve(&registry, &requested)?;
    ensure_cargo_dependencies(root, &registry, &order, dry_run)?;

    let ui_dir = root.join(&config.ui);
    let mod_path = ui_dir.join("mod.rs");
    let mut output = String::new();
    let mut modules = BTreeSet::new();

    for name in &order {
        let item = item(&registry, name)?;
        for file in &item.files {
            let target = safe_join(&ui_dir, &file.target)?;
            reject_symlink_path(root, &target)?;
            let source = template(&file.source)
                .ok_or_else(|| format!("missing embedded template `{}`", file.source))?;
            if let Some(parent) = target.parent()
                && !dry_run
            {
                fs::create_dir_all(parent).map_err(io_error)?;
            }

            if target.extension().and_then(|value| value.to_str()) == Some("rs")
                && target.file_stem().and_then(|value| value.to_str()) != Some("mod")
            {
                modules.insert(
                    target
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .ok_or_else(|| format!("invalid Rust filename `{}`", target.display()))?
                        .to_string(),
                );
            }

            match fs::read_to_string(&target) {
                Ok(existing) if existing == source => {
                    output.push_str(&format!("unchanged {}\n", target.display()));
                }
                Ok(_) if !overwrite => {
                    output.push_str(&format!("preserved {}\n", target.display()));
                }
                Ok(_) if dry_run => {
                    output.push_str(&format!("would overwrite {}\n", target.display()));
                }
                Ok(_) => {
                    fs::write(&target, source).map_err(io_error)?;
                    output.push_str(&format!("updated {}\n", target.display()));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound && dry_run => {
                    output.push_str(&format!("would add {}\n", target.display()));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::write(&target, source).map_err(io_error)?;
                    output.push_str(&format!("added {}\n", target.display()));
                }
                Err(error) => return Err(io_error(error)),
            }
        }
    }

    reject_symlink_path(root, &mod_path)?;
    let existing_mod = fs::read_to_string(&mod_path).unwrap_or_default();
    let patched = patch_modules(&existing_mod, &modules);
    if patched != existing_mod {
        if dry_run {
            output.push_str(&format!("would update {}\n", mod_path.display()));
        } else {
            fs::create_dir_all(&ui_dir).map_err(io_error)?;
            fs::write(&mod_path, patched).map_err(io_error)?;
            output.push_str(&format!("updated {}\n", mod_path.display()));
        }
    }

    Ok(output)
}

fn list() -> Result<String, String> {
    let registry = registry()?;
    let mut output = format!("{} registry v{}\n", registry.name, registry.version);
    for item in registry.items {
        if item.kind == "component" {
            output.push_str(&format!("{:<12} {}\n", item.name, item.description));
        }
    }
    Ok(output)
}

fn view(name: &str) -> Result<String, String> {
    let registry = registry()?;
    let item = item(&registry, name)?;
    let mut output = format!(
        "{} ({})\n{}\ndependencies: {}\n",
        item.name,
        item.kind,
        item.description,
        if item.dependencies.is_empty() {
            "none".into()
        } else {
            item.dependencies.join(", ")
        }
    );
    for file in &item.files {
        output.push_str(&format!("\n--- {} ---\n", file.target));
        output.push_str(
            template(&file.source)
                .ok_or_else(|| format!("missing embedded template `{}`", file.source))?,
        );
    }
    Ok(output)
}

fn diff(root: &Path, name: &str) -> Result<String, String> {
    let config = load_config(root)?;
    let registry = registry()?;
    let order = resolve(&registry, &[name.to_string()])?;
    let ui_dir = root.join(config.ui);
    let mut output = String::new();

    for name in order {
        let item = item(&registry, &name)?;
        for file in &item.files {
            let target = safe_join(&ui_dir, &file.target)?;
            reject_symlink_path(root, &target)?;
            let incoming = template(&file.source)
                .ok_or_else(|| format!("missing embedded template `{}`", file.source))?;
            let local = fs::read_to_string(&target).unwrap_or_default();
            output.push_str(&simple_diff(&target, &local, incoming));
        }
    }
    Ok(output)
}

fn resolve(registry: &Registry, requested: &[String]) -> Result<Vec<String>, String> {
    fn visit(
        registry: &Registry,
        name: &str,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        if visited.contains(name) {
            return Ok(());
        }
        if !visiting.insert(name.to_string()) {
            return Err(format!("circular component dependency involving `{name}`"));
        }
        let current = item(registry, name)?;
        for dependency in &current.dependencies {
            visit(registry, dependency, visiting, visited, order)?;
        }
        visiting.remove(name);
        visited.insert(name.to_string());
        order.push(name.to_string());
        Ok(())
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for name in requested {
        visit(registry, name, &mut visiting, &mut visited, &mut order)?;
    }
    Ok(order)
}

fn ensure_cargo_dependencies(
    root: &Path,
    registry: &Registry,
    order: &[String],
    dry_run: bool,
) -> Result<(), String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).map_err(io_error)?;
    let mut dependencies = BTreeMap::new();
    for name in order {
        dependencies.extend(item(registry, name)?.cargo_dependencies.clone());
    }

    for (name, version) in dependencies {
        if has_dependency(&manifest, &name) || dry_run {
            continue;
        }
        let status = Command::new("cargo")
            .args(["add", &format!("{name}@{version}")])
            .current_dir(root)
            .status()
            .map_err(|error| format!("failed to run cargo add: {error}"))?;
        if !status.success() {
            return Err(format!("cargo add {name}@{version} failed"));
        }
    }
    Ok(())
}

fn has_dependency(manifest: &str, name: &str) -> bool {
    let mut dependencies = false;
    manifest.lines().any(|line| {
        let line = line.trim_start();
        if line.starts_with('[') {
            dependencies = line == "[dependencies]"
                || (line.ends_with(".dependencies]")
                    && !line.contains("dev-dependencies")
                    && !line.contains("build-dependencies")
                    && !line.contains("workspace.dependencies"));
            return false;
        }
        if !dependencies || line.starts_with('#') {
            return false;
        }
        line.strip_prefix(name)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    })
}

fn item<'a>(registry: &'a Registry, name: &str) -> Result<&'a Item, String> {
    registry
        .items
        .iter()
        .find(|item| item.name == name)
        .ok_or_else(|| format!("unknown component `{name}`; run `ducktape-ui list`"))
}

fn load_config(root: &Path) -> Result<Config, String> {
    let path = root.join(CONFIG_FILE);
    let config: Config = serde_json::from_str(&fs::read_to_string(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "{} is missing; run `ducktape-ui init` first",
                path.display()
            )
        } else {
            io_error(error)
        }
    })?)
    .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    validate_relative(&config.ui)?;
    Ok(config)
}

fn require_cargo_project(root: &Path) -> Result<(), String> {
    if root.join("Cargo.toml").is_file() {
        Ok(())
    } else {
        Err(format!("{} is not a Cargo project", root.display()))
    }
}

fn validate_relative(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!(
            "path must be a non-empty relative path: {}",
            path.display()
        ));
    }
    Ok(())
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    validate_relative(path)?;
    Ok(root.join(path))
}

fn reject_symlink_path(root: &Path, path: &Path) -> Result<(), String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("path escapes project: {}", path.display()))?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "refusing to follow symlink inside component path: {}",
                    current.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(())
}

fn patch_modules(existing: &str, added: &BTreeSet<String>) -> String {
    let (outside, managed) = split_managed(existing);
    let outside_modules = module_names(&outside);
    let mut modules = module_names(&managed);
    modules.extend(added.iter().cloned());
    modules.retain(|module| !outside_modules.contains(module));

    let mut output = outside.trim_end().to_string();
    if !output.is_empty() {
        output.push_str("\n\n");
    }
    output.push_str(MANAGED_START);
    output.push('\n');
    for module in modules {
        output.push_str(&format!("pub mod {module};\n"));
    }
    output.push_str(MANAGED_END);
    output.push('\n');
    output
}

fn split_managed(existing: &str) -> (String, String) {
    match (existing.find(MANAGED_START), existing.find(MANAGED_END)) {
        (Some(start), Some(end)) if start < end => {
            let managed_start = start + MANAGED_START.len();
            let outside = format!(
                "{}{}",
                &existing[..start],
                &existing[end + MANAGED_END.len()..]
            );
            (outside, existing[managed_start..end].to_string())
        }
        _ => (existing.to_string(), String::new()),
    }
}

fn module_names(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub mod ")
                .and_then(|rest| rest.strip_suffix(';'))
                .filter(|name| {
                    name.chars()
                        .all(|character| character == '_' || character.is_ascii_alphanumeric())
                })
                .map(str::to_string)
        })
        .collect()
}

fn simple_diff(path: &Path, local: &str, incoming: &str) -> String {
    if local == incoming {
        return format!("{} is up to date\n", path.display());
    }

    let local = local.lines().collect::<Vec<_>>();
    let incoming = incoming.lines().collect::<Vec<_>>();
    let prefix = local
        .iter()
        .zip(&incoming)
        .take_while(|(left, right)| left == right)
        .count();
    let suffix = local[prefix..]
        .iter()
        .rev()
        .zip(incoming[prefix..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();

    let mut output = format!("--- {}\n+++ registry\n", path.display());
    for line in &local[prefix..local.len().saturating_sub(suffix)] {
        output.push_str(&format!("-{line}\n"));
    }
    for line in &incoming[prefix..incoming.len().saturating_sub(suffix)] {
        output.push_str(&format!("+{line}\n"));
    }
    output
}

fn template(path: &str) -> Option<&'static str> {
    match path {
        "theme.rs" => Some(include_str!("ui/theme.rs")),
        "button.rs" => Some(include_str!("ui/button.rs")),
        "input.rs" => Some(include_str!("ui/input.rs")),
        "card.rs" => Some(include_str!("ui/card.rs")),
        "field.rs" => Some(include_str!("ui/field.rs")),
        "badge.rs" => Some(include_str!("ui/badge.rs")),
        "separator.rs" => Some(include_str!("ui/separator.rs")),
        "segmented_control.rs" => Some(include_str!("ui/segmented_control.rs")),
        "surface.rs" => Some(include_str!("ui/surface.rs")),
        _ => None,
    }
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_dependencies_once_and_in_order() {
        let registry = registry().unwrap();
        let order = resolve(&registry, &["button".into(), "card".into()]).unwrap();
        assert_eq!(order, ["theme", "button", "surface", "card"]);
    }

    #[test]
    fn patch_preserves_user_modules_and_is_idempotent() {
        let existing = "pub mod custom;\n\n// ducktape-ui:managed:start\npub mod theme;\n// ducktape-ui:managed:end\n";
        let added = BTreeSet::from(["button".into(), "custom".into()]);
        let once = patch_modules(existing, &added);
        let twice = patch_modules(&once, &added);
        assert_eq!(once, twice);
        assert_eq!(once.matches("pub mod custom;").count(), 1);
        assert!(once.contains("pub mod button;"));
        assert!(once.contains("pub mod theme;"));
    }

    #[test]
    fn rejects_paths_that_escape_the_project() {
        assert!(validate_relative(Path::new("src/ui")).is_ok());
        assert!(validate_relative(Path::new("../elsewhere")).is_err());
        assert!(validate_relative(Path::new("/tmp/ui")).is_err());
    }

    #[test]
    fn dependency_detection_ignores_dev_and_workspace_sections() {
        assert!(!has_dependency(
            "[dev-dependencies]\niced = \"=0.14.0\"\n",
            "iced"
        ));
        assert!(!has_dependency(
            "[workspace.dependencies]\niced = \"=0.14.0\"\n",
            "iced"
        ));
        assert!(has_dependency(
            "[dependencies]\niced = { version = \"=0.14.0\" }\n",
            "iced"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn refuses_component_paths_through_symlinks() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!("ducktape-ui-symlink-{}", std::process::id()));
        let outside = root.with_extension("outside");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("src/ui")).unwrap();

        assert!(reject_symlink_path(&root, &root.join("src/ui/button.rs")).is_err());
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }
}

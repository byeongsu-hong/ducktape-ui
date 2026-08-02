use super::process::stop_requested;
use crate::{cargo_config, ignored_dir};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub(super) const BUILD_FINGERPRINT_ENV: &str = "ICE_DEV_BUILD_FINGERPRINT";

#[cfg(test)]
std::thread_local! {
    static FILE_STAMP_ATTEMPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FileStamp {
    Missing,
    Unreadable,
    Content(u64),
}

pub(super) type SourceStamp = Vec<(PathBuf, FileStamp)>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SettledDevSnapshot {
    pub(super) stamps: (SourceStamp, SourceStamp),
    pub(super) validated_sources: Vec<ui_lang_core::ValidatedSource>,
}

struct FileSnapshot {
    stamp: FileStamp,
    contents: Option<Vec<u8>>,
}

impl FileSnapshot {
    const fn missing() -> Self {
        Self {
            stamp: FileStamp::Missing,
            contents: None,
        }
    }

    const fn unreadable() -> Self {
        Self {
            stamp: FileStamp::Unreadable,
            contents: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CargoInputGraph {
    pub(super) package_roots: Vec<PathBuf>,
    pub(super) package_roots_by_id: std::collections::BTreeMap<String, PathBuf>,
    pub(super) participating_package_ids: Vec<String>,
    pub(super) include_ice_in_package_roots: bool,
    pub(super) workspace_files: Vec<PathBuf>,
    pub(super) excluded_roots: Vec<PathBuf>,
    pub(super) discovered_inputs: Vec<PathBuf>,
}

impl CargoInputGraph {
    #[cfg(test)]
    pub(super) fn workspace(root: &Path) -> Self {
        Self {
            package_roots: vec![root.to_owned()],
            package_roots_by_id: std::collections::BTreeMap::new(),
            participating_package_ids: Vec::new(),
            include_ice_in_package_roots: false,
            workspace_files: vec![
                root.join("Cargo.toml"),
                root.join("Cargo.lock"),
                root.join("src/main.rs"),
            ],
            excluded_roots: vec![root.join("target")],
            discovered_inputs: Vec::new(),
        }
    }

    pub(super) fn inherit_discovered_inputs(&mut self, previous: &Self) {
        self.discovered_inputs
            .clone_from(&previous.discovered_inputs);
        self.participating_package_ids
            .clone_from(&previous.participating_package_ids);
        if !self.participating_package_ids.is_empty() {
            self.include_ice_in_package_roots = false;
            self.package_roots = self
                .participating_package_ids
                .iter()
                .filter_map(|id| self.package_roots_by_id.get(id).cloned())
                .collect();
        }
    }

    pub(super) fn install_discovered_inputs(&mut self, inputs: Vec<PathBuf>) {
        self.discovered_inputs = inputs
            .into_iter()
            .map(|path| normalize_watch_path(&path))
            .filter(|path| {
                !self
                    .excluded_roots
                    .iter()
                    .any(|excluded| path.starts_with(excluded))
            })
            .collect();
        self.discovered_inputs.sort();
        self.discovered_inputs.dedup();
    }

    pub(super) fn install_build_output(&mut self, build: &CargoBuildOutput) {
        self.install_discovered_inputs(build.discovered_inputs.clone());
        self.participating_package_ids
            .clone_from(&build.participating_package_ids);
        self.include_ice_in_package_roots = false;
        self.package_roots = self
            .participating_package_ids
            .iter()
            .filter_map(|id| self.package_roots_by_id.get(id).cloned())
            .collect();
        self.package_roots.sort();
        self.package_roots.dedup();
    }
}

pub(super) struct CargoBuildOutput {
    pub(super) executable: PathBuf,
    pub(super) discovered_inputs: Vec<PathBuf>,
    pub(super) participating_package_ids: Vec<String>,
}

struct BuildScriptOutput {
    package_id: String,
    path: PathBuf,
}

#[cfg(test)]
pub(super) fn dev_stamps(
    root: &Path,
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
) -> (SourceStamp, SourceStamp) {
    dev_stamps_with_cargo_inputs(
        root,
        dependencies,
        asset_dependencies,
        &CargoInputGraph::workspace(root),
    )
}

pub(super) fn dev_stamps_with_cargo_inputs(
    _root: &Path,
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
) -> (SourceStamp, SourceStamp) {
    (
        ice_source_stamp(dependencies),
        build_input_stamp(cargo_inputs, asset_dependencies),
    )
}

#[cfg(test)]
pub(super) fn settled_dev_stamps(
    root: &Path,
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
) -> Option<(SourceStamp, SourceStamp)> {
    settled_dev_stamps_after_with_cargo_inputs(
        root,
        dependencies,
        asset_dependencies,
        &CargoInputGraph::workspace(root),
        current_ice,
        current_build,
        || thread::sleep(Duration::from_millis(50)),
    )
}

pub(super) fn settled_dev_stamps_with_cargo_inputs(
    root: &Path,
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
) -> Option<(SourceStamp, SourceStamp)> {
    settled_dev_stamps_after_with_cargo_inputs(
        root,
        dependencies,
        asset_dependencies,
        cargo_inputs,
        current_ice,
        current_build,
        || thread::sleep(Duration::from_millis(50)),
    )
}

#[cfg(test)]
pub(super) fn settled_dev_stamps_for_paths_with_cargo_inputs(
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
    changed_paths: &[PathBuf],
) -> Option<(SourceStamp, SourceStamp)> {
    settled_dev_snapshot_for_paths_after_with_cargo_inputs(
        dependencies,
        asset_dependencies,
        cargo_inputs,
        current_ice,
        current_build,
        changed_paths,
        || thread::sleep(Duration::from_millis(50)),
    )
    .map(|snapshot| snapshot.stamps)
}

pub(super) fn settled_dev_snapshot_for_paths_with_cargo_inputs(
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
    changed_paths: &[PathBuf],
) -> Option<SettledDevSnapshot> {
    settled_dev_snapshot_for_paths_after_with_cargo_inputs(
        dependencies,
        asset_dependencies,
        cargo_inputs,
        current_ice,
        current_build,
        changed_paths,
        || thread::sleep(Duration::from_millis(50)),
    )
}

#[cfg(test)]
pub(super) fn settled_dev_stamps_after(
    root: &Path,
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
    after_first_read: impl FnOnce(),
) -> Option<(SourceStamp, SourceStamp)> {
    settled_dev_stamps_after_with_cargo_inputs(
        root,
        dependencies,
        asset_dependencies,
        &CargoInputGraph::workspace(root),
        current_ice,
        current_build,
        after_first_read,
    )
}

fn settled_dev_stamps_after_with_cargo_inputs(
    _root: &Path,
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
    after_first_read: impl FnOnce(),
) -> Option<(SourceStamp, SourceStamp)> {
    settled_dev_snapshot_for_paths_after_with_cargo_inputs(
        dependencies,
        asset_dependencies,
        cargo_inputs,
        current_ice,
        current_build,
        &[],
        after_first_read,
    )
    .map(|snapshot| snapshot.stamps)
}

fn settled_dev_snapshot_for_paths_after_with_cargo_inputs(
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
    changed_paths: &[PathBuf],
    after_first_read: impl FnOnce(),
) -> Option<SettledDevSnapshot> {
    let read = || {
        if changed_paths.is_empty() {
            (
                ice_source_stamp(dependencies),
                build_input_stamp(cargo_inputs, asset_dependencies),
            )
        } else {
            dev_stamps_reusing(
                dependencies,
                asset_dependencies,
                cargo_inputs,
                current_ice,
                current_build,
                changed_paths,
            )
        }
    };
    let first = read();
    if first.0 == *current_ice && first.1 == *current_build {
        return None;
    }
    after_first_read();
    let (second, validated_sources) = if changed_paths.is_empty() {
        (read(), Vec::new())
    } else {
        dev_snapshot_reusing(
            dependencies,
            asset_dependencies,
            cargo_inputs,
            current_ice,
            current_build,
            changed_paths,
        )
    };
    (first == second).then_some(SettledDevSnapshot {
        stamps: second,
        validated_sources,
    })
}

fn dev_snapshot_reusing(
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
    changed_paths: &[PathBuf],
) -> (
    (SourceStamp, SourceStamp),
    Vec<ui_lang_core::ValidatedSource>,
) {
    let build = if can_reuse_build_inventory(current_ice, current_build, changed_paths) {
        stamp_current_files_reusing(current_build, changed_paths)
    } else {
        stamp_files_reusing(
            &build_input_files(cargo_inputs, asset_dependencies),
            current_build,
            changed_paths,
        )
    };
    let (ice, validated_sources) = stamp_sources_reusing(dependencies, current_ice, changed_paths);
    ((ice, build), validated_sources)
}

fn dev_stamps_reusing(
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
    changed_paths: &[PathBuf],
) -> (SourceStamp, SourceStamp) {
    let build = if can_reuse_build_inventory(current_ice, current_build, changed_paths) {
        stamp_current_files_reusing(current_build, changed_paths)
    } else {
        stamp_files_reusing(
            &build_input_files(cargo_inputs, asset_dependencies),
            current_build,
            changed_paths,
        )
    };
    (
        stamp_files_reusing(dependencies, current_ice, changed_paths),
        build,
    )
}

fn can_reuse_build_inventory(
    current_ice: &SourceStamp,
    current_build: &SourceStamp,
    changed_paths: &[PathBuf],
) -> bool {
    changed_paths.iter().all(|path| {
        let build_input = current_build
            .binary_search_by(|(candidate, _)| candidate.cmp(path))
            .is_ok();
        if build_input {
            return path.is_file();
        }
        !path.is_dir()
            && current_ice
                .binary_search_by(|(candidate, _)| candidate.cmp(path))
                .is_ok()
    })
}

fn stamp_files(files: &[PathBuf]) -> SourceStamp {
    let mut files = files.to_vec();
    files.sort();
    files.dedup();
    files
        .into_iter()
        .map(|path| (path.clone(), stamp_file(&path)))
        .collect()
}

fn stamp_files_reusing(
    files: &[PathBuf],
    current: &SourceStamp,
    changed_paths: &[PathBuf],
) -> SourceStamp {
    let mut files = files.to_vec();
    files.sort();
    files.dedup();
    files
        .into_iter()
        .map(|path| {
            let previous = current
                .binary_search_by(|(candidate, _)| candidate.cmp(&path))
                .ok()
                .map(|index| current[index].1);
            let affected = changed_paths
                .iter()
                .any(|changed| path == *changed || path.starts_with(changed));
            let stamp = if affected {
                stamp_file(&path)
            } else {
                previous.unwrap_or_else(|| stamp_file(&path))
            };
            (path, stamp)
        })
        .collect()
}

fn stamp_sources_reusing(
    files: &[PathBuf],
    current: &SourceStamp,
    changed_paths: &[PathBuf],
) -> (SourceStamp, Vec<ui_lang_core::ValidatedSource>) {
    let mut files = files.to_vec();
    files.sort();
    files.dedup();
    let mut sources = Vec::new();
    let stamps = files
        .into_iter()
        .map(|path| {
            let previous = current
                .binary_search_by(|(candidate, _)| candidate.cmp(&path))
                .ok()
                .map(|index| current[index].1);
            let affected = changed_paths
                .iter()
                .any(|changed| path == *changed || path.starts_with(changed));
            let stamp = if affected {
                let FileSnapshot { stamp, contents } = snapshot_file(&path);
                match contents {
                    Some(contents) => {
                        sources.push(ui_lang_core::ValidatedSource::new(path.clone(), contents))
                    }
                    None if stamp == FileStamp::Missing => {
                        sources.push(ui_lang_core::ValidatedSource::missing(path.clone()));
                    }
                    None => {}
                }
                stamp
            } else {
                previous.unwrap_or_else(|| stamp_file(&path))
            };
            (path, stamp)
        })
        .collect();
    (stamps, sources)
}

fn stamp_current_files_reusing(current: &SourceStamp, changed_paths: &[PathBuf]) -> SourceStamp {
    current
        .iter()
        .map(|(path, previous)| {
            let affected = changed_paths
                .iter()
                .any(|changed| path == changed || path.starts_with(changed));
            (
                path.clone(),
                if affected {
                    stamp_file(path)
                } else {
                    *previous
                },
            )
        })
        .collect()
}

fn stamp_file(path: &Path) -> FileStamp {
    snapshot_file(path).stamp
}

fn snapshot_file(path: &Path) -> FileSnapshot {
    #[cfg(test)]
    FILE_STAMP_ATTEMPTS.with(|attempts| attempts.set(attempts.get() + 1));

    let identity = match path.canonicalize() {
        Ok(identity) => identity,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return FileSnapshot::missing();
        }
        Err(_) => return FileSnapshot::unreadable(),
    };
    let mut opened = match same_file::Handle::from_path(path) {
        Ok(opened) => opened,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return FileSnapshot::missing();
        }
        Err(_) => return FileSnapshot::unreadable(),
    };
    let identity_handle = match same_file::Handle::from_path(&identity) {
        Ok(identity) => identity,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return FileSnapshot::missing();
        }
        Err(_) => return FileSnapshot::unreadable(),
    };
    if opened != identity_handle {
        return FileSnapshot::unreadable();
    }
    let mut bytes = Vec::new();
    if opened.as_file_mut().read_to_end(&mut bytes).is_err() {
        return FileSnapshot::unreadable();
    }
    if path.canonicalize().ok().as_ref() != Some(&identity) {
        return FileSnapshot::unreadable();
    }
    FileSnapshot {
        stamp: FileStamp::Content(stable_file_hash(&identity, &bytes)),
        contents: Some(bytes),
    }
}

#[cfg(test)]
pub(super) fn reset_file_stamp_attempts() {
    FILE_STAMP_ATTEMPTS.with(|attempts| attempts.set(0));
}

#[cfg(test)]
pub(super) fn file_stamp_attempts() -> usize {
    FILE_STAMP_ATTEMPTS.with(std::cell::Cell::get)
}

pub(super) fn first_unreadable_input(stamps: &(SourceStamp, SourceStamp)) -> Option<&Path> {
    stamps
        .0
        .iter()
        .chain(&stamps.1)
        .find_map(|(path, stamp)| matches!(stamp, FileStamp::Unreadable).then_some(path.as_path()))
}

pub(super) fn stamps_match_on_common_paths(left: &SourceStamp, right: &SourceStamp) -> bool {
    left.iter().all(|(path, hash)| {
        right
            .binary_search_by(|(candidate, _)| candidate.cmp(path))
            .map_or(true, |index| right[index].1 == *hash)
    })
}

pub(super) fn stamp_contains_snapshot(observed: &SourceStamp, snapshot: &SourceStamp) -> bool {
    snapshot.iter().all(|(path, hash)| {
        observed
            .binary_search_by(|(candidate, _)| candidate.cmp(path))
            .is_ok_and(|index| observed[index].1 == *hash)
    })
}

pub(super) fn build_observation_reuses_snapshot(
    initial: &(SourceStamp, SourceStamp),
    observed: &(SourceStamp, SourceStamp),
) -> bool {
    observed.0 == initial.0 && stamp_contains_snapshot(&initial.1, &observed.1)
}

fn ice_source_stamp(dependencies: &[PathBuf]) -> SourceStamp {
    stamp_files(dependencies)
}

fn build_input_stamp(
    cargo_inputs: &CargoInputGraph,
    asset_dependencies: &[PathBuf],
) -> SourceStamp {
    stamp_files(&build_input_files(cargo_inputs, asset_dependencies))
}

pub(super) fn build_input_files(
    cargo_inputs: &CargoInputGraph,
    asset_dependencies: &[PathBuf],
) -> Vec<PathBuf> {
    let mut files = asset_dependencies.to_vec();
    files.extend(cargo_inputs.workspace_files.iter().cloned());
    let mut visited = std::collections::HashSet::new();
    for root in &cargo_inputs.package_roots {
        visit_cargo_inputs(
            root,
            &mut files,
            &mut visited,
            &cargo_inputs.excluded_roots,
            cargo_inputs.include_ice_in_package_roots,
        );
    }
    for input in &cargo_inputs.discovered_inputs {
        visit_declared_input(
            input,
            &mut files,
            &mut visited,
            &cargo_inputs.excluded_roots,
        );
    }
    files
}

fn visit_declared_input(
    path: &Path,
    output: &mut Vec<PathBuf>,
    visited: &mut std::collections::HashSet<PathBuf>,
    excluded_roots: &[PathBuf],
) {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            visit_cargo_inputs(path, output, visited, excluded_roots, true);
        }
        _ => output.push(path.to_owned()),
    }
}

fn visit_cargo_inputs(
    path: &Path,
    output: &mut Vec<PathBuf>,
    visited: &mut std::collections::HashSet<PathBuf>,
    excluded_roots: &[PathBuf],
    include_ice: bool,
) {
    let identity = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    if excluded_roots
        .iter()
        .any(|excluded| identity.starts_with(excluded))
    {
        return;
    }
    if !visited.insert(identity) {
        return;
    }
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            output.push(path.to_owned());
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                output.push(path.to_owned());
                continue;
            }
        };
        let entry_path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) if file_type.is_symlink() => match fs::metadata(&entry_path) {
                Ok(metadata) => metadata.file_type(),
                Err(_) => {
                    output.push(entry_path);
                    continue;
                }
            },
            Ok(file_type) => file_type,
            Err(_) => {
                output.push(entry_path);
                continue;
            }
        };
        if file_type.is_dir() {
            if !ignored_dir(&entry_path)
                && entry_path.file_name().and_then(|name| name.to_str()) != Some("vendor")
            {
                visit_cargo_inputs(&entry_path, output, visited, excluded_roots, include_ice);
            }
        } else if file_type.is_file() && (include_ice || !is_ice_input(&entry_path)) {
            output.push(entry_path);
        }
    }
}

fn is_ice_input(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("ice")
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn stable_file_hash(identity: &Path, bytes: &[u8]) -> u64 {
    let identity = identity.as_os_str().as_encoded_bytes();
    identity
        .len()
        .to_le_bytes()
        .into_iter()
        .chain(identity.iter().copied())
        .chain(bytes.iter().copied())
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        })
}

pub(super) fn cargo_input_graph(
    root: &Path,
    cargo_args: &[String],
) -> Result<CargoInputGraph, String> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .arg("metadata")
        .args(["--format-version", "1"])
        .args(cargo_metadata_args(cargo_args))
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cargo metadata failed{}{}",
            if diagnostic.trim().is_empty() {
                ""
            } else {
                ": "
            },
            diagnostic.trim()
        ));
    }
    let metadata = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .map_err(|error| format!("invalid cargo metadata output: {error}"))?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata output has no package list".to_owned())?;
    let workspace_root = metadata["workspace_root"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| "cargo metadata output has no workspace root".to_owned())?;
    let vendor_root = workspace_root.join("vendor");
    let vendor_root = vendor_root
        .canonicalize()
        .unwrap_or_else(|_| normalize_watch_path(&vendor_root));
    let mut package_roots = Vec::new();
    let mut package_roots_by_id = std::collections::BTreeMap::new();
    let mut workspace_files = Vec::new();
    for package in packages
        .iter()
        .filter(|package| package["source"].is_null())
    {
        let manifest = package["manifest_path"]
            .as_str()
            .ok_or_else(|| "local cargo package has no manifest path".to_owned())?;
        let root = PathBuf::from(manifest)
            .parent()
            .map(Path::to_owned)
            .ok_or_else(|| format!("local cargo manifest has no parent: {manifest}"))?;
        let root = root
            .canonicalize()
            .map_err(|error| format!("cannot resolve local cargo package {manifest}: {error}"))?;
        if root.starts_with(&vendor_root) {
            continue;
        }
        package_roots.push(root.clone());
        let package_id = package["id"]
            .as_str()
            .ok_or_else(|| format!("local cargo package has no id: {manifest}"))?;
        package_roots_by_id.insert(package_id.to_owned(), root);
    }
    package_roots.sort();
    package_roots.dedup();

    workspace_files.extend(
        [
            "Cargo.toml",
            "Cargo.lock",
            ".cargo/config",
            ".cargo/config.toml",
            "rust-toolchain",
            "rust-toolchain.toml",
        ]
        .into_iter()
        .map(|path| workspace_root.join(path)),
    );
    workspace_files.extend(cargo_config::files(root, cargo_args));
    workspace_files.sort();
    workspace_files.dedup();

    let mut excluded_roots = std::iter::once(vendor_root)
        .chain(metadata["target_directory"].as_str().map(PathBuf::from))
        .chain(cargo_target_directories(root, cargo_args))
        .map(|path| {
            let path = if path.is_absolute() {
                path
            } else {
                root.join(path)
            };
            path.canonicalize()
                .unwrap_or_else(|_| normalize_watch_path(&path))
        })
        .collect::<Vec<_>>();
    excluded_roots.sort();
    excluded_roots.dedup();

    Ok(CargoInputGraph {
        package_roots,
        package_roots_by_id,
        participating_package_ids: Vec::new(),
        include_ice_in_package_roots: true,
        workspace_files,
        excluded_roots,
        discovered_inputs: Vec::new(),
    })
}

fn cargo_target_directories(root: &Path, cargo_args: &[String]) -> Vec<PathBuf> {
    let args = cargo_args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .collect::<Vec<_>>();
    let mut directories = Vec::new();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        if arg.as_str() == "--target-dir" {
            if let Some(path) = args.get(index + 1) {
                directories.push(root.join(path.as_str()));
                index += 1;
            }
        } else if let Some(path) = arg.strip_prefix("--target-dir=") {
            directories.push(root.join(path));
        }
        index += 1;
    }
    directories
}

pub(super) fn normalize_watch_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn cargo_metadata_args(cargo_args: &[String]) -> Vec<String> {
    let args = cargo_args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .collect::<Vec<_>>();
    let mut metadata = Vec::new();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--features" | "-F" | "--manifest-path" | "--config" | "-Z" => {
                metadata.push((*arg).clone());
                if let Some(value) = args.get(index + 1) {
                    metadata.push((**value).clone());
                    index += 1;
                }
            }
            "--target" => {
                metadata.push("--filter-platform".to_owned());
                if let Some(value) = args.get(index + 1) {
                    metadata.push((**value).clone());
                    index += 1;
                }
            }
            "--all-features" | "--no-default-features" | "--locked" | "--offline" | "--frozen" => {
                metadata.push((*arg).clone())
            }
            arg if arg.starts_with("--target=") => {
                metadata.push(format!(
                    "--filter-platform={}",
                    arg.trim_start_matches("--target=")
                ));
            }
            arg if arg.starts_with("--features=")
                || arg.starts_with("--manifest-path=")
                || arg.starts_with("--config=")
                || arg.starts_with("-F")
                || arg.starts_with("-Z") =>
            {
                metadata.push(arg.to_owned());
            }
            _ => {}
        }
        index += 1;
    }
    metadata
}

pub(super) fn source_stamp_fingerprint(stamp: &SourceStamp) -> String {
    let mut payload = Vec::new();
    for (path, stamp) in stamp {
        let path = path.as_os_str().as_encoded_bytes();
        payload.extend_from_slice(&(path.len() as u64).to_le_bytes());
        payload.extend_from_slice(path);
        match stamp {
            FileStamp::Missing => payload.push(0),
            FileStamp::Unreadable => payload.push(1),
            FileStamp::Content(hash) => {
                payload.push(2);
                payload.extend_from_slice(&hash.to_le_bytes());
            }
        }
    }
    format!("{:016x}", stable_hash(&payload))
}

pub(super) fn cargo_build(
    root: &Path,
    cargo_args: &[String],
    ice_stamp: &SourceStamp,
    build_stamp: &SourceStamp,
    cargo_inputs: &CargoInputGraph,
) -> Result<Option<CargoBuildOutput>, String> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    cargo_build_with_program(
        OsStr::new(&cargo),
        root,
        cargo_args,
        ice_stamp,
        build_stamp,
        cargo_inputs,
        stop_requested,
    )
}

pub(super) fn cargo_build_with_program(
    cargo: &OsStr,
    root: &Path,
    cargo_args: &[String],
    ice_stamp: &SourceStamp,
    build_stamp: &SourceStamp,
    cargo_inputs: &CargoInputGraph,
    should_stop: impl Fn() -> bool,
) -> Result<Option<CargoBuildOutput>, String> {
    let build_args = cargo_args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .collect::<Vec<_>>();
    let mut child = Command::new(cargo)
        .arg("build")
        .args(build_args)
        .arg("--message-format=json-diagnostic-rendered-ansi")
        .env(
            BUILD_FINGERPRINT_ENV,
            format!(
                "{}-{}",
                source_stamp_fingerprint(ice_stamp),
                source_stamp_fingerprint(build_stamp)
            ),
        )
        .current_dir(root)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let stdout = child.stdout.take().expect("piped cargo stdout");
    let (lines_tx, lines_rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if lines_tx
                .send(line.map_err(|error| error.to_string()))
                .is_err()
            {
                return;
            }
        }
    });
    let mut executables = Vec::new();
    let mut build_script_outputs = Vec::new();
    let mut participating_package_ids = std::collections::BTreeSet::new();
    let mut rustc_artifacts = std::collections::BTreeMap::<String, Vec<PathBuf>>::new();
    let mut source_maps = crate::GeneratedSourceMaps::new();
    loop {
        if should_stop() {
            let _ = child.kill();
            let _ = child.wait();
            drop(lines_rx);
            let _ = reader.join();
            return Err("ice dev: build interrupted".to_owned());
        }
        let line = match lines_rx.recv_timeout(Duration::from_millis(25)) {
            Ok(Ok(line)) => line,
            Ok(Err(error)) => {
                let _ = child.kill();
                let _ = child.wait();
                drop(lines_rx);
                let _ = reader.join();
                return Err(error);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            println!("{line}");
            continue;
        };
        if message["reason"] == "compiler-message" {
            let diagnostic = &message["message"];
            if let Some(rendered) =
                crate::remap_compiler_diagnostic_with_maps(diagnostic, &mut source_maps)
            {
                eprint!("{rendered}");
            } else if let Some(rendered) = diagnostic["rendered"].as_str() {
                eprint!("{rendered}");
            }
        } else if message["reason"] == "compiler-artifact" {
            if message["target"]["kind"].as_array().is_some_and(|kinds| {
                kinds
                    .iter()
                    .any(|kind| matches!(kind.as_str(), Some("bin" | "example")))
            }) && let Some(executable) = message["executable"].as_str()
            {
                executables.push(PathBuf::from(executable));
            }
            if let Some(package_id) = message["package_id"].as_str()
                && cargo_inputs.package_roots_by_id.contains_key(package_id)
            {
                participating_package_ids.insert(package_id.to_owned());
                rustc_artifacts
                    .entry(package_id.to_owned())
                    .or_default()
                    .extend(
                        message["filenames"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .map(PathBuf::from),
                    );
            }
        } else if message["reason"] == "build-script-executed"
            && let (Some(package_id), Some(out_dir)) =
                (message["package_id"].as_str(), message["out_dir"].as_str())
            && let Some(build_directory) = Path::new(out_dir).parent()
        {
            if cargo_inputs.package_roots_by_id.contains_key(package_id) {
                participating_package_ids.insert(package_id.to_owned());
            }
            build_script_outputs.push(BuildScriptOutput {
                package_id: package_id.to_owned(),
                path: build_directory.join("output"),
            });
        }
    }
    reader
        .join()
        .map_err(|_| "ice dev: Cargo output reader panicked".to_owned())?;
    let status = child.wait().map_err(|error| error.to_string())?;
    if !status.success() {
        return Ok(None);
    }
    executables.sort();
    executables.dedup();
    match executables.as_slice() {
        [executable] => {
            let discovery = discover_cargo_build_inputs(
                root,
                executable,
                &rustc_artifacts,
                &build_script_outputs,
                cargo_inputs,
            )?;
            Ok(Some(CargoBuildOutput {
                executable: executable.clone(),
                discovered_inputs: discovery,
                participating_package_ids: participating_package_ids.into_iter().collect(),
            }))
        }
        [] => Err("ice dev: cargo build produced no runnable binary".into()),
        _ => Err(format!(
            "ice dev: cargo build produced multiple binaries; select one with `--bin`: {}",
            executables
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn discover_cargo_build_inputs(
    root: &Path,
    executable: &Path,
    rustc_artifacts: &std::collections::BTreeMap<String, Vec<PathBuf>>,
    build_script_outputs: &[BuildScriptOutput],
    cargo_inputs: &CargoInputGraph,
) -> Result<Vec<PathBuf>, String> {
    let mut dep_info_paths = std::collections::BTreeSet::from([rustc_dep_info_path(executable)?]);
    for (package_id, artifacts) in rustc_artifacts {
        let mut found = false;
        for artifact in artifacts {
            if let Some(dep_info) = hashed_artifact_dep_info_path(artifact) {
                dep_info_paths.insert(dep_info);
                found = true;
            } else if artifact.is_file()
                && let Ok(dep_info) = rustc_dep_info_path(artifact)
            {
                dep_info_paths.insert(dep_info);
                found = true;
            }
        }
        if !found && cargo_inputs.package_roots_by_id.contains_key(package_id) {
            return Err(format!(
                "ice dev: Cargo reported local package {package_id} without identifiable rustc dep-info"
            ));
        }
    }
    let mut inputs = Vec::new();
    for dep_info_path in dep_info_paths {
        let dep_info = fs::read(&dep_info_path).map_err(|error| {
            format!(
                "ice dev: cannot read rustc dep-info {}: {error}",
                dep_info_path.display()
            )
        })?;
        inputs.extend(
            parse_dep_info(&dep_info)
                .into_iter()
                .map(|path| resolve_watch_path(root, &path)),
        );
    }
    for output in build_script_outputs {
        let Some(package_root) = cargo_inputs.package_roots_by_id.get(&output.package_id) else {
            continue;
        };
        let contents = fs::read_to_string(&output.path).map_err(|error| {
            format!(
                "ice dev: cannot read build-script output {}: {error}",
                output.path.display()
            )
        })?;
        inputs.extend(build_script_inputs(package_root, &contents));
    }
    inputs.retain(|path| {
        !cargo_inputs
            .excluded_roots
            .iter()
            .any(|excluded| path.starts_with(excluded))
    });
    inputs.sort();
    inputs.dedup();
    Ok(inputs)
}

fn hashed_artifact_dep_info_path(artifact: &Path) -> Option<PathBuf> {
    let stem = artifact.file_stem()?.to_str()?;
    let stem = stem.strip_prefix("lib").unwrap_or(stem);
    let (_, hash) = stem.rsplit_once('-')?;
    if hash.len() != 16 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let dep_info = artifact.parent()?.join(format!("{stem}.d"));
    dep_info.is_file().then_some(dep_info)
}

pub(super) fn rustc_dep_info_path(executable: &Path) -> Result<PathBuf, String> {
    let parent = executable.parent().ok_or_else(|| {
        format!(
            "ice dev: Cargo executable has no parent: {}",
            executable.display()
        )
    })?;
    let stem = executable.file_stem().ok_or_else(|| {
        format!(
            "ice dev: Cargo executable has no file stem: {}",
            executable.display()
        )
    })?;
    let mut prefixes = vec![stem.to_os_string()];
    let normalized = OsString::from(stem.to_string_lossy().replace('-', "_"));
    if normalized != prefixes[0] {
        prefixes.push(normalized);
    }
    for prefix in &mut prefixes {
        prefix.push("-");
    }
    let extension = executable.extension();
    let mut directories = vec![parent.to_owned(), parent.join("deps")];
    directories.sort();
    directories.dedup();
    let mut artifacts = Vec::new();
    for directory in directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path == executable || path.extension() != extension {
                continue;
            }
            let Some(file_stem) = path.file_stem() else {
                continue;
            };
            if !prefixes.iter().any(|prefix| {
                file_stem
                    .as_encoded_bytes()
                    .starts_with(prefix.as_encoded_bytes())
            }) {
                continue;
            }
            if same_file::is_same_file(executable, &path).unwrap_or(false) {
                artifacts.push(path);
            }
        }
    }
    artifacts.sort();
    artifacts.dedup();
    let artifact = match artifacts.as_slice() {
        [artifact] => artifact,
        [] => {
            return Err(format!(
                "ice dev: cannot identify the unique rustc artifact behind {}; refusing Cargo's aggregate dep-info",
                executable.display()
            ));
        }
        _ => {
            return Err(format!(
                "ice dev: multiple rustc artifacts share {}: {}",
                executable.display(),
                artifacts
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    };
    let dep_info = artifact.with_extension("d");
    if !dep_info.is_file() {
        return Err(format!(
            "ice dev: rustc artifact {} has no dep-info {}",
            artifact.display(),
            dep_info.display()
        ));
    }
    Ok(dep_info)
}

fn resolve_watch_path(base: &Path, path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    };
    normalize_watch_path(&path)
}

pub(super) fn build_script_rerun_path(line: &str) -> Option<&str> {
    line.strip_prefix("cargo::rerun-if-changed=")
        .or_else(|| line.strip_prefix("cargo:rerun-if-changed="))
        .filter(|path| !path.is_empty())
}

pub(super) fn build_script_inputs(package_root: &Path, contents: &str) -> Vec<PathBuf> {
    let mut inputs = contents
        .lines()
        .filter_map(build_script_rerun_path)
        .map(|path| resolve_watch_path(package_root, Path::new(path)))
        .collect::<Vec<_>>();
    inputs.sort();
    inputs.dedup();
    inputs
}

pub(super) fn parse_dep_info(contents: &[u8]) -> Vec<PathBuf> {
    let mut logical_lines = vec![Vec::new()];
    let mut index = 0;
    while index < contents.len() {
        if contents[index] == b'\\'
            && contents.get(index + 1) == Some(&b'\r')
            && contents.get(index + 2) == Some(&b'\n')
        {
            logical_lines.last_mut().unwrap().push(b' ');
            index += 3;
        } else if contents[index] == b'\\' && contents.get(index + 1) == Some(&b'\n') {
            logical_lines.last_mut().unwrap().push(b' ');
            index += 2;
        } else if contents[index] == b'\n' {
            logical_lines.push(Vec::new());
            index += 1;
        } else {
            logical_lines.last_mut().unwrap().push(contents[index]);
            index += 1;
        }
    }

    let mut paths = Vec::new();
    for line in logical_lines {
        let mut escaped = false;
        let delimiter = line.iter().enumerate().find_map(|(index, byte)| {
            if escaped {
                escaped = false;
                return None;
            }
            if *byte == b'\\' {
                escaped = true;
                return None;
            }
            (*byte == b':' && line.get(index + 1).is_none_or(u8::is_ascii_whitespace))
                .then_some(index)
        });
        let Some(delimiter) = delimiter else {
            continue;
        };
        let mut token = Vec::new();
        let mut escaped = false;
        for byte in &line[delimiter + 1..] {
            if escaped {
                token.push(*byte);
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if byte.is_ascii_whitespace() {
                if !token.is_empty() {
                    paths.push(native_path(std::mem::take(&mut token)));
                }
            } else {
                token.push(*byte);
            }
        }
        if escaped {
            token.push(b'\\');
        }
        if !token.is_empty() {
            paths.push(native_path(token));
        }
    }
    paths
}

fn native_path(bytes: Vec<u8>) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(OsString::from_vec(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(&bytes).into_owned())
    }
}

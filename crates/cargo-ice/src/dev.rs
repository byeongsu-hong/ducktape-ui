mod inputs;
mod process;
mod watcher;

#[cfg(test)]
use self::inputs::{
    BUILD_FINGERPRINT_ENV, CargoBuildOutput, CargoInputGraph, FileStamp, build_script_inputs,
    build_script_rerun_path, cargo_build_with_program, dev_stamps, file_stamp_attempts,
    parse_dep_info, reset_file_stamp_attempts, rustc_dep_info_path, settled_dev_stamps,
    settled_dev_stamps_after, settled_dev_stamps_for_paths_with_cargo_inputs,
    source_stamp_fingerprint, stamp_contains_snapshot,
};
use self::inputs::{
    build_observation_reuses_snapshot, cargo_build, cargo_input_graph,
    dev_stamps_with_cargo_inputs, first_unreadable_input, normalize_watch_path,
    settled_dev_snapshot_for_paths_with_cargo_inputs, settled_dev_stamps_with_cargo_inputs,
    stamps_match_on_common_paths,
};
use self::process::{
    ChildGuard, install_stop_handler, runtime_args, stage_executable, stop_requested,
};
#[cfg(test)]
use self::process::{READY_PATH_ENV, READY_TOKEN_ENV};
use self::watcher::{DevChange, DevWatcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

pub(crate) fn metadata(root: &Path) -> Result<serde_json::Value, String> {
    self::inputs::cargo_metadata(root, &[])
}

pub(super) fn package_ice_source(
    root: &Path,
    package_name: &str,
    cargo_args: &[String],
) -> Result<PathBuf, String> {
    inputs::package_ice_source(root, package_name, cargo_args)
}

#[derive(Debug)]
struct DevCompilation {
    dependencies: Vec<PathBuf>,
    asset_dependencies: Vec<PathBuf>,
}

#[derive(Debug)]
struct DevCompileError {
    message: String,
    dependencies: Vec<PathBuf>,
    asset_dependencies: Vec<PathBuf>,
}

/// Writes the root's current view template, returning it when the view is
/// publishable as data. `None` means this app's views only exist as compiled
/// Rust, so the runner stays in rebuild-and-restart mode.
fn publish_template(
    analysis_db: &mut ui_lang_core::AnalysisDb,
    source: &Path,
    path: &Path,
) -> Option<ui_lang_core::ViewTemplate> {
    let template = analysis_db.view_template(source).ok().flatten()?;
    fs::write(path, &template.json).ok()?;
    Some(template)
}

fn update_failed_watch(watched: &mut Vec<PathBuf>, discovered: Vec<PathBuf>) -> bool {
    if discovered.is_empty() || discovered == *watched {
        return false;
    }
    *watched = discovered;
    true
}

pub(super) fn run(root: &Path, source: &Path, cargo_args: &[String]) -> Result<(), String> {
    install_stop_handler()?;
    let source = normalize_watch_path(source);
    let ready_dir = root.join("target/ice-dev");
    fs::create_dir_all(&ready_dir).map_err(|error| error.to_string())?;
    let ready_base = ready_dir.join(format!(
        "{}-{}.ready",
        source
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("app"),
        std::process::id()
    ));
    let mut revision = 0_u64;
    let mut analysis_db = ui_lang_core::AnalysisDb::default();
    let mut previous_cargo_inputs = None;
    let (
        mut watched_dependencies,
        mut watched_assets,
        mut observed_stamps,
        mut cargo_inputs,
        executable,
    ) = loop {
        if stop_requested() {
            return Ok(());
        }
        let mut cargo_inputs = cargo_input_graph(root, cargo_args)?;
        if let Some(previous) = &previous_cargo_inputs {
            cargo_inputs.inherit_discovered_inputs(previous);
        }
        revision = revision.wrapping_add(1);
        let current =
            compile_dev_with_db(&mut analysis_db, &source, None).map_err(|error| error.message)?;
        let initial_stamps = dev_stamps_with_cargo_inputs(
            root,
            &current.dependencies,
            &current.asset_dependencies,
            &cargo_inputs,
        );
        let mut refreshed_cargo_inputs = cargo_input_graph(root, cargo_args)?;
        refreshed_cargo_inputs.inherit_discovered_inputs(&cargo_inputs);
        if refreshed_cargo_inputs != cargo_inputs {
            previous_cargo_inputs = Some(refreshed_cargo_inputs);
            continue;
        }
        let Some(build) = cargo_build(
            root,
            cargo_args,
            &initial_stamps.0,
            &initial_stamps.1,
            &cargo_inputs,
        )?
        else {
            if stop_requested() {
                return Ok(());
            }
            return Err("ice dev: initial app build failed".to_owned());
        };
        cargo_inputs.install_build_output(&build);
        previous_cargo_inputs = Some(cargo_inputs.clone());
        let observed = dev_stamps_with_cargo_inputs(
            root,
            &current.dependencies,
            &current.asset_dependencies,
            &cargo_inputs,
        );
        if !build_observation_reuses_snapshot(&initial_stamps, &observed) {
            tracing::info!("inputs changed during initial build; rebuilding the new snapshot");
            continue;
        }
        break (
            current.dependencies,
            current.asset_dependencies,
            observed,
            cargo_inputs,
            build.executable,
        );
    };
    let executable = stage_executable(&executable, revision)?;
    // The template the compiled binary was built against. While an edit only
    // changes the published view, the process reads a rewritten file instead
    // of being replaced.
    let template_path = ready_dir.join(format!(
        "{}-{}.template.json",
        source
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("app"),
        std::process::id()
    ));
    let mut live_template = publish_template(&mut analysis_db, &source, &template_path);
    let template_arg = live_template.is_some().then_some(template_path.as_path());
    let mut app =
        ChildGuard::spawn_owned(root, executable, runtime_args(cargo_args), template_arg)?;
    tracing::info!(
        ice_source_inputs = observed_stamps.0.len(),
        reload_mode = if live_template.is_some() {
            "in-place view reload"
        } else {
            "rebuild and restart"
        },
        "watching"
    );
    let mut changes = DevWatcher::new(&watched_dependencies, &watched_assets, &cargo_inputs);

    loop {
        if stop_requested() {
            tracing::info!("stopping");
            return Ok(());
        }
        if let Some(status) = app.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("ice dev: app exited with {status}"))
            };
        }
        changes.update(&watched_dependencies, &watched_assets, &cargo_inputs);
        let Some(change) = changes.wait_for_change(Duration::from_millis(100)) else {
            continue;
        };
        let next_snapshot = match change {
            DevChange::FullRescan => settled_dev_stamps_with_cargo_inputs(
                root,
                &watched_dependencies,
                &watched_assets,
                &cargo_inputs,
                &observed_stamps.0,
                &observed_stamps.1,
            )
            .map(|stamps| (stamps, None)),
            DevChange::Paths(paths) => settled_dev_snapshot_for_paths_with_cargo_inputs(
                &watched_dependencies,
                &watched_assets,
                &cargo_inputs,
                &observed_stamps.0,
                &observed_stamps.1,
                &paths,
            )
            .map(|snapshot| (snapshot.stamps, Some(snapshot.validated_sources))),
        };
        let Some((next_stamps, validated_sources)) = next_snapshot else {
            continue;
        };
        if let Some(path) = first_unreadable_input(&next_stamps) {
            tracing::warn!(
                input = %path.display(),
                "watched input is unreadable; keeping the current app open until it can be read again"
            );
            observed_stamps = next_stamps;
            continue;
        }
        if next_stamps.1 != observed_stamps.1 {
            match cargo_input_graph(root, cargo_args).map(|mut next| {
                next.inherit_discovered_inputs(&cargo_inputs);
                next
            }) {
                Ok(next_cargo_inputs) if next_cargo_inputs != cargo_inputs => {
                    cargo_inputs = next_cargo_inputs;
                    continue;
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(
                        %error,
                        "cannot refresh local Cargo inputs; keeping the current app open"
                    );
                    observed_stamps = next_stamps;
                    continue;
                }
            }
        }
        let (next_ice_stamp, next_build_stamp) = next_stamps;
        revision = revision.wrapping_add(1);
        let next = match compile_dev_with_db(&mut analysis_db, &source, validated_sources) {
            Ok(next) => next,
            Err(error) => {
                let dependencies_changed =
                    update_failed_watch(&mut watched_dependencies, error.dependencies);
                let assets_changed =
                    update_failed_watch(&mut watched_assets, error.asset_dependencies);
                if dependencies_changed || assets_changed {
                    continue;
                }
                let latest = dev_stamps_with_cargo_inputs(
                    root,
                    &watched_dependencies,
                    &watched_assets,
                    &cargo_inputs,
                );
                if latest == (next_ice_stamp, next_build_stamp) {
                    eprintln!("{}", error.message);
                    observed_stamps = latest;
                }
                continue;
            }
        };
        let candidate_stamps = dev_stamps_with_cargo_inputs(
            root,
            &next.dependencies,
            &next.asset_dependencies,
            &cargo_inputs,
        );
        if !stamps_match_on_common_paths(&candidate_stamps.0, &next_ice_stamp)
            || !stamps_match_on_common_paths(&candidate_stamps.1, &next_build_stamp)
        {
            continue;
        }
        thread::sleep(Duration::from_millis(50));
        if dev_stamps_with_cargo_inputs(
            root,
            &next.dependencies,
            &next.asset_dependencies,
            &cargo_inputs,
        ) != candidate_stamps
        {
            continue;
        }
        if candidate_stamps.1 != observed_stamps.1 {
            match cargo_input_graph(root, cargo_args).map(|mut next| {
                next.inherit_discovered_inputs(&cargo_inputs);
                next
            }) {
                Ok(next_cargo_inputs) if next_cargo_inputs != cargo_inputs => {
                    cargo_inputs = next_cargo_inputs;
                    continue;
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(
                        %error,
                        "cannot confirm local Cargo inputs; keeping the current app open"
                    );
                    observed_stamps = candidate_stamps;
                    continue;
                }
            }
        }

        // A view-only edit needs no compiler: if the running binary still
        // fills the slot table the new template asks for, rewriting the file
        // is the whole reload, and application state survives untouched.
        if let Some(current) = &live_template {
            match analysis_db.view_template(&source) {
                Ok(Some(candidate))
                    if candidate.slot_fingerprint == current.slot_fingerprint
                        && candidate.json != current.json =>
                {
                    match fs::write(&template_path, &candidate.json) {
                        Ok(()) => {
                            tracing::info!("view reloaded in place");
                            live_template = Some(candidate);
                            watched_dependencies = next.dependencies;
                            watched_assets = next.asset_dependencies;
                            observed_stamps = candidate_stamps;
                            continue;
                        }
                        Err(error) => tracing::warn!(
                            %error,
                            "cannot publish the reloaded view; rebuilding instead"
                        ),
                    }
                }
                // An unchanged view means the edit was elsewhere.
                Ok(Some(candidate)) if candidate.slot_fingerprint == current.slot_fingerprint => {}
                Ok(Some(_)) => tracing::info!(
                    "the edit needs values the running app does not compute; rebuilding"
                ),
                Ok(None) => {}
                Err(error) => {
                    eprintln!("{error}");
                    observed_stamps = candidate_stamps;
                    continue;
                }
            }
        }
        tracing::info!("inputs changed; rebuilding with the current app open");
        let build = match cargo_build(
            root,
            cargo_args,
            &candidate_stamps.0,
            &candidate_stamps.1,
            &cargo_inputs,
        ) {
            Ok(Some(build)) => build,
            Ok(None) => {
                tracing::warn!("build failed; keeping the current app open");
                watched_dependencies = next.dependencies;
                watched_assets = next.asset_dependencies;
                observed_stamps = candidate_stamps;
                continue;
            }
            Err(_) if stop_requested() => return Ok(()),
            Err(error) => {
                tracing::warn!(
                    %error,
                    "cannot complete candidate build; keeping the current app open"
                );
                watched_dependencies = next.dependencies;
                watched_assets = next.asset_dependencies;
                observed_stamps = candidate_stamps;
                continue;
            }
        };
        let mut next_cargo_inputs = cargo_inputs.clone();
        next_cargo_inputs.install_build_output(&build);
        let built_stamps = dev_stamps_with_cargo_inputs(
            root,
            &next.dependencies,
            &next.asset_dependencies,
            &next_cargo_inputs,
        );
        if !build_observation_reuses_snapshot(&candidate_stamps, &built_stamps) {
            tracing::info!("inputs changed during build; rebuilding the new snapshot");
            watched_dependencies = next.dependencies;
            watched_assets = next.asset_dependencies;
            cargo_inputs = next_cargo_inputs;
            continue;
        }
        let executable = match stage_executable(&build.executable, revision) {
            Ok(executable) => executable,
            Err(error) => {
                tracing::warn!(
                    %error,
                    "cannot stage restart candidate; keeping the current app open"
                );
                watched_dependencies = next.dependencies;
                watched_assets = next.asset_dependencies;
                cargo_inputs = next_cargo_inputs;
                observed_stamps = built_stamps;
                continue;
            }
        };
        // The rebuilt binary carries its own template and may fill a different
        // slot table, so the published file has to match it before the
        // candidate starts reading.
        live_template = publish_template(&mut analysis_db, &source, &template_path);
        if let Err(error) = app.restart(
            root,
            executable,
            runtime_args(cargo_args),
            &ready_base,
            revision,
            live_template.is_some().then_some(template_path.as_path()),
        ) {
            tracing::warn!(%error, "restart candidate failed; keeping the current app open");
            watched_dependencies = next.dependencies;
            watched_assets = next.asset_dependencies;
            cargo_inputs = next_cargo_inputs;
            observed_stamps = built_stamps;
            continue;
        }
        tracing::info!(revision, "restarted");
        watched_dependencies = next.dependencies;
        watched_assets = next.asset_dependencies;
        cargo_inputs = next_cargo_inputs;
        observed_stamps = built_stamps;
    }
}

#[cfg(test)]
fn compile_dev(source: &Path) -> Result<DevCompilation, DevCompileError> {
    compile_dev_with_db(&mut ui_lang_core::AnalysisDb::default(), source, None)
}

fn compile_dev_with_db(
    analysis_db: &mut ui_lang_core::AnalysisDb,
    source: &Path,
    validated_sources: Option<Vec<ui_lang_core::ValidatedSource>>,
) -> Result<DevCompilation, DevCompileError> {
    let result = match validated_sources {
        Some(sources) => analysis_db.analyze_root_with_validated_sources(source, sources),
        None => analysis_db.analyze_root(source),
    };
    match result {
        Ok(analysis) => Ok(DevCompilation {
            dependencies: analysis.dependencies,
            asset_dependencies: analysis.asset_dependencies,
        }),
        Err(error) => {
            let mut dependencies = ui_lang_core::discover_file_dependencies(source)
                .unwrap_or_else(|_| vec![source.to_owned()]);
            dependencies.push(source.to_owned());
            dependencies.sort();
            dependencies.dedup();
            Err(DevCompileError {
                message: error.render(&source.display().to_string()),
                dependencies,
                asset_dependencies: ui_lang_core::discover_file_asset_dependencies(source)
                    .unwrap_or_default(),
            })
        }
    }
}

#[cfg(test)]
mod tests;

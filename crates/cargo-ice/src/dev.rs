mod inputs;
mod process;
mod watcher;

#[cfg(test)]
use self::inputs::{
    BUILD_FINGERPRINT_ENV, CargoBuildOutput, CargoInputGraph, FileStamp, build_script_inputs,
    build_script_rerun_path, cargo_build_with_program, dev_stamps, file_stamp_attempts,
    parse_dep_info, reset_file_stamp_attempts, rustc_dep_info_path, settled_dev_stamps,
    settled_dev_stamps_after, source_stamp_fingerprint, stamp_contains_snapshot,
};
use self::inputs::{
    build_observation_reuses_snapshot, cargo_build, cargo_input_graph,
    dev_stamps_with_cargo_inputs, first_unreadable_input, normalize_watch_path,
    settled_dev_stamps_for_paths_with_cargo_inputs, settled_dev_stamps_with_cargo_inputs,
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
        let current = compile_dev(&source).map_err(|error| error.message)?;
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
            eprintln!("ice dev: inputs changed during initial build; rebuilding the new snapshot");
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
    let mut app = ChildGuard::spawn_owned(root, executable, runtime_args(cargo_args))?;
    println!(
        "ice dev: watching {} Ice source input(s); rebuild-and-restart mode",
        observed_stamps.0.len()
    );
    let mut changes = DevWatcher::new(&watched_dependencies, &watched_assets, &cargo_inputs);

    loop {
        if stop_requested() {
            println!("ice dev: stopping");
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
        let next_stamps = match change {
            DevChange::FullRescan => settled_dev_stamps_with_cargo_inputs(
                root,
                &watched_dependencies,
                &watched_assets,
                &cargo_inputs,
                &observed_stamps.0,
                &observed_stamps.1,
            ),
            DevChange::Paths(paths) => settled_dev_stamps_for_paths_with_cargo_inputs(
                &watched_dependencies,
                &watched_assets,
                &cargo_inputs,
                &observed_stamps.0,
                &observed_stamps.1,
                &paths,
            ),
        };
        let Some(next_stamps) = next_stamps else {
            continue;
        };
        if let Some(path) = first_unreadable_input(&next_stamps) {
            eprintln!(
                "ice dev: watched input {} is unreadable; keeping the current app open until it can be read again",
                path.display()
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
                    eprintln!(
                        "ice dev: cannot refresh local Cargo inputs: {error}; keeping the current app open"
                    );
                    observed_stamps = next_stamps;
                    continue;
                }
            }
        }
        let (next_ice_stamp, next_build_stamp) = next_stamps;
        revision = revision.wrapping_add(1);
        let next = match compile_dev(&source) {
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
                    eprintln!(
                        "ice dev: cannot confirm local Cargo inputs: {error}; keeping the current app open"
                    );
                    observed_stamps = candidate_stamps;
                    continue;
                }
            }
        }

        eprintln!("ice dev: inputs changed; rebuilding with the current app open");
        let build = match cargo_build(
            root,
            cargo_args,
            &candidate_stamps.0,
            &candidate_stamps.1,
            &cargo_inputs,
        ) {
            Ok(Some(build)) => build,
            Ok(None) => {
                eprintln!("ice dev: build failed; keeping the current app open");
                watched_dependencies = next.dependencies;
                watched_assets = next.asset_dependencies;
                observed_stamps = candidate_stamps;
                continue;
            }
            Err(_) if stop_requested() => return Ok(()),
            Err(error) => {
                eprintln!(
                    "ice dev: cannot complete candidate build; keeping the current app open: {error}"
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
            eprintln!("ice dev: inputs changed during build; rebuilding the new snapshot");
            watched_dependencies = next.dependencies;
            watched_assets = next.asset_dependencies;
            cargo_inputs = next_cargo_inputs;
            continue;
        }
        let executable = match stage_executable(&build.executable, revision) {
            Ok(executable) => executable,
            Err(error) => {
                eprintln!(
                    "ice dev: cannot stage restart candidate; keeping the current app open: {error}"
                );
                watched_dependencies = next.dependencies;
                watched_assets = next.asset_dependencies;
                cargo_inputs = next_cargo_inputs;
                observed_stamps = built_stamps;
                continue;
            }
        };
        if let Err(error) = app.restart(
            root,
            executable,
            runtime_args(cargo_args),
            &ready_base,
            revision,
        ) {
            eprintln!("ice dev: restart candidate failed; keeping the current app open: {error}");
            watched_dependencies = next.dependencies;
            watched_assets = next.asset_dependencies;
            cargo_inputs = next_cargo_inputs;
            observed_stamps = built_stamps;
            continue;
        }
        println!("ice dev: restarted on revision {revision}");
        watched_dependencies = next.dependencies;
        watched_assets = next.asset_dependencies;
        cargo_inputs = next_cargo_inputs;
        observed_stamps = built_stamps;
    }
}

fn compile_dev(source: &Path) -> Result<DevCompilation, DevCompileError> {
    match ui_lang_core::analyze_file_graph(source) {
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

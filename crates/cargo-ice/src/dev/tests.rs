use super::*;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn valid_app() -> &'static str {
    concat!(
        "app Demo\n",
        "theme contract AppTheme\n",
        "  bg\n",
        "  fg\n",
        "  primary\n",
        "  danger\n",
        "palette app for AppTheme\n",
        "  bg #000000\n",
        "  fg #ffffff\n",
        "  primary #333333\n",
        "  danger #ff0000\n",
        "view\n",
        "  text \"ready\"\n",
    )
}

#[test]
fn only_accepts_a_settled_input_snapshot() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    std::fs::create_dir(root.join("src")).unwrap();
    let source = root.join("app.ice");
    std::fs::write(&source, valid_app()).unwrap();
    let current = dev_stamps(root, std::slice::from_ref(&source), &[]);

    assert_eq!(
        settled_dev_stamps(
            root,
            std::slice::from_ref(&source),
            &[],
            &current.0,
            &current.1,
        ),
        None
    );

    std::fs::write(&source, valid_app().replace("ready", "first")).unwrap();
    assert_eq!(
        settled_dev_stamps_after(
            root,
            std::slice::from_ref(&source),
            &[],
            &current.0,
            &current.1,
            || {
                std::fs::write(&source, valid_app().replace("ready", "second")).unwrap();
            },
        ),
        None
    );

    assert!(
        settled_dev_stamps(
            root,
            std::slice::from_ref(&source),
            &[],
            &current.0,
            &current.1,
        )
        .is_some()
    );
}

#[test]
#[ignore = "manual full-snapshot scaling benchmark"]
fn benchmark_full_dev_snapshot_at_1k_and_10k_files() {
    for file_count in [1_000, 10_000] {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        let source = root.join("app.ice");
        std::fs::write(&source, valid_app()).unwrap();
        for index in 0..file_count {
            let directory = root.join("src").join(format!("{:02}", index / 100));
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                directory.join(format!("input-{index:05}.rs")),
                format!("pub const INPUT_{index}: usize = {index};\n"),
            )
            .unwrap();
        }

        let started = std::time::Instant::now();
        let snapshot = dev_stamps(root, std::slice::from_ref(&source), &[]);
        let elapsed = started.elapsed();

        assert_eq!(snapshot.0.len(), 1);
        assert!(snapshot.1.len() >= file_count);
        eprintln!("{file_count} files: one complete content snapshot in {elapsed:?}");

        let changed = root
            .join("src")
            .join(format!("{:02}", file_count / 200))
            .join(format!("input-{:05}.rs", file_count / 2));
        std::fs::write(&changed, "pub const CHANGED: bool = true;\n").unwrap();
        let graph = CargoInputGraph::workspace(root);
        reset_file_stamp_attempts();
        let started = std::time::Instant::now();
        let selective = settled_dev_stamps_for_paths_with_cargo_inputs(
            std::slice::from_ref(&source),
            &[],
            &graph,
            &snapshot.0,
            &snapshot.1,
            std::slice::from_ref(&changed),
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert_ne!(selective.1, snapshot.1);
        assert_eq!(file_stamp_attempts(), 2);
        eprintln!(
            "{file_count} files: selective two-pass verification in {elapsed:?} (2 content reads)"
        );
    }
}

#[test]
fn selective_snapshot_hashes_only_changed_file_content() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    let source = root.join("app.ice");
    std::fs::write(&source, valid_app()).unwrap();
    let directory = root.join("src");
    std::fs::create_dir(&directory).unwrap();
    for index in 0..128 {
        std::fs::write(
            directory.join(format!("input-{index:03}.rs")),
            format!("pub const INPUT_{index}: usize = {index};\n"),
        )
        .unwrap();
    }
    let changed = directory.join("input-064.rs");
    let graph = CargoInputGraph::workspace(root);
    let current = dev_stamps(root, std::slice::from_ref(&source), &[]);
    std::fs::write(&changed, "pub const INPUT_64: usize = 640;\n").unwrap();

    reset_file_stamp_attempts();
    let next = settled_dev_stamps_for_paths_with_cargo_inputs(
        std::slice::from_ref(&source),
        &[],
        &graph,
        &current.0,
        &current.1,
        std::slice::from_ref(&changed),
    )
    .unwrap();

    assert_ne!(next.1, current.1);
    assert_eq!(
        file_stamp_attempts(),
        2,
        "the changed file should be content-hashed once per settle pass"
    );

    std::fs::write(&source, valid_app().replace("ready", "changed")).unwrap();
    reset_file_stamp_attempts();
    let next = settled_dev_stamps_for_paths_with_cargo_inputs(
        std::slice::from_ref(&source),
        &[],
        &graph,
        &next.0,
        &next.1,
        std::slice::from_ref(&source),
    )
    .unwrap();

    assert_ne!(next.0, current.0);
    assert_eq!(
        file_stamp_attempts(),
        2,
        "an Ice-only edit must not reread the Rust input inventory"
    );
}

#[test]
fn selective_snapshot_refreshes_new_and_removed_input_inventory() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    let source = root.join("app.ice");
    std::fs::write(&source, valid_app()).unwrap();
    std::fs::create_dir(root.join("src")).unwrap();
    let graph = CargoInputGraph::workspace(root);
    let initial = dev_stamps(root, std::slice::from_ref(&source), &[]);
    let added = root.join("src/new.rs");
    std::fs::write(&added, "pub const NEW: bool = true;\n").unwrap();

    let with_added = settled_dev_stamps_for_paths_with_cargo_inputs(
        std::slice::from_ref(&source),
        &[],
        &graph,
        &initial.0,
        &initial.1,
        std::slice::from_ref(&added),
    )
    .unwrap();
    assert!(with_added.1.iter().any(|(path, _)| path == &added));

    std::fs::remove_file(&added).unwrap();
    let removed = settled_dev_stamps_for_paths_with_cargo_inputs(
        std::slice::from_ref(&source),
        &[],
        &graph,
        &with_added.0,
        &with_added.1,
        std::slice::from_ref(&added),
    )
    .unwrap();
    assert!(removed.1.iter().all(|(path, _)| path != &added));
}

#[test]
fn selective_snapshot_refreshes_when_an_ice_file_becomes_a_directory() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    let source = root.join("app.ice");
    let fragment = root.join("fragment.ice");
    std::fs::write(&source, valid_app()).unwrap();
    std::fs::write(&fragment, "component Fragment()\n  text \"ready\"\n").unwrap();
    let dependencies = vec![source, fragment.clone()];
    let graph = CargoInputGraph::workspace(root);
    let current = dev_stamps(root, &dependencies, &[]);
    std::fs::remove_file(&fragment).unwrap();
    std::fs::create_dir(&fragment).unwrap();
    let nested = fragment.join("nested.rs");
    std::fs::write(&nested, "pub const NESTED: bool = true;\n").unwrap();

    let next = settled_dev_stamps_for_paths_with_cargo_inputs(
        &dependencies,
        &[],
        &graph,
        &current.0,
        &current.1,
        std::slice::from_ref(&fragment),
    )
    .unwrap();

    assert!(next.1.iter().any(|(path, _)| path == &nested));
}

#[test]
fn compile_dev_uses_aot_codegen_and_keeps_the_requested_root_in_the_watch_set() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join("app.ice");
    std::fs::write(&source, valid_app()).unwrap();

    let compiled = compile_dev(&source).unwrap();

    assert!(compiled.dependencies.contains(&source));
}

#[test]
fn failed_aot_codegen_keeps_the_requested_root_watched() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join("app.ice");
    std::fs::write(&source, "app Demo\nview\n  NotAWidget\n").unwrap();

    let error = compile_dev(&source).unwrap_err();

    assert!(error.dependencies.contains(&source));
    assert!(!error.message.is_empty());
}

#[test]
fn build_script_inputs_include_ice_and_non_ice_declared_paths() {
    let root = Path::new("/workspace/app");
    let output = "cargo::rerun-if-changed=src/ui/app.ice\n\
                  cargo:rerun-if-changed=assets/icon.rgba\n\
                  cargo::rerun-if-env-changed=IGNORED\n";

    assert_eq!(
        build_script_inputs(root, output),
        [
            PathBuf::from("/workspace/app/assets/icon.rgba"),
            PathBuf::from("/workspace/app/src/ui/app.ice"),
        ]
    );
}

#[test]
fn recognizes_both_cargo_rerun_path_spellings() {
    assert_eq!(
        build_script_rerun_path("cargo::rerun-if-changed=src/main.rs"),
        Some("src/main.rs")
    );
    assert_eq!(
        build_script_rerun_path("cargo:rerun-if-changed=build.rs"),
        Some("build.rs")
    );
    assert_eq!(
        build_script_rerun_path("cargo::rerun-if-env-changed=MODE"),
        None
    );
}

#[test]
fn parses_rustc_dep_info_escapes_and_continuations() {
    let parsed =
        parse_dep_info(b"target: src/main.rs assets/space\\ name.bin \\\n          src/next.rs\n");

    assert_eq!(
        parsed,
        [
            PathBuf::from("src/main.rs"),
            PathBuf::from("assets/space name.bin"),
            PathBuf::from("src/next.rs"),
        ]
    );
}

#[test]
fn build_fingerprint_changes_with_path_content_and_file_state() {
    let content = vec![(PathBuf::from("app.ice"), FileStamp::Content(1))];
    let changed = vec![(PathBuf::from("app.ice"), FileStamp::Content(2))];
    let moved = vec![(PathBuf::from("other.ice"), FileStamp::Content(1))];
    let missing = vec![(PathBuf::from("app.ice"), FileStamp::Missing)];
    let unreadable = vec![(PathBuf::from("app.ice"), FileStamp::Unreadable)];

    assert_eq!(BUILD_FINGERPRINT_ENV, "ICE_DEV_BUILD_FINGERPRINT");
    assert_ne!(
        source_stamp_fingerprint(&content),
        source_stamp_fingerprint(&changed)
    );
    assert_ne!(
        source_stamp_fingerprint(&content),
        source_stamp_fingerprint(&moved)
    );
    assert_ne!(
        source_stamp_fingerprint(&missing),
        source_stamp_fingerprint(&unreadable)
    );
}

#[cfg(unix)]
#[test]
fn an_interrupted_cargo_build_kills_the_build_process_promptly() {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicBool, Ordering};

    let fixture = tempfile::tempdir().unwrap();
    let cargo = fixture.path().join("fake-cargo");
    std::fs::write(&cargo, "#!/bin/sh\nexec sleep 30\n").unwrap();
    let mut permissions = std::fs::metadata(&cargo).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&cargo, permissions).unwrap();
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let request = std::sync::Arc::clone(&stop);
    let interrupter = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(75));
        request.store(true, Ordering::Release);
    });
    let graph = CargoInputGraph::workspace(fixture.path());
    let started = std::time::Instant::now();
    let empty = Vec::new();

    let error = match cargo_build_with_program(
        cargo.as_os_str(),
        fixture.path(),
        &[],
        &empty,
        &empty,
        &graph,
        || stop.load(Ordering::Acquire),
    ) {
        Ok(_) => panic!("the interrupted fake Cargo process unexpectedly completed"),
        Err(error) => error,
    };

    interrupter.join().unwrap();
    assert_eq!(error, "ice dev: build interrupted");
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

#[test]
fn failed_observation_does_not_look_like_the_built_snapshot() {
    let initial = (
        vec![(PathBuf::from("app.ice"), FileStamp::Content(1))],
        vec![(PathBuf::from("src/main.rs"), FileStamp::Content(2))],
    );
    let changed = (
        vec![(PathBuf::from("app.ice"), FileStamp::Content(3))],
        initial.1.clone(),
    );

    assert!(!build_observation_reuses_snapshot(&initial, &changed));
}

#[test]
fn participating_package_roots_detect_new_rust_files_after_a_failed_build() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().to_owned();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    let package_id = "local-package".to_owned();
    let mut graph = CargoInputGraph {
        package_roots: Vec::new(),
        package_roots_by_id: [(package_id.clone(), root.clone())].into_iter().collect(),
        participating_package_ids: Vec::new(),
        include_ice_in_package_roots: true,
        workspace_files: Vec::new(),
        excluded_roots: vec![root.join("target")],
        discovered_inputs: Vec::new(),
    };
    graph.install_build_output(&CargoBuildOutput {
        executable: root.join("target/app"),
        discovered_inputs: Vec::new(),
        participating_package_ids: vec![package_id],
    });
    let before = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);

    std::fs::write(root.join("src/missing.rs"), "pub fn recovered() {}\n").unwrap();
    let after = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);

    assert_ne!(after.1, before.1);
}

#[test]
fn failed_dependency_discovery_updates_the_watch_set_once() {
    let mut watched = vec![PathBuf::from("old.ice")];

    assert!(update_failed_watch(
        &mut watched,
        vec![PathBuf::from("new.ice")]
    ));
    assert!(!update_failed_watch(
        &mut watched,
        vec![PathBuf::from("new.ice")]
    ));
}

#[test]
fn separates_cargo_build_arguments_from_application_arguments() {
    let args = [
        "-p".to_owned(),
        "demo".to_owned(),
        "--".to_owned(),
        "--profile".to_owned(),
        "test-user".to_owned(),
    ];

    assert_eq!(runtime_args(&args), ["--profile", "test-user"]);
    assert!(runtime_args(&args[..2]).is_empty());
}

#[cfg(unix)]
#[test]
fn candidate_reports_the_exact_opaque_ready_token() {
    let fixture = tempfile::tempdir().unwrap();
    let ready = fixture.path().join("ready");
    let token = "runner-7-token";
    assert_eq!(READY_PATH_ENV, "ICE_DEV_READY_PATH");
    assert_eq!(READY_TOKEN_ENV, "ICE_DEV_READY_TOKEN");
    let args = [
        "-c".to_owned(),
        "sleep 0.05; printf '%s' \"$ICE_DEV_READY_TOKEN\" > \"$ICE_DEV_READY_PATH\"; sleep 5"
            .to_owned(),
    ];
    let mut candidate =
        ChildGuard::spawn_with_ready(fixture.path(), Path::new("/bin/sh"), &args, &ready, token)
            .unwrap();

    candidate.wait_ready(&ready, token).unwrap();

    assert_eq!(std::fs::read_to_string(ready).unwrap(), token);
}

#[cfg(unix)]
#[test]
fn wrong_candidate_token_keeps_the_previous_process_alive() {
    let fixture = tempfile::tempdir().unwrap();
    let old_args = ["-c".to_owned(), "sleep 5".to_owned()];
    let mut old = ChildGuard::spawn(fixture.path(), Path::new("/bin/sh"), &old_args).unwrap();
    let old_pid = old.id();
    let artifact = fixture.path().join("candidate-sh");
    std::fs::copy("/bin/sh", &artifact).unwrap();
    let staged = stage_executable(&artifact, 2).unwrap();
    let staged_path = staged.path().to_owned();
    let candidate_args = [
        "-c".to_owned(),
        "printf 'wrong\\n' > \"$ICE_DEV_READY_PATH\"; sleep 5".to_owned(),
    ];

    let error = old
        .restart(
            fixture.path(),
            staged,
            &candidate_args,
            &fixture.path().join("ready"),
            2,
        )
        .unwrap_err();

    assert!(error.contains("unexpected readiness token"));
    assert_eq!(old.id(), old_pid);
    assert!(old.try_wait().unwrap().is_none());
    assert!(!staged_path.exists());
}

#[cfg(unix)]
#[test]
fn candidate_early_exit_keeps_the_previous_process_and_cleans_the_snapshot() {
    let fixture = tempfile::tempdir().unwrap();
    let old_args = ["-c".to_owned(), "sleep 5".to_owned()];
    let mut old = ChildGuard::spawn(fixture.path(), Path::new("/bin/sh"), &old_args).unwrap();
    let old_pid = old.id();
    let artifact = fixture.path().join("candidate-false");
    std::fs::copy("/bin/false", &artifact).unwrap();
    let staged = stage_executable(&artifact, 4).unwrap();
    let staged_path = staged.path().to_owned();
    let candidate_args = [];

    let error = old
        .restart(
            fixture.path(),
            staged,
            &candidate_args,
            &fixture.path().join("ready"),
            4,
        )
        .unwrap_err();

    assert!(
        error.contains("before reporting readiness token"),
        "unexpected candidate error: {error}"
    );
    assert_eq!(old.id(), old_pid);
    assert!(old.try_wait().unwrap().is_none());
    assert!(!staged_path.exists());
    assert!(
        std::fs::read_dir(fixture.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .as_encoded_bytes()
                .ends_with(b".ready")
        }),
        "an early candidate exit leaked its readiness file"
    );
}

#[cfg(unix)]
#[test]
fn successful_candidate_handoff_replaces_the_previous_process() {
    let fixture = tempfile::tempdir().unwrap();
    let old_args = ["-c".to_owned(), "sleep 5".to_owned()];
    let mut app = ChildGuard::spawn(fixture.path(), Path::new("/bin/sh"), &old_args).unwrap();
    let old_pid = app.id();
    let artifact = fixture.path().join("candidate-sh");
    std::fs::copy("/bin/sh", &artifact).unwrap();
    let staged = stage_executable(&artifact, 3).unwrap();
    let candidate_args = [
        "-c".to_owned(),
        "printf '%s' \"$ICE_DEV_READY_TOKEN\" > \"$ICE_DEV_READY_PATH\"; sleep 5".to_owned(),
    ];

    app.restart(
        fixture.path(),
        staged,
        &candidate_args,
        &fixture.path().join("ready"),
        3,
    )
    .unwrap();

    assert_ne!(app.id(), old_pid);
    assert!(app.try_wait().unwrap().is_none());
    assert!(
        std::fs::read_dir(fixture.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .as_encoded_bytes()
                .ends_with(b".ready")
        }),
        "a successful handoff leaked its readiness file"
    );
}

#[test]
fn staged_executable_is_an_immutable_snapshot_and_cleans_up() {
    let fixture = tempfile::tempdir().unwrap();
    let artifact = fixture.path().join("app.bin");
    std::fs::write(&artifact, b"old executable").unwrap();

    let staged = stage_executable(&artifact, 17).unwrap();
    let staged_path = staged.path().to_owned();
    std::fs::write(&artifact, b"new executable").unwrap();

    assert_ne!(staged_path, artifact);
    assert_eq!(std::fs::read(&staged_path).unwrap(), b"old executable");
    drop(staged);
    assert!(!staged_path.exists());
}

#[cfg(unix)]
#[test]
fn child_guard_removes_its_staged_executable_after_termination() {
    let fixture = tempfile::tempdir().unwrap();
    let artifact = fixture.path().join("app-sleep");
    std::fs::copy("/bin/sleep", &artifact).unwrap();
    let executable = stage_executable(&artifact, 9).unwrap();
    let executable_path = executable.path().to_owned();
    let args = ["5".to_owned()];

    let child = ChildGuard::spawn_owned(fixture.path(), executable, &args).unwrap();
    assert!(executable_path.exists());
    drop(child);

    assert!(!executable_path.exists());
}

#[cfg(unix)]
#[test]
fn readiness_rejects_whitespace_around_the_token() {
    let fixture = tempfile::tempdir().unwrap();
    let ready = fixture.path().join("ready");
    let token = "exact-token";
    let args = [
        "-c".to_owned(),
        "printf ' exact-token\\n' > \"$ICE_DEV_READY_PATH\"; sleep 5".to_owned(),
    ];
    let mut candidate =
        ChildGuard::spawn_with_ready(fixture.path(), Path::new("/bin/sh"), &args, &ready, token)
            .unwrap();

    let error = candidate.wait_ready(&ready, token).unwrap_err();

    assert!(error.contains("unexpected readiness token"));
    assert!(error.contains("\\n"));
}

#[cfg(unix)]
#[test]
fn readiness_timeout_is_bounded_and_testable() {
    let fixture = tempfile::tempdir().unwrap();
    let ready = fixture.path().join("ready");
    let args = ["-c".to_owned(), "sleep 5".to_owned()];
    let mut candidate = ChildGuard::spawn_with_ready(
        fixture.path(),
        Path::new("/bin/sh"),
        &args,
        &ready,
        "never-ready",
    )
    .unwrap();
    let started = std::time::Instant::now();

    let error = candidate
        .wait_ready_with_timeout(&ready, "never-ready", std::time::Duration::from_millis(75))
        .unwrap_err();

    assert!(error.contains("within 75 milliseconds"));
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
}

#[cfg(unix)]
#[test]
fn rustc_dep_info_uses_the_unique_hard_linked_artifact() {
    let fixture = tempfile::tempdir().unwrap();
    let executable = fixture.path().join("demo");
    let artifact = fixture.path().join("demo-0123456789abcdef");
    std::fs::write(&executable, b"binary").unwrap();
    std::fs::hard_link(&executable, &artifact).unwrap();
    let expected = artifact.with_extension("d");
    std::fs::write(&expected, b"demo: src/main.rs\n").unwrap();

    assert_eq!(rustc_dep_info_path(&executable).unwrap(), expected);
}
fn stamp_at(stamp: &[(PathBuf, FileStamp)], path: &Path) -> Option<FileStamp> {
    stamp
        .iter()
        .find_map(|(candidate, stamp)| (candidate == path).then_some(*stamp))
}

fn replace_symlink(alias: &Path, target: &Path) {
    use std::os::unix::fs::symlink;

    let replacement = alias.with_extension("ice.replacement");
    symlink(target, &replacement).unwrap();
    std::fs::rename(replacement, alias).unwrap();
}

#[test]

fn same_content_import_symlink_retarget_invalidates_the_checked_snapshot() {
    use std::os::unix::fs::symlink;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-import-retarget-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let source = root.join("app.ice");
    let alias = root.join("current.ice");
    std::fs::write(
            &source,
            "app Demo\nuse \"current.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n",
        )
        .unwrap();
    for target in ["one.ice", "two.ice"] {
        std::fs::write(
            root.join(target),
            "component Card()\n  text \"same bytes\"\n",
        )
        .unwrap();
    }
    symlink("one.ice", &alias).unwrap();
    let first = compile_dev(&source).unwrap();
    let before = dev_stamps(&root, &first.dependencies, &[]).0;

    replace_symlink(&alias, Path::new("two.ice"));
    let retargeted = dev_stamps(&root, &first.dependencies, &[]).0;
    assert_ne!(retargeted, before, "edge identity is part of the stamp");
    assert!(
        !stamp_contains_snapshot(&retargeted, &before),
        "equal target bytes cannot validate an obsolete resolved edge"
    );
    let second = compile_dev(&source).unwrap();
    assert!(second.dependencies.contains(&alias));
    assert!(second.dependencies.contains(&root.join("two.ice")));
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]

fn same_content_root_symlink_retarget_invalidates_the_checked_snapshot() {
    use std::os::unix::fs::symlink;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-root-retarget-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let alias = root.join("current.ice");
    for target in ["one.ice", "two.ice"] {
        std::fs::write(
                root.join(target),
                "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"same bytes\"\n",
            )
            .unwrap();
    }
    symlink("one.ice", &alias).unwrap();
    let first = compile_dev(&alias).unwrap();
    let before = dev_stamps(&root, &first.dependencies, &[]).0;
    assert!(first.dependencies.contains(&alias));

    replace_symlink(&alias, Path::new("two.ice"));
    let retargeted = dev_stamps(&root, &first.dependencies, &[]).0;
    assert!(
        !stamp_contains_snapshot(&retargeted, &before),
        "the original root path must retain its resolved identity"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]

fn watches_only_the_selected_ice_import_graph() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-selected-graph-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let selected = root.join("selected.ice");
    let unrelated = root.join("unrelated.ice");
    std::fs::write(&selected, "app Selected\nview\n  text \"selected\"\n").unwrap();
    std::fs::write(&unrelated, "app Other\nview\n  text \"one\"\n").unwrap();

    let before = dev_stamps(&root, std::slice::from_ref(&selected), &[]);
    assert!(stamp_at(&before.0, &unrelated).is_none());
    std::fs::write(&unrelated, "app Other\nview\n  text \"two\"\n").unwrap();
    let after = dev_stamps(&root, std::slice::from_ref(&selected), &[]);

    assert_eq!(
        after, before,
        "an unrelated app must not trigger a revision"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn watches_regular_host_build_inputs_without_extension_guesses() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-regular-build-input-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("assets")).unwrap();
    let font = root.join("assets/font.ttf");
    let shader = root.join("assets/effect.wgsl");
    std::fs::write(&font, b"AAAA").unwrap();
    std::fs::write(&shader, b"BBBB").unwrap();
    let graph = CargoInputGraph::workspace(&root);

    let before = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);
    std::fs::write(&font, b"CCCC").unwrap();
    std::fs::write(&shader, b"DDDD").unwrap();
    let after = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);

    assert_ne!(after.1, before.1);
    assert!(matches!(
        stamp_at(&after.1, &font),
        Some(FileStamp::Content(_))
    ));
    assert!(matches!(
        stamp_at(&after.1, &shader),
        Some(FileStamp::Content(_))
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn discovered_build_script_directories_track_new_files() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "cargo-ice-build-script-directory-{}-{nonce}",
        std::process::id()
    ));
    let root = base.join("app");
    let generated_inputs = base.join("external-inputs");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&generated_inputs).unwrap();
    let mut graph = CargoInputGraph::workspace(&root);
    graph.install_discovered_inputs(vec![generated_inputs.clone()]);
    let before = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);

    let created = generated_inputs.join("bindings.h");
    std::fs::write(&created, b"#define VALUE 1\n").unwrap();
    let after = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);

    assert_ne!(after.1, before.1);
    assert!(matches!(
        stamp_at(&after.1, &created),
        Some(FileStamp::Content(_))
    ));
    std::fs::remove_dir_all(base).unwrap();
}

#[test]

fn embedded_asset_content_and_creation_change_the_build_stamp() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-dev-asset-stamp-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("assets")).unwrap();
    let source = root.join("app.ice");
    let asset = root.join("assets/icon.rgba");
    std::fs::write(&source, "app Demo\nview\n  text \"asset\"\n").unwrap();
    std::fs::write(&asset, b"AAAA").unwrap();

    let original = dev_stamps(
        &root,
        std::slice::from_ref(&source),
        std::slice::from_ref(&asset),
    );
    std::fs::write(&asset, b"BBBB").unwrap();
    assert_eq!(
        settled_dev_stamps_after(
            &root,
            std::slice::from_ref(&source),
            std::slice::from_ref(&asset),
            &original.0,
            &original.1,
            || std::fs::write(&asset, b"CCCC").unwrap(),
        ),
        None,
        "an asset edit during debounce is not a stable compile input"
    );
    std::fs::write(&asset, b"BBBB").unwrap();
    let changed = dev_stamps(
        &root,
        std::slice::from_ref(&source),
        std::slice::from_ref(&asset),
    );
    assert_eq!(changed.0, original.0);
    assert_ne!(changed.1, original.1, "equal-sized edits must rebuild");

    std::fs::remove_file(&asset).unwrap();
    let missing = dev_stamps(
        &root,
        std::slice::from_ref(&source),
        std::slice::from_ref(&asset),
    );
    std::fs::write(&asset, b"CCCC").unwrap();
    let created = dev_stamps(
        &root,
        std::slice::from_ref(&source),
        std::slice::from_ref(&asset),
    );
    assert_ne!(
        created.1, missing.1,
        "creating a missing asset must rebuild"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn unreadable_asset_stamp_is_stable_and_recovers() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-dev-unreadable-asset-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("assets")).unwrap();
    let source = root.join("app.ice");
    let asset = root.join("assets/icon.rgba");
    std::fs::write(&source, "app Demo\nview\n  text \"asset\"\n").unwrap();
    std::fs::write(&asset, b"AAAA").unwrap();
    let original = dev_stamps(
        &root,
        std::slice::from_ref(&source),
        std::slice::from_ref(&asset),
    );

    std::fs::remove_file(&asset).unwrap();
    std::fs::create_dir(&asset).unwrap();
    let unreadable = dev_stamps(
        &root,
        std::slice::from_ref(&source),
        std::slice::from_ref(&asset),
    );
    assert_eq!(stamp_at(&unreadable.1, &asset), Some(FileStamp::Unreadable));
    assert_eq!(first_unreadable_input(&unreadable), Some(asset.as_path()));
    assert_ne!(unreadable.1, original.1);
    assert_eq!(
        settled_dev_stamps(
            &root,
            std::slice::from_ref(&source),
            std::slice::from_ref(&asset),
            &unreadable.0,
            &unreadable.1,
        ),
        None,
        "an observed unreadable input must not trigger a busy retry loop"
    );

    std::fs::remove_dir(&asset).unwrap();
    std::fs::write(&asset, b"BBBB").unwrap();
    let recovered = dev_stamps(
        &root,
        std::slice::from_ref(&source),
        std::slice::from_ref(&asset),
    );
    assert!(matches!(
        stamp_at(&recovered.1, &asset),
        Some(FileStamp::Content(_))
    ));
    assert_ne!(recovered.1, unreadable.1);
    assert!(first_unreadable_input(&recovered).is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn unreadable_ice_and_rust_input_stamps_recover() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-dev-unreadable-source-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("src")).unwrap();
    let source = root.join("app.ice");
    let rust = root.join("src/main.rs");
    std::fs::write(&source, "app Demo\nview\n  text \"source\"\n").unwrap();
    std::fs::write(&rust, "fn main() {}\n").unwrap();

    std::fs::remove_file(&source).unwrap();
    std::fs::create_dir(&source).unwrap();
    let unreadable_source = dev_stamps(&root, std::slice::from_ref(&source), &[]);
    assert_eq!(
        stamp_at(&unreadable_source.0, &source),
        Some(FileStamp::Unreadable)
    );
    std::fs::remove_dir(&source).unwrap();
    std::fs::write(&source, "app Demo\nview\n  text \"recovered\"\n").unwrap();
    let recovered_source = dev_stamps(&root, std::slice::from_ref(&source), &[]);
    assert!(matches!(
        stamp_at(&recovered_source.0, &source),
        Some(FileStamp::Content(_))
    ));

    std::fs::remove_file(&rust).unwrap();
    std::fs::create_dir(&rust).unwrap();
    let unreadable_rust = dev_stamps(&root, std::slice::from_ref(&source), &[]);
    assert_eq!(
        stamp_at(&unreadable_rust.1, &rust),
        Some(FileStamp::Unreadable)
    );
    std::fs::remove_dir(&rust).unwrap();
    std::fs::write(&rust, "fn main() { println!(\"recovered\"); }\n").unwrap();
    let recovered_rust = dev_stamps(&root, std::slice::from_ref(&source), &[]);
    assert!(matches!(
        stamp_at(&recovered_rust.1, &rust),
        Some(FileStamp::Content(_))
    ));
    assert!(first_unreadable_input(&recovered_rust).is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cargo_input_graph_excludes_pinned_workspace_vendor_packages() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    let app = root.join("app");
    let vendor = root.join("vendor/patched");
    for package in [&app, &vendor] {
        std::fs::create_dir_all(package.join("src")).unwrap();
        std::fs::write(package.join("src/lib.rs"), "pub fn value() {}\n").unwrap();
    }
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"vendor/patched\"]\nresolver = \"3\"\n",
    )
    .unwrap();
    std::fs::write(
        app.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        vendor.join("Cargo.toml"),
        "[package]\nname = \"patched\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();

    let graph = cargo_input_graph(root, &[]).unwrap();
    let app = app.canonicalize().unwrap();
    let vendor_root = root.join("vendor").canonicalize().unwrap();

    assert!(graph.package_roots.contains(&app));
    assert!(graph.excluded_roots.contains(&vendor_root));
    assert!(
        graph
            .package_roots
            .iter()
            .all(|package| !package.starts_with(&vendor_root))
    );
}

#[test]
fn watches_transitive_external_cargo_path_dependencies() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "cargo-ice-external-path-deps-{}-{nonce}",
        std::process::id()
    ));
    let root = base.join("app");
    let backend = base.join("backend");
    let leaf = base.join("leaf");
    for package in [&root, &backend, &leaf] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }
    std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"path-app\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\npath-backend = { path = \"../backend\" }\n",
        )
        .unwrap();
    std::fs::write(
        root.join("src/main.rs"),
        "fn main() { println!(\"{}\", path_backend::value()); }\n",
    )
    .unwrap();
    std::fs::write(
            backend.join("Cargo.toml"),
            "[package]\nname = \"path-backend\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\npath-leaf = { path = \"../leaf\" }\n",
        )
        .unwrap();
    std::fs::write(
        backend.join("src/lib.rs"),
        "pub fn value() -> u8 { path_leaf::value() }\n",
    )
    .unwrap();
    std::fs::write(
        leaf.join("Cargo.toml"),
        "[package]\nname = \"path-leaf\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    let leaf_source = leaf.join("src/lib.rs");
    std::fs::write(&leaf_source, "pub fn value() -> u8 { 1 }\n").unwrap();

    let graph = cargo_input_graph(&root, &[]).unwrap();
    let root = root.canonicalize().unwrap();
    let backend = backend.canonicalize().unwrap();
    let leaf = leaf.canonicalize().unwrap();
    assert!(graph.package_roots.contains(&root));
    assert!(graph.package_roots.contains(&backend));
    assert!(
        graph.package_roots.contains(&leaf),
        "transitive path dependency is not watched"
    );
    let before = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);
    std::fs::write(&leaf_source, "pub fn value() -> u8 { 2 }\n").unwrap();
    let after = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);

    assert_ne!(before.1, after.1);
    assert!(matches!(
        stamp_at(&after.1, &leaf_source),
        Some(FileStamp::Content(_))
    ));
    std::fs::remove_dir_all(base).unwrap();
}

#[test]

fn build_observation_drops_unrelated_workspace_packages() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-participating-packages-{}-{nonce}",
        std::process::id()
    ));
    let app = root.join("app");
    let used = root.join("used");
    let unrelated = root.join("unrelated");
    for package in [&app, &used, &unrelated] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"used\", \"unrelated\"]\nresolver = \"3\"\n",
    )
    .unwrap();
    std::fs::write(
            app.join("Cargo.toml"),
            "[package]\nname = \"selected-app\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nused = { path = \"../used\" }\n",
        )
        .unwrap();
    std::fs::write(
        app.join("src/main.rs"),
        "fn main() { println!(\"{}\", used::value()); }\n",
    )
    .unwrap();
    std::fs::write(
            used.join("Cargo.toml"),
            "[package]\nname = \"used\"\nversion = \"0.0.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n",
        )
        .unwrap();
    std::fs::write(
        used.join("build.rs"),
        "fn main() { let _ = std::fs::read_to_string(\"implicit.txt\").unwrap(); }\n",
    )
    .unwrap();
    let implicit = used.join("implicit.txt");
    std::fs::write(&implicit, "one\n").unwrap();
    let used_source = used.join("src/lib.rs");
    std::fs::write(&used_source, "pub fn value() -> u8 { 1 }\n").unwrap();
    std::fs::write(
        unrelated.join("Cargo.toml"),
        "[package]\nname = \"unrelated\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(unrelated.join("src/lib.rs"), "pub fn unused() {}\n").unwrap();
    let unrelated_asset = unrelated.join("large-font.bin");
    std::fs::write(&unrelated_asset, vec![7_u8; 1024 * 1024]).unwrap();
    let args = ["-p".to_owned(), "selected-app".to_owned()];
    let mut graph = cargo_input_graph(&root, &args).unwrap();
    let initial = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);
    assert!(stamp_at(&initial.1, &unrelated_asset).is_some());

    let build = cargo_build(&root, &args, &initial.0, &initial.1, &graph)
        .unwrap()
        .expect("selected workspace app must build");
    graph.install_build_output(&build);
    let narrowed = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);

    assert!(
        build_observation_reuses_snapshot(&initial, &narrowed),
        "dropping unrelated workspace inputs must reuse the first Cargo build"
    );
    assert!(narrowed.1.len() < initial.1.len());
    assert!(stamp_at(&narrowed.1, &unrelated_asset).is_none());
    assert!(stamp_at(&narrowed.1, &used_source).is_some());
    assert!(stamp_at(&narrowed.1, &implicit).is_some());
    assert_eq!(graph.package_roots.len(), 2);
    assert!(graph.package_roots.contains(&app.canonicalize().unwrap()));
    assert!(graph.package_roots.contains(&used.canonicalize().unwrap()));
    assert!(
        !graph
            .package_roots
            .contains(&unrelated.canonicalize().unwrap())
    );
    assert!(
        !graph.include_ice_in_package_roots,
        "successful Cargo discovery must leave Ice watching to the selected source graph"
    );
    std::fs::write(&unrelated_asset, vec![9_u8; 1024 * 1024]).unwrap();
    assert_eq!(
        dev_stamps_with_cargo_inputs(&root, &[], &[], &graph),
        narrowed,
        "unrelated workspace bytes must not wake the selected app"
    );
    std::fs::write(&implicit, "two\n").unwrap();
    assert_ne!(
        dev_stamps_with_cargo_inputs(&root, &[], &[], &graph).1,
        narrowed.1,
        "a no-directive build script retains Cargo's broad package semantics"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn watches_explicit_external_cargo_config_content_for_both_argument_forms() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "cargo-ice-explicit-config-{}-{nonce}",
        std::process::id()
    ));
    let root = base.join("app");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"explicit-config-app\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    let config = base.join("extra-config.toml");
    std::fs::write(&config, "[env]\nICE_CONFIG_PROBE = \"AAAA\"\n").unwrap();

    let separate = cargo_input_graph(
        &root,
        &["--config".to_owned(), "../extra-config.toml".to_owned()],
    )
    .unwrap();
    let equals = cargo_input_graph(&root, &["--config=../extra-config.toml".to_owned()]).unwrap();
    assert!(separate.workspace_files.contains(&config));
    assert!(equals.workspace_files.contains(&config));

    let before = dev_stamps_with_cargo_inputs(&root, &[], &[], &separate);
    std::fs::write(&config, "[env]\nICE_CONFIG_PROBE = \"BBBB\"\n").unwrap();
    let after = dev_stamps_with_cargo_inputs(&root, &[], &[], &separate);
    assert_ne!(
        before.1, after.1,
        "an equal-sized explicit config edit must rebuild"
    );
    assert!(matches!(
        stamp_at(&after.1, &config),
        Some(FileStamp::Content(_))
    ));
    std::fs::remove_dir_all(base).unwrap();
}

#[test]

fn watches_cargo_config_discovered_from_a_nested_invocation_directory() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-nested-config-{}-{nonce}",
        std::process::id()
    ));
    let invocation = root.join("tools/nested");
    let config = root.join("tools/.cargo/config");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(&invocation).unwrap();
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"nested-config-app\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(&config, "[env]\nICE_NESTED_PROBE = \"AAAA\"\n").unwrap();

    let graph = cargo_input_graph(&invocation, &[]).unwrap();
    assert!(graph.workspace_files.contains(&config));
    let before = dev_stamps_with_cargo_inputs(&invocation, &[], &[], &graph);
    std::fs::write(&config, "[env]\nICE_NESTED_PROBE = \"BBBB\"\n").unwrap();
    let after = dev_stamps_with_cargo_inputs(&invocation, &[], &[], &graph);
    assert_ne!(before.1, after.1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn watches_missing_discovered_cargo_config_until_it_is_created() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-created-config-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"created-config-app\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    let config = root.join(".cargo/config.toml");

    let graph = cargo_input_graph(&root, &[]).unwrap();
    assert!(graph.workspace_files.contains(&config));
    let missing = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);
    assert_eq!(stamp_at(&missing.1, &config), Some(FileStamp::Missing));

    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(&config, "[env]\nICE_CREATED_PROBE = \"ready\"\n").unwrap();
    let created = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);
    assert!(matches!(
        stamp_at(&created.1, &config),
        Some(FileStamp::Content(_))
    ));
    assert_ne!(missing.1, created.1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn stable_snapshot_comparison_allows_dependency_set_changes_only() {
    let left = vec![
        (PathBuf::from("old"), FileStamp::Content(1)),
        (PathBuf::from("shared"), FileStamp::Content(2)),
    ];
    let extended = vec![
        (PathBuf::from("new"), FileStamp::Content(3)),
        (PathBuf::from("shared"), FileStamp::Content(2)),
    ];
    let changed = vec![
        (PathBuf::from("new"), FileStamp::Content(3)),
        (PathBuf::from("shared"), FileStamp::Content(4)),
    ];

    assert!(stamps_match_on_common_paths(&left, &extended));
    assert!(!stamps_match_on_common_paths(&left, &changed));
}

#[test]

fn cargo_fingerprint_forces_generated_bytes_to_rebuild() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-fingerprint-build-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fingerprint-check\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
            root.join("build.rs"),
            format!(
                "fn main() {{ println!(\"cargo::rerun-if-env-changed={}\"); let bytes = std::fs::read(\"asset.bin\").unwrap(); std::fs::write(std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"asset.rs\"), format!(\"pub const ASSET: &[u8] = &{{:?}};\", bytes)).unwrap(); }}\n",
                BUILD_FINGERPRINT_ENV
            ),
        )
        .unwrap();
    std::fs::write(
            root.join("src/main.rs"),
            "include!(concat!(env!(\"OUT_DIR\"), \"/asset.rs\"));\nfn main() { print!(\"{}\", std::str::from_utf8(ASSET).unwrap()); }\n",
        )
        .unwrap();
    let asset = root.join("asset.bin");
    std::fs::write(&asset, b"AAAA").unwrap();
    let run = |fingerprint: &str| {
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
            .args(["run", "--quiet"])
            .env(BUILD_FINGERPRINT_ENV, fingerprint)
            .current_dir(&root)
            .output()
            .unwrap()
    };

    let first = run("first");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, b"AAAA");
    std::fs::write(&asset, b"BBBB").unwrap();
    let second = run("second");
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(second.stdout, b"BBBB");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn cargo_build_discovers_rustc_and_build_script_inputs() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "cargo-ice-discovered-inputs-{}-{nonce}",
        std::process::id()
    ));
    let root = base.join("app");
    let external = base.join("native/input.bin");
    let embedded = root.join("schema.ice");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(external.parent().unwrap()).unwrap();
    std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"discovered-inputs\"\nversion = \"0.0.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n",
        )
        .unwrap();
    std::fs::write(
        root.join("build.rs"),
        "fn main() { println!(\"cargo::rerun-if-changed=../native/input.bin\"); }\n",
    )
    .unwrap();
    std::fs::write(
            root.join("src/main.rs"),
            "const BYTES: &[u8] = include_bytes!(\"../schema.ice\");\nfn main() { print!(\"{}\", BYTES.len()); }\n",
        )
        .unwrap();
    std::fs::write(&external, b"native").unwrap();
    std::fs::write(&embedded, b"embedded").unwrap();
    let mut graph = cargo_input_graph(&root, &[]).unwrap();
    let stamps = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);

    let build = cargo_build(&root, &[], &stamps.0, &stamps.1, &graph)
        .unwrap()
        .expect("the fixture must build");

    assert!(build.executable.exists());
    let dep_info = rustc_dep_info_path(&build.executable).unwrap();
    assert_ne!(dep_info, build.executable.with_extension("d"));
    assert!(dep_info.starts_with(build.executable.parent().unwrap().join("deps")));
    assert!(
        build
            .discovered_inputs
            .contains(&embedded.canonicalize().unwrap()),
        "rustc dep-info did not report include_bytes input: {:?}",
        build.discovered_inputs
    );
    assert!(
        build
            .discovered_inputs
            .contains(&external.canonicalize().unwrap()),
        "build.rs rerun-if-changed input was not reported: {:?}",
        build.discovered_inputs
    );
    graph.install_build_output(&build);
    let installed = dev_stamps_with_cargo_inputs(&root, &[], &[], &graph);
    assert!(
        !build_observation_reuses_snapshot(&stamps, &installed),
        "newly discovered external inputs must force a build with their fingerprint"
    );
    assert_ne!(
        installed.1, stamps.1,
        "the first discovery must invalidate the incomplete snapshot"
    );
    let cached = cargo_build(&root, &[], &installed.0, &installed.1, &graph)
        .unwrap()
        .expect("the cached fixture build must stay runnable");
    assert!(
        cached
            .discovered_inputs
            .contains(&external.canonicalize().unwrap()),
        "a cached build forgot its build.rs input: {:?}",
        cached.discovered_inputs
    );
    graph.install_build_output(&cached);
    assert_eq!(
        dev_stamps_with_cargo_inputs(&root, &[], &[], &graph),
        installed,
        "the second pass must converge on the exact discovered-input snapshot"
    );
    std::fs::remove_dir_all(base).unwrap();
}

#[test]

fn refuses_to_treat_cargo_aggregate_dep_info_as_rustc_input() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-aggregate-dep-info-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("deps")).unwrap();
    let executable = root.join("app");
    std::fs::write(&executable, b"final artifact").unwrap();
    std::fs::write(executable.with_extension("d"), b"app: managed.ice\n").unwrap();
    std::fs::write(root.join("deps/app-deadbeef"), b"different artifact").unwrap();
    std::fs::write(root.join("deps/app-deadbeef.d"), b"app: src/main.rs\n").unwrap();

    let error = rustc_dep_info_path(&executable).unwrap_err();

    assert!(error.contains("refusing Cargo's aggregate dep-info"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn rejects_ambiguous_rustc_artifact_identity() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-ambiguous-artifact-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("deps")).unwrap();
    let executable = root.join("app");
    std::fs::write(&executable, b"artifact").unwrap();
    for hash in ["one", "two"] {
        let artifact = root.join("deps").join(format!("app-{hash}"));
        std::fs::hard_link(&executable, &artifact).unwrap();
        std::fs::write(artifact.with_extension("d"), b"app: src/main.rs\n").unwrap();
    }

    let error = rustc_dep_info_path(&executable).unwrap_err();

    assert!(error.contains("multiple rustc artifacts"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]

fn failed_analysis_watches_an_existing_external_import_until_it_is_fixed() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "cargo-ice-external-invalid-{}-{nonce}",
        std::process::id()
    ));
    let root = base.join("workspace");
    let external = base.join("outside/part.ice");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(external.parent().unwrap()).unwrap();
    let source = root.join("app.ice");
    std::fs::write(
            &source,
            "app Demo\nuse \"../outside/part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"external\"\n",
        )
        .unwrap();
    std::fs::write(&external, "not valid Ice\n").unwrap();
    let source = source.canonicalize().unwrap();
    let external = external.canonicalize().unwrap();

    let error = match compile_dev(&source) {
        Ok(_) => panic!("the invalid external import unexpectedly compiled"),
        Err(error) => error,
    };
    assert!(error.dependencies.contains(&source));
    assert!(error.dependencies.contains(&external));
    let invalid = dev_stamps(&root, &error.dependencies, &[]);

    std::fs::write(&external, "").unwrap();
    let fixed = dev_stamps(&root, &error.dependencies, &[]);
    assert_ne!(fixed.0, invalid.0);
    let compiled = compile_dev(&source).expect("the fixed external import must compile");
    assert!(compiled.dependencies.contains(&external));
    std::fs::remove_dir_all(base).unwrap();
}

#[test]

fn failed_analysis_watches_a_missing_external_import_until_it_is_created() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "cargo-ice-external-missing-{}-{nonce}",
        std::process::id()
    ));
    let root = base.join("workspace");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("app.ice");
    let missing = base.join("outside/nested/part.ice");
    std::fs::write(
            &source,
            "app Demo\nuse \"../outside/nested/part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"external\"\n",
        )
        .unwrap();
    let source = source.canonicalize().unwrap();

    let error = match compile_dev(&source) {
        Ok(_) => panic!("the missing external import unexpectedly compiled"),
        Err(error) => error,
    };
    assert!(error.dependencies.contains(&source));
    assert!(error.dependencies.contains(&missing));
    let absent = dev_stamps(&root, &error.dependencies, &[]);

    std::fs::create_dir_all(missing.parent().unwrap()).unwrap();
    std::fs::write(&missing, "").unwrap();
    let created = dev_stamps(&root, &error.dependencies, &[]);
    assert_ne!(created.0, absent.0);
    let compiled = compile_dev(&source).expect("the created external import must compile");
    assert!(
        compiled
            .dependencies
            .contains(&missing.canonicalize().unwrap())
    );
    std::fs::remove_dir_all(base).unwrap();
}

#[cfg(unix)]
#[test]

fn sigint_shutdown_cleans_child_and_shadow_executable() {
    const CHILD_ENV: &str = "ICE_CARGO_RUNNER_SIGINT_TEST_CHILD";
    const ROOT_ENV: &str = "ICE_CARGO_RUNNER_SIGINT_TEST_ROOT";
    if std::env::var_os(CHILD_ENV).is_some() {
        let root = PathBuf::from(std::env::var_os(ROOT_ENV).unwrap());
        install_stop_handler().unwrap();
        let executable = stage_executable(&root.join("app-sh"), 41).unwrap();
        let args = ["-c".to_owned(), "sleep 30".to_owned()];
        let app = ChildGuard::spawn_owned(&root, executable, &args).unwrap();
        std::fs::write(root.join("ready"), app.id().to_string()).unwrap();
        while !stop_requested() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(app);
        return;
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cargo-ice-sigint-cleanup-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::copy("/bin/sh", root.join("app-sh")).unwrap();
    let test_module = module_path!()
        .split_once("::")
        .map_or(module_path!(), |(_, module)| module);
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(format!(
            "{test_module}::sigint_shutdown_cleans_child_and_shadow_executable"
        ))
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .env(ROOT_ENV, &root)
        .spawn()
        .unwrap();
    let ready = root.join("ready");
    let started = std::time::Instant::now();
    while !ready.is_file() {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "signal cleanup child did not become ready"
        );
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let app_pid = std::fs::read_to_string(&ready).unwrap();
    let signal = std::process::Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(signal.success());
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() >= std::time::Duration::from_secs(10) {
            let _ = child.kill();
            panic!("signal cleanup child did not stop");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };

    assert!(
        status.success(),
        "signal cleanup child exited with {status}"
    );
    assert!(
        std::fs::read_dir(&root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .as_encoded_bytes()
                .windows(b".ice-dev-".len())
                .any(|window| window == b".ice-dev-")
        }),
        "SIGINT left a shadow executable behind"
    );
    let app_alive = std::process::Command::new("kill")
        .args(["-0", app_pid.trim()])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(!app_alive.success(), "SIGINT left the app child alive");
    std::fs::remove_dir_all(root).unwrap();
}

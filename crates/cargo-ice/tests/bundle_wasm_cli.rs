#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

// Claim: the CLI flag suppresses both optimizer discovery and execution even
// when PATH contains one. Ignoring the flag reaches the final no-call assertion.
#[test]
fn no_wasm_opt_skips_the_optimizer_process_while_default_invokes_it() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let target = root.join("target");
    let release = target.join("wasm32-unknown-unknown/release");
    fs::create_dir_all(&release).unwrap();
    fs::write(release.join("fixture.wasm"), b"fixture module").unwrap();
    fs::write(
        root.join("metadata.json"),
        serde_json::to_vec(&serde_json::json!({
            "target_directory": target,
            "packages": [{"source": null, "name": "fixture", "targets": [
                {"kind": ["cdylib"], "name": "fixture"}
            ]}]
        }))
        .unwrap(),
    )
    .unwrap();
    script(
        &root.join("cargo"),
        r#"
if [ "$1" = metadata ]; then /bin/cat "$FIXTURE_ROOT/metadata.json"; fi
"#,
    );
    script(
        &root.join("wasm-tools"),
        r#"
case "$1" in
  --version) echo 'wasm-tools fixture' ;;
  print) echo '(module)' ;;
  component) /bin/cp "$3" "$5" ;;
  strip) : ;;
  *) exit 1 ;;
esac
"#,
    );
    script(
        &root.join("wasm-opt"),
        r#"
echo "$*" >> "$FIXTURE_ROOT/optimizer.log"
if [ "$1" = --version ]; then echo 'wasm-opt fixture'; else /bin/cp "$3" "$5"; fi
"#,
    );
    let run = |flag: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-ice"));
        command
            .current_dir(root)
            .env("CARGO", root.join("cargo"))
            .env("PATH", root)
            .env("FIXTURE_ROOT", root)
            .args([
                "ice",
                "bundle",
                "-p",
                "fixture",
                "--target",
                "wasm32-unknown-unknown",
            ]);
        if flag {
            command.arg("--no-wasm-opt");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(target.join("app-store-catalog/fixture.wasm")).unwrap(),
            b"fixture module"
        );
    };
    run(false);
    let calls = fs::read_to_string(root.join("optimizer.log")).unwrap();
    assert!(calls.lines().any(|line| line == "--version"));
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("-Os --all-features ")),
        "default must execute the available optimizer: {calls}"
    );
    fs::remove_file(root.join("optimizer.log")).unwrap();
    run(true);
    assert!(
        !root.join("optimizer.log").exists(),
        "--no-wasm-opt must suppress all optimizer calls, including discovery"
    );
}

#[test]
fn no_wasm_opt_refuses_native_and_mixed_targets_before_running_tools() {
    for targets in [
        vec![],
        vec!["--target", "x86_64-unknown-linux-gnu"],
        vec![
            "--target",
            "wasm32-unknown-unknown",
            "--target",
            "x86_64-unknown-linux-gnu",
        ],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-ice"))
            .current_dir(dir.path())
            .env("PATH", dir.path())
            .args(["ice", "bundle", "-p", "fixture", "--no-wasm-opt"])
            .args(targets)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("--no-wasm-opt requires --target wasm32-unknown-unknown")
        );
    }
}

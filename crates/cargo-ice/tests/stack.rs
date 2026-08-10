//! Ice analysis recurses over the source graph deeply enough that a small main
//! thread stack aborts it. Windows gives one megabyte by default, so this
//! reproduces that budget on a platform where it can be set from a test.

#![cfg(target_os = "linux")]

use std::path::Path;
use std::process::Command;

/// Every `cargo ice` command that analyzes a graph has to survive the stack a
/// Windows main thread carries. `ulimit -s` bounds the process's main thread
/// only: a thread created with an explicit stack size is mapped outside that
/// limit, which is exactly the difference this checks.
#[test]
fn analysis_survives_the_stack_windows_gives_a_main_thread() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = env!("CARGO_BIN_EXE_cargo-ice");

    for command in ["api", "expand"] {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "ulimit -s 1024 && exec '{binary}' {command} examples/showcase/src/ui/app.ice"
            ))
            .current_dir(&workspace)
            .output()
            .expect("run cargo-ice under a one-megabyte main stack");
        let reported = String::from_utf8_lossy(&output.stderr);
        assert!(
            !reported.contains("overflowed its stack"),
            "`cargo ice {command}` overflowed a one-megabyte main stack:\n{reported}"
        );
        assert!(
            output.status.success(),
            "`cargo ice {command}` failed under a one-megabyte main stack:\n{reported}"
        );
    }
}

//! A panic unwinding through a `dev::Span` names the `.ice` construct.
//!
//! The report is an `eprintln!` from a `Drop` running during unwind, so the
//! only way to read it is from outside the process: the test re-executes its
//! own binary with `ICE_SPAN_PANIC_CHILD` set, lets the child panic under two
//! nested spans, and reads what the child wrote to stderr.

use std::process::Command;

const CHILD: &str = "ICE_SPAN_PANIC_CHILD";

#[test]
fn a_panic_under_two_spans_names_both_innermost_first() {
    if std::env::var_os(CHILD).is_some() {
        let _handler = ui_lang_runtime::dev::Span::handler("refresh", "src/ui/app.ice:52");
        let _call = ui_lang_runtime::dev::Span::extern_call("load_prices", "src/ui/app.ice:16");
        panic!("the extern's own body panicked");
    }

    let output = Command::new(std::env::current_exe().expect("the test binary has a path"))
        .arg("a_panic_under_two_spans_names_both_innermost_first")
        .args(["--exact", "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("the test binary re-executes");
    let stderr = String::from_utf8_lossy(&output.stderr);

    let extern_line = stderr
        .find("ice: panic while running extern `load_prices`, at src/ui/app.ice:16")
        .unwrap_or_else(|| panic!("no extern line in:\n{stderr}"));
    let handler_line = stderr
        .find("ice: panic while running handler `refresh`, at src/ui/app.ice:52")
        .unwrap_or_else(|| panic!("no handler line in:\n{stderr}"));
    assert!(
        extern_line < handler_line,
        "the call inside the turn reports first:\n{stderr}"
    );
}

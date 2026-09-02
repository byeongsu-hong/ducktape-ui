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

    let stderr = child_stderr("a_panic_under_two_spans_names_both_innermost_first");

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

/// A view span starts no clock unless `ICE_PERF` names a budget, and it still
/// has to name the view on the way out of a panic: it is the only `.ice`
/// reading a `pure` extern's panic gets.
#[test]
fn an_unbudgeted_view_span_still_names_the_view() {
    if std::env::var_os(CHILD).is_some() {
        let _view = ui_lang_runtime::dev::Span::view("Starter", "src/ui/app.ice:20");
        panic!("a pure extern panicked while the view was being built");
    }

    let stderr = child_stderr("an_unbudgeted_view_span_still_names_the_view");
    assert!(
        stderr.contains("ice: panic while running view `Starter`, at src/ui/app.ice:20"),
        "no view line in:\n{stderr}"
    );
}

/// Runs one of this file's tests in a child process and returns its stderr.
/// `ICE_PERF` is cleared so the child measures only what its build profile
/// measures on its own.
fn child_stderr(test: &str) -> String {
    let output = Command::new(std::env::current_exe().expect("the test binary has a path"))
        .arg(test)
        .args(["--exact", "--nocapture"])
        .env(CHILD, "1")
        .env_remove(ui_lang_runtime::dev::BUDGET_ENV)
        .output()
        .expect("the test binary re-executes");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

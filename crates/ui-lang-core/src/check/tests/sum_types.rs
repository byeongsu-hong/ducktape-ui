use crate::analyze;

const PREFIX: &str = r#"app Demo
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
"#;

#[test]
fn checks_option_result_and_ui_enum_patterns() {
    let source = format!(
        r#"{PREFIX}enum RequestState
  idle
  loading
  ready([str])
  failed(str)
state
  choice:str? = some("selected")
  outcome:result[str,str] = ok("done")
  request:RequestState = RequestState.ready(["one", "two"])
view
  col
    match choice
      some(value)
        text value
      none
        text "none"
    match outcome
      ok(value)
        text value
      err(error)
        text error
    match request
      RequestState.idle
        text "idle"
      RequestState.loading
        text "loading"
      RequestState.ready(items)
        text len(items)
      RequestState.failed(error)
        text error
"#
    );

    analyze(&source).unwrap();
}

#[test]
fn requires_exhaustive_typed_patterns() {
    for (declarations, state, arms, missing) in [
        (
            "",
            "value:str? = none",
            "some(value)\n        text value",
            "none",
        ),
        (
            "",
            "value:result[str,str] = ok(\"yes\")",
            "ok(value)\n        text value",
            "err",
        ),
        (
            "enum Screen\n  home\n  settings",
            "value:Screen = Screen.home",
            "Screen.home\n        text \"home\"",
            "Screen.settings",
        ),
    ] {
        let source = format!(
            "{PREFIX}{declarations}\nstate\n  {state}\nview\n  col\n    match value\n      {arms}\n"
        );
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E195");
        assert!(error.message.contains(missing), "{}", error.message);
    }
}

#[test]
fn keeps_pattern_payloads_block_local() {
    let source = format!(
        r#"{PREFIX}state
  choice:str? = none
view
  col
    match choice
      some(value)
        text value
      none
        text "none"
    text value
"#
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.message.contains("unknown value `value`"));
}

#[test]
fn rejects_recursive_and_non_cloneable_enum_payloads() {
    for (declaration, message) in [
        ("enum Node\n  next(Node)", "recursive enum `Node`"),
        (
            "enum Work\n  running(task-handle)",
            "enum payloads support ordinary cloneable data only",
        ),
    ] {
        let source = format!("{PREFIX}{declaration}\nview\n  text \"ok\"\n");
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E103");
        assert!(error.message.contains(message), "{}", error.message);
    }
}

#[test]
fn rejects_enum_names_that_collide_after_rust_lowering() {
    let source = format!("{PREFIX}enum Choice\n  foo2\n  foo_2\nview\n  text \"ok\"\n");
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E100");
    assert!(error.message.contains("generated variant name"));
}

#[test]
fn keeps_clone_only_ui_enums_out_of_comparison_and_lazy_hashing() {
    for view in [
        "text (screen == Screen.home)",
        "lazy screen as value #screen\n    text \"screen\"",
    ] {
        let source = format!(
            "{PREFIX}enum Screen\n  home\nstate\n  screen:Screen = Screen.home\nview\n  {view}\n"
        );
        let error = analyze(&source).unwrap_err();
        assert!(matches!(error.code, "E139" | "E153"));
    }
}

#[test]
fn literal_match_remains_non_exhaustive_first_match_control_flow() {
    let source = format!(
        r#"{PREFIX}state
  count = 1
view
  col
    match count
      0
        text "zero"
"#
    );
    analyze(&source).unwrap();
}

use crate::analyze;

const PREFIX: &str = r#"app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
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
fn checks_exhaustive_handler_enum_dispatch() {
    let valid = format!(
        r#"{PREFIX}enum LiveKind
  chat
  tip
state
  kind:LiveKind = LiveKind.chat
  count = 0
on update
  match kind
    LiveKind.chat
      count = 1
    LiveKind.tip
      count = 2
view
  button "Update" -> update
"#
    );
    analyze(&valid).unwrap();

    let missing = valid.replace("    LiveKind.tip\n      count = 2\n", "");
    let error = analyze(&missing).unwrap_err();
    assert_eq!(error.code, "E195");
    assert!(error.message.contains("missing LiveKind.tip"));

    let duplicate = valid.replace("    LiveKind.tip", "    LiveKind.chat");
    let error = analyze(&duplicate).unwrap_err();
    assert_eq!(error.code, "E195");
    assert!(error.message.contains("duplicate `LiveKind.chat`"));

    let wildcard = valid.replace("    LiveKind.tip", "    _");
    let error = analyze(&wildcard).unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("`Enum.variant`"));

    let foreign = valid.replace("    LiveKind.tip", "    Other.tip");
    let error = analyze(&foreign).unwrap_err();
    assert_eq!(error.code, "E195");
    assert!(error.message.contains("expected `LiveKind` pattern"));

    let ill_typed = valid.replace("count = 2", "count = \"two\"");
    let error = analyze(&ill_typed).unwrap_err();
    assert!(
        error.message.contains("expected `i64`"),
        "{}",
        error.message
    );

    let non_final = valid.replace("view\n", "  count = 3\nview\n");
    let error = analyze(&non_final).unwrap_err();
    assert_eq!(error.code, "E141");
    assert!(error.message.starts_with("handler match"));

    let payload = format!(
        r#"{PREFIX}enum Request
  ready(str)
state
  request:Request = Request.ready("done")
on update
  match request
    Request.ready
      return if true
view
  button "Update" -> update
"#
    );
    let error = analyze(&payload).unwrap_err();
    assert_eq!(error.code, "E195");
    assert!(error.message.contains("must be fieldless"));
}

#[test]
fn handler_match_routes_participate_in_reachability() {
    let source = format!(
        r#"{PREFIX}extern crate::backend
  fetch(value:i64) -> i64
enum LiveKind
  chat
  tip
state
  kind:LiveKind = LiveKind.chat
  count = 0
on update
  match kind
    LiveKind.chat
      run every fetch(1) -> chat_done _
    LiveKind.tip
      run every fetch(2) -> tip_done _
on chat_done(value)
  count = value
on tip_done(value)
  count = value
view
  button "Update" -> update
"#
    );
    let document = analyze(&source).unwrap();
    let unreachable = document
        .warnings()
        .iter()
        .filter(|warning| warning.code == "W005")
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>();
    assert!(unreachable.is_empty(), "{unreachable:?}");
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
fn compares_fieldless_ui_enums_but_keeps_them_out_of_lazy_hashing() {
    let source = format!(
        "{PREFIX}enum Screen\n  home\n  settings\nstate\n  screen:Screen = Screen.home\nview\n  if screen == Screen.home\n    text \"home\"\n"
    );
    analyze(&source).unwrap();

    let lazy = format!(
        "{PREFIX}enum Screen\n  home\n  settings\nstate\n  screen:Screen = Screen.home\nview\n  lazy screen as value #screen\n    text \"screen\"\n"
    );
    let error = analyze(&lazy).unwrap_err();
    assert_eq!(error.code, "E139");

    let ordering = format!(
        "{PREFIX}enum Screen\n  home\n  settings\nstate\n  screen:Screen = Screen.home\nview\n  text (screen < Screen.settings)\n"
    );
    let error = analyze(&ordering).unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("ordering is undefined"));
}

#[test]
fn keeps_payload_ui_enums_out_of_comparison() {
    let source = format!(
        "{PREFIX}enum Request\n  ready(str)\nstate\n  request:Request = Request.ready(\"done\")\nview\n  text (request == Request.ready(\"done\"))\n"
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("payload-carrying UI enum"));
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

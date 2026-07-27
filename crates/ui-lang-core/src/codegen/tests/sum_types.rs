use crate::compile;

#[test]
fn lowers_ui_enums_and_typed_matches() {
    let source = r#"app Demo
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
enum RequestState
  idle
  ready([str])
enum Tab
  preview
  code
state
  choice:str? = some("selected")
  outcome:result[str,str] = err("failed")
  request:RequestState = RequestState.ready(["one"])
  tab:Tab = Tab.preview
view
  col
    if tab == Tab.preview
      text "preview"
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
      RequestState.ready(items)
        text len(items)
"#;

    let generated = compile(source, "sum_types.ice").unwrap();
    assert!(generated.contains("pub(crate) enum RequestState"));
    assert!(
        generated.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub(crate) enum Tab")
    );
    assert!(generated.contains("Idle,"));
    assert!(generated.contains("Ready(::std::vec::Vec<::std::string::String>)"));
    assert!(generated.contains("RequestState::Ready(::std::vec!"));
    assert!(generated.contains("::std::option::Option::Some(value) =>"));
    assert!(generated.contains("::std::result::Result::Err(error) =>"));
    assert!(generated.contains("RequestState::Ready(items) =>"));
}

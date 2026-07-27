use super::*;

#[test]
fn rejects_a_utility_that_the_widget_would_ignore() {
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
view
  text "hello" @gap-4
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E042");
    assert!(error.message.contains("no effect on `text`"));
}

#[test]
fn names_an_undeclared_extern_type() {
    let source = r#"app Demo
extern crate::backend
  load() -> [Missing]
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
view
  text "hello"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("`Missing`"));
}

#[test]
fn requires_a_route_for_an_emitting_extern_component() {
    let source = r#"app Demo
extern crate::backend
  component native_control() -> bool
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
view
  extern native_control()
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E126");
    assert!(error.message.contains("requires a route"));
}

#[test]
fn checks_native_shader_programs() {
    let source = r#"app Demo
extern crate::backend
  shader native_shader(value:f64) -> bool
  shader passive_shader() -> unit
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
state
  amount = 1.0
on shaded(active)
view
  col
    shader native_shader(amount) w=fill h=64.0 -> shaded _
    shader passive_shader()
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace(" -> shaded _", "")).unwrap_err();
    assert_eq!(error.code, "E191");
    assert!(error.message.contains("requires a route"));

    let error = analyze(&source.replace("h=64.0", "h=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("shader size"));

    let error =
        analyze(&source.replace("native_shader(amount)", "native_shader(true)")).unwrap_err();
    assert!(error.message.contains("expected `f64`"));

    let error = analyze(&source.replace("h=64.0", "depth=64.0")).unwrap_err();
    assert_eq!(error.code, "E191");
    assert!(error.message.contains("unknown shader property"));

    let error = analyze(&source.replace(
        "shader native_shader(value:f64) -> bool",
        "shader native_shader(value:f64) -> bool ! bool",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E023");
}

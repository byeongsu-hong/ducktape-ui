use crate::test_support::example;
use crate::{PaneConfiguration, Type, ViewNode, analyze};

#[path = "tests/components.rs"]
mod components;
#[path = "tests/events.rs"]
mod events;
#[path = "tests/native.rs"]
mod native;
#[path = "tests/platform.rs"]
mod platform;
#[path = "tests/sum_types.rs"]
mod sum_types;
#[path = "tests/tasks.rs"]
mod tasks;
#[path = "tests/testing.rs"]
mod testing;
#[path = "tests/widgets.rs"]
mod widgets;

const THEMED_APP: &str = r#"app Demo
  palette active_palette
theme contract Ducktape
  bg
  fg
  primary
  danger
  surface
palette light for Ducktape
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
  surface #f4f4f4
palette dark for Ducktape
  bg #111111
  fg #ffffff
  primary #88aaff
  danger #ff6677
  surface #222222
state
  active_palette:palette[Ducktape] = Ducktape.light
view
  box bg=surface
    text "Theme"
"#;

#[test]
fn checks_complete_dynamic_palettes() {
    analyze(THEMED_APP).unwrap();
}

#[test]
fn exhaustively_matches_nominal_palettes() {
    let source = THEMED_APP.replace(
        "  box bg=surface\n    text \"Theme\"",
        "  match active_palette\n    Ducktape.light\n      text \"Light\"\n    Ducktape.dark\n      text \"Dark\"",
    );
    analyze(&source).unwrap();

    let error =
        analyze(&source.replace("    Ducktape.dark\n      text \"Dark\"\n", "")).unwrap_err();
    assert_eq!(error.code, "E195");
    assert!(error.message.contains("Ducktape.dark"));
}

#[test]
fn rejects_invalid_theme_contracts_and_palettes() {
    for (source, message) in [
        (
            THEMED_APP.replace("  danger\n  surface", "  surface"),
            "missing `danger`",
        ),
        (
            THEMED_APP.replace("palette light for Ducktape", "palette light for Other"),
            "not `Ducktape`",
        ),
        (
            THEMED_APP.replace("  surface #f4f4f4", "  accent #f4f4f4"),
            "unknown token `accent`",
        ),
        (
            THEMED_APP.replace("  surface #f4f4f4\npalette dark", "palette dark"),
            "missing token `surface`",
        ),
        (
            THEMED_APP.replace(
                "  active_palette:palette[Ducktape] = Ducktape.light",
                "  active_palette:palette[Ducktape] = true",
            ),
            "expected `palette[Ducktape]`, got `bool`",
        ),
        (
            THEMED_APP.replace("Ducktape.light", "\"light\""),
            "expected `palette[Ducktape]`, got `str`",
        ),
        (
            THEMED_APP.replace("palette active_palette", "palette Ducktape.missing"),
            "has no palette `missing`",
        ),
    ] {
        let error = analyze(&source).unwrap_err();
        assert!(
            error.message.contains(message),
            "{}: {}",
            error.code,
            error.message
        );
    }
}

#[test]
fn rejects_duplicate_or_non_color_palette_entries_during_parsing() {
    for (source, message) in [
        (
            THEMED_APP.replace("  surface #f4f4f4", "  surface #f4f4f4\n  surface #eeeeee"),
            "duplicate palette token `surface`",
        ),
        (
            THEMED_APP.replace("  surface #f4f4f4", "  surface blue"),
            "palette colors use #RRGGBB or #RRGGBBAA",
        ),
        (
            THEMED_APP.replace(
                "palette light for Ducktape",
                "palette light for Ducktape\npalette light for Ducktape",
            ),
            "duplicate palette `light`",
        ),
    ] {
        let error = analyze(&source).unwrap_err();
        assert!(
            error.message.contains(message),
            "{}: {}",
            error.code,
            error.message
        );
    }
}

#[test]
fn rejects_invalid_constant_integer_arithmetic() {
    for (expression, message) in [
        ("1 / 0", "non-zero divisor"),
        ("1 % -0", "non-zero divisor"),
        ("1 / (2 - 2)", "non-zero divisor"),
        ("9223372036854775807 + 1", "overflows"),
        ("-9223372036854775808 / -1", "overflows"),
    ] {
        let source = example!("component_state.ice")
            .replace("count = 0", &format!("count:i64 = {expression}"));
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E153");
        assert!(error.message.contains(message));
    }

    let error = analyze(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  value = 1\nview\n  text (value / (1 - 1))\n",
    )
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("non-zero divisor"));
}

#[test]
fn rejects_duplicate_handler_parameters() {
    for source in [
        r#"app Demo
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
on pressed(value, value)
view
  button "ok" -> pressed(1, 2)
"#,
        r#"app Demo
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
component Card()
  on pressed(value, value)
  button "ok" -> pressed(1, 2)
view
  Card
"#,
    ] {
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E100");
        assert!(
            error
                .message
                .contains("duplicate handler parameter `value`")
        );
    }
}

#[test]
fn checks_derived_values_and_immutable_handler_locals() {
    let source = r#"app Demo
extern crate::backend
  sync normalize(value:str) -> str
  save(title:str) -> unit
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
  draft = ""
  loading = false
derived
  normalized = trim(draft)
  can_submit = !loading && !empty(normalized)
on submit
  let title = normalized
  return if !can_submit
  run save(title) -> saved
on saved
  draft = ""
view
  col
    input "Draft" <-> draft
    button "Save" disabled=!can_submit -> submit
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.derived[0].ty, Type::Str);
    assert_eq!(document.derived[1].ty, Type::Bool);

    let forward = source.replace(
        "normalized = trim(draft)\n  can_submit = !loading && !empty(normalized)",
        "can_submit = !loading && !empty(normalized)\n  normalized = trim(draft)",
    );
    analyze(&forward).unwrap();

    let cycle = source.replace(
        "normalized = trim(draft)\n  can_submit = !loading && !empty(normalized)",
        "normalized = can_submit\n  can_submit = normalized",
    );
    let error = analyze(&cycle).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("dependency cycle"));

    let impure = source.replace("normalized = trim(draft)", "normalized = normalize(draft)");
    let error = analyze(&impure).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("pure Ice expression"));

    let shadow = source.replace("let title = normalized", "let draft = normalized");
    let error = analyze(&shadow).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("shadows an existing value"));

    let duplicate_local = source.replace(
        "let title = normalized",
        "let title = normalized\n  let title = normalized",
    );
    let error = analyze(&duplicate_local).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("shadows an existing value"));

    let parameter_shadow = source
        .replace("on submit\n", "on submit(value)\n")
        .replace("let title = normalized", "let value = normalized")
        .replace("-> submit\n", "-> submit(draft)\n");
    let error = analyze(&parameter_shadow).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("shadows an existing value"));

    let assignment = source.replace("draft = \"\"\nview", "can_submit = false\nview");
    let error = analyze(&assignment).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("not writable state"));

    let binding = source.replace("<-> draft", "<-> normalized");
    let error = analyze(&binding).unwrap_err();
    assert!(
        error.message.contains("writable")
            || error.message.contains("state binding")
            || error.message.contains("app state"),
        "{}",
        error.message
    );
}

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
#[path = "tests/tasks.rs"]
mod tasks;
#[path = "tests/testing.rs"]
mod testing;
#[path = "tests/widgets.rs"]
mod widgets;

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
        "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  value = 1\nview\n  text (value / (1 - 1))\n",
    )
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("non-zero divisor"));
}

#[test]
fn rejects_duplicate_handler_parameters() {
    for source in [
        r#"app Demo
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
on pressed(value, value)
view
  button "ok" -> pressed(1, 2)
"#,
        r#"app Demo
theme
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

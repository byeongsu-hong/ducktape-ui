use crate::compile;
use crate::test_support::example;

#[test]
fn keeps_generated_rust_names_distinct() {
    assert_ne!(
        super::handler_variant("foo_bar"),
        super::handler_variant("fooBar")
    );
    assert_ne!(
        super::binding_variant("foo_bar"),
        super::binding_variant("fooBar")
    );
    assert_ne!(
        super::component_state_field("SearchBox"),
        super::component_state_field("Searchbox")
    );
    assert_ne!(
        super::component_state_type("PaneWork"),
        super::pane_type("work_state")
    );
    assert_ne!(
        super::component_state_type("EventFilterFoo"),
        super::event_filter_type("foo_state")
    );
    assert_ne!(
        super::event_filter_type("foo2"),
        super::event_filter_type("foo_2")
    );
    assert_ne!(super::pane_type("foo2"), super::pane_type("foo_2"));
    assert_ne!(
        super::pane_field("work_splits"),
        super::pane_splits_field("work")
    );
    assert_ne!(
        super::component_handler_variant("Pane", "work_resize"),
        super::pane_resize_variant("handle_work")
    );
    assert_ne!(
        super::component_binding_variant("Bind", "foo"),
        super::binding_variant("bind_foo")
    );
    assert_ne!(
        super::canvas_group_symbol("drawings"),
        super::canvas_group_symbol("DRAWINGS")
    );
}

#[test]
fn declared_sync_calls_shadow_simple_builtins() {
    let source = r#"app Demo
extern crate::backend
  sync len(value:str) -> bool
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
  matched:bool = len("value")
view
  text "ok"
"#;

    let generated = compile(source, "app.ice").unwrap();

    assert!(generated.contains("crate::backend::len(\"value\".to_owned())"));
}

#[path = "tests/application.rs"]
mod application;
#[path = "tests/components.rs"]
mod components;
#[path = "tests/controls.rs"]
mod controls;
#[path = "tests/flows.rs"]
mod flows;
#[path = "tests/graphics.rs"]
mod graphics;
#[path = "tests/layout.rs"]
mod layout;
#[path = "tests/platform.rs"]
mod platform;
#[path = "tests/render_coverage.rs"]
mod render_coverage;
#[path = "tests/sum_types.rs"]
mod sum_types;
#[path = "tests/testing.rs"]
mod testing;

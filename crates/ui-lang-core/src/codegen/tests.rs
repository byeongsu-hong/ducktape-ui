use crate::compile;
use crate::test_support::example;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

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
fn marker_free_lines_skip_marker_resolution() {
    let program =
        crate::lower::lower(crate::analyze(example!("native_overlay.ice")).unwrap()).unwrap();
    let generated = "let marker_free = 0;\n".repeat(4_000);
    let expected = generated.clone();

    super::SOURCE_MARKER_SLOW_PATH_LINES.set(0);
    let resolved = super::resolve_source_markers(generated, &program, "app.ice");

    assert_eq!(resolved, expected);
    assert_eq!(super::SOURCE_MARKER_SLOW_PATH_LINES.get(), 0);
}

#[test]
#[ignore = "allocation contract; run alone with --test-threads=1"]
fn source_path_hex_encoding_uses_one_exact_allocation() {
    const BYTES: usize = 4_096;
    let source = "x".repeat(BYTES);
    let expected = "78".repeat(BYTES);
    let region = Region::new(GLOBAL);

    let encoded = std::hint::black_box(super::encode_source_path(std::hint::black_box(
        source.as_str(),
    )));
    let stats = region.change();

    eprintln!(
        "{BYTES} source-path bytes: {} allocations / {} reallocations / {} allocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(source.len(), BYTES);
    assert_eq!(encoded.len(), BYTES * 2);
    assert_eq!(encoded, expected);
    assert_eq!(stats.allocations, 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, BYTES * 2, "{stats:?}");
}

#[test]
#[ignore = "allocation contract; run alone with --test-threads=1"]
fn rust_identifier_hex_encoding_uses_one_allocation_per_name() {
    const CALLS: usize = 4_000;
    const BYTES: usize = 17;
    let name = "noncanonical-name";
    let expected = "6e6f6e63616e6f6e6963616c2d6e616d65";
    assert_eq!(name.len(), BYTES);
    let region = Region::new(GLOBAL);

    for _ in 0..CALLS {
        let encoded = std::hint::black_box(super::rust_identifier_hex(std::hint::black_box(name)));
        assert_eq!(encoded, expected);
    }
    let stats = region.change();

    eprintln!(
        "{CALLS} Rust identifier encodings: {} allocations / {} reallocations / {} allocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, CALLS, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, CALLS * BYTES * 2, "{stats:?}");
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

#[test]
fn isolates_generated_items_from_consumer_lints() {
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
  text "ok"
"#;

    let generated = compile(source, "app.ice").unwrap();

    assert!(generated.starts_with(
        "macro_rules! __ice_generated_items_6170702e696365 { ($($item:item)*) => { $(#[allow(warnings, clippy::all)] $item)* }; }\n__ice_generated_items_6170702e696365! {\n"
    ));
    assert!(generated.ends_with("}\n"));
}

#[path = "tests/application.rs"]
mod application;
#[path = "tests/components.rs"]
mod components;
#[path = "tests/controls.rs"]
mod controls;
#[path = "tests/derived_cache.rs"]
mod derived_cache;
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
#[path = "tests/request_lanes.rs"]
mod request_lanes;
#[path = "tests/route_snapshots.rs"]
mod route_snapshots;
#[path = "tests/sum_types.rs"]
mod sum_types;
#[path = "tests/testing.rs"]
mod testing;

#[test]
fn shown_paths_are_manifest_relative_and_foreign_paths_stay_whole() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        super::shown_path(&manifest.join("src/ui/app.ice")),
        "src/ui/app.ice"
    );
    assert_eq!(
        super::shown_path(std::path::Path::new("/elsewhere/app.ice")),
        "/elsewhere/app.ice"
    );
}

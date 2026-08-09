use super::*;

const PREFIX: &str = r#"extern crate::backend
  Row(label:str)
  pure filter(rows:[Row]) -> [Row]
  pure to_f64(value:i64) -> f64
  text-style deferred(value:i64)
  scroll-style deferred_scroll(value:i64)
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
  rows:[Row] = []
derived
  visible = filter(rows)
"#;

fn assert_deferred_reads_stay_on_getters(name: &str, body: &str) {
    let source = format!("app {name}\n{PREFIX}{body}");
    let generated = compile(&source, "deferred.ice").unwrap();

    assert!(
        !generated.contains("struct __IceDerivedSnapshot"),
        "{name} retained a view-local derived snapshot in a deferred closure"
    );
    assert!(
        generated.matches("self.__ice_derived_visible()").count() >= 2,
        "{name} did not preserve both deferred getter reads"
    );
}

#[test]
fn keeps_callback_route_derived_reads_out_of_view_snapshots() {
    assert_deferred_reads_stay_on_getters(
        "RouteEscape",
        r#"on observed(value)
view
  col
    checkbox "First" checked=false -> observed (len(visible))
    checkbox "Second" checked=false -> observed (len(visible))
"#,
    );
}

#[test]
fn keeps_dynamic_style_derived_reads_out_of_view_snapshots() {
    assert_deferred_reads_stay_on_getters(
        "StyleEscape",
        r#"view
  col
    text "First" style=deferred(len(visible))
    text "Second" style=deferred(len(visible))
"#,
    );
}

#[test]
fn keeps_pane_body_derived_reads_out_of_view_snapshots() {
    assert_deferred_reads_stay_on_getters(
        "PaneEscape",
        r#"view
  panes #work
    pane content
      col
        text len(visible)
        text len(visible)
"#,
    );
}

#[test]
fn keeps_pane_callback_derived_reads_out_of_view_snapshots() {
    assert_deferred_reads_stay_on_getters(
        "PaneCallbackEscape",
        r#"on observed(value)
view
  col
    panes #first click=observed(len(visible))
      pane content
        text "First"
    panes #second click=observed(len(visible))
      pane content
        text "Second"
"#,
    );
}

#[test]
fn keeps_table_cell_derived_reads_out_of_view_snapshots() {
    assert_deferred_reads_stay_on_getters(
        "TableEscape",
        r#"view
  table row in rows
    col
      header
        text "Rows"
      cell
        col
          text len(visible)
          text len(visible)
"#,
    );
}

#[test]
fn keeps_canvas_callback_derived_reads_out_of_view_snapshots() {
    assert_deferred_reads_stay_on_getters(
        "CanvasEscape",
        r#"view
  canvas
    text len(visible) x=0.0 y=0.0
    text len(visible) x=0.0 y=20.0
"#,
    );
}

#[test]
fn keeps_scroll_style_derived_reads_out_of_view_snapshots() {
    assert_deferred_reads_stay_on_getters(
        "ScrollStyleEscape",
        r#"view
  col
    scroll style=deferred_scroll(len(visible))
      text "First"
    scroll style=deferred_scroll(len(visible))
      text "Second"
"#,
    );
}

#[test]
fn keeps_float_callback_derived_reads_out_of_view_snapshots() {
    assert_deferred_reads_stay_on_getters(
        "FloatEscape",
        r#"view
  float x=to_f64(len(visible)) y=to_f64(len(visible))
    text "Floating"
"#,
    );
}

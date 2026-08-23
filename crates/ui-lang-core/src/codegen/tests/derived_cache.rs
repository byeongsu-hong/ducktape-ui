use super::*;

const THEME: &str = r#"theme contract AppTheme
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

/// Every app-state write form the language has, each with a derived value
/// reading the written field: plain and self-moving assignment, combo
/// replacement and `push`, animation `go`, markdown `append`, `abortable`
/// handle capture, `debug start`/`finish`, a controlled `input` (directly and
/// through a component `bind` prop), a controlled `editor` with and without
/// an action adapter, and a secret wipe.
fn every_write_form() -> String {
    format!(
        r#"app Writes
extern crate::backend
  Row(label:str)
  pure filter(rows:[Row]) -> [Row]
  pure appended(rows:[Row], label:str) -> [Row]
  editor-action track_edits()
{THEME}secret token
state
  rows:[Row] = []
  draft = ""
  body:editor = ""
  notes:editor = ""
  modes:combo[str] = ["List"]
  progress:animation[f64] = 0.0
  docs:markdown = "Docs"
  request:task-handle? = none
  timer:debug-span? = none
  count = 0
derived
  visible = filter(rows)
  total = len(visible) + count
  has_draft = !empty(draft)
  body_lines = editor_line_count(body)
  note_lines = editor_line_count(notes)
  progress_value = animation.value(progress)
  doc_text = markdown_images(docs)
  timing = debug.active(timer)
  counted = count * 2
component Field(bind value:str)
  input "Field" <-> value
on add
  rows = appended(rows, draft)
on reset
  rows = []
  draft = ""
  token = ""
on bump
  count = count + 1
on mode
  modes = [draft]
on more
  combo modes push "Board"
on animate
  progress = 1.0
on note
  markdown docs append draft
on start
  abortable request abort-on-drop
    task system theme -> themed _
on themed(next)
on begin
  debug start "span" -> timer
on finish
  debug finish timer
view
  col
    input "Draft" <-> draft submit=add
    button "Mode" -> mode
    button "Note" -> note
    button "More" -> more
    button "Reset" -> reset
    button "Bump" -> bump
    button "Animate" -> animate
    button "Start" -> start
    button "Begin" -> begin
    button "Finish" -> finish
    input "Token" <-> token
    Field value<->draft
    editor <-> body
    editor <-> notes action=track_edits()
    text total
    if has_draft
      text "Drafting"
    text body_lines
    text note_lines
    text progress_value
    text len(doc_text)
    if timing
      text "Timing"
    text counted
"#
    )
}

/// The generated line of every write to `self.<field>`: an assignment or an
/// in-place mutation the statement and update emitters produce. Each emitter
/// writes one statement per line, so the line is the unit a write and its
/// cache clears share.
fn write_lines<'a>(generated: &'a str, field: &str) -> Vec<&'a str> {
    let receiver = format!("self.{field}");
    let mutators = [
        " = ",
        ".push(",
        ".push_str(",
        ".go_mut(",
        ".perform(",
        ".take()",
    ];
    let mut lines = Vec::new();
    for line in generated.lines() {
        let writes = line.match_indices(&receiver).any(|(index, _)| {
            let rest = &line[index + receiver.len()..];
            let next = rest.chars().next().unwrap_or(' ');
            !(next.is_alphanumeric() || next == '_')
                && (line[..index].ends_with("&mut ")
                    || mutators.iter().any(|mutator| rest.starts_with(mutator)))
        });
        if writes {
            lines.push(line);
        }
    }
    lines
}

/// The derived cells a generated line clears, in order.
fn cleared_cells(line: &str) -> Vec<&str> {
    line.match_indices("self.__ice_derived.")
        .filter_map(|(index, marker)| {
            let rest = &line[index + marker.len()..];
            rest.split_once(".take();").map(|(name, _)| name)
        })
        .collect()
}

#[test]
fn every_app_state_write_clears_the_derived_cells_that_read_the_field() {
    let generated = compile(&every_write_form(), "writes.ice").unwrap();
    // Field, the write sites the fixture produces for it, and the derived
    // values a write must clear. A combo or task handle is not readable from a
    // derived expression, so their writes route through the helper and clear
    // nothing.
    let fields = [
        ("rows", 2, &["visible", "total"][..]),
        ("draft", 2, &["has_draft"]),
        ("body", 1, &["body_lines"]),
        ("notes", 1, &["note_lines"]),
        ("modes", 2, &[]),
        ("progress", 1, &["progress_value"]),
        ("docs", 1, &["doc_text"]),
        ("request", 1, &[]),
        ("timer", 2, &["timing"]),
        ("count", 1, &["total", "counted"]),
    ];
    for (field, expected, derived) in fields {
        let lines = write_lines(&generated, field);
        assert_eq!(lines.len(), expected, "`{field}` write lines: {lines:#?}");
        for line in lines {
            assert_eq!(
                cleared_cells(line),
                derived,
                "a write to `{field}` must clear exactly its dependents:\n  {line}"
            );
        }
    }
    // The secret wipe routes through the same helper with its secret as the
    // target; a secret never joins the derived expression scope, so no cell is
    // cleared there (or in the typed-secret update arm, which writes the store
    // directly).
    assert!(generated.contains("self.__ice_secrets.clear(\"token\");\n"));
    assert!(!generated.contains("self.__ice_secrets.clear(\"token\"); self.__ice_derived"));
}

#[test]
fn caches_every_derived_value_on_the_app_struct() {
    let source = format!(
        r#"app Cached
extern crate::backend
  Row(label:str)
  pure filter(rows:[Row]) -> [Row]
  pure display(rows:[Row]) -> str
{THEME}state
  rows:[Row] = []
  count = 0
derived
  visible = filter(rows)
  shown = len(visible)
  doubled = count * 2
on bump
  count = doubled + 1
  rows = visible
view
  col
    text len(visible)
    text display(visible)
    for row in visible
      text row.label
    text shown
    text doubled
"#
    );
    let generated = compile(&source, "cached.ice").unwrap();

    assert!(generated.contains(
        "#[derive(Default)]\nstruct __IceDerivedCache {\nvisible: ::std::cell::OnceCell<::std::vec::Vec<crate::backend::Row>>,\nshown: ::std::cell::OnceCell<i64>,\ndoubled: ::std::cell::OnceCell<i64>,\n}"
    ));
    assert!(generated.contains("pub(crate) __ice_derived: __IceDerivedCache,"));
    assert!(generated.contains("__ice_derived: ::std::default::Default::default(),"));
    assert!(generated.contains(
        "fn __ice_derived_visible(&self) -> &::std::vec::Vec<crate::backend::Row> { self.__ice_derived.visible.get_or_init(|| crate::backend::filter(self.rows.clone())) }"
    ));
    assert!(generated.contains(
        "fn __ice_derived_shown(&self) -> &i64 { self.__ice_derived.shown.get_or_init(|| ((*self.__ice_derived_visible())).len() as i64) }"
    ));
    // Reads borrow the cached reference and clone only where the use site
    // needs ownership.
    assert!(generated.contains("crate::backend::display((*self.__ice_derived_visible()).clone())"));
    assert!(generated.contains("((*self.__ice_derived_visible())).len() as i64"));
    // The clears ride with the revision tick, inside the compare: an
    // equal-value write leaves the cells as valid as they were.
    assert!(generated.contains(
        "{ let __ice_next = ((*self.__ice_derived_doubled()) + 1); if ::ui_lang_runtime::state_changed!(self.count, __ice_next) { self.count = __ice_next; self.__ice_rev[1] += 1; self.__ice_derived.doubled.take(); } }"
    ));
    assert!(generated.contains("{ let __ice_next = (*self.__ice_derived_visible()).clone(); if ::ui_lang_runtime::state_changed!(self.rows, __ice_next) { self.rows = __ice_next; self.__ice_rev[0] += 1; self.__ice_derived.visible.take(); self.__ice_derived.shown.take(); } }"));
}

#[test]
fn clears_only_the_cells_whose_dependency_chain_reaches_the_written_field() {
    let source = format!(
        r#"app Chain
{THEME}state
  left = 0
  right = 0
  other = ""
derived
  left_twice = left * 2
  sum = left_twice + right
  shout = other
on touch_left
  left = 1
on touch_right
  right = 1
on touch_other
  other = "x"
view
  col
    text sum
    text shout
"#
    );
    let generated = compile(&source, "chain.ice").unwrap();

    assert!(generated.contains(
        "self.left = __ice_next; self.__ice_rev[0] += 1; self.__ice_derived.left_twice.take(); self.__ice_derived.sum.take(); } }\n"
    ));
    assert!(generated.contains(
        "self.right = __ice_next; self.__ice_rev[1] += 1; self.__ice_derived.sum.take(); } }\n"
    ));
    assert!(generated.contains(
        "self.other = __ice_next; self.__ice_rev[2] += 1; self.__ice_derived.shout.take(); } }\n"
    ));
}

#[test]
fn component_state_writes_clear_nothing() {
    let source = format!(
        r#"app Local
{THEME}state
  count = 0
derived
  doubled = count * 2
component Counter()
  state
    clicks = 0
  on click
    clicks = clicks + 1
  button "Click" -> click
view
  col
    Counter
    text doubled
"#
    );
    let generated = compile(&source, "local.ice").unwrap();

    assert!(generated.contains("__local.clicks = __ice_next; __local.__ice_rev[0] += 1; } }\n"));
    assert!(!generated.contains("__local.__ice_rev[0] += 1; __local.__ice_derived"));
}

#[test]
fn a_self_assignment_reading_the_target_through_a_derived_value_does_not_take_the_field() {
    let source = format!(
        r#"app SelfMove
extern crate::backend
  Row(label:str)
  pure filter(rows:[Row]) -> [Row]
  pure appended(rows:[Row], extra:[Row]) -> [Row]
  pure label(rows:[Row]) -> str
{THEME}state
  rows:[Row] = []
  title = ""
derived
  visible = filter(rows)
  heading = label(visible)
on direct
  rows = appended(rows, [])
on through_derived
  rows = appended(rows, visible)
on through_chain
  title = label(appended(rows, []))
  rows = appended(rows, [])
  title = heading
view
  text title
"#
    );
    let generated = compile(&source, "self-move.ice").unwrap();

    // The direct form still moves the field out for the call.
    assert!(generated.contains(
        "self.rows = crate::backend::appended(::std::mem::take(&mut self.rows), ::std::vec::Vec::new());"
    ));
    // Through a derived value the field stays put: the cell may be empty here
    // (the handler just wrote `rows`), and taking the field first would let
    // the recomputation read the emptied field.
    assert!(generated.contains(
        "let __ice_next = crate::backend::appended(self.rows.clone(), (*self.__ice_derived_visible()).clone());"
    ));
    assert!(!generated.contains("appended(::std::mem::take(&mut self.rows), (*self."));
}

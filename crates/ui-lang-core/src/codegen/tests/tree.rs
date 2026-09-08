//! The `tree` target: a view compiles to code that builds `ui_lang_wire`
//! nodes, and a construct the wire does not carry fails the build at its
//! `.ice` line.

use super::*;
use crate::{Target, compile_for};

const PALETTE: &str = r#"theme contract AppTheme
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
fn mounted_components_build_a_tree_and_defer_their_boot_messages() {
    let source = format!(
        "app Mounted\n{PALETTE}component Counter(initial:i64)\n  lifetime mounted\n  state\n    count = 0\n  boot\n    count = initial\n  on increment\n    count = count + 1\n  col\n    text count\n    button \"Increment\" -> increment\nview\n  Counter initial=7 #counter\n"
    );
    let generated = compile_for(&source, "mounted.ice", Target::Tree)
        .unwrap_or_else(|error| panic!("{}", error.render("mounted.ice")));
    assert!(generated.contains("::ui_lang_guest::slots::defer"));
    assert!(generated.contains("finish_tree_render"));
    assert!(!generated.contains("::ui_lang_runtime::boot_dispatch"));
}

#[test]
fn mounted_tree_refusals_follow_lazy_and_host_condition_expansion() {
    let head = format!(
        "app Mounted\n{PALETTE}state\n  value = 1\ncomponent Leaf()\n  lifetime mounted\n  state\n    count = 0\n  text count\ncomponent Wrapper()\n  Leaf #leaf\nview\n"
    );
    for (view, reason) in [
        ("  lazy value as cached\n    Wrapper\n", "inside lazy"),
        (
            "  responsive size=(width, height)\n    col\n      if width > 100.0\n        Wrapper\n",
            "inside a host container condition",
        ),
    ] {
        let source = format!("{head}{view}");
        let error = compile_for(&source, "mounted.ice", Target::Tree)
            .unwrap_err()
            .render("mounted.ice");
        let line = source
            .lines()
            .position(|line| line == "  Leaf #leaf")
            .unwrap()
            + 1;
        assert!(error.contains("E190") && error.contains(reason), "{error}");
        assert!(error.contains(&format!("mounted.ice:{line}:")), "{error}");
    }
    // Refused nested expansion must unwind both guards. Unconditional children
    // of responsive have guest-known lifetime and need no host activation event.
    compile_for(
        &format!("{head}  responsive size=(width, height)\n    Wrapper\n"),
        "mounted.ice",
        Target::Tree,
    )
    .unwrap();
}

fn tree(view: &str) -> String {
    tree_with("", view)
}

/// A handler with a parameter is typed by the widget that routes to it, so
/// a view that uses one brings it along.
fn tree_with(handlers: &str, view: &str) -> String {
    let source = format!(
        "app Demo\n{PALETTE}state\n  draft = \"\"\n  items = [\"a\", \"b\"]\n  busy = false\n  amount = 0.0\n  choice:str? = none\n  notes:editor = \"Notes\"\non add\n  draft = \"\"\non remove(index)\n  busy = true\n{handlers}view\n{view}"
    );
    compile_for(&source, "demo.ice", Target::Tree).unwrap_or_else(|error| {
        panic!("{}", error.render("demo.ice"));
    })
}

#[test]
fn a_view_compiles_to_wire_nodes_with_values_inlined() {
    let generated = tree(
        r#"  box #app w=fill h=fill bg=bg p=24.0
    col #content gap=12.0 align=center
      text "Todo" #title
        with
          size=28.0
          @text-fg
          @font-bold
      svg "<svg xmlns='http://www.w3.org/2000/svg'/>" #icon memory w=24.0 h=24.0 color=fg
      row w=fill gap=8.0
        input "What needs doing?" #draft <-> draft w=fill
          active bg=bg border=primary border-w=1.0 r=10.0 value=fg placeholder=fg selection=primary
        button "Add" #add -> add
          active bg=primary text=fg r=8.0
          hovered bg=danger text=fg r=8.0
      scroll #list w=fill h=fill
        col w=fill gap=8.0
          for item in items
            row gap=12.0
              text item @text-fg
              button "×" -> remove 0
                active bg=bg text=danger r=8.0
      if busy
        text "working" @text-fg
"#,
    );
    // The element type is the wire's node for every generic argument.
    assert!(generated.contains(
        "type __IceElement<'a, Message, Theme = ()> = <(&'a (), Message, Theme) as ::ui_lang_guest::wire::Erase>::Node;\npub(crate) type __IceMessage = __DemoMessage;"
    ));
    for expected in [
        "::ui_lang_guest::wire::Node::Container {",
        "::ui_lang_guest::wire::Node::Linear {",
        "::ui_lang_guest::wire::Node::Scroll {",
        "::ui_lang_guest::wire::Node::Text {",
        "::ui_lang_guest::wire::Node::Svg {",
        "::ui_lang_guest::wire::Node::Input {",
        "::ui_lang_guest::wire::Node::Button {",
        "axis: ::ui_lang_guest::wire::Axis::Row",
        "direction: ::ui_lang_guest::wire::ScrollDirection::Vertical",
        "weight: ::ui_lang_guest::wire::Weight::Bold",
        "width: ::std::option::Option::Some(::ui_lang_guest::wire::Length::Fill)",
        "padding: ::std::option::Option::Some(::ui_lang_guest::wire::Edges { top: (24.0) as f32",
        // Messages and input handlers go through the guest's per-frame tables.
        "on_press: ::std::option::Option::Some(::ui_lang_guest::slots::message(",
        // A picture's bytes go through the guest's once-only table.
        "let (__hash, __bytes) = ::ui_lang_guest::slots::picture((",
        "on_input: ::ui_lang_guest::slots::handler::<::std::string::String, __DemoMessage>(",
        // Colours are the palette's, resolved in the guest.
        "::ui_lang_guest::wire::Rgba([__color.r, __color.g, __color.b, __color.a])",
        // Control flow is the shared emitter's loop over the child list.
        "for (__ice_index, item) in",
        "__children.push(",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected:?} in:\n{generated}"
        );
    }
    // `iced::widget::Id` still names the runtime shims; no widget is BUILT.
    for forbidden in [
        "::iced::widget::container(",
        "::iced::widget::text(",
        "::iced::widget::text_input(",
        "::iced::widget::button(",
        "::iced::widget::scrollable(",
        "::iced::widget::Column::",
        "::iced::widget::Row::",
        "::ui_lang_runtime::navigation(",
        "::ui_lang_runtime::dev::ready(",
        "__ICE_TEMPLATE_JSON",
    ] {
        if let Some(at) = generated.find(forbidden) {
            let start = at.saturating_sub(200);
            let end = (at + 300).min(generated.len());
            panic!("found {forbidden:?} at:\n{}", &generated[start..end]);
        }
    }
}

#[test]
fn host_surfaces_carry_typed_arguments_and_snapshot_event_routes() {
    let source = r#"app Demo
extern crate::backend
  component markdown(source:&str, dark:bool, count:i64, zoom:f64) -> str
state
  draft = "hello"
  dark = false
on opened(link, context)
  draft = link
  dark = link == context
view
  extern markdown(draft, dark, 7, 1.5) #preview -> opened(_, draft)
"#;
    let source = source.replace("state\n", &format!("{PALETTE}state\n"));
    let result = compile_for(&source, "surface.ice", Target::Tree);
    assert!(
        result.is_ok(),
        "{}",
        result.unwrap_err().render("surface.ice")
    );
    let generated = result.unwrap();
    for expected in [
        "SurfaceValue::Str(",
        "SurfaceValue::Bool(",
        "SurfaceValue::I64(",
        "SurfaceValue::F64(",
        "on_event:",
        "let __route_arg_0 =",
        "SurfaceValue::Str(__item)",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected}:\n{generated}"
        );
    }
}

#[test]
fn omitted_border_fields_remain_absent_on_the_wire() {
    let generated = tree_with(
        "on flip(value)\n  busy = value\n",
        "  col\n    checkbox \"Busy\" checked=busy -> flip _\n      active unchecked border=primary\n    button \"Remove\" -> remove 0\n",
    );
    assert!(
        generated
            .contains("width: ::std::option::Option::None, radius: ::std::option::Option::None"),
        "{generated}"
    );
}

#[test]
fn surface_records_lists_and_options_keep_their_declared_shape() {
    let source = format!(
        r#"app Demo
extern crate::data
  Row(id:i64, label:str, note:str?)
  component rows(values:&[Row], selected:Row?) -> Row
{PALETTE}state
  rows:[Row] = []
  selected:Row? = none
on picked(value)
  selected = some(value)
view
  extern rows(rows, selected) -> picked _
"#
    );
    let generated = compile_for(&source, "records.ice", Target::Tree)
        .unwrap_or_else(|error| panic!("{}", error.render("records.ice")));
    for expected in [
        "SurfaceValue::Record",
        "SurfaceValue::List",
        "SurfaceValue::Option",
        "crate::data::Row",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected}: {generated}"
        );
    }
}

#[test]
fn opaque_and_recursive_surface_data_remain_refused() {
    for (fields, reason) in [("", "opaque"), ("children:[Data]", "recursive")] {
        let source = format!(
            "app Demo\nextern crate::data\n  Data({fields})\n  component panel(data:Data?) -> unit\n{PALETTE}state\n  data:Data? = none\nview\n  extern panel(data)\n"
        );
        let error = compile_for(&source, "data.ice", Target::Tree)
            .unwrap_err()
            .render("data.ice");
        assert!(error.contains("E190") && error.contains(reason), "{error}");
    }
}

#[test]
fn form_controls_compile_to_wire_nodes_with_handler_slots() {
    let generated = tree_with(
        "on flip(value)\n  busy = value\non slide(value)\n  amount = value\non choose(value)\n  choice = some(value)\n",
        r#"  col
    checkbox "Busy" #busy checked=busy -> flip _
    toggler "Busy" checked=busy disabled=busy -> flip _
    radio "One" value=1.0 selected=(amount == 1.0) -> slide _
    slider amount min=0.0 max=100.0 -> slide _
    pick ["One", "Two"] choice hint="Pick" -> choose _
    progress amount
    button "×" -> remove 0
"#,
    );
    for expected in [
        "::ui_lang_guest::wire::Node::Toggle { key:",
        "kind: ::ui_lang_guest::wire::ToggleKind::Checkbox",
        "kind: ::ui_lang_guest::wire::ToggleKind::Switch",
        "::ui_lang_guest::wire::Node::Radio { key:",
        "::ui_lang_guest::wire::Node::Slider { key:",
        "::ui_lang_guest::wire::Node::PickList { key:",
        "::ui_lang_guest::wire::Node::Progress { key:",
        // The value-carrying handlers, one table each argument type.
        "::ui_lang_guest::slots::handler::<bool, __DemoMessage>(",
        "::ui_lang_guest::slots::handler::<f32, __DemoMessage>(",
        "::ui_lang_guest::slots::handler::<u32, __DemoMessage>(",
        // A radio's value is decided in the guest: it is a plain message.
        "on_select: ::ui_lang_guest::slots::message(",
        // A disabled toggler has no handler.
        "on_toggle: if (",
        // A toggle's two answers and a pick list's options are messages built
        // while the view runs: the table outlives the view's borrows.
        "let __on = __route(true); let __off = __route(false);",
        "__table.get(__sent as usize).cloned()",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected:?} in:\n{generated}"
        );
    }
}

/// A mouse area crosses with a message per discrete route and a handler per
/// positional one, typed by what the host sends back.
#[test]
fn a_mouse_area_compiles_to_a_mouse_area_node_with_handler_slots() {
    let generated = tree_with(
        "on moved(_x, _y)\non wheeled(_x, _y, _pixels)\n",
        "  col\n    button \"×\" -> remove 0\n    for item in items\n      mouse press=add enter=add move=moved press-at=moved scroll=wheeled\n        text item @text-fg\n",
    );
    for expected in [
        "::ui_lang_guest::wire::Node::MouseArea { key:",
        "on_press: ::std::option::Option::Some(::ui_lang_guest::slots::message(",
        "on_enter: ::std::option::Option::Some(::ui_lang_guest::slots::message(",
        "on_exit: ::std::option::Option::None",
        "on_move: ::std::option::Option::Some(::ui_lang_guest::slots::handler::<(f32, f32), __DemoMessage>(",
        "on_press_at: ::std::option::Option::Some(::ui_lang_guest::slots::handler::<(f32, f32), __DemoMessage>(",
        "on_scroll: ::std::option::Option::Some(::ui_lang_guest::slots::handler::<(f32, f32, bool), __DemoMessage>(",
        "__DemoMessage::Moved(__point.0, __point.1)",
        "__DemoMessage::Wheeled(__delta.0, __delta.1, __delta.2)",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected:?} in:\n{generated}"
        );
    }
    assert!(
        !generated.contains("::iced::widget::mouse_area("),
        "{generated}"
    );
}

/// An `editor` state field is a `String` in a view module — the host owns
/// the `text_editor::Content` — and the `<->` binding's message carries the
/// whole text back, like an input's.
#[test]
fn an_editor_is_a_string_the_host_edits() {
    let generated = tree_with(
        "on clear\n  notes = editor(\"\")\nderived\n  lines = editor_line_count(notes)\n  second = editor_line(notes, 1)\n",
        "  col\n    editor #notes <-> notes hint=\"Write\" h=fill min-h=80.0 disabled=busy\n    text editor_text(notes) @text-fg\n    text lines @text-fg\n    button \"Clear\" -> clear\n    button \"×\" -> remove 0\n",
    );
    for expected in [
        "notes: ::std::string::String,",
        "__EditNotes(::std::string::String),",
        "__DemoMessage::__EditNotes(__text) => {",
        "::ui_lang_guest::wire::Node::Editor { key:",
        "placeholder: \"Write\".to_owned()",
        "text: (self.notes).to_string()",
        "on_edit: if (self.busy) { ::std::option::Option::None } else { ::std::option::Option::Some(::ui_lang_guest::slots::handler::<::std::string::String, __DemoMessage>(",
        "height: ::std::option::Option::Some(::ui_lang_guest::wire::Length::Fill)",
        "min_height: ::std::option::Option::Some((80.0) as f32)",
        "(self.notes).clone()",
        ".split('\\n').nth(__line)",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected:?} in:\n{generated}"
        );
    }
    assert!(
        !generated.contains("text_editor::Content"),
        "a view module never holds a Content:\n{generated}"
    );
    assert!(!generated.contains("__CaretNotes"), "{generated}");
}

/// A grid crosses with its column count, cell cap, spacing and sizing
/// inlined; the host lays the cells out.
#[test]
fn a_grid_compiles_to_a_grid_node() {
    let generated = tree(
        "  grid cols=3 gap=8.0 h=96.0\n    for item in items\n      button \"×\" -> remove 0\n",
    );
    for expected in [
        "::ui_lang_guest::wire::Node::Grid { key:",
        "columns: ::std::option::Option::Some(u32::try_from(3",
        "fluid: ::std::option::Option::None",
        "spacing: ::std::option::Option::Some((8.0) as f32)",
        "height: ::std::option::Option::Some(::ui_lang_guest::wire::Length::Fixed((96.0) as f32))",
        "aspect: ::std::option::Option::None",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected:?} in:\n{generated}"
        );
    }
    let fluid = tree("  grid max-cell=120.0 h=aspect(4.0, 3.0)\n    button \"×\" -> remove 0\n");
    assert!(
        fluid.contains("fluid: ::std::option::Option::Some((120.0) as f32)"),
        "{fluid}"
    );
    assert!(
        fluid.contains("aspect: ::std::option::Option::Some((4.0) as f32 / (3.0) as f32)"),
        "{fluid}"
    );
    assert!(!fluid.contains("::iced::widget::grid("), "{fluid}");
}

/// The guest's handler table outlives the view that filled it, so a slider's
/// change closure owns what its route arguments name: an argument bound by
/// the `for` around it is evaluated in the loop body and moved in, not
/// borrowed from the loop binding.
#[test]
fn a_slider_route_argument_from_a_for_binding_is_owned_by_the_handler() {
    let generated = tree_with(
        "on slide_to(label, value)\n  amount = value\n",
        "  col\n    button \"×\" -> remove 0\n    for item in items\n      slider amount min=0.0 max=100.0 -> slide_to item _\n",
    );
    let slider = generated
        .find("::ui_lang_guest::wire::Node::Slider {")
        .map(|at| &generated[at..])
        .expect("the slider is emitted");
    let handler = slider
        .find("::ui_lang_guest::slots::handler::<f32, __DemoMessage>(")
        .map(|at| &slider[at..])
        .expect("the change route is a handler slot");
    let (hoist, closure) = handler
        .split_once("move |__value|")
        .expect("the route callback is a move closure");
    assert!(
        hoist.contains("let __route_arg_0 = item.to_owned();"),
        "the loop binding is cloned before the closure:\n{handler}"
    );
    assert!(
        closure.starts_with(
            " __DemoMessage::SlideTo(::std::clone::Clone::clone(&__route_arg_0), __value)"
        ),
        "the closure reads its own copy:\n{handler}"
    );
}

#[test]
fn a_sensor_compiles_to_a_sensor_node_with_size_handlers() {
    let generated = tree_with(
        "on measured(width, height)\n  amount = width * height\non hidden\n  busy = true\n",
        "  col\n    button \"×\" -> remove 0\n    for item in items\n      sensor show=measured resize=measured hide=hidden anticipate=48.0 delay=16\n        text item @text-fg\n",
    );
    let sensor = generated
        .find("::ui_lang_guest::wire::Node::Sensor {")
        .map(|at| &generated[at..])
        .expect("the sensor is emitted");
    for expected in [
        "on_show: ::std::option::Option::Some(::ui_lang_guest::slots::handler::<(f32, f32), __DemoMessage>(",
        "on_resize: ::std::option::Option::Some(::ui_lang_guest::slots::handler::<(f32, f32), __DemoMessage>(",
        "on_hide: ::std::option::Option::Some(::ui_lang_guest::slots::message(",
        "anticipate: ::std::option::Option::Some((48.0) as f32)",
        "delay: ::std::option::Option::Some((16) as f32)",
        "move |__size: (f64, f64)| __DemoMessage::Measured(__size.0, __size.1)",
        "move |__sent: (f32, f32)| ::std::option::Option::Some(__route((f64::from(__sent.0), f64::from(__sent.1))))",
    ] {
        assert!(
            sensor.contains(expected),
            "missing {expected:?} in:\n{sensor}"
        );
    }
}

#[test]
fn a_construct_the_wire_does_not_carry_fails_at_its_line() {
    let source = format!(
        "app Demo\n{PALETTE}state\n  draft = \"\"\nview\n  col\n    text \"before\" @text-fg\n    float x=1.0 y=2.0\n      text \"after\"\n"
    );
    let error = compile_for(&source, "demo.ice", Target::Tree).unwrap_err();
    let rendered = error.render("demo.ice");
    assert!(rendered.contains("E190"), "{rendered}");
    assert!(
        rendered.contains("`float` is not available in a view module"),
        "{rendered}"
    );
    assert!(rendered.contains("demo.ice:17"), "{rendered}");
    // The same view is fine natively.
    compile(&source, "demo.ice").unwrap();
}

/// A module has no clock: `every` and `repeat` are the guest crate's, which
/// route them to the host's ticker, and an `every` that would carry the
/// instant is refused where a payload is asked for.
#[test]
fn every_and_repeat_are_the_hosts_ticker_in_a_view_module() {
    let source = format!(
        "app Demo\n{PALETTE}extern crate::host\n  poll() -> i64\nstate\n  auto = false\n  count = 0\non tick\n  count = count + 1\non polled(value)\n  count = value\nsubscribe\n  every 1s when auto -> tick\n  repeat poll() every 250ms -> polled _\nview\n  text \"ready\" @text-fg\n"
    );
    let tree = compile_for(&source, "demo.ice", Target::Tree).unwrap();
    assert!(
        tree.contains("::ui_lang_guest::every(::std::time::Duration::from_millis(1000))"),
        "{tree}"
    );
    assert!(
        tree.contains(
            "::ui_lang_guest::repeat(crate::host::poll, ::std::time::Duration::from_millis(250))"
        ),
        "{tree}"
    );
    assert!(!tree.contains("::iced::time::every"), "{tree}");
    let native = compile(&source, "demo.ice").unwrap();
    assert!(native.contains("::iced::time::every"), "{native}");
    assert!(!native.contains("::ui_lang_guest::every"), "{native}");
}

#[test]
fn an_every_that_binds_the_instant_is_refused_in_a_view_module() {
    let source = format!(
        "app Demo\n{PALETTE}state\n  count = 0\non tick(at)\n  count = count + 1\nsubscribe\n  every 1s -> tick _\nview\n  text \"ready\" @text-fg\n"
    );
    let error = compile_for(&source, "demo.ice", Target::Tree).unwrap_err();
    let rendered = error.render("demo.ice");
    assert!(rendered.contains("E190"), "{rendered}");
    assert!(rendered.contains("carries no instant"), "{rendered}");
    assert!(rendered.contains("demo.ice:17"), "{rendered}");
    compile(&source, "demo.ice").unwrap();
}

#[test]
fn the_native_target_is_untouched_by_the_tree_emitter() {
    let source = format!("app Demo\n{PALETTE}view\n  text \"ready\" @text-fg\n");
    let native = compile(&source, "demo.ice").unwrap();
    assert!(native.contains("::iced::Element<'a, Message, Theme, __IceRenderer>"));
    assert!(!native.contains("::ui_lang_guest"));
}

// ---- the coverage contract ----------------------------------------------

/// What the tree target does with a construct the native target compiles.
#[derive(Clone, Copy, Debug)]
enum Outcome {
    /// The wire carries it: the view compiles for the tree target.
    Emitted,
    /// The wire does not carry it, so the build fails with `E190` and a
    /// message containing this phrase.
    Refused(&'static str),
}

/// One row of the coverage table: the construct, the handlers its fixture
/// routes to, the view body, and what the tree target does with it.
struct Coverage {
    construct: &'static str,
    handlers: &'static str,
    view: &'static str,
    outcome: Outcome,
}

const fn emitted(construct: &'static str, handlers: &'static str, view: &'static str) -> Coverage {
    Coverage {
        construct,
        handlers,
        view,
        outcome: Outcome::Emitted,
    }
}

const fn refused(
    construct: &'static str,
    handlers: &'static str,
    view: &'static str,
    phrase: &'static str,
) -> Coverage {
    Coverage {
        construct,
        handlers,
        view,
        outcome: Outcome::Refused(phrase),
    }
}

/// The head every coverage fixture is compiled against: enough theme, state,
/// externs and one component that any single construct can be written as a
/// view body on its own. A handler with a parameter is typed by its route,
/// so each fixture brings the parameterised handlers it routes to.
const HEAD: &str = concat!(
    "app Demo\n",
    "extern crate::backend\n",
    "  component native_help(active:bool) -> bool\n",
    "  component host_tile(caption:str) -> unit\n",
    "  component host_gauge(level:f64) -> unit\n",
    "  component host_pair(a:str, b:str) -> unit\n",
    "  component host_list(values:[str]) -> unit\n",
    "  SurfaceReply(value:str)\n",
    "  component host_reply() -> SurfaceReply\n",
    "  shader status_shader(speed:f64) -> bool\n",
    "  themer alternate_panel(active:bool) -> unit\n",
    "  checkbox-style checkbox_look(active:bool)\n",
    "  svg-style tinted(active:bool)\n",
    "  editor-action track_edits()\n",
    "  editor-binding editor_keys(readonly:bool) -> str\n",
    "  editor-highlighter editor_highlight(language:str)\n",
    "  editor-style editor_surface(readonly:bool)\n",
    "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\n",
    "palette app for AppTheme\n",
    "  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n",
    "state\n",
    "  draft = \"\"\n",
    "  items = [\"a\", \"b\"]\n",
    "  busy = false\n",
    "  amount = 0.0\n",
    "  choice:str? = none\n",
    "  search:combo[str] = [\"One\", \"Two\"]\n",
    "  notes:editor = \"Notes\"\n",
    "  docs:markdown = \"# Docs\"\n",
    "  picture = rgba(1, 1, bytes(ff 00 ff ff))\n",
    "component Slotted()\n",
    "  box #frame\n",
    "    slot\n",
    "on add\n  draft = \"\"\n",
);

const FLIP: &str = "on flip(value)\n  busy = value\n";
const CHOOSE: &str = "on choose(value)\n  draft = value\n";
const SLIDE: &str = "on slide(value)\n  amount = value\n";
const MEASURE: &str = "on measured(_width, _height)\n  busy = true\non hidden\n  busy = false\n";
const MOVED: &str = "on moved(_x, _y)\n";

/// The tree target's coverage contract.
///
/// Every view construct the native target compiles is listed here with what
/// the tree target does to it: emitted when the wire's nodes carry it,
/// refused — with the phrase the `E190` build error names — when they do
/// not. Nothing is silently dropped: a construct the wire cannot carry fails
/// the build at its `.ice` line, so a view module never renders as something
/// its author did not write.
///
/// The construct column names the emitter's `kind_name` for the construct,
/// suffixed after a colon where one name covers several spellings (the three
/// layout modes the wire carries and the four it does not, the three media
/// widgets) or where the row pins an option rather than a widget.
/// `every_view_kind_is_classified` holds the column to `kind_name`, whose
/// match has no wildcard: a construct added to the native target grows a
/// `ResolvedViewKind` variant, that match must name it, and this table must
/// then classify the name.
const COVERAGE: &[Coverage] = &[
    emitted(
        "input: presentation",
        "font code family=\"Geist\"\nrecipe control for input\n  @w-full px-13px py-11px bg-primary border border-danger rounded-10px focus:border-fg\n",
        "  input \"Name\" <-> draft hint=\"Type a name\" label=\"Search name\" description=\"Filters rows\" disabled=busy p=6.2 text-size=13.0 line-h=1.2 align=center font=code @control\n    focused-hovered border=fg\n",
    ),
    // The wire's nodes.
    emitted(
        "layout: col",
        "",
        "  col gap=4.0\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "layout: row",
        "",
        "  row gap=4.0\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "layout: grid",
        "",
        "  grid cols=2 gap=4.0 h=40.0\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "layout: grid fluid",
        "",
        "  grid max-cell=120.0 h=aspect(4.0, 3.0)\n    text \"a\" @text-fg\n",
    ),
    emitted("layout: scroll", "", "  scroll\n    text \"a\" @text-fg\n"),
    emitted(
        "sensor",
        MEASURE,
        "  sensor show=measured resize=measured hide=hidden anticipate=48.0 delay=16\n    text \"a\" @text-fg\n",
    ),
    emitted("box", "", "  box\n    text \"a\" @text-fg\n"),
    emitted(
        "mouse area",
        "",
        "  mouse press=add\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "mouse area: buttons",
        "",
        "  mouse release=add double=add right-press=add right-release=add middle-press=add middle-release=add enter=add exit=add\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "mouse area: move",
        MOVED,
        "  mouse move=moved\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "mouse area: press-at",
        MOVED,
        "  mouse press-at=moved\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "mouse area: scroll",
        "on wheeled(_x, _y, _pixels)\n",
        "  mouse scroll=wheeled\n    text \"a\" @text-fg\n",
    ),
    emitted("text", "", "  text \"a\" @text-fg\n"),
    emitted(
        "media: svg",
        "",
        "  svg \"<svg/>\" memory w=24.0 h=24.0 color=fg label=\"Icon\"\n",
    ),
    emitted("input", "", "  input \"p\" <-> draft\n"),
    emitted("button", "", "  button \"go\" -> add\n"),
    emitted("space", "", "  space w=24.0 h=8.0\n"),
    emitted("rule", "", "  rule horizontal\n"),
    emitted(
        "checkbox",
        FLIP,
        "  checkbox \"e\" checked=busy -> flip _\n",
    ),
    emitted("toggler", FLIP, "  toggler \"e\" checked=busy -> flip _\n"),
    emitted(
        "slider",
        SLIDE,
        "  slider amount min=0.0 max=100.0 -> slide _\n",
    ),
    emitted(
        "slider: route argument",
        "on slide_to(index, value)\n  amount = value\n",
        "  slider amount min=0.0 max=100.0 -> slide_to 1 _\n",
    ),
    emitted("progress", "", "  progress amount\n"),
    emitted(
        "radio",
        SLIDE,
        "  radio \"a\" value=1.0 selected=(amount == 1.0) -> slide _\n",
    ),
    emitted(
        "pick list",
        CHOOSE,
        "  pick [\"One\", \"Two\"] choice -> choose _\n",
    ),
    // Control flow, components and slots: the shared emitters, which push
    // whatever `render_node` returns into the parent's child list.
    emitted("if", "", "  col\n    if busy\n      text \"a\" @text-fg\n"),
    emitted(
        "match",
        "",
        "  col\n    match choice\n      some(label)\n        text label @text-fg\n      none\n        text \"none\" @text-fg\n",
    ),
    emitted(
        "for",
        "",
        "  col\n    for item in items\n      text item @text-fg\n",
    ),
    emitted("component", "", "  Slotted\n    text \"a\" @text-fg\n"),
    emitted("slot", "", "  Slotted\n    text \"slotted\" @text-fg\n"),
    // The layouts the wire has no node for.
    emitted(
        "layout: stack",
        "",
        "  stack w=fill h=24.0\n    text \"a\" @text-fg\n    text \"b\" @text-fg\n",
    ),
    emitted(
        "layout: hover",
        "",
        "  hover\n    text \"a\" @text-fg\n    text \"b\" @text-fg\n",
    ),
    emitted(
        "layout: flex",
        "",
        "  flex gap=4.0\n    text \"a\" @text-fg\n",
    ),
    // The widgets the wire has no node for.
    emitted(
        "overlay",
        "",
        "  overlay when=busy dismiss=add\n    content\n      text \"a\" @text-fg\n    layer\n      text \"b\" @text-fg\n",
    ),
    refused(
        "pane grid",
        "",
        "  panes #work w=fill h=80.0\n    pane first\n      text \"a\" @text-fg\n",
        "`pane grid`",
    ),
    emitted("rich text", "", "  rich-text\n    span \"a\"\n"),
    refused(
        "combo box",
        CHOOSE,
        "  combo search choice \"Search\" -> choose _\n",
        "`combo box`",
    ),
    emitted("qr code", "", "  qr draft\n"),
    emitted(
        "keyed column",
        "",
        "  keyed item in [1, 2] by=item\n    text item @text-fg\n",
    ),
    emitted(
        "lazy",
        "",
        "  lazy draft as cached\n    text cached @text-fg\n",
    ),
    refused(
        "resize handle",
        "on resized(_dx, _dy)\n",
        "  resize-handle drag=resized\n    box w=24.0 h=12.0\n      text \"a\" @text-fg\n",
        "`resize handle`",
    ),
    refused(
        "theme",
        "",
        "  theme dark\n    text \"a\" @text-fg\n",
        "`theme`",
    ),
    refused(
        "float",
        "",
        "  float x=2.0 y=3.0\n    text \"a\" @text-fg\n",
        "`float`",
    ),
    emitted(
        "pin",
        "",
        "  pin w=64.0 h=24.0 x=2.0 y=3.0\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "tooltip",
        "",
        "  tooltip delay=0\n    text \"a\" @text-fg\n    text \"tip\" @text-fg\n",
    ),
    emitted(
        "responsive size: container rules",
        "",
        "  responsive size=(w, h)\n    col\n      if w >= 300.0 && h > 100.0\n        text \"wide\"\n",
    ),
    refused(
        "responsive size",
        "",
        "  responsive size=(available_width, available_height)\n    text available_width @text-fg\n",
        "responsive measurements",
    ),
    refused(
        "table",
        "",
        "  table item in items\n    col\n      header\n        text \"h\" @text-fg\n      cell\n        text item @text-fg\n",
        "`table`",
    ),
    emitted(
        "markdown",
        "on link_opened(_url)\n",
        "  markdown docs -> link_opened _\n",
    ),
    emitted(
        "editor",
        "",
        "  editor #notes <-> notes hint=\"Write\" w=320.0 h=fill min-h=80.0 max-h=240.0 disabled=busy\n",
    ),
    refused(
        "editor: action",
        "",
        "  editor <-> notes action=track_edits()\n",
        "an editor action",
    ),
    refused(
        "editor: key binding",
        "on command(_value)\n",
        "  editor <-> notes key-binding=editor_keys(busy) -> command _\n",
        "an editor key binding",
    ),
    refused(
        "editor: highlight",
        "",
        "  editor <-> notes highlight=\"rs\"\n",
        "an editor highlighter",
    ),
    refused(
        "editor: highlighter",
        "",
        "  editor <-> notes highlighter=editor_highlight(draft)\n",
        "an editor highlighter",
    ),
    refused(
        "editor: style callback",
        "",
        "  editor <-> notes style=editor_surface(busy)\n",
        "a style on an editor",
    ),
    refused(
        "editor: status style",
        "",
        "  editor <-> notes\n    active bg=bg\n",
        "a style on an editor",
    ),
    refused(
        "editor: text option",
        "",
        "  editor <-> notes size=14.0\n",
        "this editor option",
    ),
    refused("themer", "", "  themer alternate_panel(true)\n", "`themer`"),
    emitted(
        "shader",
        FLIP,
        "  shader status_shader(1.0) w=fill h=24.0 -> flip _\n",
    ),
    refused("media: image", "", "  image picture\n", "`media`"),
    refused(
        "media: svg from a path",
        "",
        "  svg draft\n",
        "an svg read from a path",
    ),
    emitted(
        "media: svg option",
        "",
        "  svg \"<svg/>\" memory opacity=0.5 fit=contain rotate=rotation.solid(radians(0.5))\n",
    ),
    emitted(
        "media: svg hover colour",
        "",
        "  svg \"<svg/>\" memory color=fg hover=primary\n",
    ),
    emitted(
        "media: inherited svg ink",
        FLIP,
        "  button label=\"Icon\" -> flip false\n    svg \"<svg/>\" memory color=inherit\n",
    ),
    refused(
        "media: svg style callback",
        "",
        "  svg \"<svg/>\" memory style=tinted(busy)\n",
        "an svg style callback",
    ),
    refused("media: viewer", "", "  viewer picture\n", "`media`"),
    emitted(
        "canvas: geometry",
        "",
        "  canvas w=40.0 h=24.0\n    circle x=20.0 y=12.0 r=8.0 fill=primary\n",
    ),
    refused(
        "canvas: host size",
        "",
        "  canvas\n    circle x=canvas_width y=12.0 r=8.0 fill=primary\n",
        "canvas host-size bindings",
    ),
    refused(
        "canvas: cache",
        "",
        "  canvas cache=true\n    circle x=20.0 y=12.0 r=8.0 fill=primary\n",
        "native canvas options",
    ),
    refused(
        "canvas: gradient",
        "",
        "  canvas\n    circle x=20.0 y=12.0 r=8.0 fill=linear(0.0, bg@0.0, primary@1.0)\n",
        "canvas gradient",
    ),
    refused(
        "canvas: text",
        "",
        "  canvas\n    text \"hello\" x=0.0 y=0.0 color=fg\n",
        "canvas text",
    ),
    // Options on the nodes the wire does carry: painted by the host from
    // plain fields on the node.
    emitted(
        "layout: surface utility",
        "",
        "  col\n    with\n      @bg-primary\n      @rounded-md\n    text \"a\" @text-fg\n",
    ),
    refused(
        "mouse area: cursor",
        "",
        "  mouse press=add cursor=pointer\n    text \"a\" @text-fg\n",
        "a mouse cursor",
    ),
    emitted(
        "box: px-snap",
        "",
        "  box px-snap=true\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "layout: scroll bar option",
        "",
        "  scroll bar=hidden bar-w=4.0 scroller-w=2.0\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "layout: scroll anchor",
        "",
        "  scroll anchor-y=end\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "layout: scroll anchor keep",
        "",
        "  scroll anchor-y=keep\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "layout: scroll auto",
        "",
        "  scroll auto=busy\n    text \"a\" @text-fg\n",
    ),
    refused(
        "layout: scroll route",
        "on scrolled(_x, _y, _rx, _ry)\n",
        "  scroll scroll=scrolled\n    text \"a\" @text-fg\n",
        "a scroll route",
    ),
    emitted("rule: style preset", "", "  rule horizontal style=weak\n"),
    emitted("rule: snap", "", "  rule horizontal snap=busy\n"),
    emitted("rule: radius", "", "  rule horizontal r=2.0\n"),
    refused(
        "sensor: key",
        MEASURE,
        "  sensor show=measured key=draft\n    text \"a\" @text-fg\n",
        "a sensor key",
    ),
    refused(
        "rule: fill",
        "",
        "  rule horizontal fill=percent(50.0)\n",
        "a rule fill",
    ),
    // The form controls' literal per-state styles cross as faces the host
    // paints over its theme; a style given as a Rust callback does not.
    emitted(
        "checkbox: style",
        FLIP,
        "  checkbox \"e\" checked=busy -> flip _\n    active checked bg=primary icon=fg text=fg border=danger border-w=1.0 r=2.0\n    hovered unchecked bg=danger\n",
    ),
    emitted(
        "checkbox: preset",
        FLIP,
        "  checkbox \"e\" checked=busy style=success -> flip _\n",
    ),
    refused(
        "checkbox: style callback",
        FLIP,
        "  checkbox \"e\" checked=busy style=checkbox_look(busy) -> flip _\n",
        "a checkbox style callback",
    ),
    emitted(
        "toggler: style",
        FLIP,
        "  toggler \"e\" checked=busy -> flip _\n    active checked bg=primary fg=fg bg-border=danger bg-border-w=1.0 text=fg\n",
    ),
    refused(
        "toggler: knob border",
        FLIP,
        "  toggler \"e\" checked=busy -> flip _\n    active checked fg-border=danger\n",
        "a toggler knob border or padding ratio",
    ),
    emitted(
        "radio: style",
        SLIDE,
        "  radio \"a\" value=1.0 selected=(amount == 1.0) -> slide _\n    active selected dot=primary bg=bg border=fg border-w=1.0 text=fg\n",
    ),
    emitted(
        "slider: style",
        SLIDE,
        "  slider amount min=0.0 max=100.0 -> slide _\n    active rail-start=primary rail-end=bg rail-w=3.0 rail-border=fg rail-border-w=1.0 handle-color=fg handle-border=danger handle-border-w=1.0\n    dragged rail-start=danger\n",
    ),
    refused(
        "slider: handle shape",
        SLIDE,
        "  slider amount min=0.0 max=100.0 -> slide _\n    active handle=circle(4.0)\n",
        "a slider handle shape",
    ),
    emitted("progress: style", "", "  progress amount style=success\n"),
    emitted(
        "progress: colours",
        "",
        "  progress amount bg=bg bar=primary border=fg border-w=1.0 r=2.0\n",
    ),
    emitted(
        "pick list: style",
        CHOOSE,
        "  pick [\"One\", \"Two\"] choice -> choose _\n    active bg=primary text=fg placeholder=fg handle=fg border=danger border-w=1.0 r=4.0\n    opened bg=bg\n    menu bg=bg text=fg selected-bg=primary selected-text=fg\n",
    ),
    emitted("extern widget", "", "  extern host_tile(draft) #tile\n"),
    emitted(
        "extern widget: route",
        FLIP,
        "  extern native_help(busy) -> flip _\n",
    ),
    emitted(
        "extern widget: argument type",
        "",
        "  extern host_gauge(amount)\n",
    ),
    emitted(
        "extern widget: two arguments",
        "",
        "  extern host_pair(draft, draft)\n",
    ),
    emitted(
        "extern widget: compound argument",
        "",
        "  extern host_list(items)\n",
    ),
    emitted(
        "extern widget: compound event",
        "on replied(value)\n  draft = value.value\n",
        "  extern host_reply() -> replied _\n",
    ),
];

#[test]
fn the_tree_target_carries_or_refuses_every_construct() {
    let mut wrong = Vec::new();
    for case in COVERAGE {
        let construct = case.construct;
        let source = format!("{HEAD}{}view\n{}", case.handlers, case.view);
        if let Err(error) = compile(&source, "demo.ice") {
            wrong.push(format!(
                "{construct}: the fixture does not compile natively: {}",
                error.render("demo.ice")
            ));
            continue;
        }
        let rendered = compile_for(&source, "demo.ice", Target::Tree)
            .err()
            .map(|error| error.render("demo.ice"));
        match (case.outcome, rendered) {
            (Outcome::Emitted, None) => {}
            (Outcome::Emitted, Some(rendered)) => {
                wrong.push(format!(
                    "{construct}: expected the wire to carry it: {rendered}"
                ));
            }
            (Outcome::Refused(phrase), None) => {
                wrong.push(format!(
                    "{construct}: expected a refusal naming {phrase:?}, compiled for the tree"
                ));
            }
            (Outcome::Refused(phrase), Some(rendered)) => {
                if !rendered.contains("E190") || !rendered.contains(phrase) {
                    wrong.push(format!(
                        "{construct}: expected `E190` naming {phrase:?}: {rendered}"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_view_kind_is_classified() {
    let emitter = include_str!("../view/tree.rs");
    let arms = emitter
        .split_once("fn kind_name")
        .expect("the emitter names every view kind in `kind_name`")
        .1;
    let arms = &arms[..arms.find("\n}").expect("`kind_name` ends")];
    let names = arms.split("=> \"").skip(1).map(|arm| {
        arm.split_once('"')
            .expect("a `kind_name` arm names its construct")
            .0
    });
    for name in names {
        let classified = COVERAGE
            .iter()
            .any(|case| case.construct == name || case.construct.starts_with(&format!("{name}: ")));
        assert!(
            classified,
            "`{name}` is not in the tree target's coverage table"
        );
    }
}

#[test]
fn nested_markdown_state_uses_guest_content_types() {
    let source = format!(
        "app NestedMarkdown\n{PALETTE}state\n  docs:[markdown] = [markdown(\"# Docs\")]\n  selected:markdown? = none\nview\n  text \"Docs\"\n"
    );
    let generated = compile_for(&source, "nested.ice", Target::Tree).unwrap();
    assert!(
        generated.contains("::std::vec::Vec<::ui_lang_guest::Markdown>"),
        "nested markdown list must use guest content"
    );
    assert!(
        generated.contains("::std::option::Option<::ui_lang_guest::Markdown>"),
        "optional markdown must use guest content"
    );
}

#[test]
fn widget_statements_request_operations_on_the_mounted_host() {
    let source = format!(
        r#"app Operations
{PALETTE}state
  value = ""
  focused = false
on checked(value)
  focused = value
on previous
  task widget focus-prev
on next
  task widget focus-next
on focus
  task widget focus #field
on query
  task widget focused #field -> checked _
on front
  task widget cursor-front #field
on end
  task widget cursor-end #field
on cursor
  task widget cursor #field 2
on all
  task widget select-all #field
on range
  task widget select #field 1 3
on snap
  task widget snap #list 0.0 1.0
on snap_end
  task widget snap-end #list
on scroll_to
  task widget scroll-to #list 0.0 24.0
on scroll_by
  task widget scroll-by #list -4.0 8.0
view
  col
    input "Value" #field <-> value
    button "Operate" -> focus
    scroll #list
      text "Content"
"#
    );
    let generated = compile_for(&source, "operations.ice", Target::Tree).unwrap();
    assert!(
        generated.contains("::ui_lang_guest::widget::perform"),
        "tree widget statements must request mounted host operations"
    );
    assert!(generated.contains("::ui_lang_guest::widget::is_focused"));
    assert!(generated.contains("Operations/field"));
    assert!(!generated.contains("::iced::widget::operation::focus"));
}

#[test]
fn responsive_conditions_refuse_rules_beyond_the_wire_budget() {
    // Exercise 67 wire operations without also stressing recursive expression
    // lowering with a left-associated 17-clause chain on a test thread's stack.
    let mut condition = "width > 0.0".to_owned();
    for _ in 0..4 {
        condition = format!("({condition}) && ({condition})");
    }
    let condition = format!("({condition}) && width > 0.0");
    let source = format!(
        "app Demo\n{PALETTE}view\n  responsive size=(width, height)\n    col\n      if {condition}\n        text \"wide\"\n"
    );
    let error = compile_for(&source, "demo.ice", Target::Tree).unwrap_err();
    let diagnostic = error.render("demo.ice");
    assert!(diagnostic.contains("E190"), "{diagnostic}");
    assert!(
        diagnostic.contains("more than 64 operations"),
        "{diagnostic}"
    );
}

#[test]
fn responsive_conditions_refuse_short_circuit_sensitive_snapshots() {
    for condition in [
        "width > 300.0 && 1 / divisor > 0",
        "width > 300.0 || threshold()",
        "width > 300.0 && risky > 0",
    ] {
        let source = format!(
            "app Demo\nextern crate::backend\n  pure threshold() -> bool\n{PALETTE}state\n  divisor = 0\nderived\n  risky = 1 / divisor\nview\n  responsive size=(width, height)\n    col\n      if {condition}\n        text \"wide\"\n"
        );
        let error = compile_for(&source, "demo.ice", Target::Tree).unwrap_err();
        let diagnostic = error.render("demo.ice");
        assert!(diagnostic.contains("E190"), "{diagnostic}");
        assert!(
            diagnostic.contains("cannot eagerly copy calls or arithmetic"),
            "{diagnostic}"
        );
    }
}

#[test]
fn component_in_a_state_loop_does_not_emit_native_layout_memo() {
    let source = format!(
        "app Demo\n{PALETTE}state\n  rows:[str] = [\"a\", \"b\"]\ncomponent Chip(label:str)\n  box px=7.0 py=3.0\n    text label size=9.0\nview\n  col w=fill\n    for row in rows\n      Chip label=row\n"
    );
    let generated = compile_for(&source, "component.ice", Target::Tree).unwrap();
    assert!(
        !generated.contains("::ui_lang_runtime::rev_memo("),
        "Tree components must return wire nodes, not native layout memo widgets"
    );
}

#[test]
fn tree_text_keeps_named_faces_and_layout_options() {
    let source = format!(
        "app Demo\n{PALETTE}font display family=\"Geist\" weight=semibold\nfont code_medium family=\"Geist Mono\" weight=medium\nstate\n  hit = false\non clicked\n  hit = true\nview\n  box w=fill max-w=620.0 clip=true @px-4 py-2\n    col\n      text \"Label\" font=display wrap=none line-h=1.5 h=44.0 align-y=center\n      text \"Tracking\" font=code_medium tracking=2.0\n      button \"Apply\" @px-4 py-2 -> clicked\n"
    );
    let generated = compile_for(&source, "text.ice", Target::Tree)
        .unwrap_or_else(|error| panic!("{}", error.render("text.ice")));
    for expected in [
        "FontFamily::Named(\"Geist\"",
        "Weight::Semibold",
        "FontFamily::Named(\"Geist Mono\"",
        "Weight::Medium",
        "Wrapping::None",
        "LineHeight::Relative",
        "AlignY::Center",
        "tracking: 2.0f32",
        "max_width:",
        "clip: true",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected} in {generated}"
        );
    }
}

#[test]
fn tree_large_finite_tracking_emits_finite_rust_literal() {
    let source = format!(
        "app Demo\n{PALETTE}view\n  text \"Track\" tracking=1000000000000000000000000000000000000000.0\n"
    );
    let generated = compile_for(&source, "tracking.ice", Target::Tree).unwrap();
    assert!(
        !generated.contains("inff32"),
        "finite Ice values must emit valid Rust"
    );
    assert!(generated.contains("tracking: 3.4028235e38f32"));
}

#[test]
fn tree_button_carries_accessible_state_and_description() {
    let generated = tree(
        "  button \"Toggle\" checked=true expanded=false description=\"Details\" -> remove 0\n",
    );
    for expected in [
        "checked: ::std::option::Option::Some(true)",
        "expanded: ::std::option::Option::Some(false)",
        "String::from(\"Details\".to_owned())",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected} in {generated}"
        );
    }
}

#[test]
fn tree_button_recipes_copy_faces_and_focus_ring() {
    let source = format!(
        "app Demo\n{PALETTE}font ui family=\"Geist\" weight=normal default=true\nrecipe action for button\n  @px-4 py-2 font-semibold bg-primary text-fg rounded-8px hover:bg-danger disabled:opacity-50 focus-visible:border-primary\non hit\nview\n  button \"Action\" @action -> hit\n"
    );
    let generated = compile_for(&source, "recipe.ice", Target::Tree).unwrap();
    for expected in [
        "ButtonRecipe",
        "FontFamily::Named(\"Geist\"",
        "Weight::Semibold",
        "ButtonPreset::Primary",
        "disabled_opacity: ::std::option::Option::Some(0.5f32)",
        "focus_ring: ::std::option::Option::Some",
        "left: 16f32",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected} in {generated}"
        );
    }
}

#[test]
fn tree_button_defaults_keep_guest_label_typography() {
    for utility in ["", " @p-0px"] {
        let source = format!(
            "app Demo\n  text-size 24\n{PALETTE}font ui family=\"Geist\" default=true\non hit\nview\n  button \"Action\"{utility} -> hit\n"
        );
        let generated = compile_for(&source, "defaults.ice", Target::Tree).unwrap();
        assert!(
            generated.contains("FontFamily::Named(\"Geist\""),
            "guest default family must reach plain labels"
        );
        assert!(generated.contains("text_size: ::std::option::Option::Some(24.0f32)"));
    }
}

#[test]
fn explicit_zero_button_padding_clears_native_defaults() {
    let source = format!("app Demo\n{PALETTE}on hit\nview\n  button \"Action\" @p-0px -> hit\n");
    for (target, expected) in [
        (Target::Native, ".padding(::iced::Padding { top: 0.0"),
        (Target::Tree, "Edges { top: 0f32"),
    ] {
        let generated = compile_for(&source, "padding.ice", target).unwrap();
        assert!(
            generated.contains(expected),
            "explicit zero padding must be emitted for {target:?}: {generated}"
        );
    }
}

#[test]
fn tree_wrapping_layout_copies_both_axes() {
    for axis in ["row", "col"] {
        let generated = tree(&format!(
            "  {axis} wrap wrap-gap=3.0 wrap-align=end gap=8.0\n    button \"Go\" -> remove 0\n"
        ));
        assert!(generated.contains("Wrap { spacing: ::std::option::Option::Some((3.0) as f32), align: ::std::option::Option::Some(::ui_lang_guest::wire::AlignX::Right)"));
    }
}

#[test]
fn keyed_and_virtual_columns_emit_copied_rows_without_native_elements() {
    for view in [
        "  keyed item in [1, 2] by=item virtual-row=24.0 w=fill gap=3.0\n    text item\n",
        "  col virtual-row=24.0 w=fill max-w=320.0\n    for item in [1, 2]\n      text item\n",
    ] {
        let source = format!("app Demo\n{PALETTE}view\n{view}");
        let generated = compile_for(&source, "lists.ice", Target::Tree)
            .unwrap_or_else(|error| panic!("{}", error.render("lists.ice")));
        assert!(generated.contains("::ui_lang_guest::wire::Node::KeyedColumn"));
        assert!(
            generated.contains("virtual_row: Some(")
                || generated.contains("virtual_row: ::std::option::Option::Some(")
        );
        assert!(!generated.contains("::iced::widget::keyed_column("));
        assert!(!generated.contains("::ui_lang_runtime::virtual_keyed_children("));
        assert!(!generated.contains("::ui_lang_runtime::bounded_fill_element(__child"));
    }
}

#[test]
fn tree_lazy_emits_a_guest_cache_with_a_stable_wire_boundary() {
    let source = format!(
        "app Demo\n{PALETTE}state\n  draft = \"hello\"\nview\n  lazy draft as cached\n    text cached @text-fg\n"
    );
    let generated = compile_for(&source, "lazy.ice", Target::Tree).unwrap();
    assert!(generated.contains("::ui_lang_guest::memo_lazy("));
    assert!(!generated.contains("::ui_lang_runtime::memo_lazy("));
}

#[test]
fn tree_flex_copies_layout_item_options_and_control_flow() {
    let source = format!(
        r#"app Flexbox
{PALETTE}state
  values = [1, 2, 3]
view
  flex dir=row-reverse wrap=wrap-reverse w=fill h=300.0 max-w=900.0 max-h=500.0 gap=8.0 gap-y=12.0 gap-x=16.0 justify=space-evenly items=baseline content=space-between p=4.0 clip=true
    box order=2 grow=1.0 shrink=0.5 basis=percent(40.0) self=flex-end m=auto
      text "First"
    box flex=2.0,1.0,120.0 mx=percent(5.0) mt=-2.0
      text "Second"
    flex
      with
        w=100.0
        h=50.0
        @w-full
        @h-full
        @max-w-sm
        @bg-primary
      text "Utility sized surface"
    for value in values
      if value > 1
        match value
          2
            text value
          _
            text "Last"
"#
    );
    let generated = compile_for(&source, "flexbox.ice", Target::Tree).unwrap();
    for expected in [
        "::ui_lang_guest::wire::Node::Flex",
        "FlexDirection::RowReverse",
        "FlexWrap::WrapReverse",
        "FlexContentAlignment::SpaceEvenly",
        "FlexItemAlignment::Baseline",
        "FlexContentAlignment::SpaceBetween",
        "FlexBasis::Percent",
        "FlexMargin::Auto",
        "FlexMargin::Percent",
        "FlexItemAlignment::FlexEnd",
        "__items.push(",
        "enumerate()",
        "surface_width: ::std::option::Option::Some(::ui_lang_guest::wire::Length::Fill)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
    assert!(!generated.contains("::ui_lang_runtime::flex("));
    assert!(!generated.contains("::ui_lang_runtime::flex_item("));
}

#[test]
fn rich_span_gradient_refusal_names_the_span_line() {
    let source = format!(
        "{HEAD}view\n  rich-text\n    span \"highlight\" bg=linear(0.0, bg@0.0, primary@1.0)\n"
    );
    compile(&source, "spans.ice").expect("gradient span compiles natively");
    let line = source
        .lines()
        .position(|line| line.contains("span \"highlight\""))
        .unwrap()
        + 1;
    let error = compile_for(&source, "spans.ice", Target::Tree)
        .unwrap_err()
        .render("spans.ice");
    assert!(
        error.contains("E190") && error.contains("a gradient background"),
        "{error}"
    );
    assert!(
        error.contains(&format!("spans.ice:{line}:")),
        "refusal must name the span, not its paragraph: {error}"
    );
}

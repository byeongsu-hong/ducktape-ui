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
        "app Demo\n{PALETTE}state\n  draft = \"\"\nview\n  col\n    text \"before\" @text-fg\n    qr draft\n"
    );
    let error = compile_for(&source, "demo.ice", Target::Tree).unwrap_err();
    let rendered = error.render("demo.ice");
    assert!(rendered.contains("E190"), "{rendered}");
    assert!(
        rendered.contains("`qr code` is not available in a view module"),
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
    refused(
        "layout: stack",
        "",
        "  stack w=fill h=24.0\n    text \"a\" @text-fg\n    text \"b\" @text-fg\n",
        "`stack`",
    ),
    refused(
        "layout: hover",
        "",
        "  hover\n    text \"a\" @text-fg\n    text \"b\" @text-fg\n",
        "`hover`",
    ),
    refused(
        "layout: flex",
        "",
        "  flex gap=4.0\n    text \"a\" @text-fg\n",
        "`flex`",
    ),
    // The widgets the wire has no node for.
    refused(
        "overlay",
        "",
        "  overlay when=busy dismiss=add\n    content\n      text \"a\" @text-fg\n    layer\n      text \"b\" @text-fg\n",
        "`overlay`",
    ),
    refused(
        "pane grid",
        "",
        "  panes #work w=fill h=80.0\n    pane first\n      text \"a\" @text-fg\n",
        "`pane grid`",
    ),
    refused(
        "rich text",
        "",
        "  rich-text\n    span \"a\"\n",
        "`rich text`",
    ),
    refused(
        "combo box",
        CHOOSE,
        "  combo search choice \"Search\" -> choose _\n",
        "`combo box`",
    ),
    refused("qr code", "", "  qr draft\n", "`qr code`"),
    refused(
        "keyed column",
        "",
        "  keyed item in [1, 2] by=item\n    text item @text-fg\n",
        "`keyed column`",
    ),
    refused(
        "lazy",
        "",
        "  lazy draft as cached\n    text cached @text-fg\n",
        "`lazy`",
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
    refused(
        "pin",
        "",
        "  pin w=64.0 h=24.0 x=2.0 y=3.0\n    text \"a\" @text-fg\n",
        "`pin`",
    ),
    refused(
        "tooltip",
        "",
        "  tooltip delay=0\n    text \"a\" @text-fg\n    text \"tip\" @text-fg\n",
        "`tooltip`",
    ),
    refused(
        "responsive size",
        "",
        "  responsive size=(available_width, available_height)\n    text available_width @text-fg\n",
        "`responsive size`",
    ),
    refused(
        "table",
        "",
        "  table item in items\n    col\n      header\n        text \"h\" @text-fg\n      cell\n        text item @text-fg\n",
        "`table`",
    ),
    refused(
        "markdown",
        "on link_opened(_url)\n",
        "  markdown docs -> link_opened _\n",
        "`markdown`",
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
    refused(
        "shader",
        FLIP,
        "  shader status_shader(1.0) w=fill h=24.0 -> flip _\n",
        "`shader`",
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
    refused(
        "media: svg style callback",
        "",
        "  svg \"<svg/>\" memory style=tinted(busy)\n",
        "an svg style callback",
    ),
    refused("media: viewer", "", "  viewer picture\n", "`media`"),
    refused("canvas", "", "  canvas w=40.0 h=24.0\n", "`canvas`"),
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
    // An extern widget is a host surface: the host paints the region by the
    // extern's name, given its one `str` argument as text. A route, a second
    // argument or an argument of another type has no room on the node.
    emitted("extern widget", "", "  extern host_tile(draft) #tile\n"),
    refused(
        "extern widget: route",
        FLIP,
        "  extern native_help(busy) -> flip _\n",
        "a route on an extern widget",
    ),
    refused(
        "extern widget: argument type",
        "",
        "  extern host_gauge(amount)\n",
        "an extern widget argument that is not `str`",
    ),
    refused(
        "extern widget: two arguments",
        "",
        "  extern host_pair(draft, draft)\n",
        "more than one argument on an extern widget",
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

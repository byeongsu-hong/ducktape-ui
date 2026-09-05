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
    let source = format!(
        "app Demo\n{PALETTE}state\n  draft = \"\"\n  items = [\"a\", \"b\"]\n  busy = false\non add\n  draft = \"\"\non remove(index)\n  busy = true\nview\n{view}"
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
        "type __IceElement<'a, Message, Theme = ()> = <(&'a (), Message, Theme) as ::ui_lang_guest::wire::Erase>::Node;"
    ));
    for expected in [
        "::ui_lang_guest::wire::Node::Container {",
        "::ui_lang_guest::wire::Node::Linear {",
        "::ui_lang_guest::wire::Node::Scroll {",
        "::ui_lang_guest::wire::Node::Text {",
        "::ui_lang_guest::wire::Node::Input {",
        "::ui_lang_guest::wire::Node::Button {",
        "axis: ::ui_lang_guest::wire::Axis::Row",
        "direction: ::ui_lang_guest::wire::ScrollDirection::Vertical",
        "weight: ::ui_lang_guest::wire::Weight::Bold",
        "width: ::std::option::Option::Some(::ui_lang_guest::wire::Length::Fill)",
        "padding: ::std::option::Option::Some(::ui_lang_guest::wire::Edges { top: (24.0) as f32",
        // Messages and input handlers go through the guest's per-frame tables.
        "on_press: ::std::option::Option::Some(::ui_lang_guest::slots::message(",
        "on_input: ::ui_lang_guest::slots::handler(::std::boxed::Box::new(",
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
fn a_construct_the_wire_does_not_carry_fails_at_its_line() {
    let source = format!(
        "app Demo\n{PALETTE}state\n  on = false\non flip(value)\n  on = value\nview\n  col\n    text \"before\" @text-fg\n    checkbox \"Enabled\" checked=on -> flip _\n"
    );
    let error = compile_for(&source, "demo.ice", Target::Tree).unwrap_err();
    let rendered = error.render("demo.ice");
    assert!(rendered.contains("E190"), "{rendered}");
    assert!(
        rendered.contains("`checkbox` is not available in a view module"),
        "{rendered}"
    );
    assert!(rendered.contains("demo.ice:19"), "{rendered}");
    // The same view is fine natively.
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
    "  shader status_shader(speed:f64) -> bool\n",
    "  themer alternate_panel(active:bool) -> unit\n",
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

/// The tree target's coverage contract.
///
/// Every view construct the native target compiles is listed here with what
/// the tree target does to it: emitted when the wire's eight nodes carry it,
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
    // The wire's eight nodes.
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
    emitted("layout: scroll", "", "  scroll\n    text \"a\" @text-fg\n"),
    emitted("box", "", "  box\n    text \"a\" @text-fg\n"),
    emitted("text", "", "  text \"a\" @text-fg\n"),
    emitted("input", "", "  input \"p\" <-> draft\n"),
    emitted("button", "", "  button \"go\" -> add\n"),
    emitted("space", "", "  space w=24.0 h=8.0\n"),
    emitted("rule", "", "  rule horizontal\n"),
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
        "layout: grid",
        "",
        "  grid cols=2\n    text \"a\" @text-fg\n",
        "`grid`",
    ),
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
        "checkbox",
        FLIP,
        "  checkbox \"e\" checked=busy -> flip _\n",
        "`checkbox`",
    ),
    refused(
        "toggler",
        FLIP,
        "  toggler \"e\" checked=busy -> flip _\n",
        "`toggler`",
    ),
    refused(
        "slider",
        SLIDE,
        "  slider amount min=0.0 max=100.0 -> slide _\n",
        "`slider`",
    ),
    refused("progress", "", "  progress amount\n", "`progress`"),
    refused(
        "radio",
        SLIDE,
        "  radio \"a\" value=1.0 selected=(amount == 1.0) -> slide _\n",
        "`radio`",
    ),
    refused(
        "pick list",
        CHOOSE,
        "  pick [\"One\", \"Two\"] choice -> choose _\n",
        "`pick list`",
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
        "mouse area",
        "",
        "  mouse press=add\n    text \"a\" @text-fg\n",
        "`mouse area`",
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
        "sensor",
        "on measured(_width, _height)\non hidden\n",
        "  sensor show=measured resize=measured hide=hidden\n    text \"a\" @text-fg\n",
        "`sensor`",
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
    refused("editor", "", "  editor <-> notes\n", "`editor`"),
    refused(
        "extern widget",
        FLIP,
        "  extern native_help(busy) -> flip _\n",
        "`extern widget`",
    ),
    refused("themer", "", "  themer alternate_panel(true)\n", "`themer`"),
    refused(
        "shader",
        FLIP,
        "  shader status_shader(1.0) w=fill h=24.0 -> flip _\n",
        "`shader`",
    ),
    refused("media: image", "", "  image picture\n", "`media`"),
    refused("media: svg", "", "  svg \"<svg/>\" memory\n", "`media`"),
    refused("media: viewer", "", "  viewer picture\n", "`media`"),
    refused("canvas", "", "  canvas w=40.0 h=24.0\n", "`canvas`"),
    // Options on the nodes the wire does carry: painted, driven or measured
    // by widgets the wire has no room for.
    refused(
        "layout: surface utility",
        "",
        "  col\n    with\n      @bg-primary\n      @rounded-md\n    text \"a\" @text-fg\n",
        "a surface utility style on a layout",
    ),
    refused(
        "box: px-snap",
        "",
        "  box px-snap=true\n    text \"a\" @text-fg\n",
        "`px-snap` on a surface",
    ),
    refused(
        "layout: scroll bar option",
        "",
        "  scroll bar=hidden\n    text \"a\" @text-fg\n",
        "a scroll bar option",
    ),
    refused(
        "layout: scroll anchor",
        "",
        "  scroll anchor-y=end\n    text \"a\" @text-fg\n",
        "a scroll anchor",
    ),
    refused(
        "layout: scroll auto",
        "",
        "  scroll auto=busy\n    text \"a\" @text-fg\n",
        "a scroll route",
    ),
    refused(
        "rule: style preset",
        "",
        "  rule horizontal style=weak\n",
        "a rule style preset",
    ),
    refused(
        "rule: snap",
        "",
        "  rule horizontal snap=busy\n",
        "`snap` on a rule",
    ),
    refused(
        "rule: radius",
        "",
        "  rule horizontal r=2.0\n",
        "a rule radius",
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

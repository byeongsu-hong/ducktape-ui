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

/// The prelude every coverage fixture is compiled against: enough theme,
/// state, handlers and externs that any one construct can be written as a
/// view body on its own.
const PRELUDE: &str = concat!(
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
    "on flip(value)\n  busy = value\n",
    "on choose(value)\n  draft = value\n",
    "on slide(value)\n  amount = value\n",
    "on link_opened(_url)\n",
    "on resized(_dx, _dy)\n",
    "on measured(_width, _height)\n",
    "on hidden\n",
    "view\n",
);

/// The tree target's coverage contract.
///
/// Every view construct the native target compiles is listed here with what
/// the tree target does to it: `Emitted` when the wire's eight nodes carry
/// it, `Refused` with the phrase the build error names when they do not.
/// Nothing is silently dropped — a construct the wire cannot carry fails the
/// build at its `.ice` line, so a view module never renders as something it
/// did not ask for.
///
/// The construct column names the emitter's `kind_name` for the construct,
/// suffixed after a colon where one name covers several spellings (the three
/// layout modes the wire carries and the four it does not, the three media
/// widgets) or where the row pins an option rather than a widget.
/// `every_view_kind_is_classified` holds the column to `kind_name`, whose
/// match has no wildcard: a construct added to the native target grows a
/// `ResolvedViewKind` variant, that match must name it, and this table must
/// then classify the name.
const COVERAGE: &[(&str, &str, Outcome)] = &[
    // The wire's eight nodes.
    (
        "layout: col",
        "  col gap=4.0\n    text \"a\" @text-fg\n",
        Outcome::Emitted,
    ),
    (
        "layout: row",
        "  row gap=4.0\n    text \"a\" @text-fg\n",
        Outcome::Emitted,
    ),
    (
        "layout: scroll",
        "  scroll\n    text \"a\" @text-fg\n",
        Outcome::Emitted,
    ),
    ("box", "  box\n    text \"a\" @text-fg\n", Outcome::Emitted),
    ("text", "  text \"a\" @text-fg\n", Outcome::Emitted),
    ("input", "  input \"p\" <-> draft\n", Outcome::Emitted),
    ("button", "  button \"go\" -> add\n", Outcome::Emitted),
    ("space", "  space w=24.0 h=8.0\n", Outcome::Emitted),
    ("rule", "  rule horizontal\n", Outcome::Emitted),
    // Control flow, components and slots: the shared emitters, which push
    // whatever `render_node` returns into the parent's child list.
    (
        "if",
        "  if busy\n    text \"a\" @text-fg\n",
        Outcome::Emitted,
    ),
    (
        "match",
        "  match choice\n    some(label)\n      text label @text-fg\n    none\n      text \"none\" @text-fg\n",
        Outcome::Emitted,
    ),
    (
        "for",
        "  for item in items\n    text item @text-fg\n",
        Outcome::Emitted,
    ),
    (
        "component",
        "  Slotted\n    text \"a\" @text-fg\n",
        Outcome::Emitted,
    ),
    (
        "slot",
        "  Slotted\n    text \"slotted\" @text-fg\n",
        Outcome::Emitted,
    ),
    // The layouts the wire has no node for.
    (
        "layout: grid",
        "  grid cols=2\n    text \"a\" @text-fg\n",
        Outcome::Refused("`grid`"),
    ),
    (
        "layout: stack",
        "  stack w=fill h=24.0\n    text \"a\" @text-fg\n    text \"b\" @text-fg\n",
        Outcome::Refused("`stack`"),
    ),
    (
        "layout: hover",
        "  hover\n    text \"a\" @text-fg\n    text \"b\" @text-fg\n",
        Outcome::Refused("`hover`"),
    ),
    (
        "layout: flex",
        "  flex gap=4.0\n    text \"a\" @text-fg\n",
        Outcome::Refused("`flex`"),
    ),
    // The widgets the wire has no node for.
    (
        "overlay",
        "  overlay when=busy dismiss=add\n    content\n      text \"a\" @text-fg\n    layer\n      text \"b\" @text-fg\n",
        Outcome::Refused("`overlay`"),
    ),
    (
        "pane grid",
        "  panes #work w=fill h=80.0\n    pane first\n      text \"a\" @text-fg\n",
        Outcome::Refused("`pane grid`"),
    ),
    (
        "rich text",
        "  rich-text\n    span \"a\"\n",
        Outcome::Refused("`rich text`"),
    ),
    (
        "checkbox",
        "  checkbox \"e\" checked=busy -> flip _\n",
        Outcome::Refused("`checkbox`"),
    ),
    (
        "toggler",
        "  toggler \"e\" checked=busy -> flip _\n",
        Outcome::Refused("`toggler`"),
    ),
    (
        "slider",
        "  slider amount -> slide _\n",
        Outcome::Refused("`slider`"),
    ),
    (
        "progress",
        "  progress amount\n",
        Outcome::Refused("`progress`"),
    ),
    (
        "radio",
        "  radio \"a\" -> flip _\n",
        Outcome::Refused("`radio`"),
    ),
    (
        "pick list",
        "  pick [\"One\", \"Two\"] draft -> choose _\n",
        Outcome::Refused("`pick list`"),
    ),
    (
        "combo box",
        "  combo search draft \"Search\" -> choose _\n",
        Outcome::Refused("`combo box`"),
    ),
    ("qr code", "  qr draft\n", Outcome::Refused("`qr code`")),
    (
        "keyed column",
        "  keyed item in items by=item\n    text item @text-fg\n",
        Outcome::Refused("`keyed column`"),
    ),
    (
        "lazy",
        "  lazy draft as cached\n    text cached @text-fg\n",
        Outcome::Refused("`lazy`"),
    ),
    (
        "mouse area",
        "  mouse press=add\n    text \"a\" @text-fg\n",
        Outcome::Refused("`mouse area`"),
    ),
    (
        "resize handle",
        "  resize-handle drag=resized\n    box w=24.0 h=12.0\n      text \"a\" @text-fg\n",
        Outcome::Refused("`resize handle`"),
    ),
    (
        "theme",
        "  theme dark\n    text \"a\" @text-fg\n",
        Outcome::Refused("`theme`"),
    ),
    (
        "float",
        "  float x=2.0 y=3.0\n    text \"a\" @text-fg\n",
        Outcome::Refused("`float`"),
    ),
    (
        "pin",
        "  pin w=64.0 h=24.0 x=2.0 y=3.0\n    text \"a\" @text-fg\n",
        Outcome::Refused("`pin`"),
    ),
    (
        "sensor",
        "  sensor show=measured resize=measured hide=hidden\n    text \"a\" @text-fg\n",
        Outcome::Refused("`sensor`"),
    ),
    (
        "tooltip",
        "  tooltip delay=0\n    text \"a\" @text-fg\n    text \"tip\" @text-fg\n",
        Outcome::Refused("`tooltip`"),
    ),
    (
        "responsive size",
        "  responsive size=(available_width, available_height)\n    text available_width @text-fg\n",
        Outcome::Refused("`responsive size`"),
    ),
    (
        "table",
        "  table item in items\n    col\n      header\n        text \"h\" @text-fg\n      cell\n        text item @text-fg\n",
        Outcome::Refused("`table`"),
    ),
    (
        "markdown",
        "  markdown docs -> link_opened _\n",
        Outcome::Refused("`markdown`"),
    ),
    (
        "editor",
        "  editor <-> notes\n",
        Outcome::Refused("`editor`"),
    ),
    (
        "extern widget",
        "  extern native_help(busy) -> flip _\n",
        Outcome::Refused("`extern widget`"),
    ),
    (
        "themer",
        "  themer alternate_panel(true)\n",
        Outcome::Refused("`themer`"),
    ),
    (
        "shader",
        "  shader status_shader(1.0) w=fill h=24.0 -> flip _\n",
        Outcome::Refused("`shader`"),
    ),
    (
        "media: image",
        "  image picture\n",
        Outcome::Refused("`media`"),
    ),
    (
        "media: svg",
        "  svg \"<svg/>\" memory\n",
        Outcome::Refused("`media`"),
    ),
    (
        "media: viewer",
        "  viewer picture\n",
        Outcome::Refused("`media`"),
    ),
    (
        "canvas",
        "  canvas w=40.0 h=24.0\n",
        Outcome::Refused("`canvas`"),
    ),
    // Options on the nodes the wire does carry: painted, driven or measured
    // by widgets the wire has no room for.
    (
        "layout: surface utility",
        "  col @bg-primary\n    text \"a\" @text-fg\n",
        Outcome::Refused("a surface utility style on a layout"),
    ),
    (
        "box: px-snap",
        "  box px-snap=true\n    text \"a\" @text-fg\n",
        Outcome::Refused("`px-snap` on a surface"),
    ),
    (
        "layout: scroll bar option",
        "  scroll bar=hidden\n    text \"a\" @text-fg\n",
        Outcome::Refused("a scroll bar option"),
    ),
    (
        "layout: scroll anchor",
        "  scroll anchor-y=end\n    text \"a\" @text-fg\n",
        Outcome::Refused("a scroll anchor"),
    ),
    (
        "layout: scroll auto",
        "  scroll auto=busy\n    text \"a\" @text-fg\n",
        Outcome::Refused("a scroll route"),
    ),
    (
        "rule: style preset",
        "  rule horizontal style=weak\n",
        Outcome::Refused("a rule style preset"),
    ),
    (
        "rule: snap",
        "  rule horizontal snap=busy\n",
        Outcome::Refused("`snap` on a rule"),
    ),
    (
        "rule: radius",
        "  rule horizontal r=2.0\n",
        Outcome::Refused("a rule radius"),
    ),
];

#[test]
fn the_tree_target_carries_or_refuses_every_construct() {
    let mut wrong = Vec::new();
    for (construct, view, expected) in COVERAGE {
        let source = format!("{PRELUDE}{view}");
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
        match (expected, rendered) {
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
        let classified = COVERAGE.iter().any(|(construct, _, _)| {
            *construct == name || construct.starts_with(&format!("{name}: "))
        });
        assert!(
            classified,
            "`{name}` is not in the tree target's coverage table"
        );
    }
}

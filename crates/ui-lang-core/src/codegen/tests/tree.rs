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

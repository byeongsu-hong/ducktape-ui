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
fn tree_system_theme_uses_host_environment_and_refuses_unhosted_operations() {
    let source = format!(
        "app Environment\n{PALETTE}on mode(_value)\non mount\n  task system theme -> mode _\nsubscribe\n  system theme -> mode _\nview\n  text \"Environment\"\n"
    );
    let code = compile_for(&source, "environment.ice", Target::Tree).unwrap();
    assert!(code.contains("::ui_lang_guest::system::theme()"));
    assert!(code.contains("::ui_lang_guest::system::theme_changes()"));
    for operation in [
        "system info -> mode _",
        "window move -10.0 20.0",
        "window maximize true",
        "window minimize false",
        "window resizable false",
    ] {
        let handler = if operation.starts_with("system info") {
            "on mode(_value)\n"
        } else {
            ""
        };
        let source = format!(
            "app Environment\n{PALETTE}{handler}on mount\n  task {operation}\nview\n  text \"Environment\"\n"
        );
        compile_for(&source, "environment.ice", Target::Native).unwrap();
        let result = compile_for(&source, "environment.ice", Target::Tree);
        if operation.starts_with("system info") || operation.starts_with("window move") {
            assert_eq!(result.unwrap_err().code, "E190");
        } else {
            assert!(result.unwrap().contains("::ui_lang_guest::window::perform"));
        }
    }
}

#[test]
fn tree_window_geometry_subscriptions_refuse_instead_of_waiting_forever() {
    for (source, payload) in [("window moved", "_x, _y"), ("window frame", "")] {
        let source = format!(
            "app Events\n{PALETTE}on observed({payload})\nsubscribe\n  {source} -> observed{}\nview\n  text \"Events\"\n",
            if payload.contains(',') { " _ _" } else { "" }
        );
        compile_for(&source, "window-events.ice", Target::Native).expect("native window source");
        let error = compile_for(&source, "window-events.ice", Target::Tree)
            .expect_err("unsupported window source must not silently starve");
        assert_eq!(error.code, "E190");
        assert!(
            error.message.contains("window geometry or frame clocks"),
            "{error:?}"
        );
    }
}

#[test]
fn editor_highlighter_keeps_declared_empty_collection_argument_types() {
    let source = format!(
        "app Docs\n{PALETTE}extern crate::paint\n  editor-highlighter paint(dark:bool, commented:[i64])\nstate\n  notes:editor = \"hello\"\nview\n  editor <-> notes highlighter=paint(false, [])\n"
    );
    for target in [Target::Native, Target::Tree] {
        let result = compile_for(&source, "editor-arguments.ice", target);
        assert!(
            result.is_ok(),
            "declared empty list must survive {target:?} lowering: {result:?}"
        );
    }
}

#[test]
fn tree_editor_binding_uses_a_deferred_commit_and_logical_document_identity() {
    let source = format!(
        "app Docs\n{PALETTE}extern crate::keys\n  editor-binding keys(readonly:bool) -> str\nstate\n  notes:editor = \"hello\"\non command(_value)\nview\n  col\n    editor #one <-> notes key-binding=keys(false) -> command _\n    editor #two <-> notes\n"
    );
    let result = compile_for(&source, "docs.ice", Target::Tree);
    assert!(
        result.is_ok(),
        "Tree must emit checked editor bindings: {result:?}"
    );
    let code = result.unwrap();
    assert!(code.contains("EditorTransaction<"));
    assert!(code.contains(".apply(&mut self.notes)"));
    assert!(code.contains(".register("));
    assert!(code.contains("fn(bool) -> ::ui_lang_guest::EditorBinding<::std::string::String>"));
    assert!(!code.contains("fn __ice_map_editor_binding"));
    assert_eq!(
        code.matches("__editor.document((\"app:notes\").to_owned()")
            .count(),
        2,
        "both widgets deliver the one logical document"
    );
    assert_eq!(
        code.matches("::ui_lang_guest::EditorBinding::<()>::plain(")
            .count(),
        1,
        "only the editor without an authored key binding commits through the plain route"
    );
}

#[test]
fn tree_editor_binding_captures_route_arguments_and_component_scope() {
    let source = format!(
        "app Docs\n{PALETTE}extern crate::keys\n  editor-binding keys(readonly:bool) -> str\ncomponent Pad(label:str)\n  lifetime retained\n  state\n    draft:editor = \"hello\"\n  on command(_value, _label)\n  col\n    editor #one <-> draft key-binding=keys(false) -> command _ label\n    editor #two <-> draft\nview\n  col\n    Pad label=\"A\" #a\n    Pad label=\"B\" #b\n"
    );
    let code = compile_for(&source, "pads.ice", Target::Tree).expect("component editor binding");
    assert!(code.contains("component:{{:?}}") || code.contains("component:{:?}"));
    assert!(code.contains(".apply(&mut __local.draft)"));
    assert!(
        code.contains("let __route_arg_0 ="),
        "callback must own its authored argument"
    );
    assert!(code.contains("Clone::clone(&__route_arg_0)"));
    assert!(code.contains("__scope.clone(), __transaction"));
    assert!(
        code.contains("__scope.clone(), __document"),
        "each instance owns the scope its document route reports under"
    );
    assert!(
        code.contains("draft: __state.draft.text()"),
        "component test state exposes editor text on Tree too"
    );
    assert!(
        code.contains("&(__ice_use_scope)"),
        "document identity borrows the live component scope"
    );
}

#[test]
fn tree_container_linear_background_uses_native_gradient_stops() {
    let source = format!(
        "app Hero\n{PALETTE}view\n  box w=160.0 h=100.0 bg=linear(1.57, bg/10@0.0, primary/72@1.0)\n    text \"Hero\"\n"
    );
    compile_for(&source, "hero.ice", Target::Native).expect("native hero gradient");
    let tree = compile_for(&source, "hero.ice", Target::Tree);
    assert!(
        tree.is_ok(),
        "Tree must copy the native hero gradient: {tree:?}"
    );
}

#[test]
fn tree_slider_faces_carry_circle_and_rounded_rectangle_handles() {
    let source = format!(
        "app Knobs\n{PALETTE}state\n  amount = 50.0\non slide(next)\n  amount = next\nview\n  slider amount min=0.0 max=100.0 -> slide _\n    active handle=circle(0.0)\n    hovered handle=rect(12) handle-r=3.0\n    dragged handle=circle(5.0)\n"
    );
    compile_for(&source, "knobs.ice", Target::Native).expect("native handle shapes");
    let tree = compile_for(&source, "knobs.ice", Target::Tree);
    assert!(
        tree.is_ok(),
        "Tree must copy native slider handle shapes: {tree:?}"
    );
    let tree = tree.unwrap();
    assert!(tree.contains("SliderHandleShape::Circle"));
    assert!(tree.contains("SliderHandleShape::Rectangle"));
}

#[test]
fn tree_test_builds_keep_source_locations_without_native_element_wrappers() {
    let generated = tree("  col\n    editor #notes <-> notes\n    button \"Remove\" -> remove 0\n");
    assert!(generated.contains("testing::push_render_source"));
    assert!(generated.contains("// __ICE_SOURCE"));
    assert!(
        !generated.contains("testing::sourced("),
        "Tree test builds must not pass wire Nodes to the native Element wrapper"
    );
}

#[test]
fn tree_viewer_reuses_copied_image_slots_and_native_options() {
    let source = format!(
        "app Viewer\n{PALETTE}state\n  pixels = rgba(1, 1, bytes(ff 00 00 ff))\nview\n  viewer pixels #photo w=80.0 h=60.0 p=3.0 fit=contain filter=nearest min-scale=0.5 max-scale=4.0 scale-step=0.25\n"
    );
    assert!(compile_for(&source, "viewer.ice", Target::Native).is_ok());
    let generated = compile_for(&source, "viewer.ice", Target::Tree);
    assert!(
        generated.is_ok(),
        "valid copied-image viewer must lower for Tree: {generated:?}"
    );
    let generated = generated.unwrap();
    assert!(generated.contains("Node::ImageViewer"));
    assert!(generated.contains("::image("));
    assert!(generated.contains("ViewerOptions"));
}

#[test]
fn tree_images_copy_encoded_and_rgba_sources_with_native_options() {
    let source = format!(
        "app Pictures\n{PALETTE}state\n  pixels = rgba(1, 1, bytes(ff 00 00 ff))\n  data = encoded(bytes(89 50 4e 47))\nview\n  col\n    image pixels w=32.0 h=24.0 fit=cover opacity=0.5 filter=nearest rotate=rotation.solid(radians(0.5))\n    image data\n"
    );
    let code = compile_for(&source, "pictures.ice", Target::Tree).unwrap_or_else(|error| {
        panic!(
            "Tree must copy raster images: {}",
            error.render("pictures.ice")
        )
    });
    assert!(code.contains("Node::Image"));
    assert!(code.contains("::iced::advanced::image::Handle"));
}

#[test]
fn tree_images_refuse_filesystem_paths_at_the_source() {
    for widget in ["image", "viewer"] {
        for (state, source) in [
            ("", "\"/host/private.png\""),
            ("state\n  path = \"pair.png\"\n", "path"),
        ] {
            let source = format!("app Pictures\n{PALETTE}{state}view\n  {widget} {source}\n");
            let error = compile_for(&source, "pictures.ice", Target::Tree)
                .unwrap_err()
                .render("pictures.ice");
            assert!(
                error.contains("E190") && error.contains("image read from a path"),
                "{error}"
            );
        }
    }
}

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
fn mounted_tree_refusals_follow_host_condition_expansion() {
    let head = format!(
        "app Mounted\n{PALETTE}state\n  value = 1\ncomponent Leaf()\n  lifetime mounted\n  state\n    count = 0\n  text count\ncomponent Wrapper()\n  Leaf #leaf\nview\n"
    );
    let source = format!(
        "{head}  responsive size=(width, height)\n    col\n      if width > 100.0\n        Wrapper\n"
    );
    let error = compile_for(&source, "mounted.ice", Target::Tree)
        .unwrap_err()
        .render("mounted.ice");
    let line = source
        .lines()
        .position(|line| line == "  Leaf #leaf")
        .unwrap()
        + 1;
    assert!(
        error.contains("E190") && error.contains("inside a host container condition"),
        "{error}"
    );
    assert!(error.contains(&format!("mounted.ice:{line}:")), "{error}");
    let cached = compile_for(
        &format!("{head}  lazy value as cached\n    Wrapper\n"),
        "mounted.ice",
        Target::Tree,
    )
    .unwrap();
    assert!(cached.contains("::ui_lang_guest::slots::component(\"Leaf\""));
    assert!(cached.contains("::ui_lang_guest::slots::mounted_scopes(\"Leaf\""));
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

/// Tree editors carry a document reference and its delivery route; a
/// disabled editor still receives its document, and only loses host input.
#[test]
fn an_editor_carries_a_document_reference_the_host_requests() {
    let generated = tree_with(
        "on clear\n  notes = editor(\"\")\nderived\n  lines = editor_line_count(notes)\n  second = editor_line(notes, 1)\n",
        "  col\n    editor #notes <-> notes hint=\"Write\" h=fill min-h=80.0 disabled=busy\n    text editor_text(notes) @text-fg\n    text lines @text-fg\n    button \"Clear\" -> clear\n    button \"×\" -> remove 0\n",
    );
    for expected in [
        "notes: ::ui_lang_guest::Editor,",
        "__EditNotes(::ui_lang_guest::EditorDocumentUpdate),",
        "__DemoMessage::__EditNotes(__document) => {",
        "__document.apply(&mut self.notes)",
        "let (__document, __on_document) = __editor.document((\"app:notes\").to_owned(), __DemoMessage::__EditNotes as fn(::ui_lang_guest::EditorDocumentUpdate) -> __DemoMessage);",
        "::ui_lang_guest::wire::Node::Editor { options:",
        "placeholder: \"Write\".to_owned()",
        "document: __document, on_document: __on_document",
        "editable: !(self.busy)",
        "::ui_lang_guest::EditorBinding::<()>::plain(",
        "height: ::std::option::Option::Some(::ui_lang_guest::wire::Length::Fill)",
        "min_height: ::std::option::Option::Some((80.0) as f32)",
        "(self.notes).text()",
        ".line(__line)",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected:?} in:\n{generated}"
        );
    }
    assert!(
        !generated.contains("__editor.text()"),
        "the node carries a reference, never a copy of the document:\n{generated}"
    );
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
fn float_geometry_arithmetic_and_guest_snapshots_build_wire_operations() {
    let source = format!(
        "app Demo\n{PALETTE}extern crate::host\n  pure offset(value:f64) -> f64\nstate\n  shift = 2.0\nview\n  float scale=1.1 x=(viewport_x + viewport_width - original_x - original_width + offset(shift)) y=(-(viewport_y + viewport_height - original_y - original_height) * 2.0 / 3.0 % 4.0) shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 r=8.0 r-tl=1.0\n    text \"floating\"\n"
    );
    let result = compile_for(&source, "float.ice", Target::Tree);
    assert!(result.is_ok(), "float arithmetic must compile: {result:?}");
    let generated = result.unwrap();
    for expected in [
        "Node::Float",
        "FloatExpression",
        "FloatOp::Negate",
        "FloatOp::Add",
        "FloatOp::Subtract",
        "FloatOp::Multiply",
        "FloatOp::Divide",
        "FloatOp::Remainder",
        "FloatOp::Number((crate::host::offset(",
        ".max(f32::EPSILON).min(f32::MAX)",
        "Shadow",
        "radius: ::std::option::Option::Some",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected}: {generated}"
        );
    }
    for index in 0..8 {
        assert!(
            generated.contains(&format!("FloatOp::Geometry({index})")),
            "missing geometry {index}: {generated}"
        );
    }
    assert!(!generated.contains("__original"));
    assert!(!generated.contains("__viewport"));
    let defaults = compile_for(
        &format!("app Demo\n{PALETTE}view\n  float\n    text \"default\"\n"),
        "float.ice",
        Target::Tree,
    )
    .unwrap();
    assert!(
        defaults.matches("FloatOp::Number((0.0) as f64)").count() >= 2,
        "{defaults}"
    );
    assert!(
        defaults.contains("radius: ::std::option::Option::None"),
        "{defaults}"
    );
}

#[test]
fn float_geometry_calls_are_refused_even_inside_nested_arguments() {
    for expression in [
        "offset(viewport_width)",
        "offset(-viewport_width + 1.0)",
        "total([1.0, viewport_width])",
        "offset(total([viewport_width]))",
    ] {
        let source = format!(
            "app Demo\n{PALETTE}extern crate::host\n  pure offset(value:f64) -> f64\n  pure total(values:[f64]) -> f64\nview\n  float x={expression}\n    text \"floating\"\n"
        );
        compile(&source, "float.ice").unwrap();
        let error = compile_for(&source, "float.ice", Target::Tree)
            .unwrap_err()
            .render("float.ice");
        assert!(
            error.contains("E190") && error.contains("function of float geometry"),
            "{error}"
        );
    }
}

#[test]
fn float_geometry_operation_budget_accepts_64_and_refuses_65() {
    // Exercise the operation count without making native parsing/checking
    // consume a deep expression stack on smaller CI test threads.
    fn sum(leaves: usize) -> String {
        if leaves == 1 {
            return "viewport_width".into();
        }
        let child = sum(leaves / 2);
        format!("({child} + {child})")
    }
    let sum = sum(32);
    for (expression, accepted) in [(format!("-({sum})"), true), (format!("{sum} + 1.0"), false)] {
        let source =
            format!("app Demo\n{PALETTE}view\n  float x=({expression})\n    text \"floating\"\n");
        compile(&source, "float.ice").unwrap();
        let result = compile_for(&source, "float.ice", Target::Tree);
        if accepted {
            let generated = result.expect("64 operations must compile");
            assert_eq!(generated.matches("FloatOp::Geometry").count(), 32);
        } else {
            let error = result.unwrap_err().render("float.ice");
            assert!(
                error.contains("E190") && error.contains("more than 64 operations"),
                "{error}"
            );
        }
    }
}

#[test]
fn a_construct_the_wire_does_not_carry_fails_at_its_line() {
    let source = format!(
        "app Demo\n{PALETTE}state\n  draft = \"\"\nview\n  col\n    text \"before\" @text-fg\n    theme dark\n      text \"after\"\n"
    );
    let error = compile_for(&source, "demo.ice", Target::Tree).unwrap_err();
    let rendered = error.render("demo.ice");
    assert!(rendered.contains("E190"), "{rendered}");
    assert!(
        rendered.contains("`theme` is not available in a view module"),
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
    emitted(
        "combo box",
        CHOOSE,
        "  combo search choice \"Search\" -> choose _\n",
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
    emitted(
        "resize handle",
        "on resized(_dx, _dy)\n",
        "  resize-handle drag=resized\n    box w=24.0 h=12.0\n      text \"a\" @text-fg\n",
    ),
    refused(
        "theme",
        "",
        "  theme dark\n    text \"a\" @text-fg\n",
        "`theme`",
    ),
    emitted(
        "float",
        "",
        "  float x=2.0 y=3.0\n    text \"a\" @text-fg\n",
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
    emitted(
        "editor: key binding",
        "on command(_value)\n",
        "  editor <-> notes key-binding=editor_keys(busy) -> command _\n",
    ),
    refused(
        "editor: highlight",
        "",
        "  editor <-> notes highlight=\"rs\"\n",
        "an editor highlighter",
    ),
    emitted(
        "editor: highlighter",
        "",
        "  editor <-> notes highlighter=editor_highlight(draft)\n",
    ),
    refused(
        "editor: style callback",
        "",
        "  editor <-> notes style=editor_surface(busy)\n",
        "a style on an editor",
    ),
    emitted(
        "editor: status style",
        "",
        "  editor <-> notes\n    active bg=bg\n",
    ),
    emitted("editor: text option", "", "  editor <-> notes size=14.0\n"),
    refused("themer", "", "  themer alternate_panel(true)\n", "`themer`"),
    emitted(
        "shader",
        FLIP,
        "  shader status_shader(1.0) w=fill h=24.0 -> flip _\n",
    ),
    emitted("media: image", "", "  image picture\n"),
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
    emitted("media: viewer", "", "  viewer picture\n"),
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
        "box: linear gradient",
        "",
        "  box bg=linear(1.57, bg/10@0.0, primary/72@1.0)\n    text \"a\"\n",
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
    emitted(
        "layout: scroll route",
        "on scrolled(_x, _y, _rx, _ry)\n",
        "  scroll scroll=scrolled\n    text \"a\" @text-fg\n",
    ),
    emitted("rule: style preset", "", "  rule horizontal style=weak\n"),
    emitted("rule: snap", "", "  rule horizontal snap=busy\n"),
    emitted("rule: radius", "", "  rule horizontal r=2.0\n"),
    emitted(
        "sensor: key",
        MEASURE,
        "  sensor show=measured key=draft\n    text \"a\" @text-fg\n",
    ),
    emitted(
        "sensor: boolean key",
        MEASURE,
        "  sensor show=measured key=busy\n    text \"a\" @text-fg\n",
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
    emitted(
        "slider: handle shape",
        SLIDE,
        "  slider amount min=0.0 max=100.0 -> slide _\n    active handle=circle(4.0)\n",
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

#[test]
fn tree_linear_layout_options_cross_the_wire() {
    let source = format!(
        "app Demo\n{PALETTE}view\n  col w=fill max-w=860.0 clip=true\n    row clip=false\n      text \"Settings\"\n    col clip=true\n      text \"Details\"\n"
    );
    let generated = compile_for(&source, "linear.ice", Target::Tree).unwrap();
    assert!(generated.contains("max_width: ::std::option::Option::Some("));
    assert!(generated.contains("clip: true"));
    assert!(generated.contains("clip: false"));
}

#[test]
fn snapshot_schema_tracks_state_shape_but_not_view_layout() {
    let source = format!(
        "app Snap\n{PALETTE}state\n  draft = \"hello\"\ncomponent Counter()\n  lifetime retained\n  state\n    count = 0\n  text count\nview\n  col\n    text draft\n    Counter #counter\n"
    );
    let schema = |source: &str| {
        let generated = compile_for(source, "snapshot.ice", Target::Tree).unwrap();
        generated
            .lines()
            .find(|line| line.contains("const __SNAPSHOT_SCHEMA:"))
            .unwrap()
            .to_owned()
    };
    let original = schema(&source);
    assert_eq!(original, schema(&source.replace("  col\n", "  row\n")));
    assert_eq!(
        original,
        schema(&source.replace("hello", "different initializer"))
    );
    assert_ne!(
        original,
        schema(&source.replace("count = 0", "count = 0.5"))
    );
    assert_ne!(
        original,
        schema(&source.replace("lifetime retained", "lifetime mounted"))
    );
}

#[test]
fn tree_box_shadows_copy_signed_offsets_and_blur() {
    let source = format!(
        "app Shadow\n{PALETTE}state\n  offset:f64 = -12.0\nview\n  box shadow=primary/50 shadow-x=offset shadow-y=6.0 shadow-blur=18.0\n    text \"Popover\"\n"
    );
    let generated = compile_for(&source, "shadow.ice", Target::Tree).unwrap();
    assert!(generated.contains("shadow: ::ui_lang_guest::wire::Shadow"));
    assert!(generated.contains("self.offset"));
}

#[test]
fn tree_editor_copies_presentation_and_all_status_faces() {
    let source = format!(
        "app Edit\n{PALETTE}font code family=\"Geist Mono\"\nstate\n  draft:editor = editor(\"text\")\nview\n  editor <-> draft hint=\"File contents\" size=12.0 line-h=1.3 p=6.6 wrap=word font=code\n    active bg=bg border=fg border-w=1.0 r=8.0 value=fg placeholder=primary selection=fg/18\n    hovered bg=primary\n    focused border=danger\n    focused-hovered value=primary\n    disabled bg=danger\n"
    );
    let generated = compile_for(&source, "editor.ice", Target::Tree).unwrap();
    for expected in [
        "::std::boxed::Box::new(::ui_lang_guest::wire::EditorOptions",
        "..::std::default::Default::default()",
        "LineHeight::Relative",
        "Wrapping::Word",
        "Geist Mono",
        "focused_hovered:",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn tree_pick_copies_native_declarative_options() {
    let source = format!(
        "app Pick\n{PALETTE}state\n  items = [\"One\", \"Two\"]\n  selected:str? = none\non choose(value)\n  selected = some(value)\non opened\n  selected = none\non closed\n  selected = none\nview\n  pick items selected open=opened close=closed -> choose _\n    with\n      hint=\"Choose\"\n      w=180.0\n      p=8.0\n      text-size=12.5\n    active text=fg handle=primary bg=bg border=fg border-w=1.0 r=8.0\n    hovered handle=fg\n    opened border=primary\n    opened-hovered border=danger\n    menu text=fg selected-text=fg selected-bg=primary bg=bg border=fg border-w=1.0 r=10.0 shadow=fg shadow-y=6.0 shadow-blur=18.0\n    handle dynamic\n      closed code=\"▼\" size=11.0\n      open code=\"▲\" size=11.0\n"
    );
    let code = compile_for(&source, "pick.ice", Target::Tree)
        .unwrap_or_else(|error| panic!("{}", error.render("pick.ice")));
    assert!(code.contains("PickOptions"));
    assert!(code.contains("PickHandle::Dynamic"));
}

#[test]
fn tree_pick_handle_variants_compile() {
    for handle in [
        "handle arrow size=12.0",
        "handle static code=\"◆\" size=12.0",
        "handle none",
    ] {
        let source = format!(
            "app Pick\n{PALETTE}state\n  choices = [\"One\"]\n  selected:str? = none\non choose(value)\n  selected = some(value)\nview\n  pick choices selected -> choose _\n    {handle}\n"
        );
        compile(&source, "pick.ice").unwrap();
        compile_for(&source, "pick.ice", Target::Tree).unwrap();
    }
}

#[test]
fn tree_manifest_preferred_window_size_is_static_and_preserves_f32() {
    for (settings, expected) in [
        ("", "none"),
        ("  window\n    size 640.5 480.25\n", "640.5,480.25"),
    ] {
        let source = format!("app Sized\n{settings}{PALETTE}view\n  text \"Sized\"\n");
        let generated = compile_for(&source, "sized.ice", Target::Tree).unwrap();
        assert!(generated.contains(&format!(
            "__PREFERRED_WINDOW_SIZE: &'static str = {expected:?}"
        )));
    }
}

#[test]
fn tree_preferred_size_rejects_out_of_bounds_at_declaration() {
    for size in [
        "9000 500",
        "500 8192.01",
        "0.00000000000000000000000000000000000000000000000001 500",
    ] {
        let source =
            format!("app Sized\n  window\n    size {size}\n{PALETTE}view\n  text \"Sized\"\n");
        assert!(
            compile_for(&source, "sized.ice", Target::Native).is_ok(),
            "native range remains unchanged"
        );
        let error = compile_for(&source, "sized.ice", Target::Tree)
            .expect_err("Tree preferred size must fit host bounds");
        assert_eq!(error.code, "E190");
        assert_eq!(error.line, 3);
        assert!(error.message.contains("8192"));
    }
    let source =
        format!("app Sized\n  window\n    size 8192 8192\n{PALETTE}view\n  text \"Sized\"\n");
    assert!(compile_for(&source, "sized.ice", Target::Tree).is_ok());
}

#[test]
fn identified_shared_tree_layouts_never_wrap_wire_nodes_in_native_containers() {
    for view in [
        "  keyed item in [1, 2] by=item #entries\n    text item\n",
        "  lazy 1 as cached #cached\n    text cached\n",
    ] {
        let source = format!("app Demo\n{PALETTE}view\n{view}");
        let native = compile(&source, "identity.ice").unwrap();
        assert!(native.contains("container(__identified)"));
        let tree = compile_for(&source, "identity.ice", Target::Tree).unwrap();
        assert!(
            !tree.contains("container(__identified)"),
            "Tree identities belong to wire nodes, not native wrappers"
        );
        if view.contains("keyed") {
            assert!(
                tree.contains("Node::KeyedColumn { key: __ice_node_scope.clone()"),
                "keyed identity must be the node's exact key"
            );
        }
    }
}

#[test]
fn local_window_effects_use_the_guest_request_channel() {
    for (statement, command) in [
        ("task window focus", "WindowCommand::Focus"),
        ("task window resize 600.5 400.25", "WindowCommand::Resize"),
        ("task window close", "WindowCommand::Close"),
        ("exit", "WindowCommand::Close"),
    ] {
        let source = format!(
            "app WindowEffects\n{PALETTE}on mount\n  {statement}\nview\n  text \"Window\"\n"
        );
        compile_for(&source, "window-effects.ice", Target::Native).unwrap();
        let tree = compile_for(&source, "window-effects.ice", Target::Tree).unwrap();
        assert!(
            tree.contains(command),
            "{statement} must send a scoped guest command"
        );
        assert!(!tree.contains("::iced::window::oldest()"));
        assert!(!tree.contains("::iced::exit::<"));
    }
}

#[test]
fn tree_window_effects_refuse_explicit_native_window_ids() {
    let source = format!(
        "app WindowEffects\n{PALETTE}extern crate::backend\n  pure target() -> window-id\non mount\n  task window focus target=target()\nview\n  text \"Window\"\n"
    );
    compile_for(&source, "window-effects.ice", Target::Native).unwrap();
    let error = compile_for(&source, "window-effects.ice", Target::Tree).unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("own host window"));
}

#[test]
fn combo_tree_retains_typed_options_routes_and_reset_snapshot() {
    let source = format!(
        "app Search\n{PALETTE}state\n  options:combo[str] = [\"Apple\", \"Berry\"]\n  selected:str? = none\n  query = \"\"\non choose(value)\n  selected = some(value)\non input(value)\n  query = value\non hover(value)\n  query = value\non opened\non closed\non reset\n  options = [\"Apple\", \"Berry\"]\non append\n  combo options push \"Blueberry\"\nview\n  combo options selected \"Search\" input=input hover=hover open=opened close=closed -> choose _\n"
    );
    compile_for(&source, "combo.ice", Target::Native).unwrap();
    let tree = compile_for(&source, "combo.ice", Target::Tree).unwrap();
    assert!(tree.contains("Node::ComboBox"));
    assert!(tree.contains("::ui_lang_guest::Combo<"));
    assert!(tree.contains("self.options.replace("));
    assert!(tree.contains("self.options.push("));
    assert!(tree.contains("reset_revision()"));
    assert!(tree.contains("::ui_lang_guest::Combo::restore("));
    assert!(tree.contains("__table.get(__sent as usize).cloned()"));
}

#[test]
fn combo_tree_refuses_rust_style_callbacks_and_unowned_parameters() {
    let styled = format!(
        "app Search\nextern crate::backend\n  input-style custom()\n{PALETTE}state\n  values:combo[str] = [\"One\"]\n  selected:str? = none\non choose(value)\nview\n  combo values selected \"Search\" style=custom() -> choose _\n"
    );
    let parameter = format!(
        "app Search\n{PALETTE}state\n  values:combo[str] = [\"One\"]\ncomponent Picker(values:combo[str])\n  state\n    selected:str? = none\n  on choose(value)\n    selected = some(value)\n  combo values selected \"Search\" -> choose _\nview\n  Picker values=values\n"
    );
    for (source, message) in [
        (&styled, "custom combo"),
        (&parameter, "without owned app state"),
    ] {
        compile_for(source, "combo.ice", Target::Native).unwrap();
        let error = compile_for(source, "combo.ice", Target::Tree).unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains(message), "{}", error.message);
    }
}

#[test]
fn mouse_interest_is_constructed_inside_the_active_tree_subscription_branch() {
    let source = format!(
        "app Pointer\n{PALETTE}state\n  active = true\non moved(_x, _y)\non event(_event)\nsubscribe\n  mouse moved when active -> moved _ _\n  event raw when active -> event _\nview\n  text \"Pointer\"\n"
    );
    let generated = compile_for(&source, "pointer.ice", Target::Tree).unwrap();
    assert_eq!(
        generated
            .matches(
                "if self.active { ::iced::Subscription::batch([::ui_lang_guest::mouse::observe("
            )
            .count(),
        2
    );
    assert_eq!(
        generated
            .matches("::ui_lang_guest::mouse::observe(")
            .count(),
        2
    );
    let native = compile(&source, "pointer.ice").unwrap();
    assert!(!native.contains("::ui_lang_guest::mouse::observe("));
}

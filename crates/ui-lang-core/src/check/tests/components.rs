use super::*;
use crate::{EffectKind, FutureMode, Statement};

#[test]
fn checks_named_component_event_routes_and_payloads() {
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
state
  page = "home"
component Choice(page:str)
  emits
    confirm
    select(str)
    favorite(str, bool)
  col
    button "Confirm" -> emit(confirm)
    button "Select" -> emit(select, "roadmap")
    checkbox "Favorite" checked=false -> emit(favorite, page, _)
on confirmed
on selected(page)
on favorite_changed(page, next)
view
  Choice page=page
    events
      confirm -> confirmed
      select -> selected _
      favorite -> favorite_changed _ _
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().components[0].events[0].payloads,
        Vec::<Type>::new()
    );
    assert_eq!(
        document.source_document().handlers[1].params[0].ty,
        Type::Str
    );
    assert_eq!(
        document.source_document().handlers[2].params[0].ty,
        Type::Str
    );
    assert_eq!(
        document.source_document().handlers[2].params[1].ty,
        Type::Bool
    );

    for (replacement, expected) in [
        ("", "requires a route for event `favorite`"),
        (
            "      favorite -> favorite_changed _ _\n      missing -> confirmed\n",
            "does not declare event `missing`",
        ),
        (
            "      favorite -> favorite_changed _ _\n      favorite -> favorite_changed _ _\n",
            "more than once",
        ),
    ] {
        let changed = source.replace("      favorite -> favorite_changed _ _\n", replacement);
        let error = analyze(&changed).unwrap_err();
        assert!(error.message.contains(expected), "{}", error.message);
    }
}

#[test]
fn carries_ui_enum_payloads_through_named_events_into_exhaustive_matches() {
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
enum Selection
  idle
  page(str)
state
  selection:Selection = Selection.idle
component Picker()
  emits
    choose(Selection)
  button "Roadmap" -> emit(choose, Selection.page("roadmap"))
on chosen(next)
  selection = next
view
  col
    Picker
      events
        choose -> chosen _
    match selection
      Selection.idle
        text "None"
      Selection.page(page)
        text page
"#;

    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().components[0].events[0].payloads,
        vec![Type::Named("Selection".into())]
    );
}

#[test]
fn forwards_only_matching_component_events() {
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
component Item()
  emits
    select(str)
  button "Open" -> emit(select, "roadmap")
component List()
  emits
    select(str)
  Item
    forward
      select
on selected(page)
view
  List
    events
      select -> selected _
"#;
    analyze(source).unwrap();

    let verbose = source.replace(
        "    forward\n      select",
        "    events\n      select -> emit(select, _)",
    );
    let error = analyze(&verbose).unwrap_err();
    assert_eq!(error.code, "E127");
    assert!(error.message.contains("exact component event forward"));

    let app_forward = source.replace(
        "  List\n    events\n      select -> selected _",
        "  Item\n    forward\n      select",
    );
    let error = analyze(&app_forward).unwrap_err();
    assert_eq!(error.code, "E127");
    assert!(error.message.contains("only valid inside a component"));

    let mismatch = source.replace(
        "component List()\n  emits\n    select(str)",
        "component List()\n  emits\n    select(bool)",
    );
    let error = analyze(&mismatch).unwrap_err();
    assert_eq!(error.code, "E127");
    assert!(error.message.contains("has signature"));
}

#[test]
fn component_event_routes_use_caller_scope_and_components_are_closed() {
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
state
  page = "home"
component Favorite()
  emits
    changed(bool)
  checkbox "Favorite" checked=false -> emit(changed, _)
on changed(page, next)
view
  Favorite
    events
      changed -> changed(page, _)
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[0].params[0].ty,
        Type::Str
    );
    assert_eq!(
        document.source_document().handlers[0].params[1].ty,
        Type::Bool
    );

    let closed = source.replace("-> emit(changed, _)", "-> changed _");
    let error = analyze(&closed).unwrap_err();
    assert_eq!(error.code, "E132");
    assert!(error.message.contains("cannot reference app handler"));
}

#[test]
fn routes_sensor_dimensions_to_named_component_events() {
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
component Measured()
  emits
    shown(f64, f64)
    resized(f64, f64)
  sensor show=emit(shown, _, _) resize=emit(resized, _, _)
    space w=fill h=fill
on shown(width, height)
on resized(width, height)
view
  Measured
    events
      shown -> shown _ _
      resized -> resized _ _
"#;
    let document = analyze(source).unwrap();
    for handler in &document.source_document().handlers {
        assert_eq!(handler.params[0].ty, Type::F64);
        assert_eq!(handler.params[1].ty, Type::F64);
    }

    let wrong_event = source
        .replace("shown(f64, f64)", "shown(f64)")
        .replace("on shown(width, height)", "on shown(width)")
        .replace("shown -> shown _ _", "shown -> shown _");
    let error = analyze(&wrong_event).unwrap_err();
    assert_eq!(error.code, "E133");
    assert!(error.message.contains("expects 1 values, got 2"));
}

#[test]
fn requires_component_output_routes_and_matching_emit_values() {
    let missing_route = analyze(
        r#"app Demo
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
component Choice() -> bool
  checkbox "Choice" checked=false -> emit(_)
view
  Choice
"#,
    )
    .unwrap_err();
    assert_eq!(missing_route.code, "E126");

    let wrong_output = analyze(
        r#"app Demo
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
component Choice() -> str
  checkbox "Choice" checked=false -> emit(_)
on changed(next)
view
  Choice -> changed _
"#,
    )
    .unwrap_err();
    assert_eq!(wrong_output.code, "E101");
}

#[test]
fn rejects_component_output_routes_from_handlers() {
    let error = analyze(
        r#"app Demo
extern crate::backend
  fetch() -> str
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
component Search() -> str
  on search
    run every fetch() -> emit(_)
  button "Search" -> search
on changed(value)
view
  Search -> changed _
"#,
    )
    .unwrap_err();
    assert_eq!(error.code, "E135");
    assert!(error.message.contains("component view"));
}

#[test]
fn infers_single_ordered_component_output_payloads() {
    let document = analyze(
        r#"app Demo
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
component PointerCapture() -> mouse-button
  canvas w=fill h=120.0
    event mouse pressed -> emit(_)
    circle x=60.0 y=60.0 r=24.0 fill=primary
on changed(value)
view
  PointerCapture -> changed _
"#,
    )
    .unwrap();
    assert_eq!(
        document.source_document().handlers[0].params[0].ty,
        Type::MouseButton
    );
}

#[test]
fn checks_optional_selection_values() {
    let source = r#"app Demo
extern crate::backend
  pick-list-style dynamic_pick(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
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
  busy = false
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
on opened
view
  pick choices selected hint="Choose" line-h=1.2 shape=advanced font=ui open=opened style=dynamic_pick(busy) menu-style=dynamic_menu(busy) -> selected _
    active text=fg placeholder=danger handle=primary bg=bg border=fg border-w=1.0 r=4.0
    hovered text=fg
    opened text=fg
    opened-hovered text=fg
    menu text=fg selected-text=bg selected-bg=primary bg=bg border=fg shadow=danger shadow-y=2.0
    handle dynamic
      closed code="⌄" font=ui size=12.0 line-h=1.0 shape=basic
      open code="⌃" font=ui size=12.0 line-h=1.0 shape=advanced
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.source_document().states[1].ty.display(), "[str]");
    assert_eq!(document.source_document().states[2].ty.display(), "str?");
    assert_eq!(
        document.source_document().handlers[0].params[0]
            .ty
            .display(),
        "str"
    );

    let error = analyze(&source.replace("size=12.0", "size=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("icon size"));

    let error = analyze(&source.replace("dynamic_pick(busy)", "missing(busy)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("pick-list style"));

    let error = analyze(&source.replace("dynamic_menu(busy)", "missing(busy)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("menu style"));

    let error = analyze(&source.replace("dynamic_pick(busy)", "dynamic_pick(1.0)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("style=dynamic_pick(busy)", "style=primary")).unwrap_err();
    assert_eq!(error.code, "E087");
    assert!(error.message.contains("declared style call"));
}

#[test]
fn rejects_a_non_optional_pick_selection() {
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
state
  choices = ["List", "Board"]
  selected = "List"
on selected(next)
  selected = next
view
  pick choices selected -> selected _
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("optional"));
}

#[test]
fn checks_qr_payloads() {
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
state
  minted = "https://example.com/invite"
  size = 4.0
view
  qr "hello" version=micro(0)
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E136");
    assert!(error.message.contains("micro(1..4)"));

    // A literal payload is encoded at compile time, so one that cannot fit the
    // requested version fails the build.
    let error = analyze(&source.replace("version=micro(0)", "version=micro(1)")).unwrap_err();
    assert_eq!(error.code, "E136");
    assert!(error.message.contains("cannot encode qr payload"));

    // A runtime payload cannot be encoded early, but it still has to be text.
    analyze(&source.replace("qr \"hello\" version=micro(0)", "qr minted")).unwrap();
    let error = analyze(&source.replace("qr \"hello\" version=micro(0)", "qr size")).unwrap_err();
    assert_eq!(error.hint.as_deref(), Some("qr accepts str or bytes"));
}

#[test]
fn rejects_unknown_nested_theme_colors() {
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
  theme dark fg=missing
    text "Hello"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E137");
    assert!(error.message.contains("missing"));

    let source = source.replace(
        "theme dark fg=missing",
        "theme dark bg=linear(1.57, bg@0.0, missing@1.0)",
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E137");
    assert!(error.message.contains("missing"));
}

#[test]
fn checks_component_slot_contracts() {
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
state
  draft = ""
component Card(title:str, padded:bool)
  col
    text title
    slot
view
  Card padded=true title="Editor"
    input "Name" <-> draft
"#;
    analyze(source).unwrap();
    let error = analyze(&source.replace(
        "Card padded=true title=\"Editor\"",
        "Card(\"Editor\", true)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E072");
    assert!(error.message.contains("invalid component name"));

    let error = analyze(&source.replace(
        "  Card padded=true title=\"Editor\"\n    input \"Name\" <-> draft",
        "  Card padded=true title=\"Editor\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("requires slot `children`"));

    let error = analyze(&source.replace("    text title\n    slot", "    text title")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `children`"));

    let error = analyze(&source.replace("padded=true ", "")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("missing prop `padded`"));

    let error = analyze(&source.replace("padded=true", "raised=true")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("no prop `raised`"));

    let error = analyze(&source.replace("padded=true", "title=\"Again\"")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("prop `title` more than once"));

    let error = analyze(&source.replace("title=\"Editor\"", "title=true")).unwrap_err();
    assert!(error.message.contains("expected `str`, got `bool`"));

    let error = analyze(&source.replace("padded:bool", "title:bool")).unwrap_err();
    assert_eq!(error.code, "E100");
    assert!(error.message.contains("duplicate component prop `title`"));
}

#[test]
fn checks_closed_component_prop_defaults() {
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
state
  app_title = "Captured"
component Panel(title:str, description:str="", elevated:bool=false)
  col
    text title
    text description
    if elevated
      text "Elevated"
view
  Panel title="Editor"
"#;
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("description:str=\"\"", "description:str=app_title")).unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.message.contains("unknown value `app_title`"));

    let error =
        analyze(&source.replace("description:str=\"\"", "description:str=title")).unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.message.contains("unknown value `title`"));

    let error =
        analyze(&source.replace("elevated:bool=false", "elevated:bool=\"yes\"")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("title:str", "title:editor=\"\"")).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("cannot default a mutable value"));

    let error = analyze(&source.replace("Panel title=\"Editor\"", "Panel")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("missing prop `title`"));

    let error = analyze(
        &source
            .replace(
                "component Panel(",
                "extern crate::backend\n  sync fallback_title() -> str\ncomponent Panel(",
            )
            .replace("description:str=\"\"", "description:str=fallback_title()"),
    )
    .unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(
        error
            .message
            .contains("cannot call extern function `fallback_title`")
    );

    let error = analyze(&source.replace("title:str", "bind title:str=\"Editor\"")).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(
        error
            .message
            .contains("bind prop `title` cannot declare a default")
    );

    let error = analyze(&source.replace(
        "title:str, description:str=\"\"",
        "description:str=\"\", title:str",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(
        error
            .message
            .contains("required prop `title` cannot follow")
    );
}

#[test]
fn checks_named_component_slots() {
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
component Dialog(title:str)
  col
    slot header
    text title
    slot body
    slot actions
on cancel
on delete
view
  Dialog title="Delete task?"
    header:
      text "Danger zone"
    body:
      col
        text "This cannot be undone."
    actions:
      row
        button "Cancel" -> cancel
        button "Delete" -> delete
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace(
            "    actions:\n      row\n        button \"Cancel\" -> cancel\n        button \"Delete\" -> delete\n",
            "",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("requires slot `actions`"));

    let error = analyze(&source.replace("    actions:", "    footer:")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `footer`"));

    let error = analyze(&source.replace(
        "    body:\n      col\n        text \"This cannot be undone.\"",
        "    body:\n      text \"First\"\n      text \"Second\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("slot `body` needs exactly one root"));

    let error = analyze(&source.replace("    slot actions", "    slot body")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(
        error
            .message
            .contains("declares slot `body` more than once")
    );
}

#[test]
fn checks_optional_component_slots_and_provided() {
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
component Card()
  col
    slot Body
    if provided(Footer)
      box
        slot Footer?
view
  Card
    Body:
      text "Body"
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("provided(Footer)", "provided(Missing)")).unwrap_err();
    assert_eq!(error.code, "E152");
    assert!(error.message.contains("does not declare slot `Missing`"));

    let error = analyze(&source.replace("slot Footer?", "slot Footer")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("requires slot `Footer`"));
}

#[test]
fn checks_multiline_with_metadata() {
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
state
  draft = ""
component Panel(title:str)
  box
    slot
view
  Panel
    with
      title="Editor"
    input "Draft" <-> draft
      with
        hint="Write"
        disabled=false
        @p-4
      active border=primary
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "    with\n      title=\"Editor\"",
        "    with\n      title=\"Editor\"\n    with\n      title=\"Again\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("duplicate with blocks"));
}

#[test]
fn checks_compound_component_slots() {
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
component Dialog()
  col
    slot Header
    slot Body
    slot Actions
component Dialog.Header(title:str)
  col
    text title
    slot
component Dialog.Body()
  box
    slot
component Dialog.Actions()
  row
    slot
on close
view
  Dialog
    Dialog.Header title="About"
      text "Compound title"
    Dialog.Body
      text "Structured body"
    Dialog.Actions
      button "Close" -> close
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("    slot Actions\n", "")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `Actions`"));

    let error = analyze(&source.replace(
        "    Dialog.Actions\n      button \"Close\" -> close",
        "    text \"not compound\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("cannot mix compound components"));

    let error = analyze(&source.replace("Dialog.Header", "Dialog..Header")).unwrap_err();
    assert_eq!(error.code, "E072");
    assert!(error.message.contains("invalid component name"));
}

#[test]
fn checks_keyed_columns_and_copyable_keys() {
    let source = r#"app Demo
extern crate::backend
  Item(id:i64, name:str)
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
  items:[Item] = []
view
  keyed item in items by=item.id w=fill h=shrink gap=8.0 p=4.0 max-w=640.0 align=center
    text item.name
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("by=item.id", "by=item.name")).unwrap_err();
    assert_eq!(error.code, "E138");
    assert!(error.message.contains("bool, i64, or f64"));

    let error = analyze(&source.replace("gap=8.0", "gap=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));
}

#[test]
fn checks_lazy_static_boundaries() {
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
state
  title = "Hello"
  other = "Outside"
view
  lazy title as cached
    col
      text cached
      text len(cached)
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("text len(cached)", "text other")).unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.message.contains("unknown value `other`"));

    let error = analyze(&source.replace("title = \"Hello\"", "title = 1.0")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("stable hashing"));

    let error =
        analyze(&source.replace("text len(cached)", "input \"Edit\" <-> cached")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("borrows app state"));

    let component_source = source.replace(
            "view\n  lazy title as cached\n    col\n      text cached\n      text len(cached)",
            "component Editor(bind value:str)\n  input \"Edit\" <-> value\nview\n  lazy title as cached\n    Editor value<->cached",
        );
    let error = analyze(&component_source).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("borrows app state"));
}

#[test]
fn checks_markdown_content_settings_and_links() {
    let source = r##"app Docs
font ui family=sans
extern crate::backend
  markdown-viewer docs_viewer(prefix:str) -> str
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
  docs:markdown = "# Hello [world](https://example.com)"
  images:[str] = []
on open(url)
on reset
  docs = markdown("# Reset")
on extend
  markdown docs append "\n![Ice](asset://ice)"
  images = markdown_images(docs)
view
  markdown docs text-size=16.0 h1-size=32.0 h2-size=28.0 h3-size=24.0 h4-size=20.0 h5-size=18.0 h6-size=16.0 code-size=13.0 gap=12.0 viewer=docs_viewer("docs") -> open _
    style font=ui inline-code-bg=linear(1.57, bg@0.0, primary@1.0) inline-code-fg=fg inline-code-font=mono code-block-font=mono link=primary inline-code-p=2.0 inline-code-px=3.0 inline-code-py=4.0 inline-code-pt=5.0 inline-code-pr=6.0 inline-code-pb=7.0 inline-code-pl=8.0 inline-code-border=primary inline-code-border-w=1.0 inline-code-r=4.0 inline-code-r-tl=1.0 inline-code-r-tr=2.0 inline-code-r-br=3.0 inline-code-r-bl=4.0
"##;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().states[0].ty.display(),
        "markdown"
    );
    assert_eq!(
        document.source_document().handlers[0].params[0]
            .ty
            .display(),
        "str"
    );

    let error = analyze(&source.replace("gap=12.0", "gap=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));

    let error = analyze(&source.replace("markdown docs", "markdown missing")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("unknown markdown state"));

    let error =
        analyze(&source.replace("markdown docs append", "markdown missing append")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("unknown markdown state"));

    let error = analyze(&source.replace(
        "markdown docs append \"\\n![Ice](asset://ice)\"",
        "markdown docs append true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("viewer=docs_viewer", "viewer=missing")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("markdown viewer"));

    let error = analyze(&source.replace("link=primary", "link=missing")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("markdown link"));

    let error =
        analyze(&source.replace("markdown_images(docs)", "markdown_images(true)")).unwrap_err();
    assert_eq!(error.code, "E101");
}

#[test]
fn checks_structured_tables_and_metrics() {
    let source = r#"app Rows
extern crate::backend
  Item(name:str, done:bool)
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
  rows:[Item] = []
view
  table row in rows w=fill p=4.0 px=8.0 py=6.0 sep=1.0 sep-x=2.0 sep-y=3.0
    col w=fill(2) align-x=left align-y=center
      header
        text "Name"
      cell
        text row.name
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("p=4.0", "p=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));

    let error = analyze(&source.replace("table row in rows", "table row in true")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("list of rows"));
}

#[test]
fn checks_bound_text_editors_and_highlighting() {
    let source = r#"app Notes
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
  body:editor = "fn main() {}"
  locked = false
view
  editor #body <-> body hint="Write" w=640.0 h=fill min-h=80.0 max-h=240.0 size=14.0 line-h=1.3 p=8.0 wrap=word-or-glyph font=mono highlight="rs" highlight-theme=solarized-dark disabled=locked
    active bg=bg border=fg border-w=1.0 r=4.0 placeholder=danger value=fg selection=primary
    hovered bg=bg border=primary placeholder=danger value=fg selection=primary
    focused bg=bg border=primary
    focused-hovered bg=bg border=fg
    disabled bg=bg value=danger
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.source_document().states[0].ty.display(), "editor");

    let error = analyze(&source.replace("min-h=80.0", "min-h=300.0")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("cannot exceed"));

    let error = analyze(&source.replace("placeholder=danger", "icon=danger")).unwrap_err();
    assert_eq!(error.code, "E099");
    assert!(error.message.contains("unknown editor style property"));
}

#[test]
fn checks_component_controlled_state_origins() {
    let source = r#"app Notes
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(language:str)
  editor-style editor_surface(readonly:bool)
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
  body:editor = ""
  title = "Notes"
  locked = false
  language = "rs"
component EditorPanel(bind content:editor, bind heading:str, readonly:bool, syntax:str)
  emits
    command(EditorCommand)
  col
    input "Title" <-> heading
    editor <-> content highlighter=editor_highlight(syntax) key-binding=editor_keys(readonly) style=editor_surface(readonly) -> emit(command, _)
on command(value)
view
  EditorPanel content<->body heading<->title readonly=locked syntax=language
    events
      command -> command _
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[0].params[0]
            .ty
            .display(),
        "EditorCommand"
    );

    let error =
        analyze(&source.replace("content<->body", "content<->editor(\"scratch\")")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("direct writable state"));

    let error = analyze(&source.replace("editor_keys(readonly)", "missing(readonly)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor binding"));

    let error =
        analyze(&source.replace("editor_highlight(syntax)", "missing(syntax)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor highlighter"));

    let error =
        analyze(&source.replace("editor_surface(readonly)", "missing(readonly)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor style"));
}

#[test]
fn checks_explicit_component_bind_contracts() {
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
state
  draft = ""
component Field(bind value:str)
  input "Value" <-> value
component Shell(bind value:str)
  state
    local = ""
  col
    Field value<->value
    Field value<->local
view
  Shell value<->draft
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("Shell value<->draft", "Shell value=draft")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("requires `<->`"));
    assert_eq!(
        error.hint.as_deref(),
        Some("replace `value=...` with `value<->state`")
    );

    let error =
        analyze(&source.replace("Shell value<->draft", "Shell value<->trim(draft)")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("direct writable state"));

    let read_only_call = source
        .replace(
            "component Shell(bind value:str)",
            "component Shell(value:str)",
        )
        .replace("Shell value<->draft", "Shell value=draft");
    let error = analyze(&read_only_call).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("read-only"));
    assert_eq!(
        error.hint.as_deref(),
        Some("declare it as `bind value:str`")
    );

    let read_only_operator = r#"app Demo
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
  draft = ""
component Label(value:str)
  text value
view
  Label value<->draft
"#;
    let error = analyze(read_only_operator).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("read-only"));
    assert_eq!(
        error.hint.as_deref(),
        Some("replace `value<->...` with `value=...`")
    );
}

#[test]
fn checks_component_scoped_state_and_handlers() {
    let source = r#"app Local
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
component Toggle()
  state
    enabled = false
  on changed(next)
    enabled = next
  col
    checkbox "Enabled" checked=enabled -> changed _
view
  Toggle #first
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().components[0].states[0].ty,
        Type::Bool
    );
    assert_eq!(
        document.source_document().components[0].handlers[0].params[0].ty,
        Type::Bool
    );

    let error = analyze(&source.replace("enabled = false", "enabled = missing")).unwrap_err();
    assert_eq!(error.code, "E031");

    let nested_owned = source.replace(
        "enabled = false",
        "enabled = false\n    handles:[task-handle?] = []",
    );
    let error = analyze(&nested_owned).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("cloneable values"));

    let error = analyze(&source.replace("    enabled = false\n", "")).unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("state cannot be empty"));

    let error =
        analyze(&source.replace("enabled = next", "task system theme -> changed _")).unwrap_err();
    assert_eq!(error.code, "E140");
}

#[test]
fn checks_component_scoped_widget_operations() {
    let source = r#"app LocalFocus
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
component EditableTitle()
  state
    editing = false
    draft = ""
  on begin
    editing = true
    task widget focus #title
  col
    button "Edit" -> begin
    if editing
      input "Title" #title <-> draft
view
  col
    EditableTitle #first
    EditableTitle #second
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("focus #title", "focus #missing")).unwrap_err();
    assert_eq!(error.code, "E172");

    let error = analyze(&source.replace("focus #title", "focus-next")).unwrap_err();
    assert_eq!(error.code, "E140");
}

#[test]
fn checks_component_request_lanes() {
    let source = r#"app Search
extern crate::backend
  AppError(message:str)
  fetch(query:str) -> str ! AppError
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
component SearchBox()
  state
    query = ""
    loading = false
    result:str? = none
  on search
    loading = true
    run latest lane=search fetch(query) -> loaded _ | failed _
  on loaded(value)
    result = some(value)
    loading = false
  on failed(error)
    loading = false
  col
    input "Query" <-> query
    button "Search" disabled=loading -> search
view
  SearchBox #search
"#;
    let document = analyze(source).unwrap();
    assert!(matches!(
        document.source_document().components[0].handlers[0].statements[1],
        Statement::Run {
            kind: EffectKind::Future,
            mode: FutureMode::Latest,
            lane: Some(ref lane),
            ..
        } if lane == "search"
    ));
    assert_eq!(
        document.source_document().components[0].handlers[1].params[0].ty,
        Type::Str
    );
    assert_eq!(
        document.source_document().components[0].handlers[2].params[0].ty,
        Type::Named("AppError".into())
    );
    let replaced = analyze(&source.replace("run latest", "run replace")).unwrap();
    assert!(matches!(
        replaced.source_document().components[0].handlers[0].statements[1],
        Statement::Run {
            mode: FutureMode::Replace,
            ..
        }
    ));
    analyze(&source.replace("run latest lane=search", "run every")).unwrap();

    let global = r#"app GlobalLatest
extern crate::backend
  fetch(query:str) -> str
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
on search
  run latest lane=search fetch("") -> loaded _
on loaded(value)
view
  text "Search"
"#;
    analyze(global).unwrap();
    analyze(&global.replace("run latest", "run replace")).unwrap();
}

const REQUEST_LANE_APP: &str = r#"app GlobalLanes
extern crate::backend
  fetch(query:str) -> str
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
preset seeded
  boot
    run latest lane=search fetch("preset") -> loaded _
on search
  run latest lane=search fetch("handler") -> loaded _
on loaded(value)
view
  text "Search"
"#;

#[test]
fn deduplicates_app_and_preset_request_lane_hir() {
    let checked = analyze(REQUEST_LANE_APP).unwrap();
    assert_eq!(checked.declarations.run_lane_count(), 1);
    let lane = checked
        .declarations
        .try_run_lane(crate::hir::RunLaneId(0))
        .unwrap();
    assert_eq!(lane.owner, crate::hir::HandlerOwner::App);
    assert_eq!(lane.name, "search");
    assert_eq!(lane.mode, FutureMode::Latest);
    assert_eq!(lane.statements.len(), 2);
    assert!(lane.statements.iter().all(|statement| {
        checked.declarations.statement(*statement).run_lane == Some(lane.declaration.id)
    }));
}

#[test]
fn resolves_forward_lane_invalidation_without_declaring_a_start_site() {
    let source = REQUEST_LANE_APP.replace(
        "on search\n",
        "on cancel\n  invalidate lane=search\non search\n",
    );
    let checked = analyze(&source).unwrap();
    assert_eq!(checked.declarations.run_lane_count(), 1);
    let lane = checked
        .declarations
        .try_run_lane(crate::hir::RunLaneId(0))
        .unwrap();
    let cancel = checked
        .declarations
        .handlers()
        .iter()
        .find(|handler| handler.owner == crate::hir::HandlerOwner::App && handler.name == "cancel")
        .unwrap();
    let invalidation = checked.declarations.statement(cancel.statement_roots[0]);
    assert_eq!(invalidation.run_lane, Some(lane.declaration.id));
    assert_eq!(invalidation.task, None);
    assert_eq!(lane.statements.len(), 2);
    assert!(!lane.statements.contains(&invalidation.declaration.id));
}

#[test]
fn checks_lane_invalidation_declarations_and_state_owners() {
    let source = REQUEST_LANE_APP.replace(
        "view\n  text \"Search\"",
        "component SearchBox()\n  on cancel\n    invalidate lane=search\n  on search\n    run latest lane=search fetch(\"component\") -> loaded _\n  on loaded(value)\n  col\n    button \"Search\" -> search\n    button \"Cancel\" -> cancel\nview\n  SearchBox",
    );
    let checked = analyze(&source).unwrap();
    assert_eq!(checked.declarations.run_lane_count(), 2);

    let unknown_source = REQUEST_LANE_APP.replace(
        "on search\n",
        "on cancel\n  invalidate lane=missing\non search\n",
    );
    let error = analyze(&unknown_source).unwrap_err();
    assert_eq!(error.code, "E140");
    assert_eq!(
        error.message,
        "request lane `missing` is not declared for this state owner"
    );
    assert_eq!(
        error.hint.as_deref(),
        Some(
            "declare it with `run latest lane=missing ...` or `run replace lane=missing ...` for the same state owner"
        )
    );

    let wrong_owner = REQUEST_LANE_APP.replace(
        "view\n  text \"Search\"",
        "component SearchBox()\n  on cancel\n    invalidate lane=search\n  text \"Search\"\nview\n  SearchBox",
    );
    let error = analyze(&wrong_owner).unwrap_err();
    assert_eq!(error.code, "E140");
    assert_eq!(
        error.message,
        "request lane `search` is not declared for this state owner"
    );
}

#[test]
fn rejects_lane_invalidation_inside_task_composition() {
    for (composition, message) in [
        (
            "parallel\n    invalidate lane=missing",
            "task groups only accept task-producing statements",
        ),
        (
            "abortable request abort-on-drop\n    invalidate lane=missing",
            "abortable requires a task-producing statement",
        ),
    ] {
        let source = REQUEST_LANE_APP.replace(
            "on search\n",
            &format!("on cancel\n  {composition}\non search\n"),
        );
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E143");
        assert_eq!(error.message, message);
    }
}

#[test]
fn enforces_request_lane_modes_per_owner() {
    let mismatch = analyze(&REQUEST_LANE_APP.replace(
        "run latest lane=search fetch(\"preset\")",
        "run replace lane=search fetch(\"preset\")",
    ))
    .unwrap_err();
    assert_eq!(mismatch.code, "E140");
    assert!(
        mismatch
            .message
            .contains("uses both `run latest` and `run replace`")
    );

    let components = REQUEST_LANE_APP.replace(
        "preset seeded\n  boot\n    run latest lane=search fetch(\"preset\") -> loaded _\non search\n  run latest lane=search fetch(\"handler\") -> loaded _\non loaded(value)\n",
        "component LatestSearch()\n  on search\n    run latest lane=search fetch(\"latest\") -> loaded _\n  on loaded(value)\n  button \"Latest\" -> search\ncomponent ReplaceSearch()\n  lifetime mounted\n  on search\n    run replace lane=search fetch(\"replace\") -> loaded _\n  on loaded(value)\n  button \"Replace\" -> search\n",
    )
    .replace("  text \"Search\"", "  col\n    LatestSearch\n    ReplaceSearch");
    let components = analyze(&components).unwrap();
    assert_eq!(components.declarations.run_lane_count(), 2);
    assert_eq!(
        components
            .declarations
            .try_run_lane(crate::hir::RunLaneId(0))
            .unwrap()
            .owner,
        crate::hir::HandlerOwner::Component(crate::hir::ComponentId(0))
    );
    assert_eq!(
        components
            .declarations
            .try_run_lane(crate::hir::RunLaneId(1))
            .unwrap()
            .owner,
        crate::hir::HandlerOwner::Component(crate::hir::ComponentId(1))
    );
}

#[test]
fn rejects_duplicate_request_lane_in_recursive_handler_tasks() {
    let duplicate = REQUEST_LANE_APP.replace(
        "run latest lane=search fetch(\"handler\") -> loaded _",
        "parallel\n    run latest lane=search fetch(\"first\") -> loaded _\n    run latest lane=search fetch(\"second\") -> loaded _",
    );
    let duplicate = analyze(&duplicate).unwrap_err();
    assert_eq!(duplicate.code, "E140");
    assert!(
        duplicate
            .message
            .contains("cannot be started more than once")
    );
}

#[test]
fn rejects_abortable_component_request_lanes_without_handle_ownership() {
    let source = r#"app ComponentAbortable
extern crate::backend
  fetch(query:str) -> str
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
component SearchBox()
  on search
    abortable request abort-on-drop
      run latest lane=search fetch("query") -> loaded _
  on loaded(value)
  button "Search" -> search
view
  SearchBox
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("task groups"));
}

#[test]
fn nested_request_lanes_give_a_component_without_declared_state_identity() {
    let source = warning_app(
        r#"extern crate::backend
  fetch(query:str) -> str
state
  items = [1]
component SearchBox()
  on search
    parallel
      run latest lane=primary fetch("first") -> loaded _
      run latest lane=secondary fetch("second") -> loaded _
  on loaded(value)
  button "Search" -> search
view
  for item in items
    SearchBox
"#,
    );
    let checked = analyze(&source).unwrap();
    assert!(checked.source_document().components[0].states.is_empty());
    assert!(
        checked
            .warnings()
            .iter()
            .any(|warning| warning.code == "W008")
    );
}

#[test]
fn rejects_slots_outside_components_and_duplicate_slots() {
    let outside = r#"app Demo
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
  slot
"#;
    let error = analyze(outside).unwrap_err();
    assert_eq!(error.code, "E124");
    assert_eq!(error.line, 13);

    let duplicate = r#"app Demo
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
component Card()
  col
    slot
    slot
view
  text "Hello"
"#;
    let error = analyze(duplicate).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(
        error
            .message
            .contains("declares slot `children` more than once")
    );
}

#[test]
fn checks_combo_search_state_and_routes() {
    let source = r#"app Demo
extern crate::backend
  input-style dynamic_input(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
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
  busy = false
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
  query = ""
on selected(next)
  selected = some(next)
on searched(next)
  query = next
on hovered(next)
on opened
on closed
on add
  combo modes push "Timeline"
view
  combo modes selected "Search modes" line-h=1.2 shape=advanced font=ui input=searched hover=hovered open=opened close=closed style=dynamic_input(busy) menu-style=dynamic_menu(busy) -> selected _
    active bg=bg border=fg border-w=1.0 r=4.0 icon=primary placeholder=danger value=fg selection=primary
    hovered bg=bg icon=fg placeholder=danger value=fg selection=primary
    focused bg=bg border=primary
    focused-hovered bg=bg border=fg
    disabled bg=bg value=danger
    menu text=fg selected-text=bg selected-bg=primary bg=bg border=fg shadow=danger shadow-y=2.0
    icon code="⌕" font=ui size=12.0 gap=6.0 side=right
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().states[1].ty.display(),
        "combo[str]"
    );
    assert_eq!(
        document.source_document().handlers[0].params[0]
            .ty
            .display(),
        "str"
    );
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "str"
    );
    assert_eq!(
        document.source_document().handlers[2].params[0]
            .ty
            .display(),
        "str"
    );

    let error = analyze(&source.replace("gap=6.0", "gap=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("icon spacing"));

    let error = analyze(&source.replace("combo modes push", "combo missing push")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("unknown combo state"));

    let error = analyze(&source.replace("combo modes push", "combo selected push")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("not combo state"));

    let error = analyze(&source.replace("push \"Timeline\"", "push 1")).unwrap_err();
    assert_eq!(error.code, "E101");
}

#[test]
fn replaces_combo_search_options_with_a_typed_list() {
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
state
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
on reset
  modes = ["Timeline"]
on selected(next)
  selected = some(next)
view
  combo modes selected "Search modes" -> selected _
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("[\"Timeline\"]", "[1]")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `[str]`, got `[i64]`"));
}

#[test]
fn checks_structural_widget_routes_and_ranges() {
    let source = r#"app Structure
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
  sensor_key = 0
  width = 0.0
  height = 0.0
on shown(w, h)
  width = w
  height = h
on resized(w, h)
  width = w
  height = h
on hidden
view
  col
    float scale=1.1 x=(viewport_x + viewport_width - original_x - original_width) y=(viewport_y + viewport_height - original_y - original_height) shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 r=8.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0
      text "Floating"
    pin w=fill h=80.0 x=12.0 y=8.0
      text "Pinned"
    sensor show=shown resize=resized hide=hidden key=sensor_key anticipate=32.0 delay=10
      text "Observed"
    responsive at=600.0 w=fill h=40.0
      text "Narrow"
      text "Wide"
    responsive size=(available_width, available_height) w=fill h=fill
      col
        if available_width < available_height
          text "Portrait"
        if available_width >= available_height
          text "Landscape"
    stack w=fill(2) h=120.0 clip=true under=1
      text "Base"
      text "Overlay"
    rule horizontal thickness=2.0 style=weak fill=percent(75.0) color=primary/50 r=4.0 r-tl=2.0 snap=false
    space w=fill(2) h=shrink
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[0].params[0]
            .ty
            .display(),
        "f64"
    );
    assert_eq!(
        document.source_document().handlers[0].params[1]
            .ty
            .display(),
        "f64"
    );
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "f64"
    );

    let bad_float_translation = source.replace(
        "x=(viewport_x + viewport_width - original_x - original_width)",
        "x=true",
    );
    let error = analyze(&bad_float_translation).unwrap_err();
    assert!(error.message.contains("expected `f64`, got `bool`"));

    let bad_float_blur = source.replace("shadow-blur=4.0", "shadow-blur=-1.0");
    let error = analyze(&bad_float_blur).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("float style metric"));

    let bad_float_color = source.replace("shadow=black/50", "shadow=missing");
    let error = analyze(&bad_float_color).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("unknown float shadow color"));

    let bad_stack = source.replace("h=120.0 clip=true", "h=-1.0 clip=true");
    let error = analyze(&bad_stack).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("stack size"));

    let bad_under = source.replace("under=1", "under=70000");
    let error = analyze(&bad_under).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("stack under"));

    let duplicate_size_name = source.replace(
        "size=(available_width, available_height)",
        "size=(available_width, available_width)",
    );
    let error = analyze(&duplicate_size_name).unwrap_err();
    assert_eq!(error.code, "E092");
    assert!(error.message.contains("different names"));

    let conflicting_responsive = source.replace(
        "responsive size=(available_width, available_height)",
        "responsive at=600.0 size=(available_width, available_height)",
    );
    let error = analyze(&conflicting_responsive).unwrap_err();
    assert_eq!(error.code, "E092");
    assert!(error.message.contains("either `at=` or `size=`"));
}

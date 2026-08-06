use super::*;

#[test]
fn lowers_multiple_extern_namespaces() {
    let source = r#"app Plugins
extern crate::backend
  sync title() -> str
extern ducktape_ui::ice
  component native_switch(checked:bool) -> bool
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
  checked = false
on changed(next)
  checked = next
view
  col
    text title()
    extern native_switch(checked) -> changed _
"#;
    let generated = compile(source, "plugins.ice").unwrap();
    assert!(generated.contains("crate::backend::title()"));
    assert!(generated.contains("ducktape_ui::ice::native_switch(arg0)"));
}

#[test]
fn lowers_component_outputs_through_emit_routes() {
    let source = r#"app Plugins
extern crate::backend
  component native_switch(checked:bool) -> bool
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
component PluginSwitch(checked:bool) -> bool
  extern native_switch(checked) -> emit(_)
component NestedSwitch(checked:bool) -> bool
  PluginSwitch checked=checked -> emit(_)
state
  checked = false
on changed(next)
  checked = next
view
  NestedSwitch checked=checked -> changed _
"#;
    let generated = compile(source, "plugins.ice").unwrap();
    assert!(generated.contains("crate::backend::native_switch"));
    assert!(generated.contains("__PluginsMessage::Changed(__value)"));
}

#[test]
fn lowers_stateless_component_outputs_without_ambient_component_context() {
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
component Choice() -> bool
  checkbox "Choice" checked=false -> emit(_)
on changed(next)
view
  Choice -> changed _
"#;
    let generated = compile(source, "stateless-output.ice").unwrap();
    assert!(generated.contains("move |__value| __DemoMessage::Changed(__value)"));
}

#[test]
fn lowers_named_component_events_through_caller_routes() {
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
component Actions(page:str)
  emits
    cancel
    favorite(str, bool)
  col
    button "Cancel" -> emit(cancel)
    checkbox "Favorite" checked=false -> emit(favorite, page, _)
on canceled
on favorite_changed(page, next)
view
  Actions page=page
    events
      cancel -> canceled
      favorite -> favorite_changed _ _
"#;
    let generated = compile(source, "events.ice").unwrap();
    assert!(generated.contains("move || __DemoMessage::Canceled"));
    assert!(generated.contains(
        "move |__event_0, __event_1| __DemoMessage::FavoriteChanged(__event_0, __event_1)"
    ));
    assert!(generated.contains(")(self.page.to_owned(), __value)"));
}

#[test]
fn lowers_sensor_dimensions_through_named_component_events() {
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
    let generated = compile(source, "sensor-events.ice").unwrap();
    assert!(generated.contains(".on_show(move |__size|"));
    assert!(generated.contains(".on_resize(move |__size|"));
    assert!(generated.contains(")(__size.width as f64, __size.height as f64)"));
    assert!(
        generated
            .contains("move |__event_0, __event_1| __DemoMessage::Shown(__event_0, __event_1)")
    );
    assert!(
        generated
            .contains("move |__event_0, __event_1| __DemoMessage::Resized(__event_0, __event_1)")
    );
}

#[test]
fn lowers_exact_component_event_forwarding_without_an_intermediate_message() {
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
    let generated = compile(source, "event-forwarding.ice").unwrap();
    assert!(generated.contains("move |__event_0| __DemoMessage::Selected(__event_0)"));
    assert!(!generated.contains("__DemoMessage::ListSelect"));
}

#[test]
fn lowers_single_ordered_payloads_through_component_outputs() {
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
component PointerCapture() -> mouse-button
  canvas w=fill h=120.0
    event mouse pressed -> emit(_)
    circle x=60.0 y=60.0 r=24.0 fill=primary
on changed(value)
view
  PointerCapture -> changed _
"#;
    let generated = compile(source, "pointer.ice").unwrap();
    assert!(generated.contains("__DemoMessage::Changed(__value)"));
}

#[test]
fn keeps_component_canvas_draw_callbacks_reusable() {
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
component CounterCanvas()
  state
    count = 0
  on pressed
    count = count + 1
  canvas w=120.0 h=40.0
    event mouse pressed
      emit pressed
      capture
    text count x=4.0 y=16.0 color=fg size=12.0
view
  CounterCanvas
"#;
    let generated = compile(source, "component-canvas.ice").unwrap();

    assert!(generated.contains("let __paint = |__frame:"));
    assert!(!generated.contains("let __paint = move |__frame:"));
    assert!(
        generated
            .contains("if __cursor.is_over(__bounds) && let ::std::option::Option::Some(__value)")
    );
}

#[test]
fn lowers_qr_data_and_widget_options() {
    let source = r#"app Codes
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
  invite = "https://example.com/invite"
view
  col
    qr "one" cell-size=5.0
    qr "two" correction=quartile size=120.0 cell=primary bg=white
    qr "three" correction=low version=micro(4)
    qr bytes(00 ff a4)
    qr invite
"#;
    let generated = compile(source, "codes.ice").unwrap();
    assert!(generated.contains("qr_code::Data::new(&(\"one\"))"));
    assert!(generated.contains("qr_code::Data::with_error_correction(&(\"two\"), ::iced::widget::qr_code::ErrorCorrection::Quartile)"));
    assert!(generated.contains("qr_code::Data::with_version(&(\"three\"), ::iced::widget::qr_code::Version::Micro(4), ::iced::widget::qr_code::ErrorCorrection::Low)"));
    assert!(generated.contains("qr_code::Data::new(&(::std::vec![0x00u8, 0xffu8, 0xa4u8]))"));
    assert!(generated.contains("qr_code::Data::new(&(self.invite))"));
    assert!(generated.contains(".ok()).cell_size(::ui_lang_runtime::bounded_spacing(5.0, 182))"));
    assert!(generated.contains(
        ".ok()).total_size(::ui_lang_runtime::bounded_spacing(120.0, 3)).style(move |_theme|"
    ));
    assert!(generated.contains("qr_code::Style { cell: __ice_palette.colors[2]"));
    assert!(!generated.contains("let default = ::iced::widget::qr_code::default(_theme)"));
    // The matrix is built where it is rendered, never cached in app state.
    assert!(!generated.contains("qr_code::Data,"));
}
#[test]
fn lowers_nested_iced_themes() {
    let source = r#"app Themes
theme contract AppTheme
  bg
  fg
  primary
  danger
  surface
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
  surface #111111
view
  col
    theme app
      text "App theme"
    theme tokyo-night fg=fg bg=linear(1.57, surface@0.0, bg@1.0)
      text "Built-in theme"
    theme dark bg=surface
      text "Solid background"
    theme
      text "Default mode"
"#;
    let generated = compile(source, "themes.ice").unwrap();
    assert!(
        generated.contains("dynamic_themer(::std::option::Option::Some(__ice_app_theme.clone())")
    );
    assert!(
        generated.contains("dynamic_themer(::std::option::Option::Some(::iced::Theme::TokyoNight)")
    );
    assert!(
        generated
            .contains("__theme_content, ::std::option::Option::Some(__ice_palette.colors[1]),")
    );
    assert!(generated.contains("::std::option::Option::Some(::iced::Background::Color"));
    assert!(generated.contains("::std::option::Option::Some(::iced::Background::from(::iced::gradient::Linear::new(1.57 as f32).add_stop(0.0 as f32"));
    assert!(generated.contains("dynamic_themer(::std::option::Option::None"));
}

#[test]
fn lowers_native_theme_factories() {
    let source = r#"extern crate::backend
  theme native_theme(dark:bool)
app Themes
  theme native_theme(dark)
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
  dark = true
view
  theme native_theme(!dark)
    text "Nested"
"#;
    let generated = compile(source, "themes.ice").unwrap();
    assert!(generated.contains(
            "fn __ui_lang_check_theme_native_theme(arg0: bool) { let _: ::iced::Theme = crate::backend::native_theme(arg0); }"
        ));
    assert!(
        generated.contains(
            "fn __theme(&self) -> ::iced::Theme {\ncrate::backend::native_theme(self.dark)"
        )
    );
    assert!(generated.contains(
        "dynamic_themer(::std::option::Option::Some(crate::backend::native_theme((!self.dark)))"
    ));
}

#[test]
fn lowers_alternate_theme_subtrees() {
    let source = r#"extern crate::backend
  themer alternate_panel(active:bool) -> bool
app Themes
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
  active = true
on changed(value)
  active = value
view
  themer alternate_panel(active) -> changed _
"#;
    let generated = compile(source, "themer.ice").unwrap();
    assert!(generated.contains(
            "fn __ui_lang_check_themer_alternate_panel(arg0: bool) { let (__theme, __content, __text_color, __background) = crate::backend::alternate_panel(arg0); fn __accept<T: ::iced::theme::Base>(_: &::std::option::Option<T>, _: &__IceElement<'static, bool, T>"
        ));
    assert!(generated.contains("let mut __themer = ::iced::widget::themer(__theme, __content)"));
    assert!(generated.contains("__themer = __themer.text_color(__text_color)"));
    assert!(generated.contains("__themer = __themer.background(__background)"));
    assert!(generated.contains("__themed.map(move |__value| __ThemesMessage::Changed(__value))"));
}

#[test]
fn lowers_component_children_and_slot_forwarding() {
    let source = r#"app Composition
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
component Card(title:str)
  col #card
    text title
    slot
component Wrapper(title:str)
  Card title=title
    slot
view
  Wrapper title="Editor" #editor
    input "Name" #name <-> draft
"#;
    let generated = compile(source, "composition.ice").unwrap();
    assert!(generated.contains("__BindDraft(::std::string::String)"));
    assert!(generated.contains("::iced::widget::text_input(\"\", &self.draft)"));
    assert!(generated.contains("format!(\"{}/name\""));
    assert!(generated.contains("format!(\"{}/Card@"));
}

#[test]
fn keeps_for_reconciliation_scopes_private_from_explicit_ids() {
    let source = r#"app Repeated
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
  groups = [["One", "Two"], ["Three"]]
component Frame()
  col #body
    slot
component Private(value:str)
  state
    seen = ""
  text value
view
  col #list
    for group in groups
      for item in group
        Frame #frame(item)
          text item #slotted(item)
        Private value=item
"#;
    let generated = compile(source, "repeated.ice").unwrap();

    assert_eq!(generated.matches("let __for_scope = format!").count(), 2);
    assert_eq!(generated.matches(".iter().cloned().enumerate()").count(), 2);
    assert!(generated.contains("format!(\"{}/@for:"));
    assert!(generated.contains("/frame({})"));
    assert!(generated.contains("/slotted({})"));
    assert!(!generated.contains("format!(\"{}/frame({})\", __for_scope.clone(), item)"));
    assert!(!generated.contains("format!(\"{}/slotted({})\", __for_scope.clone(), item)"));
    assert!(generated.contains("format!(\"{}/Private@"));
    assert!(generated.contains("\", __for_scope.clone())"));
}

#[test]
fn lowers_named_slots_and_named_slot_forwarding() {
    let source = r#"app Composition
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
component Frame()
  col
    slot heading
    slot body
component Dialog()
  Frame
    heading:
      slot title
    body:
      col
        slot content
        slot actions
on cancel
on delete
view
  Dialog
    title:
      text "Delete task?"
    content:
      text "This cannot be undone."
    actions:
      row
        button "Cancel" -> cancel
        button "Delete" -> delete
"#;
    let generated = compile(source, "composition.ice").unwrap();
    assert!(generated.contains("Delete task?"));
    assert!(generated.contains("This cannot be undone."));
    assert!(generated.contains("Cancel"));
    assert!(generated.contains("Delete"));
}

#[test]
fn omits_missing_optional_slots_without_a_placeholder() {
    let source = r#"app Composition
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
  row
    Card
      Body:
        text "Plain"
    Card
      Body:
        text "Detailed"
      Footer:
        text "Footer"
"#;
    let generated = compile(source, "optional-slots.ice").unwrap();

    assert!(!generated.contains("if false"));
    assert!(!generated.contains("if true"));
    assert!(generated.contains("Plain"));
    assert!(generated.contains("Detailed"));
    assert!(generated.contains("Footer"));
}

#[test]
fn omits_optional_slots_through_single_child_wrappers_and_forwarding() {
    let source = r#"app Composition
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
component Leaf()
  box #optional-shell
    slot Body?
component Inner()
  col
    if provided(Footer)
      box #forwarded-footer-shell
        slot Footer?
component Outer()
  col
    Leaf
    Inner
      Footer:
        slot Footer?
view
  Outer
"#;
    let generated = compile(source, "optional-forwarding.ice").unwrap();
    assert!(!generated.contains("optional-shell"));
    assert!(!generated.contains("forwarded-footer-shell"));

    let supplied = source.replace(
        "view\n  Outer\n",
        "view\n  Outer\n    Footer:\n      text \"Footer\"\n",
    );
    let generated = compile(&supplied, "optional-forwarding-supplied.ice").unwrap();
    assert!(generated.contains("forwarded-footer-shell"));
    assert!(generated.contains("Footer"));

    let empty_root = source.replace("view\n  Outer\n", "view\n  Leaf\n");
    let generated = compile(&empty_root, "optional-empty-root.ice").unwrap();
    assert!(!generated.contains("optional-shell"));
    assert!(generated.contains("::iced::widget::Column::new().into()"));
}

/// Component uses whose arguments resolve only self-backed bindings outline
/// into per-use methods — one item per use keeps rustc's per-function
/// typeck/borrowck cost linear in view size. Uses that capture render-site
/// locals (a loop item) or sit inside a `'static` lazy closure stay inline.
#[test]
fn outlines_self_backed_component_uses_but_not_local_captures() {
    let source = r#"app Composition
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
  title = "hello"
  rows = ["a", "b"]
component Card(label: str)
  text label
view
  col
    Card label=title
    for row in rows
      Card label=row
    lazy title as cached
      Card label=cached
"#;
    let generated = compile(source, "outline.ice").unwrap();

    assert!(
        generated.contains("self.__ice_component_use_0(__ice_palette,"),
        "a self-backed use must call its outlined method"
    );
    assert!(
        generated.contains(
            "fn __ice_component_use_0(&self, __ice_palette: __IcePalette, __ice_component_"
        ),
        "the outlined method must take the palette and the scope value"
    );
    assert!(
        !generated.contains("__ice_component_use_1"),
        "the loop-item use and the lazy use must stay inline: {generated}"
    );
}

#[test]
fn preserves_component_and_slot_stack_boundaries() {
    let source = r#"app Composition
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
component Frame()
  col
    slot
view
  Frame
    text "Content"
"#;
    let generated = compile(source, "composition.ice").unwrap();

    assert!(generated.contains("let __component_content: __IceElement<'_,"));
    assert!(generated.contains("; __component_content }"));
    assert!(generated.contains("(|| { let __slot_content: __IceElement<'_,"));
    assert!(generated.contains("; __slot_content })()"));
}

#[test]
fn lowers_compound_components_into_named_slots() {
    let source = r#"app Composition
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
component Dialog.Header()
  box #root
    slot
component Dialog.Body()
  box #root
    slot
view
  Dialog
    Dialog.Header
      text "Compound title"
    Dialog.Body
      text "Structured body"
"#;
    let generated = compile(source, "composition.ice").unwrap();
    assert!(generated.contains("Compound title"));
    assert!(generated.contains("Structured body"));
    assert!(generated.contains("format!(\"{}/Dialog.Header@"));
    assert!(generated.contains("format!(\"{}/Dialog.Body@"));
}

#[test]
fn lowers_fully_configured_keyed_columns() {
    let source = r#"app Keyed
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
  keyed item in items by=item.id w=fill(2) h=120.0 gap=8.0 p=4.0 pl=12.0 max-w=640.0 align=end
    scroll #row
      text item.name
"#;
    let generated = compile(source, "keyed.ice").unwrap();
    assert!(generated.contains("for item in self.items.iter()"));
    assert!(generated.contains("__children.push((__key, __child))"));
    assert!(
        generated
            .contains("::ui_lang_runtime::bounded_fill_element(__child, __child_count, false)")
    );
    assert!(generated.contains("::iced::widget::keyed_column(__children)"));
    assert!(generated.contains(".spacing(::ui_lang_runtime::bounded_spacing(8.0, __child_count))"));
    assert!(generated.contains("::ui_lang_runtime::bounded_padding(4.0, 4.0, 4.0, 12.0)"));
    assert!(generated.contains(".width(::iced::Length::FillPortion(2))"));
    assert!(generated.contains(".height(120.0 as f32)"));
    assert!(generated.contains(".max_width(640.0 as f32)"));
    assert!(generated.contains(".align_items(::iced::Alignment::End)"));
    assert!(generated.contains("format!(\"{}/key({})\""));
}

#[test]
fn lowers_lazy_to_an_owned_static_subtree() {
    let source = r#"app LazyDemo
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
view
  lazy title as cached
    col
      text cached
      text len(cached)
"#;
    let generated = compile(source, "lazy.ice").unwrap();
    assert!(generated.contains(
        "::ui_lang_runtime::memo_lazy((self.title.to_owned(), (\"LazyDemo\").to_owned(), __ice_palette.name)"
    ));
    assert!(generated.contains("let cached: ::std::string::String = __dependency.0.clone()"));
    assert!(generated.contains("let __lazy_content: __IceElement<'static,"));
    assert!(generated.contains("let __lazy_scope = __dependency.1.clone()"));
}

#[test]
fn lowers_parsed_markdown_with_complete_sizes_and_link_route() {
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
  docs:markdown = "# Hello"
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
    let generated = compile(source, "docs.ice").unwrap();
    assert!(generated.contains("docs: ::iced::widget::markdown::Content::parse(\"# Hello\")"));
    assert!(
        generated.contains(
            "self.docs = ::iced::widget::markdown::Content::parse(&\"# Reset\".to_owned())"
        )
    );
    for field in [
        "text_size",
        "h1_size",
        "h2_size",
        "h3_size",
        "h4_size",
        "h5_size",
        "h6_size",
        "code_size",
        "spacing",
    ] {
        assert!(generated.contains(&format!("__markdown_settings.{field} =")));
    }
    assert!(generated.contains("self.docs.push_str(&\"\\n![Ice](asset://ice)\".to_owned())"));
    assert!(generated.contains(".images().iter().cloned().collect"));
    assert!(generated.contains("::iced::widget::markdown::view_with(self.docs.items()"));
    assert!(generated.contains("crate::backend::docs_viewer(\"docs\".to_owned())"));
    assert!(generated.contains("map(move |__event| __DocsMessage::Open(__event))"));
    assert!(generated.contains("fn __ui_lang_check_markdown_viewer_docs_viewer"));
    for field in [
        "style.font",
        "style.inline_code_highlight.background",
        "style.inline_code_color",
        "style.inline_code_font",
        "style.code_block_font",
        "style.link_color",
        "style.inline_code_padding",
        "style.inline_code_highlight.border.color",
        "style.inline_code_highlight.border.width",
        "style.inline_code_highlight.border.radius",
    ] {
        assert!(generated.contains(&format!("__markdown_settings.{field} =")));
    }
}

#[test]
fn lowers_structured_tables_with_complete_native_options() {
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
    col w=fill(2) align-x=right align-y=bottom
      header
        text "Name"
      cell
        scroll #value
          text row.name
"#;
    let generated = compile(source, "rows.ice").unwrap();
    assert!(generated.contains("table::table(::std::vec!["));
    assert!(generated.contains("let __table_rows = self.rows.clone();"));
    assert!(generated.contains("let __table_row_count = __table_rows.len().saturating_add(1);"));
    assert!(generated.contains("__table_rows.into_iter().enumerate()"));
    assert!(generated.contains("move |(__row, row): (usize, crate::backend::Item)|"));
    assert!(generated.contains("let _ = &row; let __table_cell"));
    assert!(generated.contains(
        ".width(::ui_lang_runtime::bounded_fill_length(::iced::Length::FillPortion(2), 1))"
    ));
    assert!(generated.contains(
        "::ui_lang_runtime::bounded_fill_element(__table_header, __table_row_count, false)"
    ));
    assert!(generated.contains(
        "::ui_lang_runtime::bounded_fill_element(__table_cell, __table_row_count, false)"
    ));
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Right)"));
    assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
    for method in [
        "padding(::ui_lang_runtime::bounded_table_metric(4.0, 1usize.max(__table_row_count)))",
        "padding_x(::ui_lang_runtime::bounded_table_metric(8.0, 1))",
        "padding_y(::ui_lang_runtime::bounded_table_metric(6.0, __table_row_count))",
        "separator(::ui_lang_runtime::bounded_table_metric(1.0, 1usize.max(__table_row_count)))",
        "separator_x(::ui_lang_runtime::bounded_table_metric(2.0, 1))",
        "separator_y(::ui_lang_runtime::bounded_table_metric(3.0, __table_row_count))",
    ] {
        assert!(generated.contains(method));
    }
    assert!(generated.contains("format!(\"{}/row({})/col(0)\""));
}

#[test]
fn lowers_bound_text_editors_and_internal_actions() {
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
  editor #body <-> body hint="Write" w=640.0 h=fill min-h=80.0 max-h=240.0 size=14.0 line-h-px=18.0 p=8.0 wrap=word-or-glyph font=mono highlight="rs" highlight-theme=inspired-github disabled=locked
    active bg=bg border=fg border-w=1.0 r=4.0 placeholder=danger value=fg selection=primary
    hovered bg=bg border=primary placeholder=danger value=fg selection=primary
    focused bg=bg border=primary
    focused-hovered bg=bg border=fg
    disabled bg=bg value=danger
"#;
    let generated = compile(source, "notes.ice").unwrap();
    assert!(generated.contains("body: ::iced::widget::text_editor::Content::with_text"));
    assert!(generated.contains("__EditBody(::iced::widget::text_editor::Action)"));
    assert!(generated.contains("self.body.perform(action)"));
    assert!(generated.contains("::iced::widget::text_editor(&self.body)"));
    assert!(generated.contains(".width(((640.0) as f32).max(0.0).min(f32::MAX))"));
    assert!(generated.contains(".height(::iced::Fill)"));
    assert!(generated.contains(".min_height(((80.0) as f32).max(0.0).min(f32::MAX))"));
    assert!(generated.contains(".max_height(((240.0) as f32).max(0.0).min(f32::MAX))"));
    assert!(
        generated.contains(
            "LineHeight::Absolute(((18.0) as f32).max(f32::EPSILON).min(f32::MAX).into())"
        )
    );
    assert!(generated.contains("Wrapping::WordOrGlyph"));
    assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
    assert!(generated.contains(".highlight(\"rs\", ::iced::highlighter::Theme::InspiredGitHub)"));
    assert!(generated.contains("::iced::widget::text_editor::default"));
    assert!(generated.contains("text_editor::Status::Focused { is_hovered: true }"));
    assert!(generated.contains("__style.placeholder ="));
    assert!(generated.contains("__style.selection ="));
    assert!(generated.contains("let __disabled = self.locked"));
    assert!(generated.contains("if __disabled"));
    assert!(generated.contains(".on_action(__NotesMessage::__EditBody"));
    assert!(generated.contains(".value(__editor_value).disabled(__disabled)"));
}

#[test]
fn lowers_component_controls_and_editor_extensions() {
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
    let generated = compile(source, "notes.ice").unwrap();
    assert!(generated.contains("__BindTitle(::std::string::String)"));
    assert!(generated.contains("__EditBody(::iced::widget::text_editor::Action)"));
    assert!(generated.contains("text_input(\"\", &self.title)"));
    assert!(generated.contains("text_editor(&self.body)"));
    assert!(generated.contains("crate::backend::editor_keys(__key_press, self.locked)"));
    assert!(generated.contains("__ice_map_editor_binding"));
    assert!(!generated.contains("Binding::Indent"));
    assert!(!generated.contains("Binding::Unindent"));
    assert!(generated.contains("__NotesMessage::Command(__event_0)"));
    assert!(generated.contains("crate::backend::editor_highlight("));
    assert!(generated.contains(", self.language.to_owned())"));
    assert!(generated.contains("fn __ui_lang_check_editor_binding_editor_keys"));
    assert!(generated.contains("fn __ui_lang_check_editor_highlighter_editor_highlight"));
    assert!(generated.contains("fn __ui_lang_check_editor_style_editor_surface"));
    assert!(generated.contains("crate::backend::editor_surface(__theme, __status, self.locked)"));
    assert!(generated.contains("self.title = value"));
    assert!(generated.contains("self.body.perform(action)"));
}

#[test]
fn lowers_bind_prop_forwarding_to_its_writable_origin() {
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
    let generated = compile(source, "bind.ice").unwrap();

    assert!(generated.contains("__BindDraft(::std::string::String)"));
    assert!(generated.contains("__ShellBindLocal(::std::string::String, ::std::string::String)"));
}

#[test]
fn lowers_component_scoped_state_and_match() {
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
component Counter()
  state
    count = 0
    draft = ""
  on increment
    count = count + 1
  col
    input "Draft" <-> draft
    button "Increment" -> increment
    match count
      0
        text "zero"
      _
        text count
view
  Counter #counter
"#;
    let generated = compile(source, "local.ice").unwrap();
    assert!(generated.contains("struct __IceCounterState"));
    assert!(generated.contains("__ice_component_counter: ::std::collections::HashMap"));
    assert!(generated.contains("__CounterHandleIncrement(::std::string::String)"));
    assert!(generated.contains("__CounterBindDraft(::std::string::String, ::std::string::String)"));
    assert!(generated.contains("self.__ice_component_counter.entry(__scope.clone()).or_default()"));
    assert!(generated.contains("__local.count = (__local.count + 1)"));
    assert!(generated.contains("self.__ice_component_counter.get(&__ice_component_counter_scope_"));
}

#[test]
fn lowers_missing_component_props_from_defaults() {
    let source = r#"app Defaults
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
component Badge(label:str="Untitled", selected:bool=false)
  col
    text label
    if selected
      text "Selected"
view
  Badge selected=true
"#;
    let generated = compile(source, "defaults.ice").unwrap();

    assert!(generated.contains("(\"Untitled\").to_string()"));
    assert!(!generated.contains("\"Untitled\".clone()"));
    assert!(generated.contains("Selected"));
    assert!(!generated.contains("if true"));
}

#[test]
fn owns_default_component_strings_passed_to_extern_functions() {
    let source = r#"app Defaults
extern crate::backend
  sync normalize(value:str) -> str
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
component Badge(label:str="Untitled")
  text normalize(label)
view
  Badge
"#;
    let generated = compile(source, "defaults.ice").unwrap();

    assert!(generated.contains("crate::backend::normalize(\"Untitled\".to_owned())"));
}

#[test]
fn lowers_component_scoped_widget_operations() {
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
    let generated = compile(source, "local_focus.ice").unwrap();
    assert!(generated.contains(
        "::iced::widget::operation::focus::<__LocalFocusMessage>(::iced::widget::Id::from(format!(\"{}/title\", __scope)))"
    ));
}

#[test]
fn lowers_component_latest_futures_with_a_scoped_generation() {
    let source = r#"app Search
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
  state
    query = ""
    result:str? = none
  on search
    run latest fetch(query) -> loaded _
  on loaded(value)
    result = some(value)
  col
    input "Query" <-> query
    button "Search" -> search
view
  SearchBox #search
"#;
    let generated = compile(source, "search.ice").unwrap();
    assert!(generated.contains("__ice_latest_0: u64"));
    assert!(generated.contains("__SearchBoxLatest0(::std::string::String, u64"));
    assert!(generated.contains("__local.__ice_latest_0.wrapping_add(1)"));
    assert!(generated.contains("__SearchMessage::__SearchBoxLatest0(__scope.clone()"));
    assert!(generated.contains("__local.__ice_latest_0 == __generation"));
    assert!(generated.contains("return self.__update(*__message)"));

    let ordinary = compile(&source.replace("run latest", "run"), "search.ice").unwrap();
    assert!(!ordinary.contains("Latest0"));
    assert!(!ordinary.contains("wrapping_add(1)"));
}

#[test]
fn lowers_mounted_components_and_replace_futures() {
    let source = r#"app Search
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
  lifetime mounted
  state
    query = ""
  on search
    run replace fetch(query) -> loaded _
  on loaded(value)
    query = value
  button "Search" -> search
on ignored
state
  scopes = [1, 2]
view
  keyed scope in scopes by=scope
    SearchBox #search(scope)
"#;
    let generated = compile(source, "search.ice").unwrap();
    assert!(generated.contains("::ui_lang_runtime::MountedComponentState<__IceSearchBoxState>"));
    assert!(generated.contains("__ice_replace_0: ::std::option::Option<::iced::task::Handle>"));
    assert!(generated.contains("__ice_latest_0: u64"));
    assert!(generated.contains(".next_generation()"));
    assert!(generated.contains(".__ice_replace_0.replace(__handle.abort_on_drop())"));
    assert!(generated.contains("__previous.abort()"));
    assert!(generated.contains("let (__task, __handle) = __task.abortable()"));
    assert!(generated.contains(".begin_render()"));
    assert!(generated.contains(".mount("));
    assert!(generated.contains(".finish_render(__ice_root_scope_ref)"));
    assert!(generated.contains("SearchBoxLatest0"));
    assert!(generated.contains("::iced::widget::keyed_column(__children)"));
    assert!(generated.contains("format!(\"{}/key({})\""));
}
/// Routes to app handlers capture no component state at the top level, so an
/// events-only use still outlines (the route-callback path visits the whole
/// env to collect state scopes — an empty collection must not count as a
/// site capture).
#[test]
fn outlines_uses_whose_events_route_to_app_handlers() {
    let source = r#"app Composition
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
  title = "hello"
on bump
component Card(label: str)
  emits
    pressed
  col
    text label
    button "go" -> emit(pressed)
view
  col
    Card label=title
      events
        pressed -> bump
"#;
    let generated = compile(source, "outline-events.ice").unwrap();
    assert!(
        generated.contains("self.__ice_component_use_0(__ice_palette,"),
        "an events-only use must still outline"
    );
}
/// A nested use inside a component body outlines too: self-backed prop
/// chains propagate through per-argument markers, and routes to the
/// enclosing component's handlers only need its scope locals, which become
/// extra `String` parameters cloned at the call site.
#[test]
fn outlines_nested_uses_passing_enclosing_scope_locals_as_parameters() {
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
  title = "hello"
component Inner(label: str)
  emits
    tapped
  button "go" -> emit(tapped)
component Outer(label: str)
  state
    open = false
  on toggle
    open = !open
  col
    text label
    Inner label=label
      events
        tapped -> toggle
view
  Outer label=title
"#;
    let generated = compile(source, "nested-events.ice").unwrap();
    assert_eq!(
        generated.matches("fn __ice_component_use_").count(),
        2,
        "both the outer use and the nested Inner use must outline"
    );
    assert!(
        generated.contains(
            "fn __ice_component_use_0(&self, __ice_palette: __IcePalette, __ice_component_inner_scope_25: ::std::string::String, __ice_component_outer_scope_29: ::std::string::String)"
        ),
        "the nested method must take the enclosing scope local as a parameter"
    );
    assert!(
        generated.contains("self.__ice_component_use_0(__ice_palette, format!(\"{}/Inner@25\", __ice_component_outer_scope_29.clone()), __ice_component_outer_scope_29.clone())"),
        "the call site must pass the enclosing scope local by clone"
    );
}

/// The capture-aliasing route path must not hide render-site locals from the
/// outlining recorder: a route payload expression referencing a loop item
/// keeps the use inline even when the enclosing component's state forces the
/// aliased-callback environment.
#[test]
fn keeps_uses_inline_when_route_payloads_reference_loop_items() {
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
  items = ["a", "b"]
component Inner()
  emits
    tapped
  button "go" -> emit(tapped)
component Outer(items: [str])
  state
    open = false
  on pick(value)
    open = !open
  col
    for item in items
      Inner
        events
          tapped -> pick item
view
  Outer items=items
"#;
    let generated = compile(source, "route-payload-locals.ice").unwrap();
    assert_eq!(
        generated.matches("fn __ice_component_use_").count(),
        1,
        "only the outer use may outline — the inner use's route captures a loop item"
    );
}

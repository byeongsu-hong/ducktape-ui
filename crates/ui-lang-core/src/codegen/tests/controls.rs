use super::*;

#[test]
fn reads_editor_text_and_replaces_content() {
    // `editor_text(state)` lets a handler read an editor's current text back as a
    // String (to send/persist it), and assigning `editor("")` replaces/clears the
    // content — the two halves a multiline composer needs.
    let source = r#"app Composer
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  notes:editor = "hello"
  snapshot = ""
on capture
  snapshot = editor_text(notes)
on clear
  notes = editor("")
view
  col
    editor <-> notes
    text snapshot
    button "Read" -> capture
    button "Clear" -> clear
"#;
    let generated = compile(source, "composer.ice").unwrap();
    // The read lowers to `Content::text()`.
    assert!(generated.contains(".text()"));
    // Clearing lowers to a fresh `Content` assigned onto the editor state.
    assert!(generated.contains(".notes = ::iced::widget::text_editor::Content::with_text"));

    // Reading a non-editor state is a type error.
    let error = compile(
        &source.replace("editor_text(notes)", "editor_text(snapshot)"),
        "composer.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E101");
}

#[test]
fn lowers_resize_handle_to_a_grabbing_widget() {
    // A `resize-handle` wraps a divider child and reports `(dx, dy)` drag deltas
    // plus press/release, so a component can drive a bound width — something
    // `pane_grid` cannot do outside the app view.
    let source = r#"app Split
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  left_width = 240.0
on drag_started
on drag_ended
on sidebar_dragged(dx, dy)
  return if dx < 0.0 && left_width + dx < 160.0
  left_width = left_width + dx
view
  row
    box w=left_width h=fill bg=primary
      text "Sidebar"
    resize-handle drag=sidebar_dragged press=drag_started release=drag_ended cursor=resize-horizontal
      box w=6.0 h=fill bg=fg
        text ""
    box w=fill h=fill bg=bg
      text "Main"
"#;
    let generated = compile(source, "split.ice").unwrap();
    assert!(generated.contains("::ui_lang_runtime::resize_handle("));
    assert!(generated.contains(".on_drag("));
    assert!(generated.contains(".on_press("));
    assert!(generated.contains(".on_release("));
    assert!(generated.contains(".interaction(::iced::mouse::Interaction::ResizingHorizontally)"));

    // The drag route carries two f64 payloads; the wrong arity is rejected.
    assert!(
        compile(
            &source.replace("on sidebar_dragged(dx, dy)", "on sidebar_dragged(dx)"),
            "split.ice",
        )
        .is_err()
    );

    // The handle refuses to exist without a drag route.
    let error = compile(&source.replace("drag=sidebar_dragged ", ""), "split.ice").unwrap_err();
    assert_eq!(error.code, "E087");
}

#[test]
fn lowers_complex_native_controls() {
    let source = r#"app Controls
extern crate::backend
  SliderNumber()
  sync slider_number(value:f64) -> SliderNumber
  slider-style dynamic_slider(active:bool)
  progress-style dynamic_progress(active:bool)
  radio-style dynamic_radio(highlight:bool)
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  grid_columns = 2
  grid_width = 640.0
  aspect_width = 16.0
  aspect_height = 9.0
  fluid_width = 240.0
  precise:SliderNumber = slider_number(50.0)
  enabled = false
  choice = "first"
on amount_changed(next)
  amount = next
on precise_changed(next)
  precise = next
on released
on enabled_changed(next)
  enabled = next
on choice_changed(next)
  choice = next
view
  col
    grid cols=grid_columns w=grid_width gap=12.0 h=aspect(aspect_width,aspect_height)
      toggler "Enabled" checked=enabled -> enabled_changed _
      slider amount min=0.0 max=100.0 step=0.5 default=50.0 shift-step=0.1 vertical w=20.0 h=fill(2) style=dynamic_slider(enabled) release=released -> amount_changed _
        active rail-start=linear(0.0, primary@0.0, danger@1.0) rail-end=linear(1.57, bg@0.0, primary/25@1.0) rail-w=4.0 rail-border=transparent rail-border-w=1.0 rail-r=2.0 rail-r-tl=1.0 handle=circle(7.0) handle-color=linear(0.785, primary@0.0, fg@1.0) handle-border=fg handle-border-w=1.0
        hovered rail-start=fg rail-end=bg handle=rect(12) handle-color=fg handle-r=3.0 handle-r-tl=1.0
        dragged rail-start=danger handle=circle(8.0) handle-color=danger
      slider amount min=0.0 max=100.0 step=1.0 w=fill h=18.0 style=dynamic_slider(enabled) -> amount_changed _
      slider precise min=slider_number(0.0) max=slider_number(100.0) step=slider_number(5.0) default=slider_number(50.0) shift-step=slider_number(1.0) -> precise_changed _
      progress amount vertical length=fill(2) girth=20.0 style=dynamic_progress(enabled) bg=linear(1.57, bg@0.0, primary/25@1.0) bar=linear(0.0, primary/75@0.0, danger@1.0) border=fg border-w=1.0 r=4.0 r-tl=2.0
      progress amount style=success
      progress amount style=warning
      progress amount style=danger
      radio "First" value="first" selected=(choice == "first") style=dynamic_radio(enabled) size=20.0 w=fill gap=8.0 text-size=14.0 line-h=1.2 shape=advanced wrap=word-or-glyph font=mono -> choice_changed _
        active selected bg=linear(1.57, primary@0.0, bg@1.0) dot=fg border=primary border-w=2.0 text=fg
        active unselected bg=bg dot=primary border=fg text=fg
        hovered selected bg=primary dot=fg border=fg text=fg
        hovered unselected bg=fg dot=bg border=primary text=primary
      rule horizontal thickness=2.0 style=weak fill=full color=primary/50 r=4.0 r-tl=2.0 snap=false
      rule horizontal fill=percent(75.0)
      rule horizontal fill=pad(4)
      rule horizontal fill=pad(4,8)
      space w=fill(2) h=shrink
      stack clip=true w=fill(2) h=120.0 under=1
        text "base"
        text "overlay"
    grid max-cell=fluid_width h=fill(2)
      text "maximum"
    grid min-cell=fluid_width gap=12.0
      text "minimum"
"#;
    let generated = compile(source, "controls.ice").unwrap();
    assert!(
        generated.contains(
            "let __grid_columns = usize::try_from(self.grid_columns).unwrap_or(0).max(1);"
        )
    );
    assert!(
            generated.contains("::iced::widget::grid(__children).spacing(::ui_lang_runtime::bounded_spacing(12.0, __child_count.max(__grid_columns))).width(((self.grid_width) as f32).max(0.0).min(f32::MAX)).height(::iced::widget::grid::Sizing::AspectRatio((((self.aspect_width) / (self.aspect_height)) as f32).max(f32::EPSILON).min(f32::MAX))).columns(__grid_columns)")
        );
    assert!(generated.contains(
            "::iced::widget::grid(__children).height(::iced::Length::FillPortion(2)).fluid(((self.fluid_width) as f32).max(f32::EPSILON).min(f32::MAX))"
        ));
    assert!(generated.contains(
        ".grow(1.0).shrink(0.0).basis(::ui_lang_runtime::FlexBasis::Fixed(((self.fluid_width) as f32).max(f32::EPSILON).min(f32::MAX)))"
    ));
    assert!(generated.contains(".wrap(::ui_lang_runtime::FlexWrap::Wrap)"));
    assert!(generated.contains("::iced::widget::vertical_slider"));
    assert!(generated.contains(
        ".default(50.0).shift_step(0.1).width(20.0 as f32).height(::iced::Length::FillPortion(2))"
    ));
    assert!(generated.contains("::iced::widget::slider"));
    assert!(generated.contains(".width(::iced::Fill).height(18.0 as f32)"));
    assert!(generated.contains(".style(move |__theme, __status|"));
    assert!(generated.contains("fn __ui_lang_check_slider_style_dynamic_slider"));
    assert_eq!(
        generated
            .matches("crate::backend::dynamic_slider(__theme, __status, self.enabled)")
            .count(),
        2
    );
    assert!(generated.contains(
            "::iced::widget::slider((crate::backend::slider_number(0.0))..=(crate::backend::slider_number(100.0)), self.precise, move |__value| __ControlsMessage::PreciseChanged(__value)).step(crate::backend::slider_number(5.0))"
        ));
    assert!(!generated.contains("self.precise.clone()"));
    assert!(generated.contains("slider::Status::Hovered"));
    assert!(generated.contains("slider::Status::Dragged"));
    assert!(generated.contains("slider::HandleShape::Circle"));
    assert!(generated.contains("slider::HandleShape::Rectangle"));
    assert!(generated.contains("__style.rail.backgrounds.0"));
    assert!(generated.contains("__style.rail.backgrounds.0 = ::iced::Background::from"));
    assert!(generated.contains("__style.rail.backgrounds.1 = ::iced::Background::from"));
    assert!(generated.contains("__style.handle.background = ::iced::Background::from"));
    assert!(generated.contains("::iced::widget::progress_bar"));
    assert!(generated.contains("::ui_lang_runtime::progress_range(0.0, 100.0, self.amount)"));
    assert!(generated.contains(".vertical()"));
    assert!(generated.contains(".length(::iced::Length::FillPortion(2)).girth(20.0 as f32)"));
    assert!(generated.contains("crate::backend::dynamic_progress(__theme, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_progress_style_dynamic_progress"));
    assert!(generated.contains("progress_bar::success(__theme)"));
    assert!(generated.contains("progress_bar::warning(__theme)"));
    assert!(generated.contains("progress_bar::danger(__theme)"));
    assert!(generated.contains("__style.background = ::iced::Background::from"));
    assert!(generated.contains("__style.bar = ::iced::Background::from"));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("::iced::gradient::Linear::new(0.0 as f32)"));
    assert!(generated.contains("__style.border.radius"));
    assert!(generated.contains("::iced::widget::radio(\"First\".to_owned(), true"));
    assert!(generated.contains("move |_| __ControlsMessage::ChoiceChanged(\"first\".to_owned())"));
    assert!(generated.contains(
        ".size(::ui_lang_runtime::bounded_table_metric(20.0, 1)).spacing(::ui_lang_runtime::bounded_table_metric(8.0, 1))"
    ));
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
    assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
    assert!(generated.contains("crate::backend::dynamic_radio(__theme, __status, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_radio_style_dynamic_radio"));
    for (status, selected) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
    ] {
        assert!(generated.contains(&format!(
            "radio::Status::{status} {{ is_selected: {selected} }}"
        )));
    }
    assert!(generated.contains("__style.background = ::iced::Background::from"));
    assert!(generated.contains("__style.dot_color ="));
    assert!(generated.contains("__style.border_width = 2.0 as f32"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    let default_radio = compile(
        &source.replace(" style=dynamic_radio(enabled)", ""),
        "controls.ice",
    )
    .unwrap();
    assert!(default_radio.contains("radio::default(__theme, __status)"));
    assert!(generated.contains("::iced::widget::rule::weak(__theme)"));
    assert!(generated.contains("rule::FillMode::Full"));
    assert!(generated.contains("rule::FillMode::Percent(((75.0) as f32).max(0.0).min(100.0))"));
    assert!(generated.contains("rule::FillMode::Padded(4)"));
    assert!(generated.contains("rule::FillMode::AsymmetricPadding(4, 8)"));
    assert!(generated.contains("__style.snap = false"));
    assert!(generated.contains(
        "::iced::widget::space().width(::iced::Length::FillPortion(2)).height(::iced::Shrink)"
    ));
    assert!(generated.contains("__children.split_off(__under)"));
    assert!(generated.contains("::iced::widget::Stack::new()"));
    assert!(generated.contains("__stack.push_under(__child)"));
    assert!(
        generated
            .contains(".clip(true).width(::iced::Length::FillPortion(2)).height(120.0 as f32)")
    );
}

#[test]
fn lowers_extended_text_input_behavior() {
    let source = r#"app Form
extern crate::backend
  input-style dynamic_input(disabled:bool)
font ui family=sans
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
  disabled = false
  secure = true
on changed(next)
  value = next
on submitted
on pasted(next)
  value = next
view
  input "Secret" #secret <-> value hint="Paste token" disabled=disabled secure=secure change=changed submit=submitted paste=pasted w=240.0 p=8.0 text-size=14.0 line-h=1.2 align=center font=mono style=dynamic_input(disabled)
    active bg=bg border=fg border-w=1.0 r=4.0 icon=primary placeholder=danger value=fg selection=primary
    hovered bg=bg border=primary border-w=1.0 r=10.0 icon=fg placeholder=danger value=fg selection=primary
    focused bg=bg border=primary border-w=1.0 r=10.0
    focused-hovered bg=bg border=fg border-w=1.0 r=10.0
    disabled bg=bg border=primary border-w=1.0 r=10.0 value=danger
    icon code="•" font=ui size=12.0 gap=4.0 side=right
"#;
    let generated = compile(source, "form.ice").unwrap();
    assert!(generated.contains("let __secure = self.secure"));
    assert!(generated.contains(".secure(__secure)"));
    assert!(generated.contains("::ui_lang_runtime::Role::PasswordInput"));
    assert!(generated.contains(".value_maybe((!__secure).then"));
    assert!(generated.contains(
        ".width(240.0 as f32).padding(8.0 as f32).size(((14.0) as f32).max(f32::EPSILON).min(f32::MAX))"
    ));
    assert!(
        generated.contains("LineHeight::Relative(((1.2) as f32).max(f32::EPSILON).min(f32::MAX))")
    );
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
    assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
    assert!(generated.contains("code_point: '•'"));
    assert!(generated.contains("family: ::iced::font::Family::SansSerif"));
    assert!(generated.contains("Side::Right"));
    assert!(generated.contains(".style(move |__theme, __status|"));
    assert!(generated.contains("crate::backend::dynamic_input(__theme, __status, self.disabled)"));
    assert!(generated.contains("fn __ui_lang_check_input_style_dynamic_input"));
    let custom = generated
        .find("crate::backend::dynamic_input(__theme, __status, self.disabled)")
        .unwrap();
    let statuses = custom + generated[custom..].find(" match __status").unwrap();
    assert!(custom < statuses);
    assert!(generated.contains("Status::Focused { is_hovered: true }"));
    assert!(generated.contains("__style.placeholder ="));
    assert!(generated.contains("__style.selection ="));
    assert!(generated.contains(".on_submit_maybe(if __disabled"));
    assert!(generated.contains(".on_paste_maybe(if __disabled"));
    assert!(generated.contains("__FormMessage::Changed(__value)"));
    let default_input = compile(
        &source.replace(" style=dynamic_input(disabled)", ""),
        "form.ice",
    )
    .unwrap();
    assert!(default_input.contains("text_input::default(__theme, __status)"));

    let compact_input = compile(
        &source.replace("input \"Secret\"", "input \"\" label=\"Secret\""),
        "form.ice",
    )
    .unwrap();
    assert!(!compact_input.contains("widget::column![::iced::widget::text(\"\")"));
    assert!(compact_input.contains("; __input.into() }"));
}

#[test]
fn lowers_button_children_and_typed_properties() {
    let source = r#"app Actions
extern crate::backend
  button-style dynamic_button(disabled:bool)
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  disabled = false
on pressed
view
  col
    button #action label="Save" disabled=disabled w=fill h=48.0 p=8.0 clip=true style=dynamic_button(disabled) @disabled:opacity-50 -> pressed
      row
        text "Save"
        text "⌘S"
      active bg=linear(1.57, primary@0.0, bg@1.0) text=fg border=primary border-w=1.0 r=4.0 r-tl=2.0 r-tr=3.0 r-br=5.0 r-bl=6.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 px-snap=true
      hovered bg=fg text=bg r=10.0
      pressed bg=primary text=white r=10.0
      disabled bg=bg text=fg r=10.0
    button "+" w=28.0 h=28.0 -> pressed
    button label="Icon" w=36.0 h=36.0 -> pressed
      text "●"
"#;
    let generated = compile(source, "actions.ice").unwrap();
    assert!(generated.contains("let __button_content: __IceElement"));
    assert!(generated.contains("::iced::widget::row(__children)"));
    assert!(
        generated.contains("::ui_lang_runtime::bounded_fill_element(__child, __child_count, true)")
    );
    assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
    assert!(generated.contains(".padding(8.0 as f32).clip(true)"));
    assert!(generated.contains(".on_press_maybe(if __disabled"));
    assert!(generated.contains("::ui_lang_runtime::Role::Button"));
    assert!(generated.contains(".label(\"Save\".to_owned())"));
    assert!(generated.contains("crate::backend::dynamic_button(__theme, __status, self.disabled)"));
    assert!(generated.contains("fn __ui_lang_check_button_style_dynamic_button"));
    assert!(generated.contains("button::Status::Hovered =>"));
    assert!(generated.contains("button::Status::Pressed =>"));
    assert!(generated.contains("button::Status::Disabled =>"));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains(
        "let __button_inner: __IceElement<'_, __ActionsMessage> = ::iced::widget::text(\"+\").into();"
    ));
    let centered_fixed_content = ".width(::iced::Fill).align_x(::iced::alignment::Horizontal::Center).height(::iced::Fill).align_y(::iced::alignment::Vertical::Center).into()";
    assert_eq!(generated.matches(centered_fixed_content).count(), 2);
    let centered_fixed_height =
        ".height(::iced::Fill).align_y(::iced::alignment::Vertical::Center).into()";
    assert_eq!(generated.matches(centered_fixed_height).count(), 3);
    assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
    assert!(generated.contains("__style.snap = true"));
    for preset in [
        "primary",
        "secondary",
        "success",
        "warning",
        "danger",
        "text",
        "background",
        "subtle",
    ] {
        let source_preset = if preset == "background" { "bg" } else { preset };
        let generated = compile(
            &source.replace(
                "style=dynamic_button(disabled)",
                &format!("style={source_preset}"),
            ),
            "actions.ice",
        )
        .unwrap();
        assert!(generated.contains(&format!("button::{preset}(__theme, __status)")));
    }
}

#[test]
fn cascades_active_style_into_interaction_states() {
    let source = r#"app Styles
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
on pressed
view
  button "Save" -> pressed
    active bg=bg text=fg r=8.0
    hovered text=primary
"#;
    let generated = compile(source, "styles.ice").unwrap();
    let background = generated.find("__style.background =").unwrap();
    let hovered = generated.find("button::Status::Hovered").unwrap();
    assert!(background < hovered);
    assert!(!generated.contains("button::Status::Active"));
    assert!(generated.contains("__style.text_color ="));
    assert!(generated.contains("__style.border.radius ="));
}

#[test]
fn lowers_complete_boolean_control_styles_and_typography() {
    let source = r#"app Preferences
extern crate::backend
  checkbox-style dynamic_checkbox(disabled:bool)
  toggler-style dynamic_toggler(disabled:bool)
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  enabled = false
on changed(next)
  enabled = next
view
  col
    checkbox "Checkbox" checked=enabled style=dynamic_checkbox(enabled) size=20.0 w=fill gap=8.0 text-size=14.0 line-h=1.2 shape=advanced wrap=word-or-glyph font=mono icon="✓" icon-size=12.0 icon-line-h=1.0 icon-shape=basic -> changed _
      active checked bg=linear(1.57, primary@0.0, bg@1.0) icon=fg text=fg border=primary border-w=1.0 r=4.0 r-tl=2.0 r-tr=3.0 r-br=5.0 r-bl=6.0
      active unchecked bg=bg icon=primary text=fg border=fg
      hovered checked bg=primary icon=fg text=fg border=primary
      hovered unchecked bg=fg icon=bg text=primary border=primary
      disabled checked bg=bg icon=fg text=fg border=fg
      disabled unchecked bg=bg icon=primary text=fg border=primary
    toggler "Toggler" checked=enabled style=dynamic_toggler(enabled) size=20.0 w=fill gap=8.0 text-size=14.0 line-h=1.2 shape=auto wrap=glyph font=default align=right -> changed _
      active checked bg=linear(1.57, primary@0.0, bg@1.0) bg-border=primary bg-border-w=1.0 fg=linear(0.0, fg@0.0, primary@1.0) fg-border=fg fg-border-w=2.0 text=fg r=7.0 r-tl=6.0 r-tr=7.0 r-br=8.0 r-bl=9.0 p-ratio=0.125
      active unchecked bg=bg fg=fg text=primary
      hovered checked bg=primary fg=fg text=fg
      hovered unchecked bg=fg fg=bg text=primary
      disabled checked bg=bg fg=fg text=fg
      disabled unchecked bg=bg fg=primary text=fg
"#;
    let generated = compile(source, "preferences.ice").unwrap();
    assert!(generated.contains(
        ".size(::ui_lang_runtime::bounded_table_metric(20.0, 1)).spacing(::ui_lang_runtime::bounded_table_metric(8.0, 1))"
    ));
    assert!(generated.contains(".width(::iced::Fill)"));
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
    assert!(generated.contains("checkbox::Icon"));
    assert!(generated.contains("code_point: '✓'"));
    assert!(generated.contains(".text_alignment(::iced::widget::text::Alignment::Right)"));
    assert!(
        generated.contains("crate::backend::dynamic_checkbox(__theme, __status, self.enabled)")
    );
    assert!(generated.contains("fn __ui_lang_check_checkbox_style_dynamic_checkbox"));
    for (status, checked) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
        ("Disabled", true),
        ("Disabled", false),
    ] {
        assert!(generated.contains(&format!(
            "checkbox::Status::{status} {{ is_checked: {checked} }}"
        )));
    }
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.icon_color ="));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.border.width = 1.0 as f32"));
    assert!(generated.contains("top_left: ((2.0) as f32).max(0.0).min(f32::MAX)"));
    for preset in ["primary", "secondary", "success", "danger"] {
        let generated = compile(
            &source.replace(
                "style=dynamic_checkbox(enabled)",
                &format!("style={preset}"),
            ),
            "preferences.ice",
        )
        .unwrap();
        assert!(generated.contains(&format!("checkbox::{preset}(__theme, __status)")));
    }
    assert!(generated.contains("crate::backend::dynamic_toggler(__theme, __status, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_toggler_style_dynamic_toggler"));
    for (status, checked) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
        ("Disabled", true),
        ("Disabled", false),
    ] {
        assert!(generated.contains(&format!(
            "toggler::Status::{status} {{ is_toggled: {checked} }}"
        )));
    }
    assert!(generated.contains("__style.background_border_width = 1.0 as f32"));
    assert!(generated.contains("__style.foreground = ::iced::Background"));
    assert!(generated.contains("__style.foreground_border_width = 2.0 as f32"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.border_radius = ::std::option::Option::Some"));
    assert!(generated.contains("top_left: ((6.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("__style.padding_ratio = ((0.125) as f32).max(0.0).min(0.5)"));
    let generated = compile(
        &source.replace(" style=dynamic_toggler(enabled)", ""),
        "preferences.ice",
    )
    .unwrap();
    assert!(generated.contains("toggler::default(__theme, __status)"));
}

#[test]
fn lowers_full_text_format() {
    let source = r#"app Typography
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  font_size = 16.0
  line_height = 20.0
view
  text "Long text" w=fill h=40.0 size=font_size line-h-px=line_height font=mono align-x=justified align-y=center shape=advanced wrap=word-or-glyph @font-bold
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(generated.contains(".width(::iced::Fill).height(40.0 as f32)"));
    assert!(generated.contains(".size(((self.font_size) as f32).max(f32::EPSILON).min(f32::MAX))"));
    assert!(generated.contains(
        "LineHeight::Absolute(((self.line_height) as f32).max(f32::EPSILON).min(f32::MAX).into())"
    ));
    assert!(generated.contains("text::Alignment::Justified"));
    assert!(generated.contains("alignment::Vertical::Center"));
    assert!(generated.contains("text::Shaping::Advanced"));
    assert!(generated.contains("text::Wrapping::WordOrGlyph"));
    assert!(generated.contains("..::iced::Font::MONOSPACE"));
    assert!(generated.contains("::ui_lang_runtime::selectable_text(__text)"));
}

#[test]
fn lowers_native_text_style_callbacks() {
    let source = r#"app Typography
extern crate::backend
  text-style dynamic_text(active:bool)
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  col
    text "Styled" style=dynamic_text(active)
    rich-text style=dynamic_text(active) color=fg
      span "Rich"
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(
        generated.contains(
            "fn __ui_lang_check_text_style_dynamic_text(theme: &::iced::Theme, arg0: bool)"
        )
    );
    assert_eq!(
        generated
            .matches(".style(move |__theme| crate::backend::dynamic_text(__theme, self.active))")
            .count(),
        2
    );
    assert!(generated.contains(
        ".style(move |__theme| crate::backend::dynamic_text(__theme, self.active)).color("
    ));
}

#[test]
fn lowers_structured_rich_text_spans() {
    let source = r#"app Typography
font ui family=sans weight=medium stretch=normal style=normal
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
on link(url)
view
  rich-text w=fill h=48.0 size=16.0 line-h=1.2 font=ui align-x=justified align-y=center wrap=word color=fg @font-bold -> link _
    span "Ice " size=18.0 line-h-px=22.0 font=ui color=primary bg=linear(1.57, bg@0.0, primary@1.0) border=fg border-w=1.0 r=4.0 r-tl=2.0 r-tr=3.0 r-br=5.0 r-bl=6.0 p=2.0 pl=4.0 underline strike=false
    span "language" link="https://example.com" bg=bg size=18.0 @font-bold text-primary
"#;
    let generated = compile(source, "rich.ice").unwrap();
    assert!(generated.contains("::iced::widget::rich_text(__rich_spans)"));
    assert!(generated.contains("::iced::widget::span(\"Ice \".to_owned())"));
    assert!(generated.contains(".size(((18.0) as f32).max(f32::EPSILON).min(f32::MAX))"));
    assert!(
        generated.contains(
            "LineHeight::Absolute(((22.0) as f32).max(f32::EPSILON).min(f32::MAX).into())"
        )
    );
    assert!(generated.contains(".background(::iced::Background::Color("));
    assert!(generated.contains(".background(::iced::Background::from(::iced::gradient::Linear::new(1.57 as f32).add_stop(0.0 as f32"));
    assert!(generated.contains(".border(::iced::Border"));
    assert!(generated.contains(".padding(::ui_lang_runtime::bounded_padding(2.0, 2.0, 2.0, 4.0))"));
    assert!(generated.contains(".underline(true).strikethrough(false)"));
    assert!(generated.contains(".link(\"https://example.com\".to_owned())"));
    assert!(generated.contains(".on_link_click(move |__link| __TypographyMessage::Link(__link))"));
    assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
    assert!(generated.contains("text::Wrapping::Word"));
}

#[test]
fn lowers_declared_font_descriptors_and_app_default() {
    let source = r#"app Typography
font brand family="Inter" weight=semibold stretch=semi-expanded style=italic default=true
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Brand" font=brand @font-bold
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(generated.contains(".default_font(::iced::Font"));
    assert!(generated.contains("Family::Name(\"Inter\")"));
    assert!(generated.contains("Weight::Semibold"));
    assert!(generated.contains("Stretch::SemiExpanded"));
    assert!(generated.contains("Style::Italic"));
    assert!(generated.contains("weight: ::iced::font::Weight::Bold, ..::iced::Font"));

    let inherited = compile(
        &source.replace("text \"Brand\" font=brand", "text \"Brand\""),
        "typography.ice",
    )
    .unwrap();
    assert!(inherited.contains(
        "weight: ::iced::font::Weight::Bold, ..::iced::Font { family: ::iced::font::Family::Name(\"Inter\")"
    ));
}

#[test]
fn lowers_builtin_and_opacity_text_color_utilities() {
    let source = r#"app Typography
theme
  bg #000000
  fg #ffffff
  primary #336699
  danger #ff0000
state
view
  col
    text "Invisible" @text-transparent
    text "Muted" @text-primary/50
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(generated.contains(".color(::iced::Color::from_rgba8(0, 0, 0, 0.000000))"));
    assert!(generated.contains(".color(::iced::Color::from_rgba8(51, 102, 153, 0.500000))"));
}

#[test]
fn identifies_leaf_widgets_at_their_native_bounds() {
    let source = r#"app Identified
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  enabled = false
  amount = 25.0
  mode = 0
  choices = ["One", "Two"]
  selected:str? = none
  search:combo[str] = ["One", "Two"]
on toggled(next)
  enabled = next
on changed(next)
  amount = next
on mode_changed(next)
  mode = next
on selected_value(next)
  selected = some(next)
view
  col #root
    text "Plain" #plain
    rich-text #rich
      span "Rich"
    toggler "Toggle" #toggle checked=enabled -> toggled _
    slider amount #horizontal min=0.0 max=100.0 -> changed _
    slider amount #vertical min=0.0 max=100.0 vertical w=20.0 h=100.0 -> changed _
    radio "Mode" #radio value=1 selected=(mode == 1) -> mode_changed _
    pick choices selected #pick -> selected_value _
    combo search selected "Search" #combo -> selected_value _
"#;
    let generated = compile(source, "identified.ice").unwrap();

    assert_eq!(generated.matches("let __identified:").count(), 6);
    assert_eq!(generated.matches("let __identified_text").count(), 1);
    assert!(generated.contains("let __a11y_key = format!(\"{}/plain\""));
    assert!(generated.contains(".logical_id(__a11y_key.clone())"));
    for id in [
        "rich",
        "toggle",
        "horizontal",
        "vertical",
        "radio",
        "pick",
        "combo",
    ] {
        assert!(generated.contains(&format!("/{id}\"")), "missing #{id}");
    }
    assert!(generated.contains("::iced::widget::vertical_slider"));
    assert!(generated.contains("::iced::widget::container(__identified).id("));
}

#[test]
fn identifies_every_other_rendered_leaf() {
    let source = r##"app Leaves
extern crate::backend
  component native() -> unit
  themer themed() -> unit
  shader shaded() -> unit
qr code "https://example.com"
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  docs:markdown = "# Docs"
on open_link(url)
view
  col #root
    progress amount #progress
    rule horizontal #rule
    qr code #qr
    space #space w=10.0 h=10.0
    markdown docs #markdown -> open_link _
    extern native() #extern
    themer themed() #themer
    shader shaded() #shader w=20.0 h=20.0
    image "image.png" #image
    svg "image.svg" #svg
    viewer "image.png" #viewer
    canvas #canvas w=20.0 h=20.0
"##;
    let generated = compile(source, "leaves.ice").unwrap();

    assert_eq!(generated.matches("let __identified:").count(), 12);
    for id in [
        "progress", "rule", "qr", "space", "markdown", "extern", "themer", "shader", "image",
        "svg", "viewer", "canvas",
    ] {
        assert!(generated.contains(&format!("/{id}\"")), "missing #{id}");
    }
}

#[test]
fn retains_logical_paths_on_accessible_wrappers() {
    let source = r#"app LogicalIds
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  draft = ""
  checked = false
on submit
on checked_changed(next)
  checked = next
view
  col #root
    box #panel
      text "Plain" #plain
    rich-text #rich
      span "Rich"
    input "Draft" #input <-> draft
    button "Save" #button -> submit
    checkbox "Ready" #checkbox checked=checked -> checked_changed _
    image "photo.ppm" #image label="Portrait"
"#;
    let generated = compile(source, "logical_ids.ice").unwrap();

    assert_eq!(
        generated.matches("::ui_lang_runtime::accessible(").count(),
        generated.matches(".logical_id(__a11y_key.clone())").count()
    );
}

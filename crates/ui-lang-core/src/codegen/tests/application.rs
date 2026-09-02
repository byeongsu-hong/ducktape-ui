use super::*;
use crate::codegen::{
    BindingEnvMetrics, ValueMode, binding_env_metrics, checked_state_env, generate_theme,
    reset_binding_env_metrics, resolved_expr_use_code,
};

#[test]
fn publishes_a_modelled_view_as_data_and_wraps_its_root_for_dev_readiness() {
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
  text "ready"
"#;
    let generated = compile(source, "ready.ice").unwrap();

    assert!(generated.contains("fn __view(&self) -> __IceElement"));
    // The view is data: its structure and its literal reach the runtime as a
    // template rather than as widget-construction Rust.
    assert!(generated.contains("__ICE_TEMPLATE_JSON"));
    assert!(generated.contains("::ui_lang_runtime::template::render("));
    assert!(generated.contains(r#"\"literal\": \"ready\""#));
    assert!(generated.contains("::ui_lang_runtime::dev::ready(__ice_root)"));

    // A template is data the runtime renders, not a program it interprets.
    // The deleted live-plan machinery must not come back with it, and the
    // compiler must stay out of the app binary.
    for forbidden in [
        "::ui_lang_runtime::live",
        "::ui_lang_core",
        "LiveRuntime",
        "LivePlan",
        "__ice_live",
        "ICE_LIVE_",
    ] {
        assert!(
            !generated.contains(forbidden),
            "generated view contains removed live-reload machinery {forbidden}"
        );
    }
}

#[test]
fn wraps_each_daemon_window_view_for_dev_readiness() {
    let source = r#"daemon Agent
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
  text "ready"
"#;
    let generated = compile(source, "ready_daemon.ice").unwrap();

    assert!(generated.contains("fn __view(&self, window: ::iced::window::Id) -> __IceElement"));
    assert!(generated.contains("::ui_lang_runtime::template::render("));
    assert!(generated.contains(r#"\"literal\": \"ready\""#));
    assert!(generated.contains("::ui_lang_runtime::dev::ready(__ice_root)"));
    assert!(!generated.contains("::ui_lang_runtime::live"));
}

#[test]
fn wraps_each_daemon_window_view_for_keyboard_traversal() {
    let source = r#"daemon Agent
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
  count = 0
on go
  count = count + 1
view
  button "Go" -> go
"#;
    let generated = compile(source, "traversal_daemon.ice").unwrap();

    // A daemon's window is a root of its own: Tab traverses the controls of
    // the window it was pressed in, so the wrapper names its window and the
    // focus messages carry it.
    assert!(generated.contains(
        "::ui_lang_runtime::navigation(__ice_content, __AgentMessage::__AccessibilityFocusNext(::std::option::Option::Some(window)), __AgentMessage::__AccessibilityFocusPrevious(::std::option::Option::Some(window))).in_window(window)"
    ));
    assert!(generated.contains(
        "__AgentMessage::__AccessibilityFocusNext(__window) => { return ::ui_lang_runtime::focus_next_in::<__AgentMessage>(__window)"
    ));
}

#[test]
fn lowers_dynamic_palettes_into_runtime_theme_and_style_selection() {
    let source = r#"app Demo
  theme native_theme(dark)
  palette active_palette
extern crate::backend
  theme native_theme(dark:bool)
theme contract Ducktape
  bg
  fg
  primary
  danger
  surface
palette light for Ducktape
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
  surface #f4f4f4
palette dark for Ducktape
  bg #111111
  fg #ffffff
  primary #88aaff
  danger #ff6677
  surface #222222
state
  dark = false
  active_palette:palette[Ducktape] = Ducktape.light
view
  box bg=surface
    theme app
      text "Theme" @text-fg
"#;
    let generated = compile(source, "dynamic_palette.ice").unwrap();

    assert!(
        generated
            .contains("struct __IcePalette { name: &'static str, colors: [::iced::Color; 5] }")
    );
    assert!(generated.contains("enum Ducktape"));
    assert!(generated.contains("Ducktape::Light => __IcePalette"));
    assert!(generated.contains("Ducktape::Dark => __IcePalette"));
    assert!(!generated.contains("_ => __IcePalette"));
    assert!(generated.contains("crate::backend::native_theme(self.dark)"));
    // The theme is memoized so a build stops regenerating iced's extended
    // palette once per view function. This app carries two palettes and swaps
    // between them at runtime, so the key has to be the whole palette — name
    // and colors — not the name alone.
    assert!(generated.contains(
        "if __cached_palette.name == __ice_palette.name && __cached_palette.colors == __ice_palette.colors { return __cached_theme.clone(); }"
    ));
    assert!(generated.contains("background: __ice_palette.colors[0]"));
    assert!(generated.contains("text: __ice_palette.colors[1]"));
    // `box bg=surface` is modelled, so its background travels as a palette
    // index rather than a generated colour expression.
    assert!(generated.contains(r#"\"background\": {\n      \"base\": {\n        \"token\": 4"#));
    assert!(
        generated.contains("dynamic_themer(::std::option::Option::Some(__ice_app_theme.clone())")
    );
}

#[test]
fn theme_callbacks_overlay_window_without_cloning_state_environment() {
    let source = r#"daemon ScopedTheme
  title describe(window, label)
  theme native_theme(window, dark)
  palette active_palette
  bg background
  fg foreground
  scale scale_for(window, zoom)
extern crate::backend
  pure describe(id:window-id, label:str) -> str
  pure scale_for(id:window-id, zoom:f64) -> f64
  theme native_theme(id:window-id, dark:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette light for AppTheme
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
palette dark for AppTheme
  bg #111111
  fg #ffffff
  primary #88aaff
  danger #ff6677
state
  label = "Scoped"
  dark = false
  active_palette:palette[AppTheme] = AppTheme.light
  background = "000000"
  foreground = "ffffff"
  zoom = 1.25
secret window
view
  text label
"#;
    let program = crate::lower::lower(crate::analyze(source).unwrap()).unwrap();
    let state_bindings = checked_state_env(&program, "self").len();
    assert_eq!(state_bindings, 7);

    reset_binding_env_metrics();
    let mut generated = String::new();
    generate_theme(&mut generated, &program).unwrap();
    let metrics = binding_env_metrics();

    assert_eq!(
        metrics.binding_clone_allocations, 0,
        "theme generation cloned its {state_bindings}-binding state environment"
    );
    assert_eq!(metrics.overlays, 1);
    assert_eq!(metrics.overlay_binding_allocations, 1);
    for expected in [
        "crate::backend::describe(window, self.label.to_owned())",
        "crate::backend::native_theme(window, self.dark)",
        "match self.active_palette.clone()",
        "self.background.to_owned()",
        "self.foreground.to_owned()",
        "crate::backend::scale_for(window, self.zoom)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
    assert!(
        !generated.contains("self.__ice_secrets"),
        "the daemon callback-local `window` must shadow the same-named secret"
    );
}

#[test]
fn keeps_distinct_handler_names_distinct_in_rust() {
    let generated = compile(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non foo_bar\non fooBar\nview\n  col\n    button \"one\" -> foo_bar\n    button \"two\" -> fooBar\n",
        "handlers.ice",
    )
    .unwrap();

    assert!(generated.contains("FooBar"));
    assert!(generated.contains("__0H666f6f426172"));
    assert!(generated.contains("impl ::std::fmt::Debug for Demo"));
    assert!(generated.contains("impl ::std::fmt::Debug for __DemoMessage"));
    assert!(!generated.contains("#[derive(Debug)]\npub struct Demo"));
    assert!(!generated.contains("#[derive(Debug, Clone)]\nenum __DemoMessage"));
}

#[test]
fn lowers_every_native_alignment_operation() {
    let source = example!("alignment.ice");
    let generated = compile(source, "alignment.ice").unwrap();
    for expected in [
        "::iced::Alignment::Start",
        "::iced::Alignment::Center",
        "::iced::Alignment::End",
        "::iced::alignment::Horizontal::Left",
        "::iced::alignment::Horizontal::Center",
        "::iced::alignment::Horizontal::Right",
        "::iced::alignment::Vertical::Top",
        "::iced::alignment::Vertical::Center",
        "::iced::alignment::Vertical::Bottom",
        "::iced::Alignment::from(",
        "::iced::alignment::Horizontal::from(",
        "::iced::alignment::Vertical::from(",
        "crate::backend::alignment_round_trip(self.center)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}
#[test]
fn lowers_every_native_shadow_operation() {
    let source = example!("shadow.ice");
    let generated = compile(source, "shadow.ice").unwrap();
    for expected in [
        "::iced::Shadow::default()",
        "::iced::Shadow { color: ::iced::Color::from_rgba(",
        "offset: ::iced::Vector::new((4.0) as f32, (8.0) as f32)",
        "blur_radius: (12.0) as f32",
        "crate::backend::shadow_round_trip(self.value)",
        "(self.value).color",
        "(self.value).offset",
        "(self.value).blur_radius as f64",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_border_and_radius_operation() {
    let source = example!("border_radius.ice");
    let generated = compile(source, "border_radius.ice").unwrap();
    for expected in [
        "::iced::Border::default()",
        "::iced::Border { color: ::iced::Color::from_rgba(",
        "width: (::iced::Pixels((2.0) as f32)).0",
        "::iced::border::color(::iced::Color::BLACK)",
        "::iced::border::width(::iced::Pixels((4.0) as f32))",
        "::iced::border::rounded(::iced::border::Radius::from((5.0) as f32))",
        ".color(::iced::Color::WHITE)",
        ".width((6.0) as f32)",
        ".rounded(::iced::border::radius((7.0) as f32))",
        "crate::backend::border_round_trip(self.built_border)",
        "(self.built_border).color",
        "(self.built_border).width as f64",
        "(self.built_border).radius",
        "::iced::border::Radius::default()",
        "::iced::border::radius(::iced::Pixels((2.0) as f32))",
        "::iced::border::Radius::new((3.0) as f32)",
        "::iced::border::top_left((1.0) as f32)",
        "::iced::border::top_right(::iced::Pixels((2.0) as f32))",
        "::iced::border::bottom_right((3.0) as f32)",
        "::iced::border::bottom_left((4.0) as f32)",
        "::iced::border::top((5.0) as f32)",
        "::iced::border::bottom((6.0) as f32)",
        "::iced::border::left((7.0) as f32)",
        "::iced::border::right((8.0) as f32)",
        ".top_left((1.0) as f32)",
        ".top_right((2.0) as f32)",
        ".bottom_right((3.0) as f32)",
        ".bottom_left((4.0) as f32)",
        ".top((5.0) as f32)",
        ".bottom((6.0) as f32)",
        ".left((7.0) as f32)",
        ".right(::iced::Pixels((8.0) as f32))",
        "::iced::border::Radius::from((9.0) as f32)",
        "::iced::border::Radius::from(10u8)",
        "::iced::border::Radius::from(11u32)",
        "::iced::border::Radius::from(((-3)) as i32)",
        "<u8>::try_from((unsigned_input) as i64)",
        "<u32>::try_from((unsigned_input) as i64)",
        "<i32>::try_from((signed_input) as i64)",
        "crate::backend::radius_round_trip(self.built_radius)",
        "self.uniform_radius * (2.0) as f32",
        "::std::convert::Into::<[f32; 4]>::into(self.built_radius)",
        "(self.built_radius).top_left as f64",
        "(self.built_radius).top_right as f64",
        "(self.built_radius).bottom_right as f64",
        "(self.built_radius).bottom_left as f64",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_background_and_gradient_operation() {
    let source = example!("background_gradient.ice");
    let generated = compile(source, "background_gradient.ice").unwrap();
    for expected in [
        "::iced::gradient::ColorStop::default()",
        "::iced::gradient::ColorStop { offset: (0.25) as f32, color:",
        "crate::backend::color_stop_round_trip(self.custom_stop)",
        "(self.custom_stop).offset as f64",
        "(self.custom_stop).color",
        "::iced::gradient::Linear::new(::iced::Radians((0.5) as f32))",
        "::iced::gradient::Linear::new(::iced::Radians((0.75) as f32))",
        "::ui_lang_runtime::add_gradient_stops(self.numeric_linear, [::iced::gradient::ColorStop { offset: (0.75) as f32, color: ::iced::Color::WHITE }])",
        "::ui_lang_runtime::add_gradient_stops(::iced::gradient::Linear::new(::iced::Radians((1.0) as f32)), ::std::vec![",
        ".scale_alpha((0.5) as f32)",
        "crate::backend::linear_round_trip(self.multi_linear)",
        "(self.numeric_linear).angle",
        ".stops.into_iter().collect::<::std::vec::Vec<::std::option::Option<::iced::gradient::ColorStop>>>()",
        "::iced::Gradient::Linear(self.added_linear)",
        "::iced::Gradient::from(self.added_linear)",
        "crate::backend::gradient_round_trip(self.converted_gradient)",
        "match (self.direct_gradient) { ::iced::Gradient::Linear(__value) => __value }",
        "::iced::Background::Color(",
        "::iced::Background::Gradient(self.direct_gradient)",
        "::iced::Background::from(::iced::Color::WHITE)",
        "::iced::Background::from(self.converted_gradient)",
        "::iced::Background::from(self.added_linear)",
        "crate::backend::background_round_trip(self.from_linear_background)",
        "::iced::Background::Color(__value) => ::std::option::Option::Some(__value)",
        "::iced::Background::Gradient(__value) => ::std::option::Option::Some(__value)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_font_operation() {
    let source = example!("font_values.ice");
    let generated = compile(source, "font_values.ice").unwrap();
    for expected in [
        "::iced::Font::default()",
        "::iced::Font::DEFAULT",
        "::iced::Font::MONOSPACE",
        "::iced::Font::with_name(\"Inter\")",
        "::iced::Font { family: ::iced::font::Family::Name(\"Display\"), weight: ::iced::font::Weight::Bold, stretch: ::iced::font::Stretch::Expanded, style: ::iced::font::Style::Italic }",
        "::iced::font::Family::default()",
        "::iced::font::Family::Name(\"Inter\")",
        "::iced::font::Family::Serif",
        "::iced::font::Family::SansSerif",
        "::iced::font::Family::Cursive",
        "::iced::font::Family::Fantasy",
        "::iced::font::Family::Monospace",
        "::iced::font::Weight::default()",
        "::iced::font::Weight::Thin",
        "::iced::font::Weight::ExtraLight",
        "::iced::font::Weight::Light",
        "::iced::font::Weight::Normal",
        "::iced::font::Weight::Medium",
        "::iced::font::Weight::Semibold",
        "::iced::font::Weight::Bold",
        "::iced::font::Weight::ExtraBold",
        "::iced::font::Weight::Black",
        "::iced::font::Stretch::default()",
        "::iced::font::Stretch::UltraCondensed",
        "::iced::font::Stretch::ExtraCondensed",
        "::iced::font::Stretch::Condensed",
        "::iced::font::Stretch::SemiCondensed",
        "::iced::font::Stretch::Normal",
        "::iced::font::Stretch::SemiExpanded",
        "::iced::font::Stretch::Expanded",
        "::iced::font::Stretch::ExtraExpanded",
        "::iced::font::Stretch::UltraExpanded",
        "::iced::font::Style::default()",
        "::iced::font::Style::Normal",
        "::iced::font::Style::Italic",
        "::iced::font::Style::Oblique",
        "crate::backend::font_round_trip(self.custom_font)",
        "crate::backend::family_round_trip(::iced::font::Family::Name(\"Inter\"))",
        "crate::backend::weight_round_trip(::iced::font::Weight::Bold)",
        "crate::backend::stretch_round_trip(::iced::font::Stretch::Expanded)",
        "crate::backend::style_round_trip(::iced::font::Style::Italic)",
        "(self.custom_font).family",
        "(self.custom_font).weight",
        "(self.custom_font).stretch",
        "(self.custom_font).style",
        "::iced::font::Family::Name(_) => \"named\"",
        "::iced::font::Family::Name(__value) => ::std::option::Option::Some(__value.to_owned())",
        "::ui_lang_runtime::memo_lazy((self.__ice_rev[",
        "= self.returned_font; let __lazy_content",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_theme_mode_operation() {
    let source = example!("theme_mode.ice");
    let generated = compile(source, "theme_mode.ice").unwrap();
    for expected in [
        "::iced::theme::Mode::default()",
        "::iced::theme::Mode::None",
        "::iced::theme::Mode::Light",
        "::iced::theme::Mode::Dark",
        "crate::backend::theme_mode_round_trip(::iced::theme::Mode::Dark)",
        "::iced::theme::Mode::Dark => \"dark\"",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_text_value_operation() {
    let source = example!("text_values.ice");
    let generated = compile(source, "text_values.ice").unwrap();
    for expected in [
        "::iced::widget::text::Alignment::default()",
        "::iced::widget::text::Alignment::Left",
        "::iced::widget::text::Alignment::Center",
        "::iced::widget::text::Alignment::Right",
        "::iced::widget::text::Alignment::Justified",
        "::iced::widget::text::Alignment::from(::iced::alignment::Horizontal::Center)",
        "::iced::widget::text::Alignment::from(::iced::Alignment::End)",
        "::iced::alignment::Horizontal::from(::iced::widget::text::Alignment::Justified)",
        "::iced::widget::text::Shaping::default()",
        "::iced::widget::text::Shaping::Auto",
        "::iced::widget::text::Shaping::Basic",
        "::iced::widget::text::Shaping::Advanced",
        "::iced::widget::text::Wrapping::default()",
        "::iced::widget::text::Wrapping::None",
        "::iced::widget::text::Wrapping::Word",
        "::iced::widget::text::Wrapping::Glyph",
        "::iced::widget::text::Wrapping::WordOrGlyph",
        "::iced::widget::text::LineHeight::default()",
        "::iced::widget::text::LineHeight::Relative((1.5) as f32)",
        "::iced::widget::text::LineHeight::Absolute(::iced::Pixels((24.0) as f32))",
        "::iced::widget::text::LineHeight::from((1.25) as f32)",
        "::iced::widget::text::LineHeight::from(::iced::Pixels((30.0) as f32))",
        ").to_absolute(::iced::Pixels((20.0) as f32))",
        "::iced::widget::text::Alignment::Justified => \"justified\"",
        "::iced::widget::text::Shaping::Advanced => \"advanced\"",
        "::iced::widget::text::Wrapping::WordOrGlyph => \"word-or-glyph\"",
        "::iced::widget::text::LineHeight::Relative(__value)",
        "::iced::widget::text::LineHeight::Absolute(__value)",
        "crate::backend::text_alignment_round_trip",
        "crate::backend::text_shaping_round_trip",
        "crate::backend::text_wrapping_round_trip",
        "crate::backend::text_line_height_round_trip",
        "::ui_lang_runtime::memo_lazy((self.__ice_rev[",
        "= self.returned_alignment; let __lazy_content",
        "= self.returned_shaping; let __lazy_content",
        "= self.returned_wrapping; let __lazy_content",
        "= self.returned_line_height; let __lazy_content",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_mouse_interaction() {
    let source = example!("mouse_interaction.ice");
    let generated = compile(source, "mouse_interaction.ice").unwrap();
    for expected in [
        "::iced::mouse::Interaction::default()",
        "::iced::mouse::Interaction::None",
        "::iced::mouse::Interaction::Hidden",
        "::iced::mouse::Interaction::Idle",
        "::iced::mouse::Interaction::ContextMenu",
        "::iced::mouse::Interaction::Help",
        "::iced::mouse::Interaction::Pointer",
        "::iced::mouse::Interaction::Progress",
        "::iced::mouse::Interaction::Wait",
        "::iced::mouse::Interaction::Cell",
        "::iced::mouse::Interaction::Crosshair",
        "::iced::mouse::Interaction::Text",
        "::iced::mouse::Interaction::Alias",
        "::iced::mouse::Interaction::Copy",
        "::iced::mouse::Interaction::Move",
        "::iced::mouse::Interaction::NoDrop",
        "::iced::mouse::Interaction::NotAllowed",
        "::iced::mouse::Interaction::Grab",
        "::iced::mouse::Interaction::Grabbing",
        "::iced::mouse::Interaction::ResizingHorizontally",
        "::iced::mouse::Interaction::ResizingVertically",
        "::iced::mouse::Interaction::ResizingDiagonallyUp",
        "::iced::mouse::Interaction::ResizingDiagonallyDown",
        "::iced::mouse::Interaction::ResizingColumn",
        "::iced::mouse::Interaction::ResizingRow",
        "::iced::mouse::Interaction::AllScroll",
        "::iced::mouse::Interaction::ZoomIn",
        "::iced::mouse::Interaction::ZoomOut",
        "crate::backend::interaction_round_trip(::iced::mouse::Interaction::Pointer)",
        "::iced::mouse::Interaction::Pointer => \"pointer\"",
        "::iced::widget::mouse_area(__mouse_content).interaction(self.returned)",
        "if __cursor.is_over(__bounds) { self.returned }",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_scroll_delta_operation() {
    let source = example!("scroll_delta.ice");
    let generated = compile(source, "scroll_delta.ice").unwrap();
    for expected in [
        "::iced::mouse::ScrollDelta::Lines { x: (1.5) as f32, y: ((-2.25)) as f32 }",
        "::iced::mouse::ScrollDelta::Pixels { x: ((-3.75)) as f32, y: (4.5) as f32 }",
        "crate::backend::scroll_delta_round_trip(self.pixels)",
        "::iced::mouse::ScrollDelta::Lines { .. } => \"lines\"",
        "::iced::mouse::ScrollDelta::Pixels { .. } => \"pixels\"",
        "::iced::mouse::ScrollDelta::Lines { x, .. } | ::iced::mouse::ScrollDelta::Pixels { x, .. } => x as f64",
        "::iced::mouse::ScrollDelta::Lines { y, .. } | ::iced::mouse::ScrollDelta::Pixels { y, .. } => y as f64",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_window_value() {
    let source = example!("window_values.ice");
    let generated = compile(source, "window_values.ice").unwrap();
    for expected in [
        "::iced::window::Direction::North",
        "::iced::window::Direction::South",
        "::iced::window::Direction::East",
        "::iced::window::Direction::West",
        "::iced::window::Direction::NorthEast",
        "::iced::window::Direction::NorthWest",
        "::iced::window::Direction::SouthEast",
        "::iced::window::Direction::SouthWest",
        "::iced::window::Level::default()",
        "::iced::window::Level::Normal",
        "::iced::window::Level::AlwaysOnBottom",
        "::iced::window::Level::AlwaysOnTop",
        "::iced::window::Mode::Windowed",
        "::iced::window::Mode::Fullscreen",
        "::iced::window::Mode::Hidden",
        "::iced::window::UserAttention::Critical",
        "::iced::window::UserAttention::Informational",
        "crate::backend::direction_round_trip(::iced::window::Direction::SouthWest)",
        "crate::backend::level_round_trip(::iced::window::Level::AlwaysOnTop)",
        "crate::backend::mode_round_trip(::iced::window::Mode::Fullscreen)",
        "crate::backend::attention_round_trip(::iced::window::UserAttention::Informational)",
        "::iced::window::Direction::SouthWest => \"south-west\"",
        "::iced::window::Level::AlwaysOnTop => \"always-on-top\"",
        "::iced::window::Mode::Fullscreen => \"fullscreen\"",
        "::iced::window::UserAttention::Informational => \"informational\"",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_window_position_operation() {
    let source = example!("window_position.ice");
    let generated = compile(source, "window_position.ice").unwrap();
    for expected in [
        "::iced::window::Position::default()",
        "::iced::window::Position::Centered",
        "::iced::window::Position::Specific(::iced::Point::new((24.0) as f32, ((-12.0)) as f32))",
        "crate::backend::responsive_position()",
        "crate::backend::position_round_trip(self.specific_position)",
        "::iced::window::Position::Default => \"default\"",
        "::iced::window::Position::Centered => \"centered\"",
        "::iced::window::Position::Specific(_) => \"specific\"",
        "::iced::window::Position::SpecificWith(_) => \"specific-with\"",
        "::iced::window::Position::Specific(__value) => ::std::option::Option::Some(__value)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_event_status_operation() {
    let source = example!("event_status.ice");
    let generated = compile(source, "event_status.ice").unwrap();
    for expected in [
        "::iced::event::Status::Ignored",
        "::iced::event::Status::Captured",
        "crate::backend::status_round_trip(::iced::event::Status::Captured)",
        "(self.ignored).merge(self.captured)",
        "::iced::event::Status::Ignored => \"ignored\"",
        "::iced::event::Status::Captured => \"captured\"",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_redraw_request_operation() {
    let source = example!("redraw_request.ice");
    let generated = compile(source, "redraw_request.ice").unwrap();
    for expected in [
        "::iced::window::RedrawRequest::NextFrame",
        "::iced::window::RedrawRequest::At(({ let __ice_call = \
         ::ui_lang_runtime::dev::Span::extern_call(\"redraw_now\", \
         \"redraw_request.ice:5\"); crate::backend::redraw_now() }))",
        "::iced::window::RedrawRequest::Wait",
        "crate::backend::redraw_round_trip(self.at)",
        "::iced::window::RedrawRequest::At(_) => \"at\"",
        "::iced::window::RedrawRequest::At(__value) => ::std::option::Option::Some(__value)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_window_id_operation() {
    let source = example!("window_id.ice");
    let generated = compile(source, "window_id.ice").unwrap();
    for expected in [
        "::iced::window::Id::unique()",
        "crate::backend::window_id_round_trip(self.first)",
        "(self.first).to_string()",
        "::ui_lang_runtime::memo_lazy((self.__ice_rev[",
        "= self.first; let __lazy_content",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_window_screenshot_operation() {
    let source = example!("window_screenshot.ice");
    let generated = compile(source, "window_screenshot.ice").unwrap();
    for expected in [
        "::iced::window::Screenshot::new(",
        "crate::backend::screenshot_round_trip(sample.clone()",
        "::ui_lang_runtime::crop_screenshot(&(sample), crate::backend::screenshot_crop_region()).ok()",
        "::iced::window::screenshot(__window).map(move |value| __NativeWindowScreenshotMessage::NativeCaptured(value))",
        "::iced::window::screenshot::CropError::Zero",
        "::iced::window::screenshot::CropError::OutOfBounds",
        ".err().map(|error| error.to_string())",
        "::std::convert::AsRef::<[u8]>::as_ref(&(self.returned)).to_vec()",
        "(self.returned.clone()).rgba.to_vec()",
        "(self.returned).rgba.to_vec()",
        "(self.returned).size",
        "(self.returned).scale_factor as f64",
        "::std::format!(\"{:?}\", &(self.returned))",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_length_operation() {
    let source = example!("length.ice");
    let generated = compile(source, "length.ice").unwrap();
    for expected in [
        "::iced::Length::Fill",
        "::iced::Length::FillPortion(3u16)",
        "::iced::Length::Shrink",
        "::iced::Length::Fixed((48.0) as f32)",
        "::iced::Length::from((64.0) as f32)",
        "::iced::Length::from(::iced::Pixels((72.0) as f32))",
        "::iced::Length::from(96u32)",
        "<u16>::try_from(3)",
        "<u32>::try_from(96)",
        ".fluid()",
        ".enclose(",
        ".fill_factor() as i64",
        ".is_fill()",
        ".width(self.fill_length)",
        ".height(self.shrink_length)",
        "crate::backend::length_round_trip(self.fixed_length)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_color_operation() {
    let source = example!("color.ice");
    let generated = compile(source, "color.ice").unwrap();
    for expected in [
        "::iced::Color::default()",
        "::iced::Color::BLACK",
        "::iced::Color::WHITE",
        "::iced::Color::TRANSPARENT",
        "::iced::Color::from_rgb(",
        "::iced::Color::from_rgba(",
        "::iced::Color::from_rgba(((self.red) as f32).max(0.0).min(1.0), ((self.green) as f32).max(0.0).min(1.0), ((self.blue) as f32).max(0.0).min(1.0), ((self.alpha) as f32).max(0.0).min(1.0))",
        "::iced::Color::from_rgb8(12u8, 34u8, 56u8)",
        "::iced::Color::from_rgba8(12u8, 34u8, 56u8,",
        "<u8>::try_from(red8)",
        "<u8>::try_from(green8)",
        "<u8>::try_from(blue8)",
        "if (0.0..=1.0).contains(&__alpha)",
        "::iced::Color::from_linear_rgba(",
        "::iced::Color::from([",
        ".parse::<::iced::Color>().ok()",
        ".inverse()",
        ".scale_alpha(",
        ".into_rgba8()",
        ".into_linear()",
        ".relative_luminance()",
        ".relative_contrast(",
        ".is_readable_on(",
        "crate::backend::color_round_trip(self.rgba8)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_content_fit_operation() {
    let source = example!("content_fit.ice");
    let generated = compile(source, "content_fit.ice").unwrap();
    for expected in [
        "::iced::ContentFit::default()",
        "::iced::ContentFit::Contain",
        "::iced::ContentFit::Cover",
        "::iced::ContentFit::Fill",
        "::iced::ContentFit::None",
        "::iced::ContentFit::ScaleDown",
        ".fit(::iced::Size::new((100.0) as f32, (50.0) as f32), ::iced::Size::new((80.0) as f32, (80.0) as f32))",
        ".content_fit(self.round_trip)",
        ".content_fit(self.scale_down_fit)",
        ".content_fit(self.fill_fit)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_native_rotation_operation() {
    let source = example!("rotation.ice");
    let generated = compile(source, "rotation.ice").unwrap();
    for expected in [
        "::iced::Rotation::default()",
        "::iced::Rotation::Floating(::iced::Radians((0.25) as f32))",
        "::iced::Rotation::Solid(::iced::Radians((0.5) as f32))",
        "*__rotation.radians_mut() = ::iced::Radians((0.75) as f32)",
        "::iced::Rotation::from(0.2 as f32)",
        ".apply(::iced::Size::new((10.0) as f32, (20.0) as f32))",
        ".radians()",
        ".degrees()",
        ".rotation(self.solid_rotation)",
        ".rotation(self.adjusted_rotation)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_debug_spans_and_timed_values() {
    let source = example!("debug_timing.ice");
    let generated = compile(source, "debug_timing.ice").unwrap();
    for expected in [
        "::std::option::Option<::iced::debug::Span>",
        "::iced::debug::time(\"interaction\".to_owned())",
        "__span.finish()",
        "(self.timer).is_some()",
        "::iced::debug::time_with(\"compute\".to_owned(), || (value + 1))",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_image_allocation_and_retention() {
    let source = example!("image_allocation.ice");
    let generated = compile(source, "image_allocation.ice").unwrap();
    for expected in [
        "::iced::widget::image::allocate(self.handle.clone())",
        "::iced::widget::image::Allocation",
        ".handle().clone()",
        ".size()",
        ".downgrade()",
        "::iced::widget::image::Allocation::upgrade",
        "::iced::widget::image::Error::OutOfMemory",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_animation_without_a_custom_runtime() {
    let source = example!("animation.ice");
    let generated = compile(source, "animation.ice").unwrap();
    for expected in [
        "::iced::Animation::new(false)",
        "::iced::animation::Easing::EaseInOut",
        ".duration(::std::time::Duration::from_millis(400))",
        ".repeat(1).auto_reverse()",
        "::iced::Animation<crate::backend::Motion>",
        ".very_quick()",
        ".slow()",
        ".very_slow().repeat_forever()",
        "self.progress.go_mut",
        ".interpolate_with(",
        "::std::option::Option::<f32>::None",
        "::ui_lang_runtime::animation_remaining_millis(&(self.expanded), ::iced::time::Instant::now())",
        "::iced::window::frames()",
        "__AnimationFrame",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn projection_binding_overlays_match_checked_and_ast_emission() {
    let source = r#"app ProjectionOverlay
extern crate::backend
  sync now() -> instant
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
  sample:f64 = 41.0
  progress:animation[f64] = 0.0
  secondary:animation[f64] = 0.0
  captured:f64? = none
  at:instant = now()
derived
  projected = animation.project(progress, sample, sample + 1.0, at)
  nested = animation.project(progress, sample, animation.project(secondary, sample, sample, at) + sample, at)
  optional = animation.project(progress, sample, some(sample), at)
on capture
  captured = animation.project(progress, sample, some(sample), at)
view
  col
    text projected
    text nested
    button "Capture" -> capture
"#;

    let generated = compile(source, "projection_overlay.ice").unwrap();

    let optional_projection = "interpolate_with(|__value| (::std::option::Option::Some((__value as f64))).map(|__value| __value as f32)";
    assert_eq!(
        generated.matches(optional_projection).count(),
        2,
        "checked derived and AST handler emission must produce the same optional projection"
    );
    assert!(generated.contains("interpolate_with(|__value| (((__value as f64) + 1.0)) as f32"));
    assert!(generated.contains(
        "interpolate_with(|__value| (((self.secondary).interpolate_with(|__value| ((__value as f64)) as f32"
    ));
    assert!(generated.contains("as f64 + (__value as f64))"));
    assert!(
        !generated.contains("|__value| (self.sample"),
        "projection locals must shadow app bindings with the same name"
    );
}

#[test]
#[ignore = "explicit full compile and codegen projection linearity contract"]
fn performance_contract_codegen_four_thousand_projections_uses_binding_overlays() {
    use std::fmt::Write as _;
    use std::time::Instant;

    fn measure(derived: usize) -> (BindingEnvMetrics, std::time::Duration, usize) {
        let mut source = String::from(
            r#"app ProjectionCodegen
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
extern crate::backend
  sync now() -> instant
state
  progress:animation[f64] = 0.0
  at:instant = now()
derived
"#,
        );
        for index in 0..derived {
            writeln!(
                source,
                "  value_{index} = animation.project(progress, sample, sample + 1.0, at)"
            )
            .unwrap();
        }
        source.push_str("view\n  text value_0\n");

        reset_binding_env_metrics();
        let started = Instant::now();
        let generated = compile(&source, "projection_codegen.ice").unwrap();
        let elapsed = started.elapsed();
        assert!(generated.contains(&format!("value_{}", derived - 1)));
        (binding_env_metrics(), elapsed, generated.len())
    }

    let (small, small_elapsed, small_output) = measure(500);
    let (large, large_elapsed, large_output) = measure(4_000);

    assert_eq!(small.overlays, 501);
    assert_eq!(large.overlays, 4_001);
    assert_eq!(large.overlay_binding_allocations, 4_001);
    assert!(
        large.binding_clone_allocations <= large.overlays * 3,
        "projection codegen copied more than a constant number of bindings per expression: {} clones for {} overlays",
        large.binding_clone_allocations,
        large.overlays
    );
    assert!(
        large.binding_clone_allocations <= small.binding_clone_allocations * 10 + 32,
        "binding clone allocations must remain linear: 500={}, 4k={}",
        small.binding_clone_allocations,
        large.binding_clone_allocations
    );
    assert!(large_output > small_output * 7);
    eprintln!(
        "500 projections in {small_elapsed:?} with {} binding clones; 4k in {large_elapsed:?} with {} binding clones",
        small.binding_clone_allocations, large.binding_clone_allocations
    );
    assert!(
        large_elapsed.as_secs_f64() < 8.0,
        "4k projection full compile and codegen completed in {large_elapsed:?}"
    );
    assert!(
        large_elapsed.as_secs_f64() <= small_elapsed.as_secs_f64() * 12.0 + 0.5,
        "full projection compile/codegen scaling exceeded the linear allowance: 500={small_elapsed:?}, 4k={large_elapsed:?}"
    );
}

#[test]
#[ignore = "explicit 500/4000 retained view-expression allocation and codegen contract"]
fn performance_contract_four_thousand_component_arguments_are_linear() {
    use std::fmt::Write as _;
    use std::time::Instant;

    fn measure(calls: usize) -> (crate::check::CheckedFactMetrics, std::time::Duration, usize) {
        let mut source = String::from(
            r#"app ViewFacts
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
  base = 1
component Label(value:i64)
  text value
view
  col
"#,
        );
        for index in 0..calls {
            writeln!(source, "    Label value=(base + {index})").unwrap();
        }

        let started = Instant::now();
        let program = crate::lower::lower(crate::analyze(&source).unwrap()).unwrap();
        let generated = crate::codegen::generate(&program, "view_facts.ice").unwrap();
        (
            program.checked_facts().metrics(),
            started.elapsed(),
            generated.len(),
        )
    }

    let (small, small_elapsed, small_output) = measure(500);
    let (large, large_elapsed, large_output) = measure(4_000);
    assert_eq!(small.view_analysis_passes, 500);
    assert_eq!(large.view_analysis_passes, 4_000);
    // The state initializer and component-body Text value are fixed checked
    // expressions; remove both before comparing per-call work.
    assert_eq!(large.expression_uses - 2, (small.expression_uses - 2) * 8);
    assert_eq!(large.expressions - 2, (small.expressions - 2) * 8);
    assert_eq!(
        large.type_analysis_nodes - 2,
        (small.type_analysis_nodes - 2) * 8
    );
    assert_eq!(large.type_scope_env_full_clones, 0);
    assert_eq!(large.scope_env_full_clones, 0);
    assert!(large_output > small_output * 7);
    eprintln!("500 component arguments in {small_elapsed:?}; 4k in {large_elapsed:?}");
    assert!(
        large_elapsed.as_secs_f64() < 8.0,
        "4k retained component arguments completed full codegen in {large_elapsed:?}"
    );
    assert!(
        large_elapsed.as_secs_f64() <= small_elapsed.as_secs_f64() * 12.0 + 0.5,
        "retained component argument scaling exceeded the linear allowance: 500={small_elapsed:?}, 4k={large_elapsed:?}"
    );
}

#[test]
fn checked_expression_emission_rejects_a_name_match_with_the_wrong_owner() {
    let source = r#"app OwnerMismatch
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
  count = 1
component Label(value:i64)
  text value
view
  Label value=count
"#;
    let program = crate::lower::lower(crate::analyze(source).unwrap()).unwrap();
    let crate::lower::ResolvedViewKind::Component { call } = program
        .resolved_view(program.app_view())
        .map(|view| &view.kind)
        .unwrap()
    else {
        panic!("application root is not a component call")
    };
    let call = program.component_call_by_id(*call).unwrap();
    let expression = call.arguments[0].expression;
    let mut env = checked_state_env(&program, "self");
    env.get_mut("count").unwrap().owner = None;

    let error = resolved_expr_use_code(&program, expression, &env, ValueMode::Owned).unwrap_err();
    assert_eq!(error.code, "E196");
    assert!(error.message.contains("mismatched emission owner"));
}

#[test]
#[ignore = "explicit 500/4000 lexical view-scope allocation contract"]
fn performance_contract_four_thousand_sibling_scopes_borrow_large_environments() {
    use std::fmt::Write as _;
    use std::time::Instant;

    fn measure(
        siblings: usize,
    ) -> (
        crate::check::CheckedFactMetrics,
        BindingEnvMetrics,
        std::time::Duration,
    ) {
        let mut source = String::from(
            r#"app ScopedSiblings
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
  items:[i64] = [1]
"#,
        );
        for index in 0..siblings {
            writeln!(source, "  filler_{index} = {index}").unwrap();
        }
        source.push_str("view\n  col\n");
        for _ in 0..siblings {
            source.push_str("    for item in items\n      text item\n");
        }

        let program = crate::lower::lower(crate::analyze(&source).unwrap()).unwrap();
        reset_binding_env_metrics();
        let started = Instant::now();
        crate::codegen::generate(&program, "scoped-siblings.ice").unwrap();
        (
            program.checked_facts().metrics(),
            binding_env_metrics(),
            started.elapsed(),
        )
    }

    let (small_facts, small_codegen, small_elapsed) = measure(500);
    let (large_facts, large_codegen, large_elapsed) = measure(4_000);
    assert_eq!(small_facts.view_scope_env_overlays, 500);
    assert_eq!(large_facts.view_scope_env_overlays, 4_000);
    assert_eq!(small_facts.view_scope_env_full_clones, 0);
    assert_eq!(large_facts.view_scope_env_full_clones, 0);
    assert_eq!(small_codegen.overlays, 501);
    assert_eq!(large_codegen.overlays, 4_001);
    assert_eq!(small_codegen.scope_env_full_clones, 0);
    assert_eq!(large_codegen.scope_env_full_clones, 0);
    assert!(
        large_codegen.binding_clone_allocations <= small_codegen.binding_clone_allocations * 9 + 32,
        "binding clones must remain linear: 500={}, 4k={}",
        small_codegen.binding_clone_allocations,
        large_codegen.binding_clone_allocations,
    );
    eprintln!(
        "500 sibling scopes in {small_elapsed:?} with {} binding clones; 4k in {large_elapsed:?} with {} binding clones",
        small_codegen.binding_clone_allocations, large_codegen.binding_clone_allocations,
    );
    assert!(
        large_elapsed.as_secs_f64() < 20.0,
        "4k sibling scopes completed in {large_elapsed:?}"
    );
}

#[test]
fn lowers_windowless_daemon_and_exit() {
    let source = r#"daemon Agent
  title label(window)
  theme "dark"
  scale scale(window)
  window dashboard
    size 800 600
extern crate::backend
  pure label(id:window-id) -> str
  pure scale(id:window-id) -> f64
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
on opened(id)
on open
  task window open dashboard -> opened _
on quit
  exit
component AgentWindow(id:window-id)
  emits
    open
    quit
  col
    text label(id)
    button "Open" -> emit(open)
    button "Quit" -> emit(quit)
view
  AgentWindow id=window
    events
      open -> open
      quit -> quit
"#;
    let generated = compile(source, "agent.ice").unwrap();
    assert!(generated.contains("::iced::daemon(Self::__boot, Self::__update, Self::__view)"));
    assert!(generated.contains(".title(Self::__title)"));
    assert!(generated.contains(".theme(Self::__theme)"));
    assert!(generated.contains(".scale_factor(Self::__scale_factor)"));
    assert!(
        generated
            .contains("fn __title(&self, window: ::iced::window::Id) -> ::std::string::String")
    );
    assert!(generated.contains("fn __theme(&self, window: ::iced::window::Id) -> ::iced::Theme"));
    assert!(generated.contains("fn __scale_factor(&self, window: ::iced::window::Id) -> f32"));
    assert!(generated.contains("fn __view(&self, window: ::iced::window::Id) -> __IceElement"));
    assert!(generated.contains("crate::backend::label(window)"));
    assert!(generated.contains("crate::backend::scale(window)"));
    assert!(generated.contains("return ::iced::exit::<__AgentMessage>();"));
    assert!(!generated.contains("::iced::application("));
    assert!(!generated.contains(".window("));
}

#[test]
fn lowers_complete_common_application_and_window_settings() {
    let source = r#"app Configured
  title "Configured app"
  theme "dark"
  bg "123456"
  fg "abcdef"
  id "dev.example.configured"
  executor iced::executor::Default
  font "fonts/Brand.ttf"
  font "fonts/Icons.otf"
  text-size 15
  antialiasing false
  vsync false
  scale 1.25
  window
    icon-rgba "assets/app.rgba" 2 1
    size 960 720
    maximized true
    fullscreen false
    position 10 -20
    min-size 480 360
    max-size 1920 1080
    visible true
    resizable false
    closeable false
    minimizable false
    decorations false
    transparent true
    blur true
    level always-on-top
    exit-on-close false
    platform linux
      app-id "dev.example.configured"
      override-redirect true
    platform windows
      drag-and-drop false
      skip-taskbar true
      undecorated-shadow true
      corner round-small
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true
    platform wasm
      target none
state
  ready = false
extern crate::backend
  task seed() -> bool
preset ready
  state
    ready = true
  boot
    task seed() -> seeded _
on seeded(value)
  ready = value
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
  text "Configured"
"#;
    let generated = compile(source, "configured.ice").unwrap();
    for expected in [
        ".title(Self::__title)",
        ".theme(Self::__theme).style(Self::__style)",
        "fn __title(&self) -> ::std::string::String",
        "fn __theme(&self) -> ::iced::Theme {\n::iced::Theme::Dark",
        "fn __style(&self, __theme: &::iced::Theme)",
        "parse::<::iced::Color>()",
        ".executor::<iced::executor::Default>()",
        ".presets([::iced::Preset::new(\"ready\", Self::__preset_0)])",
        "fn __preset_0()",
        "state_changed!(self.ready, __ice_next) { self.ready = __ice_next; self.__ice_rev[",
        "({ let __ice_call = ::ui_lang_runtime::dev::Span::extern_call(\"seed\", \"configured.ice:48\"); crate::backend::seed() }).map(|value| __ConfiguredMessage::Seeded(value))",
        "id: ::std::option::Option::Some(\"dev.example.configured\".to_owned())",
        ".font(include_bytes!(\"fonts/Brand.ttf\").as_slice())",
        ".font(include_bytes!(\"fonts/Icons.otf\").as_slice())",
        "default_text_size: ::iced::Pixels(15 as f32)",
        "antialiasing: false",
        "vsync: false",
        "size: ::iced::Size::new(960 as f32, 720 as f32)",
        "maximized: true",
        "fullscreen: false",
        "Position::Specific(::iced::Point::new(10 as f32, -20 as f32))",
        "min_size: ::std::option::Option::Some(::iced::Size::new(480 as f32, 360 as f32))",
        "max_size: ::std::option::Option::Some(::iced::Size::new(1920 as f32, 1080 as f32))",
        "visible: true",
        "resizable: false",
        "closeable: false",
        "minimizable: false",
        "decorations: false",
        "transparent: true",
        "blur: true",
        "level: ::iced::window::Level::AlwaysOnTop",
        "const __ICE_RGBA: &[u8] = include_bytes!(\"assets/app.rgba\")",
        "__ICE_RGBA.len() == 8",
        "__ICE_RGBA }.to_vec(), 2, 1)",
        "__ICE_RGBA.len() == 8, \"window icon RGBA byte length does not match width × height × 4\"",
        "exit_on_close_request: false",
        "__platform.application_id = \"dev.example.configured\".to_owned()",
        "__platform.override_redirect = true",
        "__platform.drag_and_drop = false",
        "__platform.skip_taskbar = true",
        "__platform.undecorated_shadow = true",
        "CornerPreference::RoundSmall",
        "__platform.title_hidden = true",
        "__platform.titlebar_transparent = true",
        "__platform.fullsize_content_view = true",
        "__platform.target = ::std::option::Option::None",
        "#[cfg(target_os = \"linux\")]",
        "#[cfg(target_os = \"windows\")]",
        "#[cfg(target_os = \"macos\")]",
        "#[cfg(target_arch = \"wasm32\")]",
        ".scale_factor(Self::__scale_factor)",
        "fn __scale_factor(&self) -> f32",
        ".max(f32::EPSILON).min(f32::MAX)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }

    let error = compile(
        &source.replace("ready = true", "ready = 1"),
        "configured.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E101");

    for (from, to, expected) in [
        ("title \"Configured app\"", "title ready", "expected `str`"),
        ("theme \"dark\"", "theme \"unknown\"", "unknown iced theme"),
        ("bg \"123456\"", "bg \"not-a-color\"", "hexadecimal"),
        ("scale 1.25", "scale 0", "greater than zero"),
        ("scale 1.25", "scale 3.5e38", "scale"),
    ] {
        let error = compile(&source.replace(from, to), "configured.ice").unwrap_err();
        assert!(error.message.contains(expected), "{error:?}");
    }
}

#[test]
fn emits_a_probe_for_every_extern_function() {
    let source = r#"app Demo
extern crate::backend
  Item(id:i64)
  AppError(message:str)
  load(id:i64) -> [Item] ! AppError
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
on mount
  return if false
  run every load(1) -> loaded _ | failed _
on loaded(next)
  items = next
on failed(error)
  items = []
view
  text len(items)
"#;
    let generated = compile(source, "demo.ice").unwrap();
    assert!(generated.contains("async fn __ui_lang_check_future_load"));
    assert!(generated.contains("crate::backend::load(arg0).await"));
    assert!(generated.contains("let task = (||"));
}

#[test]
fn keeps_extern_struct_and_future_probe_names_distinct() {
    let generated = compile(
        "app Demo\nextern crate::backend\n  Load()\n  load() -> unit\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"ok\"\n",
        "probes.ice",
    )
    .unwrap();

    assert!(generated.contains(
        "#[allow(dead_code)]\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub(crate) enum AppTheme"
    ));
    assert!(generated.contains("fn __ui_lang_check_Load"));
    assert!(generated.contains("#[allow(dead_code, non_snake_case)] fn __ui_lang_check_Load"));
    assert!(generated.contains("async fn __ui_lang_check_future_load"));
}

#[test]
fn lowers_accessibility_into_the_runtime_bridge() {
    let source = r#"app Accessible
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
  name = ""
  checked = false
on press
on toggle(value)
view
  col
    text 42
    input "Name" #name label="Full name" description="Profile name" <-> name
    button "Save" #save description="Save changes" -> press
    checkbox "Ready" #ready label="Ready state" checked=checked -> toggle _
    image "photo.ppm" label="Portrait" description="Profile portrait"
"#;
    let generated = compile(source, "accessible.ice").unwrap();
    for expected in [
        "::ui_lang_runtime::Bridge<__AccessibleMessage>",
        "::ui_lang_runtime::snapshot::<__AccessibleMessage>(\"Accessible\")",
        "::ui_lang_runtime::navigation(",
        // Checkbox and image are not modelled, so they keep their generated
        // semantics; text, input, and button are published as data and the
        // renderer assigns their roles.
        "::ui_lang_runtime::Role::CheckBox",
        "::ui_lang_runtime::Role::Image",
        ".description(\"Profile portrait\".to_owned())",
        ".chain(::ui_lang_runtime::snapshot",
        "let __refresh = matches!(__request.action, ::ui_lang_runtime::Action::Focus)",
        "__AccessibilityNativeWindow(::ui_lang_runtime::NativeWindow)",
        "__window.visible = false",
        "__window.maximized = false",
        "__window.fullscreen = false",
        "::iced::window::oldest().then",
        "#[cfg(all(target_os = \"windows\", not(test)))]",
        "#[cfg(not(all(target_os = \"windows\", not(test))))]\n{\nlet task = state.__boot_task();",
        "::ui_lang_runtime::native_window(__id)",
        "!self.__ice_accessibility.is_attached()",
        "self.__ice_accessibility_pending.push(message)",
        "if !self.__ice_accessibility.attach_window(__window)",
        "__pending.push(self.__update(__message))",
        "__restore.chain(::iced::Task::batch([__initial, __pending, __snapshot]))",
        "::iced::window::Mode::Windowed",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
    assert!(!generated.contains("dispatch(__request).chain"));
    assert!(!generated.contains("claim_window"));

    // The one modelled widget here carries its accessibility contract in the
    // published data. The input and button are not modelled, because explicit
    // `label=`/`description=` have no template field yet, so they keep their
    // generated semantics.
    assert!(generated.contains(r#"\"kind\": \"text\""#));
    assert!(generated.contains("::ui_lang_runtime::Role::TextInput"));
    assert!(generated.contains("::ui_lang_runtime::Role::Button"));
    assert!(generated.contains(".label(\"Full name\".to_owned())"));
}

#[test]
fn defers_windows_show_state_until_native_adapter_setup() {
    let source = |window: &str| {
        format!(
            "app Accessible\n  window\n    {window}\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Ready\"\n"
        )
    };

    let fullscreen = compile(
        &source("fullscreen true\n    maximized true"),
        "accessible.ice",
    )
    .unwrap();
    assert!(fullscreen.contains("__window.maximized = false"));
    assert!(fullscreen.contains("__window.fullscreen = false"));
    assert!(fullscreen.contains("::iced::window::Mode::Fullscreen"));
    assert!(!fullscreen.contains("::iced::window::maximize(__id, true).chain"));

    let maximized = compile(&source("maximized true"), "accessible.ice").unwrap();
    assert!(maximized.contains(
        "::iced::window::set_mode(__id, ::iced::window::Mode::Windowed).chain(::iced::window::maximize(__id, true))"
    ));

    let hidden = compile(
        &source("visible false\n    fullscreen true\n    maximized true"),
        "accessible.ice",
    )
    .unwrap();
    assert!(hidden.contains("let __restore = ::iced::Task::none();"));
    assert!(!hidden.contains("::iced::window::set_mode(__id, ::iced::window::Mode::Fullscreen)"));
    assert!(!hidden.contains("::iced::window::maximize(__id, true)"));
}

#[test]
fn restores_only_the_initial_primary_window() {
    let source = r#"app Accessible
  window
    fullscreen true
  window child
    maximized true
    visible false
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
  text "Ready"
"#;
    let generated = compile(source, "accessible.ice").unwrap();
    let named = generated
        .split_once("fn __window_0()")
        .unwrap()
        .1
        .split_once("fn __program()")
        .unwrap()
        .0;

    assert!(named.contains("maximized: true"));
    assert!(named.contains("visible: false"));
    assert!(!named.contains("__window.visible = false"));
    assert!(generated.contains("::iced::window::oldest().then"));
    assert!(generated.contains(
        "let __restore = ::iced::window::set_mode(__id, ::iced::window::Mode::Fullscreen);"
    ));
    assert!(!generated.contains("claim_window"));
    assert!(!generated.contains("::iced::window::Event::Opened"));
}

#[test]
fn exposes_the_generated_program_to_in_crate_test_harnesses() {
    let generated = compile(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Ready\"\n",
        "app.ice",
    )
    .unwrap();

    assert!(generated.contains("fn __program() -> ::iced::Application<impl ::iced::Program"));
    assert!(generated.contains("Self::__program().run()"));
}

#[test]
fn waits_for_windows_attachment_before_boot_and_replays_messages_in_order() {
    let source = r#"extern crate::backend
  stream load() -> bool
app Accessible
state
  ready = false
preset seeded
  boot
    stream every load() -> loaded _
on mount
  stream every load() -> loaded _
on loaded(value)
  ready = value
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
  text "Ready"
"#;
    let generated = compile(source, "accessible.ice").unwrap();

    for expected in [
        "fn __boot_task(&mut self)",
        "fn __preset_task_0(&mut self)",
        "state.__ice_accessibility_initial = ::std::option::Option::Some(0)",
        "state.__ice_accessibility_initial = ::std::option::Option::Some(1)",
        "#[cfg(not(all(target_os = \"windows\", not(test))))]\n{\nlet task = state.__preset_task_0();",
        "!matches!(&message, __AccessibleMessage::__AccessibilityNativeWindow(_))",
        "::std::mem::take(&mut self.__ice_accessibility_pending)",
        "for __message in ::std::mem::take(&mut self.__ice_accessibility_pending)",
        "__pending.push(self.__update(__message))",
        "let __initial = self.__accessibility_initial_task();",
        "::iced::Task::run",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
    assert!(!generated.contains("claim_window"));
}

#[test]
fn snapshots_after_handlers_that_return_tasks_early() {
    let source = r#"extern crate::backend
  save() -> unit
app Accessible
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
on press
  return if false
  run every save() -> saved
on saved
view
  button "Save" -> press
"#;
    let generated = compile(source, "accessible.ice").unwrap();

    assert!(generated.contains("__AccessibleMessage::Press => (|| {"));
    assert!(generated.contains("if false { return ::iced::Task::none(); }"));
    assert!(generated.contains("return ::iced::Task::perform"));
    // The snapshot rides EVERY update — including handlers that return a
    // task early — but only behind the activation gate: with no assistive
    // technology attached it is a whole-tree walk nobody reads.
    assert!(generated.contains(
        "let __accessibility = if cfg!(test) || ::ui_lang_runtime::accessibility_active()"
    ));
    assert!(generated.contains("::ui_lang_runtime::snapshot::<__AccessibleMessage>"));
    assert!(generated.contains("::iced::Task::batch([__task, __accessibility])"));
}

#[test]
fn clones_handler_parameters_at_every_owned_use() {
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
  latch = ""
  echo = ""
  count = 0
  cursor = 0
on pick(value, times)
  latch = value
  echo = value
  count = times
  cursor = times
view
  button "one" -> pick("x", 2)
"#;
    let generated = compile(source, "params.ice").unwrap();

    // A parameter is a Rust binding: the first owned use would MOVE it, so
    // both reads create owned values. Without this the generated Rust fails borrowck at the
    // `include_app!` line, where no span points back at the `.ice` source.
    assert!(generated.contains("let __ice_next = value.to_owned(); if ::ui_lang_runtime::state_changed!(self.latch, __ice_next) { self.latch = __ice_next; self.__ice_rev["));
    assert!(generated.contains("let __ice_next = value.to_owned(); if ::ui_lang_runtime::state_changed!(self.echo, __ice_next) { self.echo = __ice_next; self.__ice_rev["));
    // Copy parameters keep the bare read — cloning those is pure noise.
    assert!(generated.contains("let __ice_next = times; if ::ui_lang_runtime::state_changed!(self.count, __ice_next) { self.count = __ice_next; self.__ice_rev["));
    assert!(generated.contains(
        "let __ice_next = times; if ::ui_lang_runtime::state_changed!(self.cursor, __ice_next)"
    ));
}

/// The theme boilerplate every tray fixture needs, so the assertions below
/// are only about the tray.
const TRAY_TAIL: &str = r#"extern crate::backend
  pure describe(value:i64) -> str
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
  count = 1
on quit
  exit
view
  text count
"#;

/// "A tray installs no event source a program did not declare" — made
/// falsifiable. A status item is declared, and without a `menu` there is
/// nothing to press, so the program subscribes to nothing.
#[test]
fn a_tray_without_menu_installs_no_subscription() {
    let source = format!(
        r#"app Status
  tray
    icon-rgba "assets/tray.rgba" 2 2
    label describe(count)
{TRAY_TAIL}"#
    );
    let generated = compile(&source, "status.ice").unwrap();

    assert!(generated.contains("::ui_lang_runtime::tray::init("));
    assert!(!generated.contains("::ui_lang_runtime::tray::events()"));
}

/// And with a routed row it subscribes to exactly one thing: the rows.
#[test]
fn a_menu_tray_subscribes_to_its_rows() {
    let source = format!(
        r#"app Status
  tray
    icon-rgba "assets/tray.rgba" 2 2
    menu
      "Quit" -> quit
{TRAY_TAIL}"#
    );
    let generated = compile(&source, "status.ice").unwrap();

    assert!(generated.contains("::ui_lang_runtime::tray::events().filter_map("));
}

/// The literal hoist: a tray of nothing but constants is applied once at
/// startup and never syncs. A constant cannot become stale.
#[test]
fn a_tray_with_only_literals_emits_no_tray_sync() {
    let source = format!(
        r#"app Status
  tray
    icon-rgba "assets/tray.rgba" 2 2
    label "Status"
    tooltip "Ducktape"
    menu
      "Quit" -> quit
{TRAY_TAIL}"#
    );
    let generated = compile(&source, "status.ice").unwrap();

    assert!(!generated.contains("fn __tray_sync(&self)"));
    assert_eq!(
        generated
            .matches("::ui_lang_runtime::tray::set_tooltip")
            .count(),
        1
    );
    assert!(generated.contains(r#"::ui_lang_runtime::tray::set_tooltip("Ducktape");"#));
    assert!(generated.contains(r#"::ui_lang_runtime::tray::set_item(0usize, "Quit");"#));
}

/// The status item shows the program's first answer, not an empty label until
/// the first message. Syncing only at the end of `update` was the bug.
///
/// Placement, not just presence: a sync beside `init` reads `__state()`, which
/// for a preset is never what the preset says, so the first sync has to come
/// after the initial task that applies it.
#[test]
fn a_reactive_tray_syncs_after_the_state_each_entry_point_starts_from() {
    let source = format!(
        r#"app Status
  tray
    icon-rgba "assets/tray.rgba" 2 2
    label describe(count)
preset loaded
  state
    count = 7
{TRAY_TAIL}"#
    );
    let generated = compile(&source, "status.ice").unwrap();

    for (name, applies_the_state) in [
        ("fn __boot()", "state.__boot_task();"),
        ("fn __preset_0()", "state.__preset_task_0();"),
    ] {
        let start = generated
            .find(name)
            .unwrap_or_else(|| panic!("generated source has no `{name}`"));
        let body = &generated[start + name.len()..];
        let body = &body[..body.find("\nfn ").unwrap_or(body.len())];
        let applied = body.find(applies_the_state).unwrap_or_else(|| {
            panic!("`{name}` never runs `{applies_the_state}`");
        });
        let synced = body.find("state.__tray_sync();").unwrap_or_else(|| {
            panic!(
                "`{name}` never syncs the tray, so the item shows nothing until the first message"
            );
        });
        assert!(
            synced > applied,
            "`{name}` syncs the tray before `{applies_the_state}`, so the item shows the declared defaults instead of what this entry point starts from"
        );
    }

    let start = generated
        .find("fn __update(")
        .expect("generated `__update`");
    let body = &generated[start..];
    assert!(
        body[..body.find("\nfn ").unwrap_or(body.len())].contains("self.__tray_sync();"),
        "`__update` never syncs the tray, so nothing a handler changes reaches the status item"
    );
}

/// A guard is the one thing that makes a menu of literals reactive: the row's
/// presence is what changes, and it is synced at the row's own index.
#[test]
fn a_guarded_row_syncs_its_visibility() {
    let source = format!(
        r#"app Status
  tray
    icon-rgba "assets/tray.rgba" 2 2
    menu
      "Quit" -> quit
      separator
      "Reset" -> reset when count > 1
on reset
  count = 1
{TRAY_TAIL}"#
    );
    let generated = compile(&source, "status.ice").unwrap();

    assert!(generated.contains("fn __tray_sync(&self)"));
    assert!(generated.contains("::ui_lang_runtime::tray::set_visible(2usize, "));
    assert!(!generated.contains("::ui_lang_runtime::tray::set_visible(0usize, "));
    assert!(!generated.contains("::ui_lang_runtime::tray::set_visible(1usize, "));
}

/// Row indices are declaration indices, so a separator simply has no line and
/// no later row shifts under the runtime's row vector.
#[test]
fn menu_row_indices_include_separators() {
    let source = format!(
        r#"app Status
  tray
    icon-rgba "assets/tray.rgba" 2 2
    menu
      describe(count)
      separator
      describe(count)
{TRAY_TAIL}"#
    );
    let generated = compile(&source, "status.ice").unwrap();

    assert!(generated.contains("::ui_lang_runtime::tray::set_item(0usize,"));
    assert!(generated.contains("::ui_lang_runtime::tray::set_item(2usize,"));
    assert!(!generated.contains("::ui_lang_runtime::tray::set_item(1usize,"));
    assert!(generated.contains("::ui_lang_runtime::tray::TrayRow::Separator,"));
}

/// The tray contributes no state at all. Every field the popover needed is
/// gone with the window it tracked.
#[test]
fn a_tray_contributes_no_private_state() {
    let source = format!(
        r#"app Status
  tray
    icon-rgba "assets/alarm.rgba" 2 2 when count > 3
    icon-rgba "assets/tray.rgba" 2 2
    label describe(count)
    menu
      describe(count)
      "Quit" -> quit
{TRAY_TAIL}"#
    );
    let generated = compile(&source, "status.ice").unwrap();

    assert!(!generated.contains("__ice_tray"));
    // One guard per guarded icon; the runtime owns first-match-wins and the
    // fall back to the unguarded last icon.
    assert!(generated.contains("::ui_lang_runtime::tray::select_icon(&[(self.count > 3)]);"));
    assert!(generated.contains("::ui_lang_runtime::tray::events().filter_map("));
    assert!(generated.contains("1usize => ::std::option::Option::Some(__StatusMessage::Quit),"));
}

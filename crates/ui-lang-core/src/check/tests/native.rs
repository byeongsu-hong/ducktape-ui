use super::*;

#[test]
fn checks_native_alignment_values_and_hashing() {
    let source = example!("alignment.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "to_vertical = vertical.from_alignment(alignment_round_trip(end))",
        "to_vertical = vertical.from_alignment(right)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `alignment`"));
}

#[test]
fn checks_native_shadow_values_and_fields() {
    let source = example!("shadow.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "shadow.new(color.rgba(0.1, 0.2, 0.3, 0.4), vector(4.0, 8.0), 12.0)",
        "shadow.new(true, vector(4.0, 8.0), 12.0)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `color`"));
}

#[test]
fn checks_native_border_and_radius_values() {
    let source = example!("border_radius.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "border.new(color.rgba(0.1, 0.2, 0.3, 0.4), pixels(2.0), radius(3.0))",
        "border.new(color.rgba(0.1, 0.2, 0.3, 0.4), true, radius(3.0))",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `f64` or `pixels`"));

    let error = analyze(&source.replace("radius.from_u8(10)", "radius.from_u8(256)")).unwrap_err();
    assert_eq!(error.code, "E152");
    assert!(error.message.contains("0..=255"));

    let error = analyze(&source.replace(
        "radii_equal = built_radius == returned_radius",
        "radii_equal = built_radius < returned_radius",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("does not accept `radius`"));
}

#[test]
fn checks_native_background_and_gradient_values() {
    let source = example!("background_gradient.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace("linear(radians(0.75))", "linear(true)")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `f64` or `radians`"));

    let error = analyze(&source.replace(
        "backgrounds_equal = from_linear_background == returned_background",
        "backgrounds_equal = from_linear_background < returned_background",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("does not accept `background`"));
}

#[test]
fn checks_native_font_values_and_static_names() {
    let source = example!("font_values.ice");
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("font.with_name(\"Inter\")", "font.with_name(family_kind)"))
            .unwrap_err();
    assert_eq!(error.code, "E152");
    assert!(error.message.contains("expects one string literal"));

    let error = analyze(&source.replace(
        "fonts_equal = custom_font == returned_font",
        "fonts_equal = custom_font < returned_font",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("does not accept `font`"));
}

#[test]
fn checks_native_theme_mode_values_and_traits() {
    let source = example!("theme_mode.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "values_equal = returned == theme_mode.dark()",
        "values_equal = returned < theme_mode.dark()",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("does not accept `theme-mode`"));

    let error = analyze(&source.replace(
        "    button \"Inspect\" -> inspect",
        "    lazy returned as cached\n      text cached.kind",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("does not implement stable hashing"));
}

#[test]
fn checks_native_text_values_and_traits() {
    let source = example!("text_values.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "values_equal = returned_alignment == text_alignment.right()",
        "values_equal = returned_alignment < text_alignment.right()",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("does not accept `text-alignment`"));

    let error = analyze(&source.replace(
        "relative_height = line_height.relative(1.5)",
        "relative_height = line_height.relative(true)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `f64`, got `bool`"));
}

#[test]
fn checks_native_mouse_interaction_and_widget_passage() {
    let source = example!("mouse_interaction.ice");
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("mouse cursor=(returned)", "mouse cursor=(kind)")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `mouse-interaction`"));

    let error = analyze(&source.replace(
        "    button \"Inspect\" -> inspect",
        "    lazy returned as cached\n      text cached.kind",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("does not implement stable hashing"));
}

#[test]
fn checks_native_scroll_delta_values() {
    let source = example!("scroll_delta.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace("scroll.lines(1.5, -2.25)", "scroll.lines(true, -2.25)"))
        .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `f64`"));

    let error = analyze(&source.replace(
        "values_equal = returned == pixels",
        "values_equal = returned < pixels",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("does not accept `scroll-delta`"));

    let error = analyze(&source.replace(
        "    button \"Inspect\" -> inspect",
        "    lazy returned as cached\n      text cached.kind",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("does not implement stable hashing"));
}

#[test]
fn checks_native_window_value_traits() {
    let source = example!("window_values.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "levels_equal = returned_level == window_level.always_on_top()",
        "levels_equal = returned_direction == window_direction.south_west()",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("window-direction"));
    assert!(error.message.contains("do not support comparisons"));

    let error = analyze(&source.replace(
        "levels_equal = returned_level == window_level.always_on_top()",
        "levels_equal = returned_level < window_level.always_on_top()",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("does not accept `window-level`"));

    let error = analyze(&source.replace(
        "    button \"Inspect\" -> inspect",
        "    lazy returned_mode as cached\n      text cached.kind",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("does not implement stable hashing"));
}

#[test]
fn checks_native_window_position_and_callback_boundary() {
    let source = example!("window_position.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "window_position.specific(point(24.0, -12.0))",
        "window_position.specific(true)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `point`"));

    let error = analyze(&source.replace(
        "default_kind = default_position.kind",
        "default_kind = returned == specific_position",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("window-position"));
    assert!(error.message.contains("do not support comparisons"));

    let error = analyze(&source.replace(
        "    button \"Inspect\" -> inspect",
        "    lazy responsive as cached\n      text cached.kind",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("does not implement stable hashing"));
}

#[test]
fn checks_native_event_status_values_and_traits() {
    let source = example!("event_status.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "event_status.merge(ignored, captured)",
        "event_status.merge(true, captured)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `event-status`"));

    let error = analyze(&source.replace(
        "values_equal = returned == captured",
        "values_equal = returned < captured",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("does not accept `event-status`"));

    let error = analyze(&source.replace(
        "    button \"Inspect\" -> inspect",
        "    lazy returned as cached\n      text cached.kind",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("does not implement stable hashing"));
}

#[test]
fn checks_native_redraw_request_values_and_traits() {
    let source = example!("redraw_request.ice");
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("redraw_request.at(redraw_now())", "redraw_request.at(true)"))
            .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `instant`"));

    let error = analyze(&source.replace(
        "    button \"Inspect\" -> inspect",
        "    lazy returned as cached\n      text cached.kind",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("does not implement stable hashing"));
}

#[test]
fn checks_native_window_id_values_and_traits() {
    let source = example!("window_id.ice");
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("window_id.unique()", "window_id.unique(true)")).unwrap_err();
    assert_eq!(error.code, "E152");
    assert!(error.message.contains("expects 0 argument"));
}

#[test]
fn checks_native_window_screenshot_values_and_routes() {
    let source = example!("window_screenshot.ice");
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[2].params[0].ty, Type::WindowScreenshot);

    let error = analyze(&source.replace(
        "scale_factor = returned.scale_factor",
        "scale_factor = sample == returned",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("do not support comparisons"));

    let error = analyze(&source.replace(
        "    button \"Inspect\" -> inspect",
        "    lazy sample as cached\n      text cached.debug",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("does not implement stable hashing"));

    let error = analyze(&source.replace(
        "task window screenshot -> native_captured _",
        "task window screenshot -> native_captured _ _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E133");
    assert!(error.message.contains("expects 1 arguments, got 2"));

    let error = analyze(&source.replace(
        "screenshot.crop(sample, screenshot_crop_region())",
        "screenshot.crop(sample, sample.size)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `rectangle-u32`"));
}

#[test]
fn checks_native_length_values_and_widget_passage() {
    let source = example!("length.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "portion_length = length.fill_portion(3)",
        "portion_length = length.fill_portion(65536)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E152");
    assert!(error.message.contains("0..=65535"));

    let error = analyze(&source.replace("w=fill_length", "w=true")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `f64` or `length`"));

    let error = analyze(&source.replace("w=96.0", "w=round_trip")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `f64`"));
    assert!(error.message.contains("got `length`"));
}

#[test]
fn checks_native_color_values_and_boundaries() {
    let source = example!("color.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "rgb8 = color.rgb8(12, 34, 56)",
        "rgb8 = color.rgb8(256, 34, 56)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E152");
    assert!(error.message.contains("channels must be in 0..=255"));

    let error = analyze(&source.replace(
        "rgb = color.rgb(0.25, 0.5, 0.75)",
        "rgb = color.rgb(1.1, 0.5, 0.75)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("color channel"));

    let error = analyze(&source.replace(
        "contrast = color.contrast(black, white)",
        "contrast = color.contrast(black, true)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `color`"));
}

#[test]
fn checks_native_content_fit_values_and_widgets() {
    let source = example!("content_fit.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "fit.apply(contain_fit, size(100.0, 50.0), size(80.0, 80.0))",
        "fit.apply(contain_fit, true, size(80.0, 80.0))",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `size`"));

    let error = analyze(&source.replace("fit=round_trip", "fit=true")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `content-fit`"));
}

#[test]
fn checks_native_rotation_values_and_widgets() {
    let source = example!("rotation.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace("rotation.solid(radians(0.5))", "rotation.solid(true)"))
        .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `radians`"));

    let error = analyze(&source.replace("rotate=solid_rotation", "rotate=true")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `rotation`"));
}

#[test]
fn checks_owned_native_debug_timing_boundaries() {
    let source = example!("debug_timing.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replace("timer:debug-span?", "timer:str?")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("debug-span?"));

    let error = analyze(&source.replace("label = \"interaction\"", "label = 1")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `str`"));

    let error = analyze(&source.replace("timer:debug-span?", "timer:[debug-span]")).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("must have type `debug-span?`"));

    let error = analyze(&source.replace("debug finish timer", "timer = none")).unwrap_err();
    assert_eq!(error.code, "E144");
    assert!(error.message.contains("`debug start` and `debug finish`"));

    let error = analyze(&source.replace("on begin", "on begin(span)").replace(
        "button \"Begin\" -> begin",
        "button \"Begin\" -> begin timer",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E135");
    assert!(error.message.contains("cannot cross a handler route"));

    let error = analyze(
        &source
            .replace("measured = 0", "measured = 0\n  active = false")
            .replace(
                "measured = debug.time_with(\"compute\", value + 1)",
                "active = timer == none",
            ),
    )
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("debug spans are opaque"));
}

#[test]
fn checks_native_image_allocation_results_and_errors() {
    let source = example!("image_allocation.ice");
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty, Type::ImageAllocation);
    assert_eq!(document.handlers[2].params[0].ty, Type::ImageError);

    let error = analyze(&source.replace(" | failed _", "")).unwrap_err();
    assert_eq!(error.code, "E131");
    assert!(error.message.contains("requires an error route"));

    let error = analyze(&source.replace("allocate handle", "allocate width")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `image`"));
}

#[test]
fn rejects_invalid_animation_boundaries_before_codegen() {
    let source = r#"app Motion
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
  label:animation[str] = ""
view
  text "Motion"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("supports `bool`, `f64`"));

    let error = analyze(&source.replace(
        "label:animation[str] = \"\"",
        "label:animation[f64] = 3.5e38",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("animation value"));

    let source = source
        .replace("label:animation[str] = \"\"", "label = \"\"")
        .replace(
            "view",
            "on change\n  label = \"next\" at instant.now()\nview",
        );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(
        error
            .message
            .contains("only valid when assigning animation")
    );
}

#[test]
fn checks_exit_is_a_final_native_task() {
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
on quit
  exit
  ready = true
state
  ready = false
view
  button "Quit" -> quit
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E141");
    assert!(error.message.contains("exit must be the final statement"));

    analyze(&source.replace("  exit\n  ready = true", "  exit")).unwrap();
}

#[test]
fn exposes_the_current_window_only_to_daemon_views_and_callbacks() {
    let source = r#"daemon Agent
  title label(window)
  scale scale(window)
extern crate::backend
  sync label(id:window-id) -> str
  sync scale(id:window-id) -> f64
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
component WindowBody(id:window-id)
  text "Window"
view
  WindowBody id=window
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "component WindowBody(id:window-id)",
        "state\n  window:window-id? = none\ncomponent WindowBody(id:window-id)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E100");
    assert!(error.message.contains("cannot be named `window`"));

    let error = analyze(&source.replace("daemon Agent", "app Agent")).unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.message.contains("unknown value `window`"));
}

#[test]
fn checks_native_timer_subscription() {
    let source = example!("timer.ice");
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty, Type::Instant);
    assert_eq!(document.handlers[2].params[0].ty, Type::I64);
    assert_eq!(document.handlers[2].params[1].ty, Type::I64);
    assert_eq!(document.handlers[3].params[0].ty, Type::I64);
    assert_eq!(document.handlers[3].params[1].ty, Type::Str);
    assert_eq!(document.handlers[4].params[0].ty, Type::Bool);

    let error = analyze(&source.replace(
        "every 250ms when auto_refresh -> tick _",
        "every 250ms when auto_refresh -> tick",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E133");
    assert!(error.message.contains("expects 1 arguments, got 0"));

    let error = analyze(&source.replace("refresh_time() -> i64", "refresh_time(seed:i64) -> i64"))
        .unwrap_err();
    assert_eq!(error.code, "E142");

    for invalid in ["0ms", "1m", "1.5s"] {
        let error = analyze(&source.replace("250ms", invalid)).unwrap_err();
        assert_eq!(error.code, "E084");
    }

    let error = analyze(&source.replace("when auto_refresh", "when 1")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `bool`"));

    let error = analyze(&source.replace(
        "every 250ms when auto_refresh",
        "every 250ms status=captured when auto_refresh",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(
        error
            .message
            .contains("only available on non-frame runtime events")
    );

    let error = analyze(&source.replace(
        "sync even_refresh(value:i64) -> i64?",
        "sync even_refresh(value:i64, extra:i64) -> i64?",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E142");
    assert!(error.message.contains("expects 2 payloads, got 1"));

    let error = analyze(&source.replace(
        "sync even_refresh(value:i64) -> i64?",
        "sync even_refresh(value:i64) -> i64",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E142");
    assert!(error.message.contains("must return an optional value"));

    let error = analyze(&source.replace("with=generation", "with=1.5")).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("context must be hashable"));
}

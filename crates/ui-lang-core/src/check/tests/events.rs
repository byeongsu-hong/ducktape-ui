use super::*;

#[test]
fn checks_native_theme_factories() {
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
    analyze(source).unwrap();

    let error = analyze(&source.replace("native_theme(dark)", "native_theme(1)")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `bool`"));

    let error = analyze(&source.replace("native_theme(!dark)", "missing(!dark)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("theme factory"));
}

#[test]
fn checks_alternate_theme_subtrees() {
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
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("alternate_panel(active)", "alternate_panel(1)")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `bool`"));

    let error = analyze(&source.replace(" -> changed _", "")).unwrap_err();
    assert_eq!(error.code, "E126");
    assert!(error.message.contains("requires a route"));

    let error =
        analyze(&source.replace("themer alternate_panel(active)", "themer missing(active)"))
            .unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("unknown extern themer `missing`"));
}

#[test]
fn checks_generic_event_values_and_filters() {
    let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
  sync event_label(value:event) -> str?
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
  last = "none"
  last_window:window-id? = none
on received(value)
  last = event_name(value)
on labeled(value)
  last = value
on identified(id, value)
  last_window = some(id)
  last = event_name(value)
subscribe
  event -> received _
  event filter=event_label status=any -> labeled _
  event raw with-id status=captured -> identified _ _
view
  text last
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[0].params[0].ty,
        Type::Event
    );
    assert_eq!(
        document.source_document().handlers[1].params[0].ty,
        Type::Str
    );
    assert_eq!(
        document.source_document().handlers[2].params[0].ty,
        Type::WindowId
    );
    assert_eq!(
        document.source_document().handlers[2].params[1].ty,
        Type::Event
    );

    let error = analyze(&source.replace(
        "sync event_label(value:event) -> str?",
        "sync event_label(value:str) -> str?",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `str`, got `event`"));
}

#[test]
fn checks_all_native_input_method_subscription_payloads() {
    let source = example!("input_method_events.ice");
    let document = analyze(source).unwrap();
    let preedit = document
        .source_document()
        .handlers
        .iter()
        .find(|handler| handler.name == "preedit")
        .unwrap();
    assert_eq!(
        preedit
            .params
            .iter()
            .map(|param| param.ty.display())
            .collect::<Vec<_>>(),
        ["str", "i64?", "i64?"]
    );

    let error = analyze(&source.replace(
        "input-method preedit status=any -> preedit _ _ _",
        "input-method preedit status=any -> preedit _ _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("expects 3 payloads"));

    let error = analyze(&source.replace(
        "input-method closed -> closed",
        "input-method disabled -> closed",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("input-method event must be"));
}

#[test]
fn checks_all_native_mouse_subscription_payloads() {
    let source = example!("mouse_events.ice");
    let document = analyze(source).unwrap();
    let handlers = document
        .source_document()
        .handlers
        .iter()
        .map(|handler| {
            (
                handler.name.as_str(),
                handler
                    .params
                    .iter()
                    .map(|param| param.ty.display())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(handlers["entered"], Vec::<String>::new());
    assert_eq!(handlers["left"], Vec::<String>::new());
    assert_eq!(handlers["moved"], ["f64", "f64"]);
    assert_eq!(handlers["pressed"], ["mouse-button"]);
    assert_eq!(handlers["released"], ["mouse-button"]);
    assert_eq!(handlers["wheel"], ["f64", "f64", "bool"]);

    let error = analyze(&source.replace(
        "mouse moved status=captured -> moved _ _",
        "mouse moved status=captured -> moved _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("expects 2 payloads"));

    let error = analyze(&source.replace(
        "mouse wheel -> wheel _ _ _",
        "mouse wheel -> wheel 1.0 2.0 true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error =
        analyze(&source.replace("mouse left -> left", "mouse dragged -> left")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("mouse event must be"));

    let error = analyze(&source.replace("status=captured", "status=handled")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("status must be"));
}

#[test]
fn checks_all_native_touch_subscription_payloads() {
    let source = example!("touch_events.ice");
    let document = analyze(source).unwrap();
    for handler in &document.source_document().handlers {
        assert_eq!(
            handler
                .params
                .iter()
                .map(|param| param.ty.display())
                .collect::<Vec<_>>(),
            ["touch-finger", "f64", "f64"]
        );
    }

    let error = analyze(&source.replace("touch moved -> moved _ _ _", "touch moved -> moved _ _"))
        .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("expects 3 payloads"));

    let error = analyze(&source.replace("touch lost -> lost _ _ _", "touch ended -> lost _ _ _"))
        .unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("touch event must be"));
}

#[test]
fn checks_typed_pointer_values() {
    let source = example!("pointer_values.ice");
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0].ty,
        Type::MouseButton
    );
    assert_eq!(
        document.source_document().handlers[2].params[0].ty,
        Type::TouchFinger
    );

    for (from, to, code, message) in [
        (
            "mouse.button(\"left\")",
            "mouse.button(\"side\")",
            "E152",
            "must be left, right, middle, back, or forward",
        ),
        (
            "mouse.other_button(9)",
            "mouse.other_button(65536)",
            "E152",
            "must be in 0..=65535",
        ),
        (
            "mouse.cursor(point(12.0, 24.0))",
            "mouse.cursor(true)",
            "E101",
            "expected `point`, got `bool`",
        ),
        (
            "mouse.button(\"left\"), none",
            "mouse.button(\"left\"), some(mouse.unavailable())",
            "E101",
            "expected `mouse-click?`, got `mouse-cursor?`",
        ),
        (
            "touch.finger(\"18446744073709551615\")",
            "touch.finger(\"18446744073709551616\")",
            "E152",
            "must contain a decimal u64",
        ),
        (
            "touch.finger(\"18446744073709551615\")",
            "touch.finger(\"+42\")",
            "E152",
            "must contain a decimal u64",
        ),
        (
            "cursor.kind",
            "cursor.missing",
            "E151",
            "has no field `missing`",
        ),
        (
            "cursor_levitating = mouse.cursor_is_levitating(mouse.cursor_levitate(cursor))",
            "cursor_levitating = click == click",
            "E153",
            "mouse-click values are opaque",
        ),
    ] {
        let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
        assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
        assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
    }
}

#[test]
fn checks_native_transformations() {
    let source = example!("transformation_values.ice");
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().states[0].ty,
        Type::Transformation
    );
    assert_eq!(document.source_document().states[6].ty, Type::Vector);
    assert_eq!(document.source_document().states[11].ty, Type::Size);

    for (from, to, code, message) in [
        (
            "transform.identity()",
            "transform.identity(1)",
            "E152",
            "expects 0 argument(s)",
        ),
        (
            "transform.orthographic(640, 480)",
            "transform.orthographic(640.0, 480)",
            "E152",
            "expects two integer literals",
        ),
        (
            "transform.orthographic(640, 480)",
            "transform.orthographic(4294967296, 480)",
            "E152",
            "dimensions must be in 0..=4294967295",
        ),
        (
            "transform.translate(10.0, 20.0)",
            "transform.translate(true, 20.0)",
            "E101",
            "expected `f64`, got `bool`",
        ),
        (
            "transform.scale(2.0))",
            "point(2.0, 2.0))",
            "E101",
            "expected `transformation`, got `point`",
        ),
        (
            "transform.point(point(1.0, 2.0), combined)",
            "transform.point(combined, point(1.0, 2.0))",
            "E101",
            "expected `point`, got `transformation`",
        ),
        (
            "translation = combined.translation",
            "translation = combined.missing",
            "E151",
            "has no field `missing`",
        ),
    ] {
        let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
        assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
        assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
    }
}

#[test]
fn checks_native_geometry_values() {
    let source = example!("geometry_values.ice");
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().functions[0].output,
        Type::RectangleU32
    );
    assert_eq!(
        document.source_document().functions[1].params[1].1,
        Type::PointU32
    );
    assert_eq!(
        document.source_document().functions[1].params[5].1,
        Type::Option(Box::new(Type::RectangleU32))
    );

    for (from, to, code, message) in [
        (
            "point.origin()",
            "point.origin(1)",
            "E152",
            "expects 0 argument(s)",
        ),
        ("point.origin()", "point(3.5e38, 0.0)", "E128", "point"),
        (
            "size.from_u32(640, 480)",
            "size.from_u32(point_distance, 480)",
            "E152",
            "expects two integer literals",
        ),
        (
            "size.from_u32(640, 480)",
            "size.from_u32(4294967296, 480)",
            "E152",
            "dimensions must be in 0..=4294967295",
        ),
        (
            "\"right\", \"bottom\"",
            "\"around\", \"bottom\"",
            "E152",
            "horizontal alignment must be left, center, or right",
        ),
        (
            "point_value = (point.origin() + vector(3.25, 4.75)) - vector.zero()",
            "point_value = point.origin() + point.origin()",
            "E153",
            "does not accept `point` and `point`",
        ),
        (
            "vector_value = ((-vector(1.0, 2.0)",
            "vector_value = ((-point(1.0, 2.0)",
            "E153",
            "negation expects i64, f64, or vector",
        ),
        (
            "snapped_x = snapped_point.x",
            "snapped_x = snapped_point.missing",
            "E151",
            "has no field `missing`",
        ),
        (
            "scaled_bounds = bounds * 2.0",
            "scaled_bounds = bounds / 2.0",
            "E153",
            "does not accept `rectangle` and `f64`",
        ),
        (
            "scaled_bounds = bounds * 2.0",
            "scaled_bounds = bounds * 3.5e38",
            "E128",
            "native arithmetic value",
        ),
        (
            "contains_point = rectangle.contains(bounds, point(20.0, 30.0))",
            "contains_point = point_value < point.origin()",
            "E153",
            "does not accept `point` and `point`",
        ),
    ] {
        let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
        assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
        assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
    }
}

#[test]
fn checks_native_padding_and_angles() {
    let source = example!("padding_angles.ice");
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().functions[0].params[0].1,
        Type::Pixels
    );
    assert_eq!(
        document.source_document().functions[0].params[1].1,
        Type::Padding
    );
    assert_eq!(
        document.source_document().functions[0].params[2].1,
        Type::Degrees
    );
    assert_eq!(
        document.source_document().functions[0].params[3].1,
        Type::Radians
    );

    for (from, to, code, message) in [
        (
            "pixels.from_u32(4294967295)",
            "pixels.from_u32(4294967296)",
            "E152",
            "value must be in 0..=4294967295",
        ),
        (
            "pixels.from_u32(4294967295)",
            "pixels.from_u32(pixel_value.value)",
            "E152",
            "expects one integer literal",
        ),
        (
            "all_padding = padding.all(5.0)",
            "all_padding = padding.all(true)",
            "E101",
            "expected `f64` or `pixels`, got `bool`",
        ),
        (
            "all_padding = padding.all(5.0)",
            "all_padding = padding.all(5.0, 6.0)",
            "E152",
            "expects one argument",
        ),
        (
            "padding_equal = direct_padding == padding(1.0, 2.0, 3.0, 4.0)",
            "padding_equal = direct_padding < padding(1.0, 2.0, 3.0, 4.0)",
            "E153",
            "does not accept `padding` and `padding`",
        ),
        (
            "pixel_value = ((((pixels(4.0) + pixels(2.0))",
            "pixel_value = ((((pixels(4.0) - pixels(2.0))",
            "E153",
            "does not accept `pixels` and `pixels`",
        ),
        (
            "degree_value = degrees(45.0) * 2.0",
            "degree_value = degrees(45.0) + degrees(2.0)",
            "E153",
            "does not accept `degrees` and `degrees`",
        ),
        (
            "radians_reverse = 2.0 * radians(1.5)",
            "radians_reverse = 2.0 + radians(1.5)",
            "E153",
            "does not accept `f64` and `radians`",
        ),
        (
            "radians(5.0) % radians(2.0)",
            "radians(5.0) % degrees(2.0)",
            "E153",
            "does not accept `radians` and `degrees`",
        ),
        (
            "rotated_size = size.rotate(size(10.0, 20.0), radians_value)",
            "rotated_size = size.rotate(size(10.0, 20.0), degree_value)",
            "E101",
            "expected `f64` or `radians`, got `degrees`",
        ),
        (
            "radians_equal_scalar = radians_value == 1.0",
            "radians_equal_scalar = 1.0 == radians_value",
            "E101",
            "expected `radians`, got `f64`",
        ),
        (
            "radians_display = radians_value.display",
            "radians_display = radians_value.missing",
            "E151",
            "has no field `missing`",
        ),
    ] {
        let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
        assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
        assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
    }
}

#[test]
fn checks_all_native_window_subscription_payloads() {
    let source = example!("window_events.ice");
    let document = analyze(source).unwrap();
    let opened = document
        .source_document()
        .handlers
        .iter()
        .find(|handler| handler.name == "opened")
        .unwrap();
    assert_eq!(
        opened
            .params
            .iter()
            .map(|param| param.ty.display())
            .collect::<Vec<_>>(),
        ["window-id", "f64?", "f64?", "f64", "f64"]
    );

    let error = analyze(&source.replace(
        "window moved with-id status=captured -> moved _ _ _",
        "window moved with-id status=captured -> moved _ _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("expects 3 payloads"));

    let error = analyze(&source.replace(
        "window resized with-id -> resized _ _ _",
        "window resized with-id -> resized 1.0 2.0 3.0",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error = analyze(&source.replace(
        "window frame when listen_frames -> frame",
        "window frame with-id when listen_frames -> frame _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("does not expose a window ID"));
}

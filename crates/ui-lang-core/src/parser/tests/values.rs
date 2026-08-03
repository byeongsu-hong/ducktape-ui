use super::*;

#[test]
fn parses_first_class_native_alignments() {
    let source = example!("alignment.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Alignment);
    assert_eq!(document.functions[1].params[0].1, Type::HorizontalAlignment);
    assert_eq!(document.functions[2].params[0].1, Type::VerticalAlignment);
    assert_eq!(document.states[0].ty, Type::Alignment);
}

#[test]
fn parses_first_class_native_shadow() {
    let source = example!("shadow.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Shadow);
    assert_eq!(document.functions[0].output, Type::Shadow);
    assert_eq!(document.states[0].ty, Type::Shadow);
    assert!(matches!(
        &document.handlers[0].statements[1],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "shadow.new"
    ));
}

#[test]
fn parses_first_class_native_border_and_radius() {
    let source = example!("border_radius.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Border);
    assert_eq!(document.functions[0].output, Type::Border);
    assert_eq!(document.functions[1].params[0].1, Type::Radius);
    assert_eq!(document.functions[1].output, Type::Radius);
    assert_eq!(document.states[0].ty, Type::Border);
    assert_eq!(document.states[9].ty, Type::Radius);
    assert!(
        document.handlers[0]
            .statements
            .iter()
            .any(|statement| matches!(
                statement,
                Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "border.new"
            ))
    );
}

#[test]
fn parses_first_class_native_background_and_gradient() {
    let source = example!("background_gradient.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Background);
    assert_eq!(document.functions[1].params[0].1, Type::Gradient);
    assert_eq!(document.functions[2].params[0].1, Type::LinearGradient);
    assert_eq!(document.functions[3].params[0].1, Type::ColorStop);
    assert_eq!(document.states[0].ty, Type::ColorStop);
    assert_eq!(document.states[6].ty, Type::LinearGradient);
    assert!(matches!(
        &document.handlers[0].statements[1],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "color_stop"
    ));
}

#[test]
fn parses_first_class_native_font_values() {
    let source = example!("font_values.ice");
    let document = parse(source).unwrap();
    for (function, expected) in document.functions.iter().zip([
        Type::Font,
        Type::FontFamily,
        Type::FontWeight,
        Type::FontStretch,
        Type::FontStyle,
    ]) {
        assert_eq!(function.params[0].1, expected);
        assert_eq!(function.output, expected);
    }
    assert_eq!(document.states[0].ty, Type::Font);
    assert_eq!(
        document.states[7].ty,
        Type::List(Box::new(Type::FontFamily))
    );
    assert!(matches!(
        &document.handlers[0].statements[4],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "font.new"
    ));
}

#[test]
fn parses_first_class_native_theme_mode() {
    let source = example!("theme_mode.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::ThemeMode);
    assert_eq!(document.functions[0].output, Type::ThemeMode);
    assert_eq!(document.states[0].ty, Type::ThemeMode);
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "theme_mode.default"
    ));
}

#[test]
fn parses_first_class_native_text_values() {
    let source = example!("text_values.ice");
    let document = parse(source).unwrap();
    for (function, expected) in document.functions.iter().zip([
        Type::TextAlignment,
        Type::TextShaping,
        Type::TextWrapping,
        Type::TextLineHeight,
    ]) {
        assert_eq!(function.params[0].1, expected);
        assert_eq!(function.output, expected);
    }
    assert_eq!(document.states[0].ty, Type::TextAlignment);
    assert_eq!(document.states[7].ty, Type::TextShaping);
    assert_eq!(document.states[11].ty, Type::TextWrapping);
    assert_eq!(document.states[15].ty, Type::TextLineHeight);
    assert!(matches!(
        &document.handlers[0].statements[1],
        Statement::Assign { value: Expr::List(values), .. } if values.len() == 4
    ));
}

#[test]
fn parses_first_class_native_mouse_interaction() {
    let source = example!("mouse_interaction.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::MouseInteraction);
    assert_eq!(document.functions[0].output, Type::MouseInteraction);
    assert_eq!(document.states[0].ty, Type::MouseInteraction);
    assert_eq!(
        document.states[2].ty,
        Type::List(Box::new(Type::MouseInteraction))
    );
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "interaction.default"
    ));
}

#[test]
fn parses_first_class_native_scroll_delta() {
    let source = example!("scroll_delta.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::ScrollDelta);
    assert_eq!(document.functions[0].output, Type::ScrollDelta);
    assert_eq!(document.states[0].ty, Type::ScrollDelta);
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "scroll.lines"
    ));
}

#[test]
fn parses_first_class_native_window_values() {
    let source = example!("window_values.ice");
    let document = parse(source).unwrap();
    for (function, expected) in document.functions.iter().zip([
        Type::WindowDirection,
        Type::WindowLevel,
        Type::WindowMode,
        Type::WindowAttention,
    ]) {
        assert_eq!(function.params[0].1, expected);
        assert_eq!(function.output, expected);
    }
    assert_eq!(
        document.states[0].ty,
        Type::List(Box::new(Type::WindowDirection))
    );
    assert_eq!(document.states[7].ty, Type::WindowDirection);
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Assign {
            value: Expr::List(_),
            ..
        }
    ));
}

#[test]
fn parses_first_class_native_window_position() {
    let source = example!("window_position.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::WindowPosition);
    assert_eq!(document.functions[0].output, Type::WindowPosition);
    assert_eq!(document.functions[1].output, Type::WindowPosition);
    assert_eq!(document.states[0].ty, Type::WindowPosition);
    assert_eq!(document.states[5].ty, Type::Option(Box::new(Type::Point)));
    assert!(matches!(
        &document.handlers[0].statements[2],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "window_position.specific"
    ));
}

#[test]
fn parses_first_class_native_event_status() {
    let source = example!("event_status.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::EventStatus);
    assert_eq!(document.functions[0].output, Type::EventStatus);
    assert_eq!(document.states[0].ty, Type::EventStatus);
    assert!(matches!(
        &document.handlers[0].statements[3],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "event_status.merge"
    ));
}

#[test]
fn parses_first_class_native_redraw_request() {
    let source = example!("redraw_request.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::RedrawRequest);
    assert_eq!(document.functions[0].output, Type::RedrawRequest);
    assert_eq!(document.functions[1].output, Type::Instant);
    assert_eq!(document.states[0].ty, Type::RedrawRequest);
    assert_eq!(document.states[4].ty, Type::Option(Box::new(Type::Instant)));
    assert!(matches!(
        &document.handlers[0].statements[1],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "redraw_request.at"
    ));
}

#[test]
fn parses_first_class_native_window_id() {
    let source = example!("window_id.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::WindowId);
    assert_eq!(document.functions[0].output, Type::WindowId);
    assert_eq!(document.states[0].ty, Type::WindowId);
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "window_id.unique"
    ));
}

#[test]
fn parses_first_class_native_window_screenshot() {
    let source = example!("window_screenshot.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].output, Type::WindowScreenshot);
    assert_eq!(document.functions[1].params[0].1, Type::WindowScreenshot);
    assert_eq!(document.functions[1].output, Type::WindowScreenshot);
    assert_eq!(document.states[0].ty, Type::WindowScreenshot);
    assert_eq!(
        document.states[2].ty,
        Type::Option(Box::new(Type::WindowScreenshot))
    );
    assert!(
        document.handlers[0]
            .statements
            .iter()
            .any(|statement| matches!(
                statement,
                Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "screenshot.new"
            ))
    );
}

#[test]
fn parses_first_class_native_length() {
    let source = example!("length.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Length);
    assert_eq!(document.functions[0].output, Type::Length);
    assert_eq!(document.states[0].ty, Type::Length);
    assert_eq!(
        document
            .states
            .iter()
            .find(|state| state.name == "dynamic_portion")
            .unwrap()
            .ty,
        Type::Option(Box::new(Type::Length))
    );
    assert!(matches!(
        &document.handlers[0].statements[1],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "length.fill_portion"
    ));
}

#[test]
fn parses_first_class_native_color() {
    let source = example!("color.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Color);
    assert_eq!(document.functions[0].output, Type::Color);
    assert_eq!(document.states[0].ty, Type::Color);
    assert_eq!(
        document
            .states
            .iter()
            .find(|state| state.name == "parsed3")
            .unwrap()
            .ty,
        Type::Option(Box::new(Type::Color))
    );
    assert!(
        document.handlers[0]
            .statements
            .iter()
            .any(|statement| matches!(
                statement,
                Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "color.rgb"
            ))
    );
}

#[test]
fn parses_first_class_native_content_fit() {
    let source = example!("content_fit.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::ContentFit);
    assert_eq!(document.functions[0].output, Type::ContentFit);
    assert_eq!(document.states[1].ty, Type::ContentFit);
    assert!(matches!(
        &document.handlers[0].statements[2],
        Statement::Assign { value: Expr::Call { name, .. }, .. } if name == "fit.cover"
    ));
}

#[test]
fn parses_first_class_native_rotation() {
    let source = example!("rotation.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Rotation);
    assert_eq!(document.functions[0].output, Type::Rotation);
    assert_eq!(document.states[1].ty, Type::Rotation);
    assert!(matches!(
        &document.handlers[0].statements[1],
        Statement::Assign { value: Expr::Call { name, .. }, .. }
            if name == "rotation.floating"
    ));
}

#[test]
fn parses_native_debug_timing_state_and_statements() {
    let source = example!("debug_timing.ice");
    let document = parse(source).unwrap();
    assert_eq!(
        document.states[0].ty,
        Type::Option(Box::new(Type::DebugSpan))
    );
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::DebugStart { target, .. } if target == "timer"
    ));
    assert!(matches!(
        &document.handlers[1].statements[0],
        Statement::DebugFinish { target, .. } if target == "timer"
    ));
}

#[test]
fn parses_native_image_allocation_types_and_task() {
    let source = example!("image_allocation.ice");
    let document = parse(source).unwrap();
    assert_eq!(
        document.states[1].ty,
        Type::Option(Box::new(Type::ImageAllocation))
    );
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Run { function, args, error: Some(_), .. }
            if function == "__ice_image_allocate" && args.len() == 1
    ));
}

#[test]
fn parses_native_animation_configuration_and_explicit_time() {
    let source = example!("animation.ice");
    let document = parse(source).unwrap();
    let state = &document.states[0];
    assert_eq!(state.ty, Type::Animation(Box::new(Type::Bool)));
    let options = state.animation.as_ref().unwrap();
    assert_eq!(options.easing.as_deref(), Some("ease-in-out"));
    assert_eq!(options.duration, Some(AnimationDuration::Milliseconds(400)));
    assert_eq!(options.delay_ms, Some(1));
    assert_eq!(options.repeat, Some(1));
    assert!(options.auto_reverse);
    assert!(matches!(
        &document.handlers[2].statements[0],
        Statement::Assign { at: Some(_), .. }
    ));
}

#[test]
fn parses_typed_keyboard_values() {
    let source = example!("keyboard_values.ice");
    let document = parse(source).unwrap();
    let state_type = |name: &str| {
        document
            .states
            .iter()
            .find(|state| state.name == name)
            .map(|state| state.ty.clone())
            .unwrap()
    };
    assert_eq!(state_type("logical"), Type::Key);
    assert_eq!(state_type("physical"), Type::PhysicalKey);
    assert_eq!(
        state_type("dynamic_native"),
        Type::Option(Box::new(Type::PhysicalKey))
    );
    assert_eq!(state_type("location"), Type::KeyLocation);
    assert_eq!(state_type("modifiers"), Type::KeyModifiers);
    assert!(matches!(
        &document.states[0].initial,
        Expr::Call { name, args } if name == "key.unidentified" && args.is_empty()
    ));
    assert!(document.handlers[0].statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assign {
                target,
                value: Expr::Call { name, args },
                ..
            } if target == "latin" && name == "key.latin" && args.len() == 2
        )
    }));
}

#[test]
fn parses_typed_pointer_values() {
    let source = example!("pointer_values.ice");
    let document = parse(source).unwrap();
    let state = |name: &str| {
        document
            .states
            .iter()
            .find(|state| state.name == name)
            .unwrap()
    };
    assert_eq!(state("click_position").ty, Type::Point);
    assert_eq!(state("button").ty, Type::MouseButton);
    assert_eq!(state("cursor").ty, Type::MouseCursor);
    assert_eq!(state("click").ty, Type::MouseClick);
    assert_eq!(state("finger").ty, Type::TouchFinger);
    assert!(matches!(
        &state("click").initial,
        Expr::Call { name, args } if name == "mouse.click" && args.len() == 3
    ));
    assert!(document.handlers[0].statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Let {
                name,
                value: Expr::Call { name: call, args },
                ..
            } if name == "bounds" && call == "rectangle" && args.len() == 4
        )
    }));
    assert!(document.handlers[0].statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assign {
                target,
                value: Expr::Call { name, args },
                ..
            } if target == "cursor_position" && name == "mouse.cursor_position" && args.len() == 1
        )
    }));
}

#[test]
fn parses_native_transformations() {
    let source = example!("transformation_values.ice");
    let document = parse(source).unwrap();
    let state = |name: &str| {
        document
            .states
            .iter()
            .find(|state| state.name == name)
            .unwrap()
    };
    assert_eq!(state("combined").ty, Type::Transformation);
    assert_eq!(state("translation").ty, Type::Vector);
    assert_eq!(state("size_value").ty, Type::Size);
    assert!(matches!(
        &state("combined").initial,
        Expr::Call { name, args } if name == "transform.compose" && args.len() == 2
    ));
    assert!(document.handlers[0].statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assign {
                target,
                value: Expr::Call { name, args },
                ..
            } if target == "point_value" && name == "transform.point" && args.len() == 2
        )
    }));
}

#[test]
fn parses_native_geometry_values() {
    let source = example!("geometry_values.ice");
    let document = parse(source).unwrap();
    let state_type = |name: &str| {
        document
            .states
            .iter()
            .find(|state| state.name == name)
            .map(|state| state.ty.clone())
            .unwrap()
    };
    assert_eq!(state_type("point_value"), Type::Point);
    assert_eq!(state_type("snapped_point"), Type::PointU32);
    assert_eq!(
        state_type("snapped_bounds"),
        Type::Option(Box::new(Type::RectangleU32))
    );
    assert_eq!(state_type("bounds_size"), Type::Size);
    assert!(document.handlers[0].statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assign {
                target,
                value: Expr::Binary { op: BinaryOp::Mul, .. },
                ..
            } if target == "scaled_bounds"
        )
    }));
}

#[test]
fn parses_native_padding_and_angles() {
    fn contains_remainder(expr: &Expr) -> bool {
        match expr {
            Expr::Binary {
                op: BinaryOp::Rem, ..
            } => true,
            Expr::Binary { left, right, .. } => {
                contains_remainder(left) || contains_remainder(right)
            }
            Expr::Unary { value, .. } => contains_remainder(value),
            Expr::Call { args, .. } | Expr::List(args) => args.iter().any(contains_remainder),
            _ => false,
        }
    }

    let source = example!("padding_angles.ice");
    let document = parse(source).unwrap();
    let state_type = |name: &str| {
        document
            .states
            .iter()
            .find(|state| state.name == name)
            .map(|state| state.ty.clone())
            .unwrap()
    };
    assert_eq!(state_type("pixel_value"), Type::Pixels);
    assert_eq!(state_type("direct_padding"), Type::Padding);
    assert_eq!(state_type("degree_value"), Type::Degrees);
    assert_eq!(state_type("radians_value"), Type::Radians);
    assert!(document.handlers[0].statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assign { target, value, .. }
                if target == "radians_math" && contains_remainder(value)
        )
    }));
}

use super::*;
use crate::unqualified_name;

pub(crate) fn expr_type(
    expr: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    match expr {
        Expr::Bool(_) => Ok(Type::Bool),
        Expr::I64(_) => Ok(Type::I64),
        Expr::F64(_) => Ok(Type::F64),
        Expr::Str(_) => Ok(Type::Str),
        Expr::Bytes(_) => Ok(Type::Bytes),
        Expr::EmptyList => Ok(Type::List(Box::new(Type::Unknown))),
        Expr::List(values) => {
            let Some(first) = values.first() else {
                return Ok(Type::List(Box::new(Type::Unknown)));
            };
            let ty = expr_type(first, env, document, span)?;
            for value in &values[1..] {
                let actual = expr_type(value, env, document, span)?;
                require_type(&actual, &ty, span)?;
            }
            Ok(Type::List(Box::new(ty)))
        }
        Expr::None => Ok(Type::Option(Box::new(Type::Unknown))),
        Expr::Path(path) => {
            if let [contract, palette] = path.as_slice()
                && document
                    .theme_contract
                    .as_ref()
                    .is_some_and(|item| item.name == *contract)
            {
                if document.palettes.iter().any(|item| item.name == *palette) {
                    return Ok(Type::Palette(contract.clone()));
                }
                return Err(Error::new(
                    "E152",
                    span,
                    format!("theme contract `{contract}` has no palette `{palette}`"),
                ));
            }
            if let [enum_name, variant_name] = path.as_slice()
                && let Some((_, variant)) = ui_enum_variant(document, enum_name, variant_name)
            {
                if variant.payload.is_some() {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("enum variant `{enum_name}.{variant_name}` requires one payload"),
                    ));
                }
                return Ok(Type::Named(enum_name.clone()));
            }
            let mut ty = env
                .get(&path[0])
                .cloned()
                .ok_or_else(|| Error::new("E150", span, format!("unknown value `{}`", path[0])))?;
            for field in &path[1..] {
                ty = field_type(&ty, field, document, span)?;
            }
            Ok(ty)
        }
        Expr::Call { name, args } => {
            if unqualified_name(name) == "provided" {
                let [Expr::Path(path)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "provided expects one component slot name",
                    ));
                };
                let [slot] = path.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "provided expects one component slot name",
                    ));
                };
                let slot = unqualified_name(slot);
                if !env.contains_key(&format!("\0slot-provided:{slot}")) {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("component does not declare slot `{slot}`"),
                    ));
                }
                return Ok(Type::Bool);
            }
            if let Some((enum_name, variant_name)) = name.split_once('.')
                && let Some((_, variant)) = ui_enum_variant(document, enum_name, variant_name)
            {
                let Some(payload) = &variant.payload else {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("enum variant `{name}` has no payload and is written without `()`"),
                    ));
                };
                if args.len() != 1 {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("enum variant `{name}` expects one payload"),
                    ));
                }
                require_type(&expr_type(&args[0], env, document, span)?, payload, span)?;
                return Ok(Type::Named(enum_name.to_owned()));
            }
            if let Some(function) = document
                .functions
                .iter()
                .find(|function| function.name == *name && function.kind == ExternKind::Sync)
            {
                check_call_args(function, args, env, document, span)?;
                return Ok(function.output.clone());
            }
            let name = unqualified_name(name);
            match name {
                "color.default" | "color.black" | "color.white" | "color.transparent" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Color)
                }
                "color.rgb" | "color.from3" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::F64, Type::F64, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    for value in args {
                        require_literal_range(value, 0.0, Some(1.0), "color channel", span)?;
                    }
                    Ok(Type::Color)
                }
                "color.rgba" | "color.linear_rgba" | "color.from4" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::F64, Type::F64, Type::F64, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    for value in args {
                        require_literal_range(value, 0.0, Some(1.0), "color channel", span)?;
                    }
                    Ok(Type::Color)
                }
                "color.parse" => {
                    check_builtin_args(name, args, &[Type::Str], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::Color)))
                }
                "color.rgb8" => {
                    if args.len() != 3 {
                        return Err(Error::new("E152", span, "color.rgb8 expects 3 arguments"));
                    }
                    check_u8_literals(name, args, 3, span)?;
                    Ok(Type::Color)
                }
                "color.rgba8" => {
                    if args.len() != 4 {
                        return Err(Error::new("E152", span, "color.rgba8 expects 4 arguments"));
                    }
                    check_u8_literals(name, args, 3, span)?;
                    require_type(&expr_type(&args[3], env, document, span)?, &Type::F64, span)?;
                    require_literal_range(&args[3], 0.0, Some(1.0), "color alpha", span)?;
                    Ok(Type::Color)
                }
                "color.try_rgb8" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::I64, Type::I64, Type::I64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Option(Box::new(Type::Color)))
                }
                "color.try_rgba8" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::I64, Type::I64, Type::I64, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    require_literal_range(&args[3], 0.0, Some(1.0), "color alpha", span)?;
                    Ok(Type::Option(Box::new(Type::Color)))
                }
                "color.inverse" | "color.invert" => {
                    check_builtin_args(name, args, &[Type::Color], env, document, span)?;
                    Ok(Type::Color)
                }
                "color.scale_alpha" => {
                    check_f32_args(name, args, &[Type::Color, Type::F64], env, document, span)?;
                    Ok(Type::Color)
                }
                "color.luminance" => {
                    check_builtin_args(name, args, &[Type::Color], env, document, span)?;
                    Ok(Type::F64)
                }
                "color.contrast" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Color, Type::Color],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::F64)
                }
                "color.readable" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Color, Type::Color],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Bool)
                }
                "color_stop.default" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::ColorStop)
                }
                "color_stop" => {
                    check_f32_args(name, args, &[Type::F64, Type::Color], env, document, span)?;
                    Ok(Type::ColorStop)
                }
                "linear" => {
                    if args.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "linear expects one f64 or radians angle",
                        ));
                    }
                    require_radians_value(&args[0], env, document, span)?;
                    Ok(Type::LinearGradient)
                }
                "linear.add_stop" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::LinearGradient, Type::F64, Type::Color],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::LinearGradient)
                }
                "linear.add_stops" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::LinearGradient, Type::List(Box::new(Type::ColorStop))],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::LinearGradient)
                }
                "linear.scale_alpha" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::LinearGradient, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::LinearGradient)
                }
                "gradient.linear" | "gradient.from_linear" => {
                    check_builtin_args(name, args, &[Type::LinearGradient], env, document, span)?;
                    Ok(Type::Gradient)
                }
                "gradient.scale_alpha" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::Gradient, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Gradient)
                }
                "background.color" | "background.from_color" => {
                    check_builtin_args(name, args, &[Type::Color], env, document, span)?;
                    Ok(Type::Background)
                }
                "background.gradient" | "background.from_gradient" => {
                    check_builtin_args(name, args, &[Type::Gradient], env, document, span)?;
                    Ok(Type::Background)
                }
                "background.from_linear" => {
                    check_builtin_args(name, args, &[Type::LinearGradient], env, document, span)?;
                    Ok(Type::Background)
                }
                "background.scale_alpha" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::Background, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Background)
                }
                "font.default" | "font.sans" | "font.monospace" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Font)
                }
                "font.with_name" => {
                    if !matches!(args.as_slice(), [Expr::Str(_)]) {
                        return Err(Error::new(
                            "E152",
                            span,
                            "font.with_name expects one string literal",
                        ));
                    }
                    Ok(Type::Font)
                }
                "font.new" => {
                    check_builtin_args(
                        name,
                        args,
                        &[
                            Type::FontFamily,
                            Type::FontWeight,
                            Type::FontStretch,
                            Type::FontStyle,
                        ],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Font)
                }
                "family.default" | "family.serif" | "family.sans_serif" | "family.cursive"
                | "family.fantasy" | "family.monospace" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::FontFamily)
                }
                "family.named" => {
                    if !matches!(args.as_slice(), [Expr::Str(_)]) {
                        return Err(Error::new(
                            "E152",
                            span,
                            "family.named expects one string literal",
                        ));
                    }
                    Ok(Type::FontFamily)
                }
                "weight.default" | "weight.thin" | "weight.extra_light" | "weight.light"
                | "weight.normal" | "weight.medium" | "weight.semibold" | "weight.bold"
                | "weight.extra_bold" | "weight.black" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::FontWeight)
                }
                "stretch.default"
                | "stretch.ultra_condensed"
                | "stretch.extra_condensed"
                | "stretch.condensed"
                | "stretch.semi_condensed"
                | "stretch.normal"
                | "stretch.semi_expanded"
                | "stretch.expanded"
                | "stretch.extra_expanded"
                | "stretch.ultra_expanded" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::FontStretch)
                }
                "font_style.default" | "font_style.normal" | "font_style.italic"
                | "font_style.oblique" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::FontStyle)
                }
                "theme_mode.default" | "theme_mode.none" | "theme_mode.light"
                | "theme_mode.dark" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::ThemeMode)
                }
                "text_alignment.default"
                | "text_alignment.left"
                | "text_alignment.center"
                | "text_alignment.right"
                | "text_alignment.justified" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::TextAlignment)
                }
                "text_alignment.from_horizontal" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::HorizontalAlignment],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::TextAlignment)
                }
                "text_alignment.from_alignment" => {
                    check_builtin_args(name, args, &[Type::Alignment], env, document, span)?;
                    Ok(Type::TextAlignment)
                }
                "horizontal.from_text_alignment" => {
                    check_builtin_args(name, args, &[Type::TextAlignment], env, document, span)?;
                    Ok(Type::HorizontalAlignment)
                }
                "text_shaping.default"
                | "text_shaping.auto"
                | "text_shaping.basic"
                | "text_shaping.advanced" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::TextShaping)
                }
                "text_wrapping.default"
                | "text_wrapping.none"
                | "text_wrapping.word"
                | "text_wrapping.glyph"
                | "text_wrapping.word_or_glyph" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::TextWrapping)
                }
                "line_height.default" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::TextLineHeight)
                }
                "line_height.relative" | "line_height.from_f64" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::TextLineHeight)
                }
                "line_height.absolute" | "line_height.from_pixels" => {
                    check_builtin_args(name, args, &[Type::Pixels], env, document, span)?;
                    Ok(Type::TextLineHeight)
                }
                "line_height.to_absolute" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::TextLineHeight, Type::Pixels],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Pixels)
                }
                "screenshot.new" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::Bytes, Type::SizeU32, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::WindowScreenshot)
                }
                "screenshot.crop" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::WindowScreenshot, Type::RectangleU32],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Option(Box::new(Type::WindowScreenshot)))
                }
                "screenshot.crop_error" | "screenshot.crop_error_message" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::WindowScreenshot, Type::RectangleU32],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Option(Box::new(Type::Str)))
                }
                "screenshot.as_bytes" | "screenshot.into_bytes" => {
                    check_builtin_args(name, args, &[Type::WindowScreenshot], env, document, span)?;
                    Ok(Type::Bytes)
                }
                "interaction.default"
                | "interaction.none"
                | "interaction.hidden"
                | "interaction.idle"
                | "interaction.context_menu"
                | "interaction.help"
                | "interaction.pointer"
                | "interaction.progress"
                | "interaction.wait"
                | "interaction.cell"
                | "interaction.crosshair"
                | "interaction.text"
                | "interaction.alias"
                | "interaction.copy"
                | "interaction.move"
                | "interaction.no_drop"
                | "interaction.not_allowed"
                | "interaction.grab"
                | "interaction.grabbing"
                | "interaction.resize_horizontal"
                | "interaction.resize_vertical"
                | "interaction.resize_diagonal_up"
                | "interaction.resize_diagonal_down"
                | "interaction.resize_column"
                | "interaction.resize_row"
                | "interaction.all_scroll"
                | "interaction.zoom_in"
                | "interaction.zoom_out" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::MouseInteraction)
                }
                "scroll.lines" | "scroll.pixels" => {
                    check_f32_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                    Ok(Type::ScrollDelta)
                }
                "event_status.ignored" | "event_status.captured" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::EventStatus)
                }
                "event_status.merge" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::EventStatus, Type::EventStatus],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::EventStatus)
                }
                "redraw_request.next_frame" | "redraw_request.wait" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::RedrawRequest)
                }
                "redraw_request.at" => {
                    check_builtin_args(name, args, &[Type::Instant], env, document, span)?;
                    Ok(Type::RedrawRequest)
                }
                "window_id.unique" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::WindowId)
                }
                "window_direction.north"
                | "window_direction.south"
                | "window_direction.east"
                | "window_direction.west"
                | "window_direction.north_east"
                | "window_direction.north_west"
                | "window_direction.south_east"
                | "window_direction.south_west" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::WindowDirection)
                }
                "window_level.default"
                | "window_level.normal"
                | "window_level.always_on_bottom"
                | "window_level.always_on_top" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::WindowLevel)
                }
                "window_mode.windowed" | "window_mode.fullscreen" | "window_mode.hidden" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::WindowMode)
                }
                "window_attention.critical" | "window_attention.informational" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::WindowAttention)
                }
                "window_position.default" | "window_position.centered" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::WindowPosition)
                }
                "window_position.specific" => {
                    check_builtin_args(name, args, &[Type::Point], env, document, span)?;
                    Ok(Type::WindowPosition)
                }
                "length.fill" | "length.shrink" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Length)
                }
                "length.fill_portion" => {
                    check_u16_literal(name, args, span)?;
                    Ok(Type::Length)
                }
                "length.try_fill_portion" => {
                    check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::Length)))
                }
                "length.fixed" | "length.from_f64" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::Length)
                }
                "length.from_pixels" => {
                    check_builtin_args(name, args, &[Type::Pixels], env, document, span)?;
                    Ok(Type::Length)
                }
                "length.from_u32" => {
                    check_u32_literal(name, args, span)?;
                    Ok(Type::Length)
                }
                "length.try_from_u32" => {
                    check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::Length)))
                }
                "length.fluid" => {
                    check_builtin_args(name, args, &[Type::Length], env, document, span)?;
                    Ok(Type::Length)
                }
                "length.enclose" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Length, Type::Length],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Length)
                }
                "alignment.start" | "alignment.center" | "alignment.end" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Alignment)
                }
                "horizontal.left" | "horizontal.center" | "horizontal.right" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::HorizontalAlignment)
                }
                "vertical.top" | "vertical.center" | "vertical.bottom" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::VerticalAlignment)
                }
                "alignment.from_horizontal" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::HorizontalAlignment],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Alignment)
                }
                "alignment.from_vertical" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::VerticalAlignment],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Alignment)
                }
                "horizontal.from_alignment" => {
                    check_builtin_args(name, args, &[Type::Alignment], env, document, span)?;
                    Ok(Type::HorizontalAlignment)
                }
                "vertical.from_alignment" => {
                    check_builtin_args(name, args, &[Type::Alignment], env, document, span)?;
                    Ok(Type::VerticalAlignment)
                }
                "border.default" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Border)
                }
                "border.new" => {
                    if args.len() != 3 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "border.new expects color, width, and radius",
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Color,
                        span,
                    )?;
                    require_pixel_value(&args[1], env, document, span)?;
                    require_radius_value(&args[2], env, document, span)?;
                    Ok(Type::Border)
                }
                "border.color" => {
                    check_builtin_args(name, args, &[Type::Color], env, document, span)?;
                    Ok(Type::Border)
                }
                "border.width" => {
                    if args.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "border.width expects one pixel value",
                        ));
                    }
                    require_pixel_value(&args[0], env, document, span)?;
                    Ok(Type::Border)
                }
                "border.rounded" => {
                    if args.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "border.rounded expects one radius value",
                        ));
                    }
                    require_radius_value(&args[0], env, document, span)?;
                    Ok(Type::Border)
                }
                "border.with_color" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Border, Type::Color],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Border)
                }
                "border.with_width" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "border.with_width expects a border and pixel value",
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Border,
                        span,
                    )?;
                    require_pixel_value(&args[1], env, document, span)?;
                    Ok(Type::Border)
                }
                "border.with_radius" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "border.with_radius expects a border and radius value",
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Border,
                        span,
                    )?;
                    require_radius_value(&args[1], env, document, span)?;
                    Ok(Type::Border)
                }
                "radius" | "radius.new" => {
                    if args.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            format!("{name} expects one pixel value"),
                        ));
                    }
                    require_pixel_value(&args[0], env, document, span)?;
                    Ok(Type::Radius)
                }
                "radius.default" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Radius)
                }
                "radius.top_left"
                | "radius.top_right"
                | "radius.bottom_right"
                | "radius.bottom_left"
                | "radius.top"
                | "radius.bottom"
                | "radius.left"
                | "radius.right" => {
                    if args.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            format!("{name} expects one pixel value"),
                        ));
                    }
                    require_pixel_value(&args[0], env, document, span)?;
                    Ok(Type::Radius)
                }
                "radius.with_top_left"
                | "radius.with_top_right"
                | "radius.with_bottom_right"
                | "radius.with_bottom_left"
                | "radius.with_top"
                | "radius.with_bottom"
                | "radius.with_left"
                | "radius.with_right" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            format!("{name} expects a radius and pixel value"),
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Radius,
                        span,
                    )?;
                    require_pixel_value(&args[1], env, document, span)?;
                    Ok(Type::Radius)
                }
                "radius.from_f64" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::Radius)
                }
                "radius.from_u8" => {
                    check_u8_literal(name, args, span)?;
                    Ok(Type::Radius)
                }
                "radius.from_u32" => {
                    check_u32_literal(name, args, span)?;
                    Ok(Type::Radius)
                }
                "radius.from_i32" => {
                    check_i32_literal(name, args, span)?;
                    Ok(Type::Radius)
                }
                "radius.try_from_u8" | "radius.try_from_u32" | "radius.try_from_i32" => {
                    check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::Radius)))
                }
                "shadow.default" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Shadow)
                }
                "shadow.new" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::Color, Type::Vector, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Shadow)
                }
                "fit.default" | "fit.contain" | "fit.cover" | "fit.fill" | "fit.none"
                | "fit.scale_down" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::ContentFit)
                }
                "fit.apply" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::ContentFit, Type::Size, Type::Size],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Size)
                }
                "rotation.default" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Rotation)
                }
                "rotation.floating" | "rotation.solid" => {
                    check_builtin_args(name, args, &[Type::Radians], env, document, span)?;
                    Ok(Type::Rotation)
                }
                "rotation.from" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::Rotation)
                }
                "rotation.with_radians" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Rotation, Type::Radians],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Rotation)
                }
                "rotation.apply" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Rotation, Type::Size],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Size)
                }
                "debug.active" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Option(Box::new(Type::DebugSpan))],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Bool)
                }
                "debug.time_with" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "debug.time_with expects a name and one value",
                        ));
                    }
                    require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                    let output = expr_type(&args[1], env, document, span)?;
                    if contains_debug_span(&output) {
                        return Err(Error::new(
                            "E152",
                            span,
                            "debug.time_with cannot move debug span state",
                        ));
                    }
                    Ok(output)
                }
                "image.downgrade" => {
                    check_builtin_args(name, args, &[Type::ImageAllocation], env, document, span)?;
                    Ok(Type::ImageMemory)
                }
                "image.upgrade" => {
                    check_builtin_args(name, args, &[Type::ImageMemory], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::ImageAllocation)))
                }
                "animation.value" => {
                    check_animation_instant(name, args, 1, false, env, document, span)?;
                    animation_inner(&args[0], env, document, span)
                }
                "animation.animating" => {
                    check_animation_instant(name, args, 1, true, env, document, span)?;
                    animation_inner(&args[0], env, document, span)?;
                    Ok(Type::Bool)
                }
                "animation.interpolate" => {
                    check_animation_instant(name, args, 3, true, env, document, span)?;
                    require_type(
                        &animation_inner(&args[0], env, document, span)?,
                        &Type::Bool,
                        span,
                    )?;
                    let output = expr_type(&args[1], env, document, span)?;
                    let output = if output == Type::F64 {
                        Type::F64
                    } else {
                        let optional = Type::Option(Box::new(Type::F64));
                        require_type(&output, &optional, span).map_err(|_| {
                            Error::new(
                                "E152",
                                span,
                                "animation.interpolate values must be f64 or f64?",
                            )
                        })?;
                        optional
                    };
                    require_type(&expr_type(&args[2], env, document, span)?, &output, span)?;
                    if output == Type::F64 {
                        for value in &args[1..=2] {
                            require_f32_literal_range(
                                value,
                                f64::NEG_INFINITY,
                                None,
                                "animation interpolation value",
                                span,
                            )?;
                        }
                    }
                    Ok(output)
                }
                "animation.remaining" => {
                    check_animation_instant(name, args, 1, true, env, document, span)?;
                    require_type(
                        &animation_inner(&args[0], env, document, span)?,
                        &Type::Bool,
                        span,
                    )?;
                    Ok(Type::F64)
                }
                "animation.project" => {
                    check_animation_instant(name, args, 3, true, env, document, span)?;
                    let inner = animation_inner(&args[0], env, document, span)?;
                    let Expr::Path(binding) = &args[1] else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "animation.project second argument must be a binding name",
                        ));
                    };
                    if binding.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "animation.project second argument must be a binding name",
                        ));
                    }
                    let mut projection_env = env.clone();
                    projection_env.insert(binding[0].clone(), inner);
                    let output = expr_type(&args[2], &projection_env, document, span)?;
                    if output != Type::F64 && output != Type::Option(Box::new(Type::F64)) {
                        return Err(Error::new(
                            "E152",
                            span,
                            "animation.project expression must produce f64 or f64?",
                        ));
                    }
                    if output == Type::F64 {
                        require_f32_literal_range(
                            &args[2],
                            f64::NEG_INFINITY,
                            None,
                            "animation projection value",
                            span,
                        )?;
                    }
                    Ok(output)
                }
                "pixels" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::Pixels)
                }
                "pixels.zero" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Pixels)
                }
                "pixels.from_u32" => {
                    check_u32_literal(name, args, span)?;
                    Ok(Type::Pixels)
                }
                "pixels.try_from_u32" => {
                    check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::Pixels)))
                }
                "padding" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::F64, Type::F64, Type::F64, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Padding)
                }
                "padding.zero" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Padding)
                }
                "padding.all" | "padding.top" | "padding.right" | "padding.bottom"
                | "padding.left" | "padding.horizontal" | "padding.vertical" => {
                    if args.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            format!("{name} expects one argument"),
                        ));
                    }
                    require_pixel_value(&args[0], env, document, span)?;
                    Ok(Type::Padding)
                }
                "padding.axes" => {
                    check_f32_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                    Ok(Type::Padding)
                }
                "padding.from_pixels" => {
                    check_builtin_args(name, args, &[Type::Pixels], env, document, span)?;
                    Ok(Type::Padding)
                }
                "padding.with_top"
                | "padding.with_right"
                | "padding.with_bottom"
                | "padding.with_left"
                | "padding.with_horizontal"
                | "padding.with_vertical" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            format!("{name} expects padding and a pixel value"),
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Padding,
                        span,
                    )?;
                    require_pixel_value(&args[1], env, document, span)?;
                    Ok(Type::Padding)
                }
                "padding.fit" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Padding, Type::Size, Type::Size],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Padding)
                }
                "degrees" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::Degrees)
                }
                "radians" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::Radians)
                }
                "degrees.range_start" | "degrees.range_end" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Degrees)
                }
                "radians.range_start" | "radians.range_end" | "radians.pi" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Radians)
                }
                "degrees.in_range" => {
                    check_builtin_args(name, args, &[Type::Degrees], env, document, span)?;
                    Ok(Type::Bool)
                }
                "radians.in_range" => {
                    check_builtin_args(name, args, &[Type::Radians], env, document, span)?;
                    Ok(Type::Bool)
                }
                "radians.from_degrees" => {
                    check_builtin_args(name, args, &[Type::Degrees], env, document, span)?;
                    Ok(Type::Radians)
                }
                "radians.distance_start" | "radians.distance_end" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Radians, Type::Rectangle],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Point)
                }
                "point" => {
                    check_f32_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                    Ok(Type::Point)
                }
                "vector" => {
                    check_f32_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                    Ok(Type::Vector)
                }
                "size" => {
                    check_f32_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                    Ok(Type::Size)
                }
                "rectangle" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::F64, Type::F64, Type::F64, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Rectangle)
                }
                "point.origin" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Point)
                }
                "point.distance" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Point, Type::Point],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::F64)
                }
                "point.snap" => {
                    check_builtin_args(name, args, &[Type::Point], env, document, span)?;
                    Ok(Type::PointU32)
                }
                "vector.zero" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Vector)
                }
                "size.zero" | "size.unit" | "size.infinite" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Size)
                }
                "size.min" | "size.max" | "size.expand" => {
                    check_builtin_args(name, args, &[Type::Size, Type::Size], env, document, span)?;
                    Ok(Type::Size)
                }
                "size.rotate" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "size.rotate expects a size and radians",
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Size,
                        span,
                    )?;
                    require_radians_value(&args[1], env, document, span)?;
                    Ok(Type::Size)
                }
                "size.ratio" => {
                    check_f32_args(name, args, &[Type::Size, Type::F64], env, document, span)?;
                    Ok(Type::Size)
                }
                "size.from_vector" => {
                    check_builtin_args(name, args, &[Type::Vector], env, document, span)?;
                    Ok(Type::Size)
                }
                "vector.from_size" => {
                    check_builtin_args(name, args, &[Type::Size], env, document, span)?;
                    Ok(Type::Vector)
                }
                "size.from_padding" => {
                    check_builtin_args(name, args, &[Type::Padding], env, document, span)?;
                    Ok(Type::Size)
                }
                "size.from_u32" => {
                    check_u32_literals(name, args, span)?;
                    Ok(Type::Size)
                }
                "size.try_from_u32" => {
                    check_builtin_args(name, args, &[Type::I64, Type::I64], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::Size)))
                }
                "rectangle.zero" | "rectangle.infinite" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Rectangle)
                }
                "rectangle.with_size" => {
                    check_builtin_args(name, args, &[Type::Size], env, document, span)?;
                    Ok(Type::Rectangle)
                }
                "rectangle.with_radius" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::Rectangle)
                }
                "rectangle.with_vertices"
                | "rectangle.vertices_rotation"
                | "rectangle.vertices_angle" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Point, Type::Point, Type::Point],
                        env,
                        document,
                        span,
                    )?;
                    Ok(match name {
                        "rectangle.with_vertices" => Type::Rectangle,
                        "rectangle.vertices_rotation" => Type::F64,
                        _ => Type::Radians,
                    })
                }
                "rectangle.contains" | "rectangle.distance" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Rectangle, Type::Point],
                        env,
                        document,
                        span,
                    )?;
                    Ok(if name == "rectangle.contains" {
                        Type::Bool
                    } else {
                        Type::F64
                    })
                }
                "rectangle.offset" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Rectangle, Type::Rectangle],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Vector)
                }
                "rectangle.is_within" | "rectangle.intersects" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Rectangle, Type::Rectangle],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Bool)
                }
                "rectangle.intersection" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Rectangle, Type::Rectangle],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Option(Box::new(Type::Rectangle)))
                }
                "rectangle.union" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Rectangle, Type::Rectangle],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Rectangle)
                }
                "rectangle.snap" => {
                    check_builtin_args(name, args, &[Type::Rectangle], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::RectangleU32)))
                }
                "rectangle.expand" | "rectangle.shrink" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::Rectangle, Type::F64, Type::F64, Type::F64, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Rectangle)
                }
                "rectangle.expand_padding" | "rectangle.shrink_padding" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Rectangle, Type::Padding],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Rectangle)
                }
                "rectangle.rotate" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rectangle.rotate expects a rectangle and radians",
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Rectangle,
                        span,
                    )?;
                    require_radians_value(&args[1], env, document, span)?;
                    Ok(Type::Rectangle)
                }
                "rectangle.zoom" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::Rectangle, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Rectangle)
                }
                "rectangle.anchor" => {
                    if args.len() != 4 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rectangle.anchor expects a rectangle, size, horizontal alignment, and vertical alignment",
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Rectangle,
                        span,
                    )?;
                    require_type(
                        &expr_type(&args[1], env, document, span)?,
                        &Type::Size,
                        span,
                    )?;
                    let Expr::Str(horizontal) = &args[2] else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rectangle.anchor horizontal alignment must be left, center, or right",
                        ));
                    };
                    let Expr::Str(vertical) = &args[3] else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rectangle.anchor vertical alignment must be top, center, or bottom",
                        ));
                    };
                    if !matches!(horizontal.as_str(), "left" | "center" | "right") {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rectangle.anchor horizontal alignment must be left, center, or right",
                        ));
                    }
                    if !matches!(vertical.as_str(), "top" | "center" | "bottom") {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rectangle.anchor vertical alignment must be top, center, or bottom",
                        ));
                    }
                    Ok(Type::Point)
                }
                "rectangle.from_u32" => {
                    check_builtin_args(name, args, &[Type::RectangleU32], env, document, span)?;
                    Ok(Type::Rectangle)
                }
                "transform.identity" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::Transformation)
                }
                "transform.orthographic" => {
                    check_u32_literals(name, args, span)?;
                    Ok(Type::Transformation)
                }
                "transform.try_orthographic" => {
                    check_builtin_args(name, args, &[Type::I64, Type::I64], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::Transformation)))
                }
                "transform.translate" => {
                    check_f32_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                    Ok(Type::Transformation)
                }
                "transform.scale" => {
                    check_f32_args(name, args, &[Type::F64], env, document, span)?;
                    Ok(Type::Transformation)
                }
                "transform.inverse" => {
                    check_builtin_args(name, args, &[Type::Transformation], env, document, span)?;
                    Ok(Type::Transformation)
                }
                "transform.compose" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::Transformation, Type::Transformation],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Transformation)
                }
                "transform.point"
                | "transform.vector"
                | "transform.size"
                | "transform.rectangle"
                | "transform.cursor"
                | "transform.click" => {
                    let value = match name {
                        "transform.point" => Type::Point,
                        "transform.vector" => Type::Vector,
                        "transform.size" => Type::Size,
                        "transform.rectangle" => Type::Rectangle,
                        "transform.cursor" => Type::MouseCursor,
                        "transform.click" => Type::MouseClick,
                        _ => unreachable!(),
                    };
                    check_builtin_args(
                        name,
                        args,
                        &[value.clone(), Type::Transformation],
                        env,
                        document,
                        span,
                    )?;
                    Ok(value)
                }
                "mouse.button" => {
                    let [Expr::Str(value)] = args.as_slice() else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "mouse.button expects one string literal",
                        ));
                    };
                    if !matches!(
                        value.as_str(),
                        "left" | "right" | "middle" | "back" | "forward"
                    ) {
                        return Err(Error::new(
                            "E152",
                            span,
                            "mouse.button must be left, right, middle, back, or forward",
                        ));
                    }
                    Ok(Type::MouseButton)
                }
                "mouse.other_button" => {
                    let [Expr::I64(value)] = args.as_slice() else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "mouse.other_button expects one integer literal",
                        ));
                    };
                    if !(0..=u16::MAX as i64).contains(value) {
                        return Err(Error::new(
                            "E152",
                            span,
                            format!("mouse.other_button must be in 0..={}", u16::MAX),
                        ));
                    }
                    Ok(Type::MouseButton)
                }
                "mouse.try_other_button" => {
                    check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::MouseButton)))
                }
                "mouse.cursor" | "mouse.levitating" => {
                    check_builtin_args(name, args, &[Type::Point], env, document, span)?;
                    Ok(Type::MouseCursor)
                }
                "mouse.unavailable" => {
                    check_builtin_args(name, args, &[], env, document, span)?;
                    Ok(Type::MouseCursor)
                }
                "mouse.cursor_position" => {
                    check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::Point)))
                }
                "mouse.cursor_over" | "mouse.cursor_in" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::MouseCursor, Type::Rectangle],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Option(Box::new(Type::Point)))
                }
                "mouse.cursor_from" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::MouseCursor, Type::Point],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Option(Box::new(Type::Point)))
                }
                "mouse.cursor_is_over" => {
                    check_builtin_args(
                        name,
                        args,
                        &[Type::MouseCursor, Type::Rectangle],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::Bool)
                }
                "mouse.cursor_is_levitating" => {
                    check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                    Ok(Type::Bool)
                }
                "mouse.cursor_levitate" | "mouse.cursor_land" => {
                    check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                    Ok(Type::MouseCursor)
                }
                "mouse.cursor_translate" => {
                    check_f32_args(
                        name,
                        args,
                        &[Type::MouseCursor, Type::F64, Type::F64],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::MouseCursor)
                }
                "mouse.click" => {
                    check_builtin_args(
                        name,
                        args,
                        &[
                            Type::Point,
                            Type::MouseButton,
                            Type::Option(Box::new(Type::MouseClick)),
                        ],
                        env,
                        document,
                        span,
                    )?;
                    Ok(Type::MouseClick)
                }
                "touch.finger" => {
                    let [Expr::Str(value)] = args.as_slice() else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "touch.finger expects one decimal string literal",
                        ));
                    };
                    if value.is_empty()
                        || !value.bytes().all(|byte| byte.is_ascii_digit())
                        || value.parse::<u64>().is_err()
                    {
                        return Err(Error::new(
                            "E152",
                            span,
                            "touch.finger must contain a decimal u64",
                        ));
                    }
                    Ok(Type::TouchFinger)
                }
                "touch.try_finger" => {
                    check_builtin_args(name, args, &[Type::Str], env, document, span)?;
                    Ok(Type::Option(Box::new(Type::TouchFinger)))
                }
                "key.named" => {
                    keyboard_variant(name, args, span)?;
                    Ok(Type::Key)
                }
                "key.character" => {
                    if args.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.character expects one str argument",
                        ));
                    }
                    require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                    Ok(Type::Key)
                }
                "key.unidentified" | "key.native_unidentified" | "key.command_modifiers" => {
                    if !args.is_empty() {
                        return Err(Error::new(
                            "E152",
                            span,
                            format!("{name} expects no arguments"),
                        ));
                    }
                    Ok(match name {
                        "key.unidentified" => Type::Key,
                        "key.native_unidentified" => Type::PhysicalKey,
                        _ => Type::KeyModifiers,
                    })
                }
                "key.code" => {
                    keyboard_variant(name, args, span)?;
                    Ok(Type::PhysicalKey)
                }
                "key.try_native" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.try_native expects a platform and integer code",
                        ));
                    }
                    let Expr::Str(platform) = &args[0] else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.try_native platform must be a string literal",
                        ));
                    };
                    if !matches!(platform.as_str(), "android" | "macos" | "windows" | "xkb") {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.try_native platform must be android, macos, windows, or xkb",
                        ));
                    }
                    require_type(&expr_type(&args[1], env, document, span)?, &Type::I64, span)?;
                    Ok(Type::Option(Box::new(Type::PhysicalKey)))
                }
                "key.native" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.native expects a platform and integer code",
                        ));
                    }
                    let Expr::Str(platform) = &args[0] else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.native platform must be a string literal",
                        ));
                    };
                    let Expr::I64(code) = args[1] else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.native code must be an integer literal",
                        ));
                    };
                    let maximum = match platform.as_str() {
                        "android" | "xkb" => u32::MAX as i64,
                        "macos" | "windows" => u16::MAX as i64,
                        _ => {
                            return Err(Error::new(
                                "E152",
                                span,
                                "key.native platform must be android, macos, windows, or xkb",
                            ));
                        }
                    };
                    if !(0..=maximum).contains(&code) {
                        return Err(Error::new(
                            "E152",
                            span,
                            format!("key.native {platform} code must be in 0..={maximum}"),
                        ));
                    }
                    Ok(Type::PhysicalKey)
                }
                "key.location" => {
                    let [Expr::Str(value)] = args.as_slice() else {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.location expects one string literal",
                        ));
                    };
                    if !matches!(value.as_str(), "standard" | "left" | "right" | "numpad") {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.location must be standard, left, right, or numpad",
                        ));
                    }
                    Ok(Type::KeyLocation)
                }
                "key.modifiers" => {
                    if args.len() != 4 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.modifiers expects shift, control, alt, and logo booleans",
                        ));
                    }
                    for value in args {
                        require_type(&expr_type(value, env, document, span)?, &Type::Bool, span)?;
                    }
                    Ok(Type::KeyModifiers)
                }
                "key.latin" => {
                    if args.len() != 2 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.latin expects a logical key and physical key",
                        ));
                    }
                    require_type(&expr_type(&args[0], env, document, span)?, &Type::Key, span)?;
                    require_type(
                        &expr_type(&args[1], env, document, span)?,
                        &Type::PhysicalKey,
                        span,
                    )?;
                    Ok(Type::Option(Box::new(Type::Str)))
                }
                "len" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "len expects one argument"));
                    }
                    match expr_type(&args[0], env, document, span)? {
                        Type::List(_) | Type::Str | Type::Bytes => Ok(Type::I64),
                        actual => Err(Error::new(
                            "E152",
                            span,
                            format!("len does not accept `{}`", actual.display()),
                        )),
                    }
                }
                "empty" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "empty expects one argument"));
                    }
                    match expr_type(&args[0], env, document, span)? {
                        Type::List(_) | Type::Str | Type::Bytes => Ok(Type::Bool),
                        actual => Err(Error::new(
                            "E152",
                            span,
                            format!("empty does not accept `{}`", actual.display()),
                        )),
                    }
                }
                "trim" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "trim expects one argument"));
                    }
                    require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                    Ok(Type::Str)
                }
                "some" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "some expects one argument"));
                    }
                    Ok(Type::Option(Box::new(expr_type(
                        &args[0], env, document, span,
                    )?)))
                }
                "ok" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "ok expects one argument"));
                    }
                    Ok(Type::Result(
                        Box::new(expr_type(&args[0], env, document, span)?),
                        Box::new(Type::Unknown),
                    ))
                }
                "err" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "err expects one argument"));
                    }
                    Ok(Type::Result(
                        Box::new(Type::Unknown),
                        Box::new(expr_type(&args[0], env, document, span)?),
                    ))
                }
                "markdown" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "markdown expects one argument"));
                    }
                    require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                    Ok(Type::Markdown)
                }
                "markdown_images" => {
                    if args.len() != 1 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "markdown_images expects one argument",
                        ));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Markdown,
                        span,
                    )?;
                    Ok(Type::List(Box::new(Type::Str)))
                }
                "editor" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "editor expects one argument"));
                    }
                    require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                    Ok(Type::Editor)
                }
                "editor_text" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "editor_text expects one argument"));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Editor,
                        span,
                    )?;
                    Ok(Type::Str)
                }
                "encoded" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "encoded expects one argument"));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Bytes,
                        span,
                    )?;
                    Ok(Type::Image)
                }
                "rgba" => {
                    if args.len() != 3 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rgba expects width, height, and pixel bytes",
                        ));
                    }
                    for (value, label) in [(&args[0], "rgba width"), (&args[1], "rgba height")] {
                        require_type(&expr_type(value, env, document, span)?, &Type::I64, span)?;
                        require_literal_range(value, 0.0, Some(u32::MAX as f64), label, span)?;
                    }
                    require_type(
                        &expr_type(&args[2], env, document, span)?,
                        &Type::Bytes,
                        span,
                    )?;
                    if let (Expr::I64(width), Expr::I64(height), Expr::Bytes(pixels)) =
                        (&args[0], &args[1], &args[2])
                    {
                        let expected = (*width as u128) * (*height as u128) * 4;
                        if expected != pixels.len() as u128 {
                            return Err(Error::new(
                                "E152",
                                span,
                                "rgba pixel data must contain width × height × 4 bytes",
                            ));
                        }
                    }
                    Ok(Type::Image)
                }
                "aborted" => {
                    if args.len() != 1 {
                        return Err(Error::new("E152", span, "aborted expects one argument"));
                    }
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Option(Box::new(Type::TaskHandle)),
                        span,
                    )?;
                    Ok(Type::Bool)
                }
                _ => {
                    let function = extern_function(document, name, ExternKind::Sync, span)?;
                    check_call_args(function, args, env, document, span)?;
                    Ok(function.output.clone())
                }
            }
        }
        Expr::Unary { op, value } => {
            let actual = expr_type(value, env, document, span)?;
            match op {
                UnaryOp::Not => {
                    require_type(&actual, &Type::Bool, span)?;
                    Ok(Type::Bool)
                }
                UnaryOp::Neg if matches!(actual, Type::I64 | Type::F64 | Type::Vector) => {
                    Ok(actual)
                }
                UnaryOp::Neg => Err(Error::new(
                    "E153",
                    span,
                    "negation expects i64, f64, or vector",
                )),
            }
        }
        Expr::Binary { left, op, right } => {
            let left_expr = left.as_ref();
            let right_expr = right.as_ref();
            let left = expr_type(left_expr, env, document, span)?;
            let right = expr_type(right_expr, env, document, span)?;
            match op {
                BinaryOp::And | BinaryOp::Or => {
                    require_type(&left, &Type::Bool, span)?;
                    require_type(&right, &Type::Bool, span)?;
                    Ok(Type::Bool)
                }
                BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq => {
                    let compares_fieldless_enum = matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
                        && left == right
                        && matches!(&left, Type::Named(name) if document.enums.iter().any(|item| {
                            item.name == *name
                                && item.variants.iter().all(|variant| variant.payload.is_none())
                        }));
                    if (contains_ui_enum(&left, document) || contains_ui_enum(&right, document))
                        && !compares_fieldless_enum
                    {
                        let message = if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) {
                            "payload-carrying UI enum values use exhaustive match instead of comparison"
                        } else {
                            "UI enum ordering is undefined; use exhaustive match"
                        };
                        return Err(Error::new("E153", span, message));
                    }
                    if contains_task_handle(&left) || contains_task_handle(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "task handles are opaque; use `aborted(handle)`",
                        ));
                    }
                    if contains_debug_span(&left) || contains_debug_span(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "debug spans are opaque; use `debug.active(state)`",
                        ));
                    }
                    if contains_mouse_click(&left) || contains_mouse_click(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "mouse-click values are opaque; compare their kind or position",
                        ));
                    }
                    if contains_window_screenshot(&left) || contains_window_screenshot(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "window-screenshot values do not support comparisons",
                        ));
                    }
                    if contains_target(&left) || contains_target(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "target values do not support comparisons; compare an explicit field such as `.kind` or `.width` instead",
                        ));
                    }
                    if matches!(
                        left,
                        Type::WindowPosition | Type::WindowDirection | Type::WindowAttention
                    ) || matches!(
                        right,
                        Type::WindowPosition | Type::WindowDirection | Type::WindowAttention
                    ) {
                        return Err(Error::new(
                            "E153",
                            span,
                            format!("`{}` values do not support comparisons", left.display()),
                        ));
                    }
                    if !matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
                        && matches!(
                            left,
                            Type::Padding
                                | Type::Background
                                | Type::Gradient
                                | Type::LinearGradient
                                | Type::ColorStop
                                | Type::Font
                                | Type::FontFamily
                                | Type::FontWeight
                                | Type::FontStretch
                                | Type::FontStyle
                                | Type::ThemeMode
                                | Type::TextAlignment
                                | Type::TextShaping
                                | Type::TextWrapping
                                | Type::TextLineHeight
                                | Type::Border
                                | Type::Radius
                                | Type::Shadow
                                | Type::Point
                                | Type::PointU32
                                | Type::Vector
                                | Type::Size
                                | Type::Rectangle
                                | Type::RectangleU32
                                | Type::Transformation
                                | Type::ScrollDelta
                                | Type::EventStatus
                                | Type::WindowLevel
                                | Type::WindowMode
                        )
                    {
                        return Err(Error::new(
                            "E153",
                            span,
                            format!(
                                "operator `{op:?}` does not accept `{}` and `{}`",
                                left.display(),
                                right.display()
                            ),
                        ));
                    }
                    if !matches!((&left, &right), (Type::Degrees | Type::Radians, Type::F64)) {
                        require_type(&left, &right, span)?;
                    }
                    Ok(Type::Bool)
                }
                _ => {
                    if left == Type::I64
                        && right == Type::I64
                        && matches!(
                            op,
                            BinaryOp::Add
                                | BinaryOp::Sub
                                | BinaryOp::Mul
                                | BinaryOp::Div
                                | BinaryOp::Rem
                        )
                    {
                        let right = constant_i64(right_expr);
                        if matches!(op, BinaryOp::Div | BinaryOp::Rem) && right == Some(0) {
                            return Err(Error::new(
                                "E153",
                                span,
                                "integer division and remainder require a non-zero divisor",
                            ));
                        }
                        if let (Some(left), Some(right)) = (constant_i64(left_expr), right)
                            && checked_integer_arithmetic(left, *op, right).is_none()
                        {
                            return Err(Error::new("E153", span, "integer arithmetic overflows"));
                        }
                    }
                    let output = arithmetic_type(&left, *op, &right).ok_or_else(|| {
                        Error::new(
                            "E153",
                            span,
                            format!(
                                "operator `{op:?}` does not accept `{}` and `{}`",
                                left.display(),
                                right.display()
                            ),
                        )
                    })?;
                    let cast = if left == Type::F64 && right == Type::Radians {
                        Some(left_expr)
                    } else if right == Type::F64
                        && matches!(
                            left,
                            Type::Pixels
                                | Type::Degrees
                                | Type::Radians
                                | Type::Radius
                                | Type::Vector
                                | Type::Size
                                | Type::Rectangle
                        )
                    {
                        Some(right_expr)
                    } else {
                        None
                    };
                    if let Some(value) = cast {
                        require_f32_literal_range(
                            value,
                            f64::NEG_INFINITY,
                            None,
                            "native arithmetic value",
                            span,
                        )?;
                    }
                    Ok(output)
                }
            }
        }
    }
}

fn ui_enum_variant<'a>(
    document: &'a Document,
    enum_name: &str,
    variant_name: &str,
) -> Option<(&'a UiEnum, &'a UiEnumVariant)> {
    let item = document.enums.iter().find(|item| item.name == enum_name)?;
    let variant = item
        .variants
        .iter()
        .find(|variant| variant.name == variant_name)?;
    Some((item, variant))
}

pub(in crate::check) fn contains_ui_enum(ty: &Type, document: &Document) -> bool {
    match ty {
        Type::Named(name) => document.enums.iter().any(|item| item.name == *name),
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) | Type::Animation(inner) => {
            contains_ui_enum(inner, document)
        }
        Type::Result(output, error) => {
            contains_ui_enum(output, document) || contains_ui_enum(error, document)
        }
        _ => false,
    }
}

mod fields;
mod validation;

pub(super) use fields::*;
pub(super) use validation::*;

fn constant_i64(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::I64(value) => Some(*value),
        Expr::Unary {
            op: UnaryOp::Neg,
            value,
        } => constant_i64(value)?.checked_neg(),
        Expr::Binary { left, op, right } => {
            checked_integer_arithmetic(constant_i64(left)?, *op, constant_i64(right)?)
        }
        _ => None,
    }
}

fn checked_integer_arithmetic(left: i64, op: BinaryOp, right: i64) -> Option<i64> {
    match op {
        BinaryOp::Add => left.checked_add(right),
        BinaryOp::Sub => left.checked_sub(right),
        BinaryOp::Mul => left.checked_mul(right),
        BinaryOp::Div => left.checked_div(right),
        BinaryOp::Rem => left.checked_rem(right),
        _ => None,
    }
}

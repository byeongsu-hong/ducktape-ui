use super::*;

pub(in crate::check) fn check_canvas_commands(
    commands: &[CanvasCommand],
    env: &dyn ExprTypeEnv,
    document: &Document,
) -> Result<(), Error> {
    for command in commands {
        match command {
            CanvasCommand::Rectangle {
                x,
                y,
                width,
                height,
                radius,
                paint,
                span,
            } => {
                check_canvas_number(x, env, document, span, "rectangle x", None)?;
                check_canvas_number(y, env, document, span, "rectangle y", None)?;
                check_canvas_number(width, env, document, span, "rectangle width", Some(0.0))?;
                check_canvas_number(height, env, document, span, "rectangle height", Some(0.0))?;
                check_canvas_radius(radius, env, document, span)?;
                check_canvas_paint(paint, env, document, span)?;
            }
            CanvasCommand::Circle {
                x,
                y,
                radius,
                paint,
                span,
            } => {
                check_canvas_number(x, env, document, span, "circle x", None)?;
                check_canvas_number(y, env, document, span, "circle y", None)?;
                check_canvas_number(radius, env, document, span, "circle radius", Some(0.0))?;
                check_canvas_paint(paint, env, document, span)?;
            }
            CanvasCommand::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                span,
            } => {
                for (value, label) in [
                    (x1, "line x1"),
                    (y1, "line y1"),
                    (x2, "line x2"),
                    (y2, "line y2"),
                ] {
                    check_canvas_number(value, env, document, span, label, None)?;
                }
                check_canvas_stroke(stroke, env, document, span)?;
            }
            CanvasCommand::Text {
                value,
                x,
                y,
                max_width,
                color,
                size,
                line_height,
                font,
                span,
                ..
            } => {
                let ty = expr_type(value, env, document, span)?;
                if !matches!(ty, Type::Str | Type::I64 | Type::F64) {
                    return Err(type_error(span, &Type::Str, &ty)
                        .hint("canvas text accepts str, i64, or f64"));
                }
                check_canvas_number(x, env, document, span, "text x", None)?;
                check_canvas_number(y, env, document, span, "text y", None)?;
                if let Some(value) = max_width {
                    check_canvas_number(value, env, document, span, "text max width", Some(0.0))?;
                }
                if let Some(value) = size {
                    check_canvas_number(
                        value,
                        env,
                        document,
                        span,
                        "text size",
                        Some(f64::EPSILON),
                    )?;
                }
                if let Some(height) = line_height {
                    let (value, label) = match height {
                        TextLineHeight::Relative(value) => (value, "text line height"),
                        TextLineHeight::Absolute(value) => (value, "text line height pixels"),
                    };
                    check_canvas_number(value, env, document, span, label, Some(f64::EPSILON))?;
                }
                if color
                    .as_ref()
                    .is_some_and(|color| !valid_theme_color(color, document))
                {
                    return Err(Error::new("E190", span, "unknown canvas text color"));
                }
                check_font(font.as_ref(), document, span)?;
            }
            CanvasCommand::Image {
                source,
                x,
                y,
                width,
                height,
                rotation,
                opacity,
                snap,
                radius,
                span,
                ..
            } => {
                let source_ty = expr_type(source, env, document, span)?;
                if !matches!(source_ty, Type::Str | Type::Image) {
                    return Err(type_error(span, &Type::Image, &source_ty)
                        .hint("canvas image accepts a path string or image handle"));
                }
                for (value, label, min) in [
                    (x, "image x", None),
                    (y, "image y", None),
                    (width, "image width", Some(0.0)),
                    (height, "image height", Some(0.0)),
                    (rotation, "image rotation", None),
                    (opacity, "image opacity", Some(0.0)),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
                require_literal_range(opacity, 0.0, Some(1.0), "image opacity", span)?;
                require_type(&expr_type(snap, env, document, span)?, &Type::Bool, span)?;
                check_canvas_radius(radius, env, document, span)?;
            }
            CanvasCommand::Svg {
                source,
                memory,
                x,
                y,
                width,
                height,
                color,
                rotation,
                opacity,
                span,
            } => {
                let source_ty = expr_type(source, env, document, span)?;
                let valid_source = if *memory {
                    matches!(source_ty, Type::Str | Type::Bytes)
                } else {
                    source_ty == Type::Str
                };
                if !valid_source {
                    return Err(type_error(
                        span,
                        if *memory { &Type::Bytes } else { &Type::Str },
                        &source_ty,
                    )
                    .hint("canvas svg accepts a path string, or UTF-8/raw bytes with `memory`"));
                }
                for (value, label, min) in [
                    (x, "svg x", None),
                    (y, "svg y", None),
                    (width, "svg width", Some(0.0)),
                    (height, "svg height", Some(0.0)),
                    (rotation, "svg rotation", None),
                    (opacity, "svg opacity", Some(0.0)),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
                require_literal_range(opacity, 0.0, Some(1.0), "svg opacity", span)?;
                if color
                    .as_ref()
                    .is_some_and(|color| !valid_theme_color(color, document))
                {
                    return Err(Error::new("E190", span, "unknown canvas svg color"));
                }
            }
            CanvasCommand::Path {
                segments,
                paint,
                span,
            } => {
                check_canvas_path(segments, env, document, span)?;
                check_canvas_paint(paint, env, document, span)?;
            }
            CanvasCommand::Group {
                transform,
                commands,
                span,
            } => {
                for (value, label) in [
                    (&transform.x, "group x"),
                    (&transform.y, "group y"),
                    (&transform.rotate, "group rotation"),
                ] {
                    if let Some(value) = value {
                        check_canvas_number(value, env, document, span, label, None)?;
                    }
                }
                for (value, label) in [
                    (&transform.scale, "group scale"),
                    (&transform.scale_x, "group x scale"),
                    (&transform.scale_y, "group y scale"),
                ] {
                    if let Some(value) = value {
                        check_canvas_number(value, env, document, span, label, Some(f64::EPSILON))?;
                    }
                }
                if let Some([x, y, width, height]) = &transform.clip {
                    check_canvas_number(x, env, document, span, "clip x", None)?;
                    check_canvas_number(y, env, document, span, "clip y", None)?;
                    check_canvas_number(width, env, document, span, "clip width", Some(0.0))?;
                    check_canvas_number(height, env, document, span, "clip height", Some(0.0))?;
                }
                check_canvas_commands(commands, env, document)?;
            }
            CanvasCommand::If {
                condition,
                commands,
                span,
            } => {
                require_type(
                    &expr_type(condition, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
                check_canvas_commands(commands, env, document)?;
            }
            CanvasCommand::For {
                item,
                items,
                commands,
                span,
            } => {
                let Type::List(inner) = expr_type(items, env, document, span)? else {
                    return Err(Error::new(
                        "E190",
                        span,
                        "canvas for expects a list expression",
                    ));
                };
                let mut child_env = ScopedTypeEnv::new(env);
                child_env.insert(item.clone(), *inner);
                check_canvas_commands(commands, &child_env, document)?;
            }
        }
    }
    Ok(())
}

pub(in crate::check) fn check_canvas_path(
    segments: &[CanvasPathSegment],
    env: &dyn ExprTypeEnv,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for segment in segments {
        match segment {
            CanvasPathSegment::Move(x, y) | CanvasPathSegment::Line(x, y) => {
                check_canvas_number(x, env, document, span, "path x", None)?;
                check_canvas_number(y, env, document, span, "path y", None)?;
            }
            CanvasPathSegment::Arc {
                x,
                y,
                radius,
                start,
                end,
            } => {
                for (value, label, min) in [
                    (x, "arc x", None),
                    (y, "arc y", None),
                    (radius, "arc radius", Some(0.0)),
                    (start, "arc start", None),
                    (end, "arc end", None),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
            }
            CanvasPathSegment::ArcTo {
                ax,
                ay,
                bx,
                by,
                radius,
            } => {
                for (value, label, min) in [
                    (ax, "arc-to ax", None),
                    (ay, "arc-to ay", None),
                    (bx, "arc-to bx", None),
                    (by, "arc-to by", None),
                    (radius, "arc-to radius", Some(0.0)),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
            }
            CanvasPathSegment::Ellipse {
                x,
                y,
                radius_x,
                radius_y,
                rotation,
                start,
                end,
            } => {
                for (value, label, min) in [
                    (x, "ellipse x", None),
                    (y, "ellipse y", None),
                    (radius_x, "ellipse x radius", Some(0.0)),
                    (radius_y, "ellipse y radius", Some(0.0)),
                    (rotation, "ellipse rotation", None),
                    (start, "ellipse start", None),
                    (end, "ellipse end", None),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
            }
            CanvasPathSegment::Bezier {
                control_ax,
                control_ay,
                control_bx,
                control_by,
                x,
                y,
            } => {
                for value in [control_ax, control_ay, control_bx, control_by, x, y] {
                    check_canvas_number(value, env, document, span, "bezier coordinate", None)?;
                }
            }
            CanvasPathSegment::Quadratic {
                control_x,
                control_y,
                x,
                y,
            } => {
                for value in [control_x, control_y, x, y] {
                    check_canvas_number(value, env, document, span, "quadratic coordinate", None)?;
                }
            }
            CanvasPathSegment::Rectangle {
                x,
                y,
                width,
                height,
            }
            | CanvasPathSegment::RoundedRectangle {
                x,
                y,
                width,
                height,
                ..
            } => {
                check_canvas_number(x, env, document, span, "path rectangle x", None)?;
                check_canvas_number(y, env, document, span, "path rectangle y", None)?;
                check_canvas_number(
                    width,
                    env,
                    document,
                    span,
                    "path rectangle width",
                    Some(0.0),
                )?;
                check_canvas_number(
                    height,
                    env,
                    document,
                    span,
                    "path rectangle height",
                    Some(0.0),
                )?;
                if let CanvasPathSegment::RoundedRectangle { radius, .. } = segment {
                    check_canvas_radius(radius, env, document, span)?;
                }
            }
            CanvasPathSegment::Circle { x, y, radius } => {
                check_canvas_number(x, env, document, span, "path circle x", None)?;
                check_canvas_number(y, env, document, span, "path circle y", None)?;
                check_canvas_number(radius, env, document, span, "path circle radius", Some(0.0))?;
            }
            CanvasPathSegment::Close => {}
        }
    }
    Ok(())
}

pub(in crate::check) fn check_canvas_paint(
    paint: &CanvasPaint,
    env: &dyn ExprTypeEnv,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if let Some(fill) = &paint.fill {
        check_background_value(fill, env, document, span, "E190", "canvas fill")?;
    }
    if let Some(stroke) = &paint.stroke {
        check_canvas_stroke(stroke, env, document, span)?;
    }
    Ok(())
}

pub(in crate::check) fn check_canvas_stroke(
    stroke: &CanvasStroke,
    env: &dyn ExprTypeEnv,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    check_background_value(&stroke.style, env, document, span, "E190", "canvas stroke")?;
    check_canvas_number(
        &stroke.width,
        env,
        document,
        span,
        "stroke width",
        Some(0.0),
    )?;
    require_type(
        &expr_type(&stroke.dash_offset, env, document, span)?,
        &Type::I64,
        span,
    )?;
    require_literal_range(&stroke.dash_offset, 0.0, None, "dash offset", span)?;
    for value in &stroke.dash {
        check_canvas_number(value, env, document, span, "dash segment", Some(0.0))?;
    }
    Ok(())
}

pub(in crate::check) fn check_canvas_radius(
    radius: &CanvasRadius,
    env: &dyn ExprTypeEnv,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for value in [
        &radius.all,
        &radius.top_left,
        &radius.top_right,
        &radius.bottom_right,
        &radius.bottom_left,
    ]
    .into_iter()
    .flatten()
    {
        check_canvas_number(value, env, document, span, "corner radius", Some(0.0))?;
    }
    Ok(())
}

pub(in crate::check) fn check_canvas_number(
    value: &Expr,
    env: &dyn ExprTypeEnv,
    document: &Document,
    span: &Span,
    label: &str,
    min: Option<f64>,
) -> Result<(), Error> {
    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
    require_f32_literal_range(value, min.unwrap_or(f64::NEG_INFINITY), None, label, span)
}

use super::*;

pub(in crate::check) fn infer_structure_group(
    node: &ViewNode,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
        ViewNode::Theme {
            id,
            preset,
            text,
            background,
            content,
            span,
            ..
        } => {
            check_id(id, env, document, ids, span)?;
            if let ThemePreset::Factory(factory) = preset {
                let function =
                    extern_function(document, &factory.function, ExternKind::Theme, span)?;
                check_call_args(function, &factory.args, env, document, span)?;
            }
            if let Some(color) = text {
                require_theme_color(color, document, span, "E137", "nested theme text")?;
            }
            if let Some(background) = background {
                check_background_value(
                    background,
                    env,
                    document,
                    span,
                    "E137",
                    "nested theme background",
                )?;
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Float {
            id,
            scale,
            x,
            y,
            style,
            content,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            require_type(&expr_type(scale, env, document, span)?, &Type::F64, span)?;
            let mut translate_env = env.clone();
            for name in [
                "original_x",
                "original_y",
                "original_width",
                "original_height",
                "viewport_x",
                "viewport_y",
                "viewport_width",
                "viewport_height",
            ] {
                translate_env.insert(name.to_owned(), Type::F64);
            }
            for value in [x, y] {
                require_f32_value(value, &translate_env, document, "float translation", span)?;
            }
            require_f32_literal_range(scale, f64::EPSILON, None, "float scale", span)?;
            check_float_style_options(style, env, document, span)?;
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Pin {
            id,
            width,
            height,
            x,
            y,
            content,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            for value in [x, y] {
                require_f32_value(value, env, document, "pin position", span)?;
            }
            for length in [width, height].into_iter().flatten() {
                check_length_value(length, env, document, span, "pin size")?;
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Sensor {
            id,
            options,
            content,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            for (route, label) in [
                (&options.show, "sensor show"),
                (&options.resize, "sensor resize"),
            ]
            .into_iter()
            .filter_map(|(route, label)| route.as_ref().map(|route| (route, label)))
            {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64],
                    env,
                    document,
                    signatures,
                    label,
                )?;
            }
            if let Some(route) = &options.hide {
                infer_route(route, None, env, document, signatures)?;
            }
            if let Some(key) = &options.key {
                let ty = expr_type(key, env, document, span)?;
                if !matches!(
                    ty,
                    Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
                ) {
                    return Err(Error::new(
                        "E129",
                        span,
                        "sensor key must be bool, i64, f64, str, or an extern type",
                    ));
                }
            }
            if let Some(distance) = &options.anticipate {
                require_nonnegative_f64(distance, env, document, "sensor anticipation", span)?;
            }
            if let Some(delay) = &options.delay_ms {
                require_type(&expr_type(delay, env, document, span)?, &Type::I64, span)?;
                if matches!(delay, Expr::I64(value) if *value < 0) {
                    return Err(Error::new("E128", span, "sensor delay cannot be negative"));
                }
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Responsive {
            id,
            content,
            width,
            height,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            for length in [width, height].into_iter().flatten() {
                check_length_value(length, env, document, span, "responsive size")?;
            }
            match content {
                ResponsiveContent::Breakpoint {
                    breakpoint,
                    narrow,
                    wide,
                } => {
                    require_type(
                        &expr_type(breakpoint, env, document, span)?,
                        &Type::F64,
                        span,
                    )?;
                    require_f32_literal_range(
                        breakpoint,
                        f64::EPSILON,
                        None,
                        "responsive breakpoint",
                        span,
                    )?;
                    infer_view(narrow, env, document, signatures, ids)?;
                    infer_view(wide, env, document, signatures, ids)?;
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let mut child_env = env.clone();
                    child_env.insert(width.clone(), Type::F64);
                    child_env.insert(height.clone(), Type::F64);
                    infer_view(content, &child_env, document, signatures, ids)?;
                }
            }
        }
        _ => return Ok(false),
    };
    Ok(true)
}

use super::*;

pub(in crate::check) fn infer_content_group(
    node: &ViewNode,
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
        ViewNode::Text {
            value,
            id,
            options,
            styles,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let ty = expr_type(value, env, document, span)?;
            if !matches!(ty, Type::Str | Type::I64 | Type::F64) {
                return Err(type_error(span, &Type::Str, &ty).hint("text accepts str, i64, or f64"));
            }
            check_text_options(options, env, document, span)?;
            check_text_tracking(options, value, span)?;
            check_styles(styles, document, span, StyleTarget::Text(options))?;
        }
        ViewNode::RichText {
            id,
            options,
            color,
            spans,
            styles,
            route,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            check_text_options(options, env, document, span)?;
            check_styles(
                styles,
                document,
                span,
                StyleTarget::RichText {
                    typed_color: color.is_some(),
                    typed_size: options.size.is_some(),
                    typed_line_height: options.line_height.is_some(),
                    typed_font: options.font.is_some(),
                },
            )?;
            if color
                .as_ref()
                .is_some_and(|color| !valid_theme_color(color, document))
            {
                return Err(Error::new("E186", span, "unknown rich-text color"));
            }
            let mut has_links = false;
            for item in spans {
                let ty = expr_type(&item.value, env, document, &item.span)?;
                if !matches!(ty, Type::Str | Type::I64 | Type::F64 | Type::Bool) {
                    return Err(Error::new(
                        "E186",
                        &item.span,
                        "span text must be str, i64, f64, or bool",
                    ));
                }
                check_font(item.options.font.as_ref(), document, &item.span)?;
                check_styles(
                    &item.styles,
                    document,
                    &item.span,
                    StyleTarget::RichSpan(&item.options),
                )?;
                for color in [&item.options.color, &item.options.border]
                    .into_iter()
                    .flatten()
                {
                    require_theme_color(color, document, &item.span, "E186", "span")?;
                }
                if let Some(background) = &item.options.background {
                    check_background_value(
                        background,
                        env,
                        document,
                        &item.span,
                        "E186",
                        "span background",
                    )?;
                }
                for (value, label, min) in [
                    (item.options.size.as_ref(), "span size", f64::EPSILON),
                    (
                        item.options
                            .line_height
                            .as_ref()
                            .map(|height| match height {
                                TextLineHeight::Relative(value)
                                | TextLineHeight::Absolute(value) => value,
                            }),
                        "span line height",
                        f64::EPSILON,
                    ),
                    (item.options.border_width.as_ref(), "span border width", 0.0),
                    (item.options.radius.as_ref(), "span radius", 0.0),
                    (item.options.radius_top_left.as_ref(), "span radius", 0.0),
                    (item.options.radius_top_right.as_ref(), "span radius", 0.0),
                    (
                        item.options.radius_bottom_right.as_ref(),
                        "span radius",
                        0.0,
                    ),
                    (item.options.radius_bottom_left.as_ref(), "span radius", 0.0),
                    (item.options.padding.all.as_ref(), "span padding", 0.0),
                    (item.options.padding.x.as_ref(), "span padding", 0.0),
                    (item.options.padding.y.as_ref(), "span padding", 0.0),
                    (item.options.padding.top.as_ref(), "span padding", 0.0),
                    (item.options.padding.right.as_ref(), "span padding", 0.0),
                    (item.options.padding.bottom.as_ref(), "span padding", 0.0),
                    (item.options.padding.left.as_ref(), "span padding", 0.0),
                ] {
                    if let Some(value) = value {
                        require_type(
                            &expr_type(value, env, document, &item.span)?,
                            &Type::F64,
                            &item.span,
                        )?;
                        require_f32_literal_range(value, min, None, label, &item.span)?;
                    }
                }
                for value in [&item.options.underline, &item.options.strikethrough]
                    .into_iter()
                    .flatten()
                {
                    require_type(
                        &expr_type(value, env, document, &item.span)?,
                        &Type::Bool,
                        &item.span,
                    )?;
                }
                if let Some(link) = &item.options.link {
                    has_links = true;
                    require_type(
                        &expr_type(link, env, document, &item.span)?,
                        &Type::Str,
                        &item.span,
                    )?;
                }
            }
            match (has_links, route) {
                (true, Some(route)) => {
                    infer_route(route, Some(Type::Str), env, document, signatures)?;
                }
                (true, None) => {
                    return Err(Error::new(
                        "E186",
                        span,
                        "rich-text spans with `link=` require `-> handler _`",
                    ));
                }
                (false, Some(_)) => {
                    return Err(Error::new(
                        "E186",
                        span,
                        "rich-text without linked spans cannot emit a route",
                    ));
                }
                (false, None) => {}
            }
        }
        ViewNode::Input {
            id,
            binding,
            disabled,
            options,
            styles,
            span,
            ..
        } => {
            record_read(binding, span);
            record_write(binding, span);
            check_id(id, env, document, ids, span)?;
            let Some(binding_ty) = env.get_type(binding) else {
                return Err(Error::new(
                    "E120",
                    span,
                    format!("unknown binding `{binding}`"),
                ));
            };
            require_type(binding_ty, &Type::Str, span)?;
            if let Some(disabled) = disabled {
                let ty = expr_type(disabled, env, document, span)?;
                require_type(&ty, &Type::Bool, span)?;
            }
            check_accessibility_options(&options.accessibility, env, document, span)?;
            if let Some(secure) = &options.secure {
                require_type(&expr_type(secure, env, document, span)?, &Type::Bool, span)?;
            }
            if let Some(route) = &options.change {
                infer_route(route, Some(Type::Str), env, document, signatures)?;
            }
            if let Some(route) = &options.submit {
                infer_route(route, None, env, document, signatures)?;
            }
            if let Some(route) = &options.paste {
                infer_route(route, Some(Type::Str), env, document, signatures)?;
            }
            if let Some(length) = &options.width {
                check_length_value(length, env, document, span, "input width")?;
            }
            for (value, label, min) in [
                (&options.padding, "input padding", 0.0),
                (&options.text_size, "input text size", f64::EPSILON),
                (&options.line_height, "input line height", f64::EPSILON),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_f32_literal_range(value, min, None, label, span)?;
                }
            }
            check_font(options.font.as_ref(), document, span)?;
            check_text_input_icon(options.icon.as_ref(), env, document, "input")?;
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::InputStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_text_input_styles(&options.style, env, document, span, "input")?;
            check_styles(styles, document, span, StyleTarget::Input(options))?;
        }
        _ => return Ok(false),
    };
    Ok(true)
}

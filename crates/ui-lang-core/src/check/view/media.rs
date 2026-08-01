use super::*;

pub(in crate::check) fn infer_media_group(
    node: &ViewNode,
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
        ViewNode::Media {
            kind,
            id,
            source,
            options,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            check_accessibility_options(&options.accessibility, env, document, span)?;
            if options.accessibility.label.is_none() && options.accessibility.description.is_some()
            {
                return Err(Error::new(
                    "E105",
                    span,
                    "media `description=...` requires an accessibility `label=...`",
                ));
            }
            let source_ty = expr_type(source, env, document, span)?;
            let valid_source = match kind {
                MediaKind::Image | MediaKind::Viewer => {
                    source_ty == Type::Str || source_ty == Type::Image
                }
                MediaKind::Svg if options.svg_memory => {
                    source_ty == Type::Str || source_ty == Type::Bytes
                }
                MediaKind::Svg => source_ty == Type::Str,
            };
            if !valid_source {
                let error = type_error(
                    span,
                    if matches!(kind, MediaKind::Image | MediaKind::Viewer) {
                        &Type::Image
                    } else if options.svg_memory {
                        &Type::Bytes
                    } else {
                        &Type::Str
                    },
                    &source_ty,
                );
                return Err(if matches!(kind, MediaKind::Image | MediaKind::Viewer) {
                    error.hint("image and viewer accept a path string or image handle")
                } else if options.svg_memory {
                    error.hint("SVG memory accepts UTF-8 text or raw bytes")
                } else {
                    error
                });
            }
            for length in [&options.width, &options.height].into_iter().flatten() {
                check_length_value(length, env, document, span, "media size")?;
            }
            if let Some(rotation) = &options.rotation {
                require_type(
                    &expr_type(rotation, env, document, span)?,
                    &Type::Rotation,
                    span,
                )?;
            }
            if let Some(fit) = &options.fit {
                require_type(
                    &expr_type(fit, env, document, span)?,
                    &Type::ContentFit,
                    span,
                )?;
            }
            for (value, label, min, max) in [
                (&options.opacity, "opacity", Some(0.0), Some(1.0)),
                (&options.scale, "scale", Some(f64::EPSILON), None),
                (&options.radius, "radius", Some(0.0), None),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_f32_literal_range(
                        value,
                        min.unwrap_or(f64::NEG_INFINITY),
                        max,
                        label,
                        span,
                    )?;
                }
            }
            for value in [
                &options.radius_top_left,
                &options.radius_top_right,
                &options.radius_bottom_right,
                &options.radius_bottom_left,
            ]
            .into_iter()
            .flatten()
            {
                require_nonnegative_f64(value, env, document, "radius", span)?;
            }
            if let Some(expand) = &options.expand {
                require_type(&expr_type(expand, env, document, span)?, &Type::Bool, span)?;
            }
            if let Some(crop) = &options.crop {
                for value in crop {
                    require_type(&expr_type(value, env, document, span)?, &Type::I64, span)?;
                    require_literal_range(
                        value,
                        0.0,
                        Some(u32::MAX as f64),
                        "image crop coordinate",
                        span,
                    )?;
                }
            }
            for (value, label, min) in [
                (&options.padding, "viewer padding", 0.0),
                (&options.min_scale, "viewer minimum scale", f64::EPSILON),
                (&options.max_scale, "viewer maximum scale", f64::EPSILON),
                (&options.scale_step, "viewer scale step", f64::EPSILON),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_f32_literal_range(value, min, None, label, span)?;
                }
            }
            let min_scale = options.min_scale.as_ref().map_or(Some(0.25), f64_literal);
            let max_scale = options.max_scale.as_ref().map_or(Some(10.0), f64_literal);
            if matches!((min_scale, max_scale), (Some(min), Some(max)) if min > max) {
                return Err(Error::new(
                    "E128",
                    span,
                    "viewer minimum scale cannot exceed maximum scale",
                ));
            }
            for color in options
                .svg_color
                .iter()
                .chain(options.svg_hover_color.iter().flatten())
            {
                require_theme_color(color, document, span, "E129", "svg")?;
            }
            if let Some(style) = &options.svg_style {
                let function =
                    extern_function(document, &style.function, ExternKind::SvgStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
        }
        ViewNode::Tooltip {
            id,
            options,
            content,
            tip,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            for (value, label) in [
                (&options.gap, "tooltip gap"),
                (&options.padding, "tooltip padding"),
            ] {
                require_nonnegative_f64(value, env, document, label, span)?;
            }
            require_type(
                &expr_type(&options.delay_ms, env, document, span)?,
                &Type::I64,
                span,
            )?;
            if matches!(&options.delay_ms, Expr::I64(value) if *value < 0) {
                return Err(Error::new("E128", span, "tooltip delay cannot be negative"));
            }
            require_type(
                &expr_type(&options.snap, env, document, span)?,
                &Type::Bool,
                span,
            )?;
            if let Some(background) = &options.background {
                check_background_value(
                    background,
                    env,
                    document,
                    span,
                    "E129",
                    "tooltip background",
                )?;
            }
            for color in [
                &options.text_color,
                &options.border_color,
                &options.shadow_color,
            ]
            .into_iter()
            .flatten()
            {
                require_theme_color(color, document, span, "E129", "tooltip")?;
            }
            for (value, label) in [
                (&options.border_width, "tooltip border width"),
                (&options.radius, "tooltip radius"),
                (&options.radius_top_left, "tooltip radius"),
                (&options.radius_top_right, "tooltip radius"),
                (&options.radius_bottom_right, "tooltip radius"),
                (&options.radius_bottom_left, "tooltip radius"),
                (&options.shadow_blur, "tooltip shadow blur"),
            ] {
                if let Some(value) = value {
                    require_nonnegative_f64(value, env, document, label, span)?;
                }
            }
            for value in [&options.shadow_x, &options.shadow_y].into_iter().flatten() {
                require_f32_value(value, env, document, "tooltip shadow offset", span)?;
            }
            if let Some(pixel_snap) = &options.pixel_snap {
                require_type(
                    &expr_type(pixel_snap, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::ContainerStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            infer_view(content, env, document, signatures, ids)?;
            infer_view(tip, env, document, signatures, ids)?;
        }
        ViewNode::MouseArea {
            id,
            options,
            content,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            if let Some(interaction) = &options.interaction_expr {
                require_type(
                    &expr_type(interaction, env, document, span)?,
                    &Type::MouseInteraction,
                    span,
                )?;
            }
            for route in [
                &options.press,
                &options.release,
                &options.double_click,
                &options.right_press,
                &options.right_release,
                &options.middle_press,
                &options.middle_release,
                &options.enter,
                &options.exit,
            ]
            .into_iter()
            .flatten()
            {
                infer_route(route, None, env, document, signatures)?;
            }
            if let Some(route) = &options.move_route {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64],
                    env,
                    document,
                    signatures,
                    "mouse move",
                )?;
            }
            if let Some(route) = &options.scroll {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64, Type::Bool],
                    env,
                    document,
                    signatures,
                    "mouse scroll",
                )?;
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::ResizeHandle {
            id,
            options,
            content,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            if let Some(route) = &options.drag {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64],
                    env,
                    document,
                    signatures,
                    "resize-handle drag",
                )?;
            }
            for route in [&options.press, &options.release].into_iter().flatten() {
                infer_route(route, None, env, document, signatures)?;
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Canvas {
            id,
            options,
            locals,
            commands,
            events,
            span,
        } => {
            let canvas_analysis_guard = expr::HandlerAnalysisGuard::start();
            check_id(id, env, document, ids, span)?;
            for length in [&options.width, &options.height].into_iter().flatten() {
                check_length_value(length, env, document, span, "canvas size")?;
            }
            if let Some(dependency) = &options.cache {
                let ty = expr_type(dependency, env, document, span)?;
                if !lazy_hashable(&ty) {
                    return Err(Error::new(
                        "E190",
                        span,
                        format!(
                            "canvas cache dependency type `{}` does not implement stable hashing",
                            ty.display()
                        ),
                    )
                    .hint("use bool, i64, str, bytes, an extern Hash + Clone type, or a list/optional of those"));
                }
            }
            if options.cache_group.is_some() && options.cache.is_none() {
                return Err(Error::new(
                    "E190",
                    span,
                    "canvas cache-group requires `cache=`",
                ));
            }
            if let Some(capture) = &options.capture {
                require_type(&expr_type(capture, env, document, span)?, &Type::Bool, span)?;
            }
            for route in [&options.enter, &options.exit].into_iter().flatten() {
                infer_route(route, None, env, document, signatures)?;
            }
            for route in [
                &options.press,
                &options.release,
                &options.right_press,
                &options.right_release,
                &options.middle_press,
                &options.middle_release,
                &options.move_route,
            ]
            .into_iter()
            .flatten()
            {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64],
                    env,
                    document,
                    signatures,
                    "canvas pointer event",
                )?;
            }
            if let Some(route) = &options.scroll {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64, Type::Bool],
                    env,
                    document,
                    signatures,
                    "canvas scroll",
                )?;
            }
            let known = document
                .structs
                .iter()
                .map(|item| item.name.as_str())
                .collect::<HashSet<_>>();
            let mut canvas_env = scoped_view_env(env);
            let mut local_types = HashMap::new();
            for local in locals {
                if matches!(
                    local.name.as_str(),
                    "cache" | "cache_key" | "inside" | "canvas_width" | "canvas_height"
                ) {
                    return Err(Error::new(
                        "E190",
                        &local.span,
                        format!("canvas state name `{}` is reserved", local.name),
                    ));
                }
                if env.contains_type(&local.name) {
                    return Err(Error::new(
                        "E190",
                        &local.span,
                        format!(
                            "canvas state `{}` conflicts with an app state or component parameter",
                            local.name
                        ),
                    ));
                }
                if local_types
                    .insert(local.name.clone(), local.ty.clone())
                    .is_some()
                {
                    return Err(Error::new(
                        "E190",
                        &local.span,
                        format!("duplicate canvas state `{}`", local.name),
                    ));
                }
                if matches!(local.ty, Type::Animation(_)) {
                    return Err(Error::new(
                        "E190",
                        &local.span,
                        "canvas-local animation is not supported; declare it in app state",
                    ));
                }
                check_declared_type(
                    &local.ty,
                    &local.span,
                    &known,
                    document
                        .theme_contract
                        .as_ref()
                        .map(|item| item.name.as_str()),
                )?;
                let actual = expr_type(&local.initial, &HashMap::new(), document, &local.span)?;
                if let Type::Combo(expected) = &local.ty {
                    let Type::List(actual) = actual else {
                        return Err(Error::new(
                            "E104",
                            &local.span,
                            "combo canvas state must be initialized with a list",
                        ));
                    };
                    require_type(&actual, expected, &local.span)?;
                } else {
                    let text_initial =
                        matches!(local.ty, Type::Markdown | Type::Editor) && actual == Type::Str;
                    if actual != Type::Unknown && !text_initial && !compatible(&local.ty, &actual) {
                        return Err(type_error(&local.span, &local.ty, &actual));
                    }
                }
                canvas_env.insert(local.name.clone(), local.ty.clone());
            }
            canvas_env.insert("canvas_width".into(), Type::F64);
            canvas_env.insert("canvas_height".into(), Type::F64);
            if let Some(interaction) = &options.interaction_expr {
                let actual = expr_type(interaction, &canvas_env, document, span)?;
                if !matches!(actual, Type::Str | Type::MouseInteraction) {
                    return Err(Error::new(
                        "E101",
                        span,
                        format!(
                            "expected `str` or `mouse-interaction`, got `{}`",
                            actual.display()
                        ),
                    ));
                }
                if actual == Type::Str
                    && let Expr::Str(value) = interaction
                    && !valid_canvas_cursor(value)
                {
                    return Err(Error::new(
                        "E190",
                        span,
                        format!("unknown canvas cursor `{value}`"),
                    ));
                }
            }
            if let Some(outside) = &options.interaction_outside {
                if options.interaction.is_none() && options.interaction_expr.is_none() {
                    return Err(Error::new(
                        "E190",
                        span,
                        "canvas cursor-outside requires `cursor=`",
                    ));
                }
                require_type(
                    &expr_type(outside, &canvas_env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            check_canvas_commands(commands, &canvas_env, document)?;
            let mut seen = HashSet::new();
            for event in events {
                let name = canvas_event_name(&event.source).ok_or_else(|| {
                    Error::new("E190", &event.span, "invalid canvas event source")
                })?;
                if !seen.insert(name) {
                    return Err(Error::new(
                        "E190",
                        &event.span,
                        format!("duplicate canvas event `{name}`"),
                    ));
                }
                let payloads =
                    native_subscription_payloads(&event.source, false).ok_or_else(|| {
                        Error::new("E190", &event.span, "invalid canvas event source")
                    })?;
                if !event.route_payload
                    && !event.bindings.is_empty()
                    && event.bindings.len() != payloads.len()
                {
                    return Err(Error::new(
                        "E190",
                        &event.span,
                        format!(
                            "canvas event `{name}` exposes {} values, but {} bindings were declared",
                            payloads.len(),
                            event.bindings.len()
                        ),
                    ));
                }
                let mut event_env = scoped_view_env(&canvas_env);
                for (binding, ty) in event.bindings.iter().zip(&payloads) {
                    if event_env.contains_type(binding) {
                        return Err(Error::new(
                            "E190",
                            &event.span,
                            format!(
                                "canvas event binding `{binding}` conflicts with existing state"
                            ),
                        ));
                    }
                    event_env.insert(binding.clone(), ty.clone());
                }
                for update in &event.updates {
                    let expected = local_types.get(&update.name).ok_or_else(|| {
                        Error::new(
                            "E190",
                            &update.span,
                            format!("unknown canvas state `{}`", update.name),
                        )
                    })?;
                    let actual = expr_type(&update.value, &event_env, document, &update.span)?;
                    require_type(&actual, expected, &update.span)?;
                }
                if event.updates.is_empty() && event.action.is_none() && !event.capture {
                    return Err(Error::new(
                        "E190",
                        &event.span,
                        "canvas event block has no effect",
                    ));
                };
                let Some(CanvasEventAction::Route(route)) = &event.action else {
                    continue;
                };
                if event.route_payload {
                    infer_ordered_payload_route(
                        route,
                        &payloads,
                        env,
                        document,
                        signatures,
                        "canvas event",
                    )?;
                } else {
                    if route
                        .args
                        .iter()
                        .any(|arg| matches!(arg, RouteArg::Payload))
                    {
                        return Err(Error::new(
                            "E190",
                            &event.span,
                            "canvas event block `emit` uses named bindings instead of `_`",
                        ));
                    }
                    infer_route(route, None, &event_env, document, signatures)?;
                }
            }
            retain_canvas_analyses(span, canvas_analysis_guard.finish())?;
        }
        _ => return Ok(false),
    };
    Ok(true)
}

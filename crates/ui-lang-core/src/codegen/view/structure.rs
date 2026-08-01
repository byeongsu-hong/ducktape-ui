use super::*;

pub(in crate::codegen) fn render_structure(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let id = match node {
        ViewNode::Theme { id, .. }
        | ViewNode::Float { id, .. }
        | ViewNode::Pin { id, .. }
        | ViewNode::Sensor { id, .. }
        | ViewNode::Responsive { id, .. }
        | ViewNode::KeyedColumn { id, .. }
        | ViewNode::Lazy { id, .. } => id.as_ref(),
        _ => None,
    };
    let child_scope = rendered_child_scope(id, scope, env, document)?;
    let rendered = match node {
        ViewNode::Theme { content, span, .. } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let theme = document.program().nested_theme(span)?;
            let preset = resolved_theme_preset_code(&theme.preset, env, document.program())?;
            let text = theme.text.as_ref().map(resolved_theme_color).map_or_else(
                || "::std::option::Option::None".into(),
                |color| format!("::std::option::Option::Some({color})"),
            );
            let background = theme
                .background
                .as_ref()
                .map(|background| resolved_background_code(background, env, document))
                .transpose()?
                .map_or_else(
                    || "::std::option::Option::None".into(),
                    |background| format!("::std::option::Option::Some({background})"),
                );
            Ok(format!(
                "{{ let __theme_content: __IceElement<'_, {message}> = {content}; ::ui_lang_runtime::dynamic_themer({preset}, __theme_content, {text}, {background}).into() }}"
            ))
        }
        ViewNode::Float {
            scale,
            x,
            y,
            style,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let scale = clamped_f32_code(scale, "f32::EPSILON", "f32::MAX", env, document)?;
            let mut translate_env = ScopedBindingEnv::new(env);
            for (name, code) in [
                ("original_x", "(__original.x as f64)"),
                ("original_y", "(__original.y as f64)"),
                ("original_width", "(__original.width as f64)"),
                ("original_height", "(__original.height as f64)"),
                ("viewport_x", "(__viewport.x as f64)"),
                ("viewport_y", "(__viewport.y as f64)"),
                ("viewport_width", "(__viewport.width as f64)"),
                ("viewport_height", "(__viewport.height as f64)"),
            ] {
                translate_env.insert(
                    name.to_owned(),
                    Binding {
                        code: code.to_owned(),
                        ty: Type::F64,
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
            }
            let x = expr_code(x, &translate_env, document, ValueMode::Owned)?;
            let y = expr_code(y, &translate_env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __float_content: __IceElement<'_, {message}> = {content}; let __float = ::iced::widget::float(__float_content).scale({scale}).translate(move |__original, __viewport| ::iced::Vector::new({x} as f32, {y} as f32))"
            );
            append_float_style(&mut code, style, env, document)?;
            Ok(format!("{code}; __float.into() }}"))
        }
        ViewNode::Pin {
            width,
            height,
            x,
            y,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let x = expr_code(x, env, document, ValueMode::Owned)?;
            let y = expr_code(y, env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __pin_content: __IceElement<'_, {message}> = {content}; ::iced::widget::pin(__pin_content).x({x} as f32).y({y} as f32)"
            );
            append_dimensions(&mut code, [width, height], env, document)?;
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Sensor { content, .. } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let program = document.hir();
            let sensor = program.resolved_sensor_for(node)?;
            render_resolved_sensor(sensor, program, message, env, content)
        }
        ViewNode::Responsive {
            content,
            width,
            height,
            span,
            ..
        } => {
            let builder = match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    let CheckedViewFlow::ResponsiveBreakpoint { breakpoint } =
                        &document.program().checked_view(span)?.flow
                    else {
                        return Err(Error::new(
                            "E196",
                            span,
                            "responsive breakpoint has no checked flow",
                        ));
                    };
                    let breakpoint = checked_expr_use_code(
                        document.program(),
                        *breakpoint,
                        env,
                        ValueMode::Owned,
                    )?;
                    let breakpoint =
                        format!("(({breakpoint}) as f32).max(f32::EPSILON).min(f32::MAX)");
                    let narrow = render_node(narrow, document, message, env, &child_scope, slot)?;
                    let wide = render_node(wide, document, message, env, &child_scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: __IceElement<'_, {message}> = if __size.width < {breakpoint} {{ {narrow} }} else {{ {wide} }}; __responsive }}"
                    )
                }
                ResponsiveContent::Size { content, .. } => {
                    let CheckedViewFlow::ResponsiveSize { width, height } =
                        &document.program().checked_view(span)?.flow
                    else {
                        return Err(Error::new(
                            "E196",
                            span,
                            "responsive size has no checked flow",
                        ));
                    };
                    let width_name = document
                        .program()
                        .checked_facts()
                        .local(*width)
                        .name
                        .clone();
                    let height_name = document
                        .program()
                        .checked_facts()
                        .local(*height)
                        .name
                        .clone();
                    let mut child_env = ScopedBindingEnv::new(env);
                    child_env.insert(
                        width_name,
                        checked_local_binding(
                            document.program(),
                            *width,
                            "(__size.width as f64)".into(),
                            true,
                        ),
                    );
                    child_env.insert(
                        height_name,
                        checked_local_binding(
                            document.program(),
                            *height,
                            "(__size.height as f64)".into(),
                            true,
                        ),
                    );
                    let content =
                        render_node(content, document, message, &child_env, &child_scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: __IceElement<'_, {message}> = {content}; __responsive }}"
                    )
                }
            };
            let mut code = format!("::iced::widget::responsive({builder})");
            append_dimensions(&mut code, [width, height], env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::KeyedColumn {
            options,
            child,
            span,
            ..
        } => render_keyed_column(
            options,
            child,
            span,
            document,
            message,
            env,
            &child_scope,
            slot,
        ),
        ViewNode::Lazy { child, span, .. } => {
            let CheckedViewFlow::Lazy {
                dependency,
                binding,
            } = &document.program().checked_view(span)?.flow
            else {
                return Err(Error::new("E196", span, "lazy view has no checked flow"));
            };
            let checked_binding = document.program().checked_facts().local(*binding);
            let binding_name = &checked_binding.name;
            let dependency_type = checked_binding.ty.clone();
            let dependency =
                checked_expr_use_code(document.program(), *dependency, env, ValueMode::Owned)?;
            let mut child_env = HashMap::new();
            child_env.insert(
                binding_name.clone(),
                checked_local_binding(document.program(), *binding, binding_name.clone(), false),
            );
            let child = render_node(
                child,
                document,
                message,
                &child_env,
                "__lazy_scope.clone()",
                None,
            )?;
            let dependency_rust = dependency_type.rust(&document.structs);
            Ok(format!(
                "::iced::widget::lazy(({dependency}, ({child_scope}).to_owned(), __ice_palette.name), move |__dependency| {{ let {binding_name}: {dependency_rust} = __dependency.0.clone(); let __lazy_scope = __dependency.1.clone(); let __lazy_content: __IceElement<'static, {message}> = {child}; __lazy_content }}).into()"
            ))
        }
        _ => return Ok(None),
    }?;
    Ok(Some(identify_rendered(
        rendered, id, message, env, document, scope,
    )?))
}

fn render_resolved_sensor(
    sensor: &ResolvedSensor,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    content: String,
) -> Result<String, Error> {
    let mut code = format!(
        "{{ let __sensor_content: __IceElement<'_, {message}> = {content}; ::iced::widget::sensor(__sensor_content)"
    );
    for (route, method) in [(&sensor.show, "on_show"), (&sensor.resize, "on_resize")] {
        if let Some(route) = route {
            let callback = resolved_interaction_route_callback_code(
                route,
                "__size",
                &["__size.width as f64", "__size.height as f64"],
                env,
                program,
                message,
            )?;
            write!(code, ".{method}({callback})").unwrap();
        }
    }
    if let Some(route) = &sensor.hide {
        write!(
            code,
            ".on_hide({})",
            resolved_interaction_route_code(route, &[], env, program, message)?
        )
        .unwrap();
    }
    if let Some(key) = sensor.key {
        write!(
            code,
            ".key({})",
            checked_expr_use_code(program, key, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(anticipate) = sensor.anticipate {
        let anticipate = checked_expr_use_code(program, anticipate, env, ValueMode::Owned)?;
        write!(
            code,
            ".anticipate((({anticipate}) as f32).max(0.0).min(f32::MAX))"
        )
        .unwrap();
    }
    if let Some(delay) = sensor.delay_ms {
        let delay = checked_expr_use_code(program, delay, env, ValueMode::Owned)?;
        write!(
            code,
            ".delay(::std::time::Duration::from_millis(u64::try_from({delay}).unwrap_or(0)))"
        )
        .unwrap();
    }
    Ok(format!("{code}.into() }}"))
}

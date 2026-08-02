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
        ViewNode::Theme { content, .. } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let program = document.program();
            let theme = program.resolved_nested_theme_for(node)?;
            let preset = resolved_theme_preset_code(&theme.preset, env, program)?;
            let text = theme.text.as_ref().map(resolved_theme_color).map_or_else(
                || "::std::option::Option::None".into(),
                |color| format!("::std::option::Option::Some({color})"),
            );
            let background = theme
                .background
                .as_ref()
                .map(|background| resolved_background_code(background, env, program))
                .transpose()?
                .map_or_else(
                    || "::std::option::Option::None".into(),
                    |background| format!("::std::option::Option::Some({background})"),
                );
            Ok(format!(
                "{{ let __theme_content: __IceElement<'_, {message}> = {content}; ::ui_lang_runtime::dynamic_themer({preset}, __theme_content, {text}, {background}).into() }}"
            ))
        }
        ViewNode::Float { content, .. } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let program = document.hir();
            let float = program.resolved_float_for(node)?;
            render_resolved_float(float, program, message, env, content)
        }
        ViewNode::Pin { content, .. } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let program = document.hir();
            let pin = program.resolved_pin_for(node)?;
            render_resolved_pin(pin, program, message, env, content)
        }
        ViewNode::Sensor { content, .. } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let program = document.hir();
            let sensor = program.resolved_sensor_for(node)?;
            render_resolved_sensor(sensor, program, message, env, content)
        }
        ViewNode::Responsive { content, .. } => {
            let program = document.hir();
            let responsive = program.resolved_responsive_for(node)?;
            let builder = match (&responsive.kind, content) {
                (
                    ResolvedResponsiveKind::Breakpoint { breakpoint },
                    ResponsiveContent::Breakpoint { narrow, wide, .. },
                ) => {
                    let breakpoint =
                        checked_expr_use_code(program, *breakpoint, env, ValueMode::Owned)?;
                    let breakpoint =
                        format!("(({breakpoint}) as f32).max(f32::EPSILON).min(f32::MAX)");
                    let narrow = render_node(narrow, document, message, env, &child_scope, slot)?;
                    let wide = render_node(wide, document, message, env, &child_scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: __IceElement<'_, {message}> = if __size.width < {breakpoint} {{ {narrow} }} else {{ {wide} }}; __responsive }}"
                    )
                }
                (
                    ResolvedResponsiveKind::Size { width, height },
                    ResponsiveContent::Size { content, .. },
                ) => {
                    let mut child_env = ScopedBindingEnv::new(env);
                    child_env.insert(
                        width.name.clone(),
                        checked_local_binding(
                            program,
                            width.local,
                            "(__size.width as f64)".into(),
                            true,
                        ),
                    );
                    child_env.insert(
                        height.name.clone(),
                        checked_local_binding(
                            program,
                            height.local,
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
                _ => {
                    return Err(Error::new(
                        "E196",
                        node.span(),
                        "responsive source tree diverged from normalized HIR",
                    ));
                }
            };
            let mut code = format!("::iced::widget::responsive({builder})");
            for (method, length) in [("width", &responsive.width), ("height", &responsive.height)] {
                if let Some(length) = length {
                    write!(
                        code,
                        ".{method}({})",
                        resolved_responsive_length_code(length, program, env)?
                    )
                    .unwrap();
                }
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::KeyedColumn { child, .. } => {
            let program = document.hir();
            let keyed = program.resolved_keyed_column_for(node)?;
            render_keyed_column(keyed, child, document, message, env, &child_scope, slot)
        }
        ViewNode::Lazy { child, .. } => {
            let program = document.hir();
            let lazy = program.resolved_lazy_for(node)?;
            let binding_name = &lazy.binding.name;
            let dependency =
                checked_expr_use_code(program, lazy.dependency, env, ValueMode::Owned)?;
            let mut child_env = HashMap::new();
            child_env.insert(
                binding_name.clone(),
                checked_local_binding(program, lazy.binding.local, binding_name.clone(), false),
            );
            let child = render_node(
                child,
                document,
                message,
                &child_env,
                "__lazy_scope.clone()",
                None,
            )?;
            let dependency_rust = lazy.binding.ty.rust(&document.structs);
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

fn render_resolved_float(
    float: &ResolvedFloat,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    content: String,
) -> Result<String, Error> {
    let checked_f32 = |expression| -> Result<String, Error> {
        Ok(format!(
            "{} as f32",
            checked_expr_use_code(program, expression, env, ValueMode::Owned)?
        ))
    };
    let scale = checked_expr_use_code(program, float.scale, env, ValueMode::Owned)?;
    let scale = format!("(({scale}) as f32).max(f32::EPSILON).min(f32::MAX)");
    let mut translate_env = ScopedBindingEnv::new(env);
    for (geometry, code) in float.geometry.iter().zip([
        "(__original.x as f64)",
        "(__original.y as f64)",
        "(__original.width as f64)",
        "(__original.height as f64)",
        "(__viewport.x as f64)",
        "(__viewport.y as f64)",
        "(__viewport.width as f64)",
        "(__viewport.height as f64)",
    ]) {
        translate_env.insert(
            geometry.name.clone(),
            checked_local_binding(program, geometry.local, code.into(), true),
        );
    }
    let x = checked_expr_use_code(program, float.x, &translate_env, ValueMode::Owned)?;
    let y = checked_expr_use_code(program, float.y, &translate_env, ValueMode::Owned)?;
    let mut code = format!(
        "{{ let __float_content: __IceElement<'_, {message}> = {content}; let __float = ::iced::widget::float(__float_content).scale({scale}).translate(move |__original, __viewport| ::iced::Vector::new({x} as f32, {y} as f32))"
    );
    let radius = resolved_float_radius_code(&float.radius, program, env)?;
    if float.shadow_color.is_some()
        || float.shadow_x.is_some()
        || float.shadow_y.is_some()
        || float.shadow_blur.is_some()
        || radius.is_some()
    {
        code.push_str(
            ".style(move |_| { let mut __style = ::iced::widget::float::Style::default();",
        );
        if let Some(color) = &float.shadow_color {
            write!(
                code,
                " __style.shadow.color = {};",
                resolved_theme_color(color)
            )
            .unwrap();
        }
        for (expression, field) in [
            (float.shadow_x, "__style.shadow.offset.x"),
            (float.shadow_y, "__style.shadow.offset.y"),
            (float.shadow_blur, "__style.shadow.blur_radius"),
        ] {
            if let Some(expression) = expression {
                write!(code, " {field} = {};", checked_f32(expression)?).unwrap();
            }
        }
        if let Some(radius) = radius {
            write!(code, " __style.shadow_border_radius = {radius};").unwrap();
        }
        code.push_str(" __style })");
    }
    Ok(format!("{code}; __float.into() }}"))
}

fn render_resolved_pin(
    pin: &ResolvedPin,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    content: String,
) -> Result<String, Error> {
    let x = checked_expr_use_code(program, pin.x, env, ValueMode::Owned)?;
    let y = checked_expr_use_code(program, pin.y, env, ValueMode::Owned)?;
    let mut code = format!(
        "{{ let __pin_content: __IceElement<'_, {message}> = {content}; ::iced::widget::pin(__pin_content).x({x} as f32).y({y} as f32)"
    );
    for (method, length) in [("width", &pin.width), ("height", &pin.height)] {
        if let Some(length) = length {
            write!(
                code,
                ".{method}({})",
                resolved_pin_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    Ok(format!("{code}.into() }}"))
}

fn resolved_pin_length_code(
    length: &ResolvedPinLength,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match length {
        ResolvedPinLength::Fill => "::iced::Fill".into(),
        ResolvedPinLength::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        ResolvedPinLength::Shrink => "::iced::Shrink".into(),
        ResolvedPinLength::FixedF64(expression) => format!(
            "{} as f32",
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedPinLength::FixedLength(expression) => {
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)?
        }
    })
}

fn resolved_responsive_length_code(
    length: &ResolvedResponsiveLength,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match length {
        ResolvedResponsiveLength::Fill => "::iced::Fill".into(),
        ResolvedResponsiveLength::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        ResolvedResponsiveLength::Shrink => "::iced::Shrink".into(),
        ResolvedResponsiveLength::FixedF64(expression) => format!(
            "{} as f32",
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedResponsiveLength::FixedLength(expression) => {
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)?
        }
    })
}

fn resolved_float_radius_code(
    radius: &ResolvedFloatRadius,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if radius.all.is_none()
        && radius.top_left.is_none()
        && radius.top_right.is_none()
        && radius.bottom_right.is_none()
        && radius.bottom_left.is_none()
    {
        return Ok(None);
    }
    let code = |expression| -> Result<String, Error> {
        let value = checked_expr_use_code(program, expression, env, ValueMode::Owned)?;
        Ok(format!("(({value}) as f32).max(0.0).min(f32::MAX)"))
    };
    let base = radius
        .all
        .map(&code)
        .transpose()?
        .unwrap_or_else(|| "0.0".into());
    let corners = [
        radius.top_left,
        radius.top_right,
        radius.bottom_right,
        radius.bottom_left,
    ]
    .map(|corner| corner.map(&code).transpose())
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .map(|corner| corner.unwrap_or_else(|| base.clone()))
    .collect::<Vec<_>>();
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {}, top_right: {}, bottom_right: {}, bottom_left: {} }}",
        corners[0], corners[1], corners[2], corners[3]
    )))
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

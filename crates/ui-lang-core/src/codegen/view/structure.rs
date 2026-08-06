use super::*;

pub(in crate::codegen) fn render_structure(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let view = document.resolved_view(node)?;
    let identity = view.identity.as_ref();
    let child_scope = rendered_child_scope(identity, scope, env, document)?;
    let rendered = match &view.kind {
        ResolvedViewKind::Theme { content } => {
            let content = render_node(*content, document, message, env, &child_scope, slot)?;
            let program = document;
            let theme = program.resolved_nested_theme(node)?;
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
        ResolvedViewKind::Float { content } => {
            let content = render_node(*content, document, message, env, &child_scope, slot)?;
            let program = document;
            let float = program.resolved_float(node)?;
            render_resolved_float(float, program, message, env, content)
        }
        ResolvedViewKind::Pin { content } => {
            let content = render_node(*content, document, message, env, &child_scope, slot)?;
            let program = document;
            let pin = program.resolved_pin(node)?;
            render_resolved_pin(pin, program, message, env, content)
        }
        ResolvedViewKind::Sensor { content } => {
            let content = render_node(*content, document, message, env, &child_scope, slot)?;
            let program = document;
            let sensor = program.resolved_sensor(node)?;
            render_resolved_sensor(sensor, program, message, env, content)
        }
        ResolvedViewKind::ResponsiveBreakpoint { narrow, wide } => {
            let program = document;
            let responsive = program.resolved_responsive(node)?;
            let builder = match &responsive.kind {
                ResolvedResponsiveKind::Breakpoint { breakpoint } => {
                    let breakpoint =
                        resolved_expr_use_code(program, *breakpoint, env, ValueMode::Owned)?;
                    let breakpoint =
                        format!("(({breakpoint}) as f32).max(f32::EPSILON).min(f32::MAX)");
                    let narrow = render_node(*narrow, document, message, env, &child_scope, slot)?;
                    let wide = render_node(*wide, document, message, env, &child_scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: __IceElement<'_, {message}> = if __size.width < {breakpoint} {{ {narrow} }} else {{ {wide} }}; __responsive }}"
                    )
                }
                _ => {
                    return Err(document.invariant_at_origin(
                        view.origin,
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
        ResolvedViewKind::ResponsiveSize { content } => {
            let program = document;
            let responsive = program.resolved_responsive(node)?;
            let ResolvedResponsiveKind::Size { width, height } = &responsive.kind else {
                return Err(document.invariant_at_origin(
                    view.origin,
                    "responsive size topology diverged from normalized HIR",
                ));
            };
            let mut child_env = ScopedBindingEnv::new(env);
            child_env.insert(
                width.name.clone(),
                resolved_local_binding(
                    LocalBindingTypeSource::Resolved(program),
                    width.local,
                    "(__size.width as f64)".into(),
                    true,
                ),
            );
            child_env.insert(
                height.name.clone(),
                resolved_local_binding(
                    LocalBindingTypeSource::Resolved(program),
                    height.local,
                    "(__size.height as f64)".into(),
                    true,
                ),
            );
            let content = render_node(*content, document, message, &child_env, &child_scope, slot)?;
            let builder = format!(
                "move |__size| {{ let __responsive: __IceElement<'_, {message}> = {content}; __responsive }}"
            );
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
        ResolvedViewKind::KeyedColumn { child } => {
            let program = document;
            let keyed = program.resolved_keyed_column(node)?;
            render_keyed_column(keyed, *child, document, message, env, &child_scope, slot)
        }
        ResolvedViewKind::Lazy { child } => {
            let program = document;
            let lazy = program.resolved_lazy(node)?;
            let binding_name = &lazy.binding.name;
            let dependency =
                resolved_expr_use_code(program, lazy.dependency, env, ValueMode::Owned)?;
            let mut child_env = HashMap::new();
            child_env.insert(
                binding_name.clone(),
                resolved_local_binding(
                    LocalBindingTypeSource::Resolved(program),
                    lazy.binding.local,
                    binding_name.clone(),
                    false,
                ),
            );
            let hoisted = hoist_lazy_component_context(node, program, env, &mut child_env);
            let child = render_node(
                *child,
                document,
                message,
                &child_env,
                "__lazy_scope.clone()",
                None,
            )?;
            let dependency_rust = rust_type_code(program, &lazy.binding.ty);
            // memo_lazy is iced's Lazy plus LAYOUT memoization (a cached row
            // also skips the per-pass layout walk) plus unmount parking: the
            // trailing site id — this lazy expression's view-node id — keys
            // the parked subtree so a torn-down mount (a `match` arm switch)
            // rehydrates on re-entry instead of re-shaping every row.
            let site = node.0;
            let lazy_code = format!(
                "::ui_lang_runtime::memo_lazy(({dependency}, ({child_scope}).to_owned(), __ice_palette.name), move |__dependency| {{ let {binding_name}: {dependency_rust} = __dependency.0.clone(); let __lazy_scope = __dependency.1.clone(); let __lazy_content: __IceElement<'static, {message}> = {child}; __lazy_content }}, {site}u64).into()"
            );
            Ok(if hoisted.is_empty() {
                lazy_code
            } else {
                format!("{{ {hoisted}{lazy_code} }}")
            })
        }
        _ => return Ok(None),
    }?;
    Ok(Some(identify_rendered(
        rendered, identity, message, env, document, scope,
    )?))
}

/// A lazy closure rebuilds its subtree from owned data only, but routes and
/// forwards inside it still address the enclosing component. Hoist the
/// component's routing bindings — the reconciliation scope, the output
/// callback, and the call-site event callbacks — into owned locals declared
/// before the closure, and rebind the child environment to those locals so
/// the closure captures them by move. Unused locals are simply not captured.
fn hoist_lazy_component_context(
    node: ViewId,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    child_env: &mut HashMap<String, Binding>,
) -> String {
    let Some((component, context)) = component_context(env) else {
        return String::new();
    };
    let component = component.to_owned();
    let mut hoisted = String::new();
    let context_local = format!("__ice_lazy_context_{}", node.0);
    write!(
        hoisted,
        "let {context_local} = ({}).to_owned(); ",
        context.code
    )
    .unwrap();
    if let Some(output) = env.get(&component_output_key(&component)) {
        let output_local = format!("__ice_lazy_output_{}", node.0);
        write!(hoisted, "let {output_local} = {}; ", output.code).unwrap();
        child_env.insert(
            component_output_key(&component),
            Binding {
                code: output_local,
                ty: output.ty.clone(),
                local: true,
                state: None,
                owner: None,
            },
        );
    }
    for (index, event) in program.component_event_names(&component).enumerate() {
        let Some(callback) = component_event(env, &component, event) else {
            continue;
        };
        let event_local = format!("__ice_lazy_event_{}_{index}", node.0);
        write!(hoisted, "let {event_local} = {}; ", callback.code).unwrap();
        child_env.insert(
            component_event_key(&component, event),
            Binding {
                code: event_local,
                ty: callback.ty.clone(),
                local: true,
                state: None,
                owner: None,
            },
        );
    }
    insert_component_context(
        child_env,
        &component,
        Binding {
            code: context_local,
            ty: Type::Unit,
            local: true,
            state: None,
            owner: None,
        },
    );
    hoisted
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
            resolved_expr_use_code(program, expression, env, ValueMode::Owned)?
        ))
    };
    let scale = resolved_expr_use_code(program, float.scale, env, ValueMode::Owned)?;
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
            resolved_local_binding(
                LocalBindingTypeSource::Resolved(program),
                geometry.local,
                code.into(),
                true,
            ),
        );
    }
    let x = resolved_expr_use_code(program, float.x, &translate_env, ValueMode::Owned)?;
    let y = resolved_expr_use_code(program, float.y, &translate_env, ValueMode::Owned)?;
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
    let x = resolved_expr_use_code(program, pin.x, env, ValueMode::Owned)?;
    let y = resolved_expr_use_code(program, pin.y, env, ValueMode::Owned)?;
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
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedPinLength::FixedLength(expression) => {
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
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
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedResponsiveLength::FixedLength(expression) => {
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
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
        let value = resolved_expr_use_code(program, expression, env, ValueMode::Owned)?;
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
            resolved_expr_use_code(program, key, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(anticipate) = sensor.anticipate {
        let anticipate = resolved_expr_use_code(program, anticipate, env, ValueMode::Owned)?;
        write!(
            code,
            ".anticipate((({anticipate}) as f32).max(0.0).min(f32::MAX))"
        )
        .unwrap();
    }
    if let Some(delay) = sensor.delay_ms {
        let delay = resolved_expr_use_code(program, delay, env, ValueMode::Owned)?;
        write!(
            code,
            ".delay(::std::time::Duration::from_millis(u64::try_from({delay}).unwrap_or(0)))"
        )
        .unwrap();
    }
    Ok(format!("{code}.into() }}"))
}

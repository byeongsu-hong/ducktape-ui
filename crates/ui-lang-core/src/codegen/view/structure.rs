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
        // The responsive arm does not memoize what its closure builds, and
        // it stays that way for two reasons that survive each other.
        //
        // The compile failure first: `MemoLazy` stores an `Element<'static>`,
        // and wrapping the builder in `memo_lazy((size, palette), ..)` fails
        // ("returning this value requires that `'1` must outlive `'static`").
        // The lifetime is pinned by a mounted animation's `.style()` closure
        // reading mounted state off `&self` — NOT by the inputs: `TextInput`
        // copies its value into an owned `Value` (iced 0.14 has no borrowed
        // field), and a spike building exactly what codegen emits for six
        // inputs coerced to `Element<'static>` clean. E139's sentence now
        // states the surviving reason instead: the cached element freezes
        // the typed text.
        //
        // Solving the lifetime would not make the memo right. `MemoLazy`
        // caches the *element*, so an input under a key that omits its value
        // stops showing what was typed — the reason E139's ban is still
        // correct — and this page's only sound key is "everything the
        // subtree reads", which a market beat changes. Memoizing it by size
        // is not hard; it is wrong-keyed.
        //
        // The obvious suspects are innocent, which is why this is written
        // down: no build-time read under a responsive sees the clock (the
        // trading view reads `clock`, a state field a tick moves), and a
        // memoized element would keep animating either way — the fade
        // interpolates at draw time; it is the animation's mounted-state
        // READ, not its motion, that pins the lifetime.
        ResolvedViewKind::ResponsiveSize { content } => {
            let program = document;
            let responsive = program.resolved_responsive(node)?;
            let mut child_env = ScopedBindingEnv::new(env);
            child_env.insert(
                responsive.measured_width.name.clone(),
                resolved_local_binding(
                    LocalBindingTypeSource::Resolved(program),
                    responsive.measured_width.local,
                    "(__size.width as f64)".into(),
                    true,
                ),
            );
            child_env.insert(
                responsive.measured_height.name.clone(),
                resolved_local_binding(
                    LocalBindingTypeSource::Resolved(program),
                    responsive.measured_height.local,
                    "(__size.height as f64)".into(),
                    true,
                ),
            );
            // The `move` closure would move a shared scope local out of the
            // enclosing render (a component's scope binding); rebind the chain
            // to a closure-owned string instead.
            child_env.insert(
                RECONCILIATION_SCOPE_BINDING.into(),
                reconciliation_scope_binding("__ice_responsive_recon.clone()".into()),
            );
            let responsive_recon = reconciliation_scope(&child_scope, env).to_owned();
            let content = render_node(
                *content,
                document,
                message,
                &child_env,
                "__ice_responsive_scope.clone()",
                slot,
            )?;
            let builder = format!(
                "{{ let __ice_responsive_scope = ({child_scope}).to_owned(); let __ice_responsive_recon = ({responsive_recon}).to_owned(); let _ = (&__ice_responsive_scope, &__ice_responsive_recon); move |__size| {{ let __responsive: __IceElement<'_, {message}> = {content}; __responsive }} }}"
            );
            let mut code = format!("::iced::widget::responsive({builder})");
            for (method, length) in [("width", &responsive.width), ("height", &responsive.height)] {
                if let Some(length) = length {
                    write!(
                        code,
                        ".{method}({})",
                        resolved_length_code(length, program, env)?
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
            for binding in &lazy.key_bindings {
                child_env.insert(
                    binding.binding.name.clone(),
                    resolved_local_binding(
                        LocalBindingTypeSource::Resolved(program),
                        binding.binding.local,
                        binding.binding.name.clone(),
                        false,
                    ),
                );
            }
            let (hoisted, hoist_params) =
                hoist_lazy_component_context(node, program, env, &mut child_env, message);
            // The lazy closure is `'static`, so component uses inside it must
            // never outline into `&self` methods.
            let _lazy_guard = outline::enter_lazy_render();
            let child = render_node(
                *child,
                document,
                message,
                &child_env,
                "__lazy_scope.clone()",
                None,
            )?;
            drop(_lazy_guard);
            let dependency_rust = rust_type_code(program, &lazy.binding.ty);
            // memo_lazy is iced's Lazy plus LAYOUT memoization (a cached row
            // also skips the per-pass layout walk) plus unmount parking. The
            // view-node id identifies the expression and the reconciliation
            // scope identifies this concrete row/mount, so stale dependency
            // revisions replace one another without collapsing sibling rows.
            let site = node.0;
            let parking_scope = borrowed_scope(reconciliation_scope(&child_scope, env));
            if !lazy.keys.is_empty() {
                // `lazy value by key, key as name`: the keys stand in for the
                // value in the dependency tuple, and the value never crosses
                // a frame — the builder captures it by reference (or by Copy)
                // and clones it into the binding only when a key changes, so
                // an unchanged frame performs no deep clone of the value.
                let keys = lazy
                    .keys
                    .iter()
                    .map(|key| resolved_expr_use_code(program, *key, env, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                // Unlike the plain form's, this value expression lands INSIDE
                // the `move` builder — a component-state read must not name
                // the call-site scope binding there, or the closure captures
                // the String by move and any later read of the component's
                // state in the same render is a use-after-move. Emit the read
                // against the closure-owned context local the hoist declares
                // instead; its value IS the component scope, so the state
                // lookup key is unchanged.
                let component_context_hoisted = !hoisted.is_empty();
                let dependency = if component_context_hoisted {
                    resolved_expr_use_code_with_state_scope(
                        program,
                        lazy.dependency,
                        env,
                        ValueMode::Owned,
                        &lazy_context_local(node),
                    )?
                } else {
                    resolved_expr_use_code(program, lazy.dependency, env, ValueMode::Owned)?
                };
                let scope_index = lazy.keys.len();
                let key_bindings = lazy
                    .key_bindings
                    .iter()
                    .map(|binding| {
                        let name = &binding.binding.name;
                        let ty = rust_type_code(program, &binding.binding.ty);
                        let index = binding.index;
                        format!("let {name}: {ty} = __dependency.{index}.clone(); ")
                    })
                    .collect::<String>();
                let lazy_body = format!(
                    "{key_bindings}let __lazy_scope = __dependency.{scope_index}.clone(); let {binding_name}: {dependency_rust} = {dependency}; let __lazy_content: __IceElement<'static, {message}> = {child}; __lazy_content"
                );
                let builder = format!("move |__dependency| {{ {lazy_body} }}");
                let lazy_code = format!(
                    "::ui_lang_runtime::memo_lazy(({keys}, ({child_scope}).to_owned(), __ice_palette.name), {builder}, {site}u64, &({parking_scope})).into()"
                );
                return Ok(Some(identify_rendered(
                    if hoisted.is_empty() {
                        lazy_code
                    } else {
                        format!("{{ {hoisted}{lazy_code} }}")
                    },
                    identity,
                    message,
                    env,
                    document,
                    scope,
                )?));
            }
            // The plain form's dependency is evaluated eagerly into the memo
            // tuple, outside the builder, where borrowing the call-site scope
            // binding is fine.
            let dependency =
                resolved_expr_use_code(program, lazy.dependency, env, ValueMode::Owned)?;
            let lazy_body = format!(
                "let {binding_name}: {dependency_rust} = __dependency.0.clone(); let __lazy_scope = __dependency.1.clone(); let __lazy_content: __IceElement<'static, {message}> = {child}; __lazy_content"
            );
            // The lazy body is `'static` by contract, so it can live as an
            // associated fn over the dependency tuple plus the hoisted
            // routing context — moving the row subtree out of the enclosing
            // render function. Falls back to the inline closure when a
            // hoisted callback has no signature marker.
            let builder = if outline::outlining_active()
                && let Some(params) = hoist_params
            {
                let tuple = format!("({dependency_rust}, ::std::string::String, &'static str)");
                // Group by the fragment holding the `lazy` block — the body
                // is written there, so its file changes only with it.
                let group = origin_fragment_slug(program, view.origin);
                let body_fn = outline::push_lazy_body(message, &group, &tuple, &params, &lazy_body);
                let arguments = params
                    .iter()
                    .map(|(local, _)| format!(", ({local}).clone()"))
                    .collect::<String>();
                format!(
                    "move |__dependency| Self::{body_fn}(__ice_palette, __dependency{arguments})"
                )
            } else {
                format!("move |__dependency| {{ {lazy_body} }}")
            };
            let lazy_code = format!(
                "::ui_lang_runtime::memo_lazy(({dependency}, ({child_scope}).to_owned(), __ice_palette.name), {builder}, {site}u64, &({parking_scope})).into()"
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

/// The owned clone of the enclosing component's scope that
/// [`hoist_lazy_component_context`] declares ahead of a lazy closure. The
/// keyed dependency emission names it too, so both must agree on the
/// spelling.
fn lazy_context_local(node: ViewId) -> String {
    format!("__ice_lazy_context_{}", node.0)
}

/// A lazy closure rebuilds its subtree from owned data only, but routes and
/// forwards inside it still address the enclosing component. Hoist the
/// component's routing bindings — the reconciliation scope, the output
/// callback, and the call-site event callbacks — into owned locals declared
/// before the closure, and rebind the child environment to those locals so
/// the closure captures them by move. Unused locals are simply not captured.
/// Returns the hoist prelude and, when every hoisted binding has a
/// parameter type (`Some`), the `(local, type)` manifest that lets the lazy
/// body outline as an associated fn taking the context as arguments.
fn hoist_lazy_component_context(
    node: ViewId,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    child_env: &mut HashMap<String, Binding>,
    message: &str,
) -> (String, Option<Vec<(String, String)>>) {
    let Some((component, context)) = component_context(env) else {
        return (String::new(), Some(Vec::new()));
    };
    let component = component.to_owned();
    let mut hoisted = String::new();
    let mut params: Option<Vec<(String, String)>> = Some(Vec::new());
    let context_local = lazy_context_local(node);
    write!(
        hoisted,
        "let {context_local} = ({}).to_owned(); ",
        context.code
    )
    .unwrap();
    if let Some(params) = params.as_mut() {
        params.push((context_local.clone(), "::std::string::String".into()));
    }
    if let Some(output) = env.get(&component_output_key(&component)) {
        let output_local = format!("__ice_lazy_output_{}", node.0);
        // Cloned, not moved: the enclosing render can run more than once per
        // frame — a `lazy` inside a `for` hoists this on every iteration — and
        // a moved callback leaves the second one with nothing to bind.
        write!(hoisted, "let {output_local} = ({}).clone(); ", output.code).unwrap();
        match env.get(&callback_sig_key(&component_output_key(&component))) {
            Some(sig) => {
                if let Some(params) = params.as_mut() {
                    params.push((output_local.clone(), sig.code.clone()));
                }
            }
            None => {
                let _ = rust_type_code;
                params = None;
            }
        }
        let _ = message;
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
        write!(hoisted, "let {event_local} = ({}).clone(); ", callback.code).unwrap();
        match env.get(&callback_sig_key(&component_event_key(&component, event))) {
            Some(sig) => {
                if let Some(params) = params.as_mut() {
                    params.push((event_local.clone(), sig.code.clone()));
                }
            }
            None => params = None,
        }
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
    (hoisted, params)
}

pub(in crate::codegen) fn render_resolved_float(
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
    let scale = clamped_f32_code(float.scale, "f32::EPSILON", "f32::MAX", program, env)?;
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
    let (x, y) = {
        (
            resolved_expr_use_code(program, float.x, &translate_env, ValueMode::Owned)?,
            resolved_expr_use_code(program, float.y, &translate_env, ValueMode::Owned)?,
        )
    };
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
                resolved_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    Ok(format!("{code}.into() }}"))
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

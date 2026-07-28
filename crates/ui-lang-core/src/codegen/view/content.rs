use super::*;

pub(in crate::codegen) fn render_content(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let id = match node {
        ViewNode::Rule { id, .. }
        | ViewNode::QrCode { id, .. }
        | ViewNode::Space { id, .. }
        | ViewNode::ExternComponent { id, .. }
        | ViewNode::Themer { id, .. }
        | ViewNode::Shader { id, .. } => id.as_ref(),
        _ => None,
    };
    let rendered = match node {
        ViewNode::Rule {
            axis,
            thickness,
            options,
            ..
        } => {
            let thickness = expr_code(thickness, env, document, ValueMode::Owned)?;
            let axis = match axis {
                Axis::Horizontal => "horizontal",
                Axis::Vertical => "vertical",
            };
            let mut code = format!("::iced::widget::rule::{axis}({thickness} as f32)");
            append_rule_options(&mut code, options, env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::QrCode {
            payload,
            correction,
            version,
            cell_size,
            total_size,
            cell,
            background,
            ..
        } => {
            let data = qr_data_code(payload, *correction, *version, env, document)?;
            let mut code = format!("::ui_lang_runtime::qr_code({data}.ok())");
            // QR v40 has 177 cells plus four quiet-zone cells; spacing counts gaps.
            if let Some(value) = cell_size {
                write!(
                    code,
                    ".cell_size(::ui_lang_runtime::bounded_spacing({}, 182))",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(value) = total_size {
                write!(
                    code,
                    ".total_size(::ui_lang_runtime::bounded_spacing({}, 3))",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if cell.is_some() || background.is_some() {
                let cell = cell.as_deref().map(|value| theme_color(document, value));
                let background = background
                    .as_deref()
                    .map(|value| theme_color(document, value));
                let (theme, default) = if cell.is_none() || background.is_none() {
                    (
                        "theme",
                        "let default = ::iced::widget::qr_code::default(theme); ",
                    )
                } else {
                    ("_theme", "")
                };
                write!(
                    code,
                    ".style(move |{theme}| {{ {default}::iced::widget::qr_code::Style {{ cell: {}, background: {} }} }})",
                    cell.unwrap_or_else(|| "default.cell".into()),
                    background.unwrap_or_else(|| "default.background".into())
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Space { width, height, .. } => {
            let mut code = String::from("::iced::widget::space()");
            append_dimensions(&mut code, [width, height], env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::Component {
            name,
            args,
            id,
            slots,
            events,
            route,
            span,
        } => {
            let component = document
                .components
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
            let mut component_env = HashMap::new();
            let default_env = HashMap::new();
            for param in &component.params {
                let arg = args.iter().find(|arg| arg.name == param.name);
                let state = match (param.bind, arg.map(|arg| &arg.value)) {
                    (true, Some(Expr::Path(path))) if path.len() == 1 => {
                        env.get(&path[0]).and_then(|binding| binding.state.clone())
                    }
                    _ => None,
                };
                let value = arg.map(|arg| (&arg.value, env)).or_else(|| {
                    param
                        .default
                        .as_ref()
                        .map(|default| (default, &default_env))
                });
                let (value, value_env) = value.expect("checker requires a component prop value");
                component_env.insert(
                    param.name.clone(),
                    Binding {
                        code: expr_code(value, value_env, document, ValueMode::Borrowed)?,
                        ty: param.ty.clone(),
                        local: false,
                        state,
                    },
                );
            }
            if let Some(route) = route {
                component_env.insert(
                    component_output_key(name),
                    Binding {
                        code: route_callback_code(
                            route, "__value", "__value", env, document, message,
                        )?,
                        ty: component.output.clone(),
                        local: true,
                        state: None,
                    },
                );
            }
            for event in &component.events {
                let supplied = events
                    .iter()
                    .find(|supplied| supplied.name == event.name)
                    .expect("checker requires every component event route");
                let payloads = (0..event.payloads.len())
                    .map(|index| format!("__event_{index}"))
                    .collect::<Vec<_>>();
                let payload_refs = payloads.iter().map(String::as_str).collect::<Vec<_>>();
                let code = if let Some(route) = &supplied.route {
                    ordered_route_callback_code(
                        route,
                        &payloads.join(", "),
                        &payload_refs,
                        env,
                        document,
                        message,
                    )?
                } else {
                    let (outer, _) = component_context(env)
                        .expect("checker requires forward inside a component");
                    component_event(env, outer, &event.name)
                        .expect("checker requires matching forwarded event")
                        .code
                        .clone()
                };
                component_env.insert(
                    component_event_key(name, &event.name),
                    Binding {
                        code,
                        ty: Type::Unit,
                        local: true,
                        state: None,
                    },
                );
            }
            let component_slots = component_slot_context(slots, document, env, slot)?;
            for component_slot in component_slots
                .iter()
                .flat_map(|slots| slots.entries.iter())
            {
                component_env.insert(
                    format!("\0slot-provided:{}", component_slot.name),
                    Binding {
                        code: "true".into(),
                        ty: Type::Bool,
                        local: true,
                        state: None,
                    },
                );
            }
            let component_scope = id.as_ref().map_or_else(
                || {
                    let scope = reconciliation_scope(scope, env);
                    format!("format!(\"{{}}/{}@{}\", {scope})", name, span.line)
                },
                |id| id_code(id, scope, env, document).unwrap_or_else(|_| scope.into()),
            );
            set_reconciliation_scope(&mut component_env, component_scope.clone());
            let scope_binding = component_scope_binding(name, span.line);
            if !component.states.is_empty() || !component.handlers.is_empty() {
                let field = component_state_field(name);
                let states = match component.lifetime {
                    ComponentLifetime::Retained => format!("self.{field}"),
                    ComponentLifetime::Mounted => format!("self.{field}.values()"),
                };
                for state in &component.states {
                    component_env.insert(
                        state.name.clone(),
                        Binding {
                            code: format!(
                                "{states}.get(&{scope_binding}).map_or_else(|| {}, |__state| __state.{}.clone())",
                                initial_code(state, document),
                                state.name
                            ),
                            ty: state.ty.clone(),
                            local: true,
                            state: Some(StateBinding::Component {
                                component: name.clone(),
                                name: state.name.clone(),
                                scope: scope_binding.clone(),
                            }),
                        },
                    );
                }
                component_env.insert(
                    component_context_key(name),
                    Binding {
                        code: scope_binding.clone(),
                        ty: Type::Unit,
                        local: true,
                        state: None,
                    },
                );
            }
            if !component.events.is_empty()
                && component.states.is_empty()
                && component.handlers.is_empty()
            {
                component_env.insert(
                    component_context_key(name),
                    Binding {
                        code: component_scope.clone(),
                        ty: Type::Unit,
                        local: true,
                        state: None,
                    },
                );
            }
            let render_scope = if component.states.is_empty() && component.handlers.is_empty() {
                component_scope.clone()
            } else {
                format!("{scope_binding}.clone()")
            };
            let rendered = render_node(
                &component.root,
                document,
                message,
                &component_env,
                &render_scope,
                component_slots.as_ref(),
            )?;
            let rendered = if component.states.is_empty() && component.handlers.is_empty() {
                rendered
            } else {
                let mount = (component.lifetime == ComponentLifetime::Mounted).then(|| {
                    let field = component_state_field(name);
                    format!("self.{field}.mount({scope_binding}.clone());")
                });
                format!(
                    "{{ let {scope_binding} = {component_scope}; {} {rendered} }}",
                    mount.as_deref().unwrap_or("")
                )
            };
            Ok(format!(
                "(|| {{ let __component_content: __IceElement<'_, {message}> = {rendered}; __component_content }})()"
            ))
        }
        ViewNode::Slot {
            name,
            optional,
            span,
        } => {
            let slot = slot.ok_or_else(|| {
                Error::new(
                    "E170",
                    span,
                    "slot reached codegen without component content",
                )
            })?;
            let content = slot
                .entries
                .iter()
                .find(|entry| entry.name == *name)
                .map_or_else(
                    || {
                        if *optional {
                            Ok(None)
                        } else {
                            Err(Error::new(
                                "E170",
                                span,
                                format!("slot `{name}` reached codegen without component content"),
                            ))
                        }
                    },
                    |content| Ok(Some(content)),
                )?;
            let Some(content) = content else {
                return Ok(None);
            };
            let mut content_env = content.env.clone();
            set_reconciliation_scope(&mut content_env, scope.to_owned());
            let rendered = render_node(
                &content.node,
                document,
                message,
                &content_env,
                scope,
                slot.parent.as_deref(),
            )?;
            Ok(format!(
                "(|| {{ let __slot_content: __IceElement<'_, {message}> = {rendered}; __slot_content }})()"
            ))
        }
        ViewNode::ExternComponent {
            function,
            args,
            route,
            span,
            ..
        } => {
            let component = find_extern_function(document, function, ExternKind::Component)
                .ok_or_else(|| {
                    Error::new(
                        "E130",
                        span,
                        format!("unknown extern component `{function}`"),
                    )
                })?;
            let args = args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    if component.borrowed[index] {
                        let arg = expr_code(arg, env, document, ValueMode::Borrowed)?;
                        let borrow = if matches!(
                            component.params[index].1,
                            Type::Str | Type::Bytes | Type::List(_)
                        ) {
                            "::std::convert::AsRef::as_ref"
                        } else {
                            "::std::borrow::Borrow::borrow"
                        };
                        Ok(format!("{borrow}(&({arg}))"))
                    } else {
                        expr_code(arg, env, document, ValueMode::Owned)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let mapped = if let Some(route) = route {
                route_callback_code(route, "__value", "__value", env, document, message)?
            } else {
                format!("move |__value| {message}::__ExternNoop")
            };
            Ok(format!(
                "{}({args}).map({mapped}).into()",
                component.rust_path
            ))
        }
        ViewNode::Themer {
            function,
            args,
            route,
            span,
            ..
        } => {
            let themer =
                find_extern_function(document, function, ExternKind::Themer).ok_or_else(|| {
                    Error::new("E130", span, format!("unknown extern themer `{function}`"))
                })?;
            let args = expr_list_code(args, env, document)?;
            let mapped = if let Some(route) = route {
                route_callback_code(route, "__value", "__value", env, document, message)?
            } else {
                format!("move |__value| {message}::__ExternNoop")
            };
            Ok(format!(
                "{{ let (__theme, __content, __text_color, __background) = {}({args}); let mut __themer = ::iced::widget::themer(__theme, __content); if let ::std::option::Option::Some(__text_color) = __text_color {{ __themer = __themer.text_color(__text_color); }} if let ::std::option::Option::Some(__background) = __background {{ __themer = __themer.background(__background); }} let __themed: __IceElement<'_, {}> = __themer.into(); __themed.map({mapped}).into() }}",
                themer.rust_path,
                themer.output.rust(&document.structs)
            ))
        }
        ViewNode::Shader {
            function,
            args,
            width,
            height,
            route,
            span,
            ..
        } => {
            let shader = find_extern_function(document, function, ExternKind::Shader)
                .ok_or_else(|| Error::new("E191", span, format!("unknown shader `{function}`")))?;
            let args = expr_list_code(args, env, document)?;
            let mut code = format!("::iced::widget::Shader::new({}({args}))", shader.rust_path);
            append_dimensions(&mut code, [width, height], env, document)?;
            let output = shader.output.rust(&document.structs);
            let mapped = if let Some(route) = route {
                route_callback_code(route, "__value", "__value", env, document, message)?
            } else {
                format!("move |__value| {message}::__ExternNoop")
            };
            Ok(format!(
                "{{ let __shader: __IceElement<'_, {output}> = {code}.into(); __shader.map({mapped}).into() }}"
            ))
        }
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, id, message, env, document, scope)?;
    Ok(Some(rendered))
}

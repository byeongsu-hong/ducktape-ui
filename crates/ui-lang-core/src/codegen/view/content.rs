use super::*;

pub(in crate::codegen) fn render_content(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
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
        ViewNode::Rule { .. } => {
            render_rule(document.program().resolved_rule_for(node)?, document, env)
        }
        ViewNode::QrCode { .. } => render_qr_code(
            document.program().resolved_qr_code_for(node)?,
            document,
            env,
        ),
        ViewNode::Space { .. } => {
            render_space(document.program().resolved_space_for(node)?, document, env)
        }
        ViewNode::Component { span, .. } => {
            let call = document.program().component_call(span)?;
            let component = document.program().component(call.component);
            let name = &component.name;
            let mut component_env = HashMap::new();
            let default_env = HashMap::new();
            for argument in &call.arguments {
                let value_env = if argument.uses_definition_scope() {
                    &default_env
                } else {
                    env
                };
                let state = argument
                    .writable
                    .as_ref()
                    .map(|state| {
                        env.get(state.name())
                            .and_then(|binding| binding.state.clone())
                            .ok_or_else(|| {
                                Error::new(
                                    "E196",
                                    span,
                                    format!(
                                        "lowered writable state `{}` is absent from the render environment",
                                        state.name()
                                    ),
                                )
                            })
                    })
                    .transpose()?;
                component_env.insert(
                    argument.name.clone(),
                    Binding {
                        code: checked_expr_use_code(
                            document.program(),
                            argument.expression,
                            value_env,
                            ValueMode::Borrowed,
                        )?,
                        ty: argument.ty.clone(),
                        local: false,
                        state,
                        owner: Some(BindingOwner::Value(CheckedValueRef::ComponentParam(
                            argument.param,
                        ))),
                    },
                );
            }
            if let ComponentOutputRoute::Direct { output, route, .. } = &call.output {
                component_env.insert(
                    component_output_key(name),
                    Binding {
                        code: route_callback_code(
                            route, "__value", "__value", env, document, message,
                        )?,
                        ty: output.clone(),
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
            }
            for event in &call.events {
                let payloads = (0..event.payloads().len())
                    .map(|index| format!("__event_{index}"))
                    .collect::<Vec<_>>();
                let payload_refs = payloads.iter().map(String::as_str).collect::<Vec<_>>();
                let code = match event {
                    ResolvedEventRoute::Direct { route, .. } => ordered_route_callback_code(
                        route,
                        &payloads.join(", "),
                        &payload_refs,
                        env,
                        document,
                        message,
                    )?,
                    ResolvedEventRoute::Forward {
                        outer_component, ..
                    } => {
                        let outer = &document.program().component(*outer_component).name;
                        component_event(env, outer, event.name())
                            .ok_or_else(|| {
                                Error::new(
                                    "E196",
                                    span,
                                    format!(
                                        "lowered forwarded event `{}` is absent from component context",
                                        event.name()
                                    ),
                                )
                            })?
                            .code
                            .clone()
                    }
                };
                component_env.insert(
                    component_event_key(name, event.name()),
                    Binding {
                        code,
                        ty: Type::Unit,
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
            }
            let component_slots = component_slot_context(&call.slots, document, env, slot)?;
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
                        owner: None,
                    },
                );
            }
            let component_scope = match &call.scope {
                ComponentScope::Implicit { call_site, .. } => {
                    let scope = reconciliation_scope(scope, env);
                    format!("format!(\"{{}}/{}@{}\", {scope})", name, call_site)
                }
                ComponentScope::Explicit { id, .. } => id_code(id, scope, env, document)?,
            };
            set_reconciliation_scope(&mut component_env, component_scope.clone());
            let scope_binding = component_scope_binding(name, call.binding_site);
            if call.storage != ComponentStorage::Stateless {
                let field = component_state_field(name);
                let states = match call.storage {
                    ComponentStorage::Retained => format!("self.{field}"),
                    ComponentStorage::Mounted => format!("self.{field}.values()"),
                    ComponentStorage::Stateless => unreachable!(),
                };
                for state in &component.states {
                    let initial =
                        resolved_initializer_code(&state.initializer, document.program())?;
                    component_env.insert(
                        state.name.clone(),
                        Binding {
                            code: format!(
                                "{states}.get(&{scope_binding}).map_or_else(|| {}, |__state| __state.{}.clone())",
                                initial,
                                state.name
                            ),
                            ty: state.ty.clone(),
                            local: true,
                            state: Some(StateBinding::Component {
                                component: name.clone(),
                                name: state.name.clone(),
                                scope: scope_binding.clone(),
                            }),
                            owner: Some(BindingOwner::Value(CheckedValueRef::ComponentState(
                                state.id,
                            ))),
                        },
                    );
                }
                insert_component_context(
                    &mut component_env,
                    name,
                    Binding {
                        code: scope_binding.clone(),
                        ty: Type::Unit,
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
            }
            if !call.events.is_empty() && call.storage == ComponentStorage::Stateless {
                insert_component_context(
                    &mut component_env,
                    name,
                    Binding {
                        code: component_scope.clone(),
                        ty: Type::Unit,
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
            }
            let render_scope = if call.storage == ComponentStorage::Stateless {
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
            let rendered = if call.storage == ComponentStorage::Stateless {
                rendered
            } else {
                let mount = (call.storage == ComponentStorage::Mounted).then(|| {
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

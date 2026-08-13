use super::*;

pub(in crate::codegen) fn render_content(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let view = document.resolved_view(node)?;
    let identity = match view.kind {
        ResolvedViewKind::Rule
        | ResolvedViewKind::QrCode
        | ResolvedViewKind::Space
        | ResolvedViewKind::ExternComponent
        | ResolvedViewKind::Themer
        | ResolvedViewKind::Shader => view.identity.as_ref(),
        _ => None,
    };
    let rendered = match &view.kind {
        ResolvedViewKind::Rule => render_rule(document.resolved_rule(node)?, document, env),
        ResolvedViewKind::QrCode => render_qr_code(document.resolved_qr_code(node)?, document, env),
        ResolvedViewKind::Space => render_space(document.resolved_space(node)?, document, env),
        ResolvedViewKind::Component { call } => {
            let call = document.component_call_by_id(*call)?;
            let component = document.component(call.component);
            let name = &component.name;
            // Alias the enclosing environment's callback bindings (an outer
            // component's output/event callbacks — caller-built closures) to
            // stable identifiers: an outlined method can then take each used
            // callback as a typed parameter, with the original closure
            // expression evaluated at the call site. Sorted for a
            // deterministic numbering; skipped inside lazy closures where
            // nothing outlines anyway.
            let mut callback_aliases: Vec<(String, String, String)> = Vec::new();
            let mut aliased_env = ScopedBindingEnv::new(env);
            if outline::outlining_active() {
                let mut sources: Vec<(String, Binding)> = Vec::new();
                env.visit(&mut |binding_name, binding| {
                    if is_component_callback_key(binding_name) {
                        sources.push((binding_name.to_owned(), binding.clone()));
                    }
                });
                sources.sort_by(|left, right| left.0.cmp(&right.0));
                for (index, (key, binding)) in sources.into_iter().enumerate() {
                    let ident = format!("__ice_cb_{index}");
                    let mut alias = binding.clone();
                    alias.code = ident.clone();
                    callback_aliases.push((ident, key.clone(), binding.code));
                    aliased_env.insert(key, alias);
                }
            }
            // Records whether any argument, route, or slot resolution touched
            // a binding that could reference a render-site local. Scope
            // expressions are deliberately resolved against the RAW `env`
            // below: the scope value is evaluated at the call site either
            // way, so its locality never blocks outlining.
            let recording = RecordingEnv::new(&aliased_env);
            let scope_binding = component_scope_binding(name, call.binding_site);
            let mut component_env = HashMap::new();
            let default_env = HashMap::new();
            // Arguments whose expressions reference render-site local VALUES
            // (loop items, a window id) become by-value parameters of the
            // outlined method: (ident, rust type, owned call-site code).
            let mut value_params: Vec<(String, String, String)> = Vec::new();
            for (argument_index, argument) in call.arguments.iter().enumerate() {
                // A per-argument recorder (independent of the arm's — its
                // findings are absorbed below so locals covered by a value
                // parameter never block the enclosing decision) decides how
                // this argument's baked expression travels.
                let arg_recording = RecordingEnv::new(&aliased_env);
                let value_env: &dyn BindingEnvironment = if argument.uses_definition_scope() {
                    &default_env
                } else {
                    &arg_recording
                };
                let state = argument
                    .writable
                    .as_ref()
                    .map(|state| {
                        arg_recording
                            .get(state.name())
                            .and_then(|binding| binding.state.clone())
                            .ok_or_else(|| {
                                document.invariant_at_origin(
                                    view.origin,
                                    format!(
                                        "lowered writable state `{}` is absent from the render environment",
                                        state.name()
                                    ),
                                )
                            })
                    })
                    .transpose()?;
                let baked = resolved_expr_use_code(
                    document,
                    argument.expression,
                    value_env,
                    ValueMode::Borrowed,
                )?;
                let parameterize = outline::outlining_active()
                    && !arg_recording.site_capturing()
                    && arg_recording.touched_local_values()
                    && state.is_none();
                let code = if parameterize {
                    let ident = format!("__ice_arg_{argument_index}");
                    let owned = resolved_expr_use_code(
                        document,
                        argument.expression,
                        env,
                        ValueMode::Owned,
                    )?;
                    value_params.push((
                        ident.clone(),
                        rust_type_code(document, &argument.ty),
                        owned,
                    ));
                    component_env.insert(
                        value_param_key(&argument.name),
                        Binding {
                            code: String::new(),
                            ty: Type::Bool,
                            local: true,
                            state: None,
                            owner: None,
                        },
                    );
                    ident
                } else {
                    // Locals baked verbatim (a writable prop, or while the
                    // use is bound to render inline) must block outlining.
                    if arg_recording.touched_local_values() {
                        recording.absorb_locals(&arg_recording);
                    }
                    baked
                };
                recording.absorb_non_locals(&arg_recording);
                component_env.insert(
                    argument.name.clone(),
                    Binding {
                        code,
                        ty: argument.ty.clone(),
                        local: false,
                        state,
                        owner: Some(BindingOwner::Value(ResolvedValueRef::ComponentParam(
                            argument.param,
                        ))),
                    },
                );
                if !arg_recording.site_capturing() && !arg_recording.touched_local_values() {
                    let locals = arg_recording.scope_locals();
                    component_env.insert(
                        self_backed_param_key(&argument.name),
                        Binding {
                            code: locals.into_iter().collect::<Vec<_>>().join(","),
                            ty: Type::Bool,
                            local: true,
                            state: None,
                            owner: None,
                        },
                    );
                }
            }
            if let ComponentOutputRoute::Direct { output, route, .. } = &call.output {
                component_env.insert(
                    component_output_key(name),
                    Binding {
                        code: resolved_interaction_route_callback_code(
                            route,
                            "__value",
                            &["__value"],
                            &recording,
                            document,
                            message,
                        )?,
                        ty: output.clone(),
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
                component_env.insert(
                    callback_sig_key(&component_output_key(name)),
                    Binding {
                        code: format!(
                            "impl Fn({}) -> {message} + Clone + 'static",
                            rust_type_code(document, output)
                        ),
                        ty: Type::Unit,
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
            }
            for (event_index, event) in call.events.iter().enumerate() {
                document.validate_component_call_event_contract(call, event_index)?;
                let payloads = (0..event.payloads().len())
                    .map(|index| format!("__event_{index}"))
                    .collect::<Vec<_>>();
                let payload_refs = payloads.iter().map(String::as_str).collect::<Vec<_>>();
                let code = match event {
                    ResolvedEventRoute::Direct { route, .. } => {
                        resolved_interaction_route_callback_code(
                            route,
                            &payloads.join(", "),
                            &payload_refs,
                            &recording,
                            document,
                            message,
                        )?
                    }
                    ResolvedEventRoute::Forward {
                        event: _,
                        name: callee_event_name,
                        payloads: callee_payloads,
                        outer_component,
                        outer_component_name,
                        outer_event,
                        outer_event_name,
                        outer_payloads,
                        origin,
                        ..
                    } => {
                        let program = document;
                        let outer = program
                            .try_component(*outer_component)
                            .filter(|component| {
                                component.id == *outer_component
                                    && component.name == *outer_component_name
                                    && outer_event.component == *outer_component
                                    && *outer_event_name == *callee_event_name
                                    && outer_payloads == callee_payloads
                                    && program.component_event_matches(
                                        *outer_event,
                                        outer_event_name,
                                        outer_payloads,
                                    )
                            })
                            .ok_or_else(|| {
                                program.invariant_at_origin(
                                    *origin,
                                    format!(
                                        "lowered forwarded event `{}` has an invalid outer contract",
                                        callee_event_name
                                    ),
                                )
                            })?;
                        component_event(&recording, &outer.name, outer_event_name)
                            .ok_or_else(|| {
                                program.invariant_at_origin(
                                    *origin,
                                    format!(
                                        "lowered forwarded event `{}` is absent from component context",
                                        callee_event_name
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
                let payload_types = event
                    .payloads()
                    .iter()
                    .map(|ty| rust_type_code(document, ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                component_env.insert(
                    callback_sig_key(&component_event_key(name, event.name())),
                    Binding {
                        code: format!("impl Fn({payload_types}) -> {message} + Clone + 'static"),
                        ty: Type::Unit,
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
            }
            let component_slots = component_slot_context(&call.slots, document, &recording, slot)?;
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
                    let scope = borrowed_scope(reconciliation_scope(scope, env));
                    format!("format!(\"{{}}/{}@{}\", {scope})", name, call_site)
                }
                ComponentScope::Explicit => resolved_view_identity_code(
                    view.identity.as_ref().ok_or_else(|| {
                        document.invariant_at_origin(
                            view.origin,
                            "explicit component scope has no resolved view identity",
                        )
                    })?,
                    scope,
                    env,
                    document,
                )?,
            };
            set_reconciliation_scope(&mut component_env, format!("{scope_binding}.clone()"));
            if call.storage != ComponentStorage::Stateless {
                let field = component_state_field(name);
                let states = match call.storage {
                    ComponentStorage::Retained => format!("self.{field}"),
                    ComponentStorage::Mounted => format!("self.{field}.values()"),
                    ComponentStorage::Stateless => unreachable!(),
                };
                for state in &component.states {
                    let initial = resolved_initializer_code(&state.initializer, document)?;
                    component_env.insert(
                        state.name.clone(),
                        Binding {
                            code: component_state_read_code(
                                &states,
                                &scope_binding,
                                &initial,
                                &state.name,
                            ),
                            ty: state.ty.clone(),
                            local: true,
                            state: Some(StateBinding::Component {
                                component: name.clone(),
                                name: state.name.clone(),
                                scope: scope_binding.clone(),
                            }),
                            owner: Some(BindingOwner::Value(ResolvedValueRef::ComponentState(
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
                        code: scope_binding.clone(),
                        ty: Type::Unit,
                        local: true,
                        state: None,
                        owner: None,
                    },
                );
            }
            let render_scope = format!("{scope_binding}.clone()");
            let rendered = render_node(
                component.root,
                document,
                message,
                &component_env,
                &render_scope,
                component_slots.as_ref(),
            )?;
            let mount = (call.storage == ComponentStorage::Mounted).then(|| {
                let field = component_state_field(name);
                // An animation's identity is the instant it started, so an
                // instance that owns one has to be stored the first time it
                // renders instead of re-running its initializer every pass.
                let materialize = if component
                    .states
                    .iter()
                    .any(|state| matches!(state.ty, Type::Animation(_)))
                {
                    format!(
                        "self.{field}.values_mut().entry({scope_binding}.clone()).or_default(); "
                    )
                } else {
                    String::new()
                };
                format!("self.{field}.mount({scope_binding}.clone()); {materialize}")
            });
            let body = format!(
                "{}let __component_content: __IceElement<'_, {message}> = {rendered}; __component_content",
                mount.as_deref().unwrap_or("")
            );
            // A use whose arguments, routes, and slots resolved only
            // self-backed bindings captures nothing from this render site:
            // move its body to a method so rustc checks it as its own item
            // instead of growing `__view` (typeck/borrowck are superlinear in
            // function size). The scope expression stays here as the call
            // argument, so scopes chained through loop keys still evaluate in
            // the loop.
            let used_callbacks: Vec<&(String, String, String)> = callback_aliases
                .iter()
                .filter(|(ident, _, _)| recording.callback_uses().contains(ident))
                .collect();
            // Every used callback needs its parameter type from the sig
            // marker its inserting arm left beside it; a missing marker
            // falls back to inline rendering.
            let callback_params: Option<Vec<(String, String, String)>> = used_callbacks
                .iter()
                .map(|(ident, key, orig)| {
                    env.get(&callback_sig_key(key))
                        .map(|sig| (ident.clone(), sig.code.clone(), orig.clone()))
                })
                .collect();
            Ok(
                if outline::outlining_active()
                    && !recording.site_capturing()
                    && !recording.touched_local_values()
                    && let Some(callback_params) = callback_params
                {
                    let mut scope_locals = recording.scope_locals();
                    scope_locals.remove(&scope_binding);
                    // Group by the component DEFINITION's fragment: the body
                    // text derives from the definition, so its methods land in
                    // one file that only changes when that fragment does.
                    let group = origin_fragment_slug(
                        document,
                        document.resolved_view(component.root)?.origin,
                    );
                    let method = outline::push_outlined_method(
                        message,
                        &group,
                        &scope_binding,
                        &scope_locals,
                        recording.uses_derived_snapshot(),
                        &callback_params,
                        &value_params,
                        &body,
                    );
                    let arguments = scope_locals
                        .iter()
                        .map(|local| format!(", {local}.clone()"))
                        .collect::<String>();
                    // Cloned, not moved: the original may itself be an
                    // enclosing method's callback parameter consumed inside
                    // a loop.
                    let callback_arguments = callback_params
                        .iter()
                        .map(|(_, _, orig)| format!(", ({orig}).clone()"))
                        .collect::<String>();
                    let value_arguments = value_params
                        .iter()
                        .map(|(_, _, owned)| format!(", {owned}"))
                        .collect::<String>();
                    let derived_argument = if recording.uses_derived_snapshot() {
                        ", __ice_derived"
                    } else {
                        ""
                    };
                    // grow_stack keeps deep outlined chains from exhausting
                    // small thread stacks at debug opt levels — see
                    // ui_lang_runtime::stack_relief.
                    format!(
                        "::ui_lang_runtime::grow_stack(|| self.{method}(__ice_palette, {component_scope}{derived_argument}{arguments}{callback_arguments}{value_arguments}))"
                    )
                } else {
                    let callback_lets = used_callbacks
                        .iter()
                        .map(|(ident, _, orig)| format!("let {ident} = ({orig}).clone(); "))
                        .collect::<String>();
                    // The inline fallback still binds any value-parameterized
                    // argument idents the body was baked with.
                    let value_lets = value_params
                        .iter()
                        .map(|(ident, _, owned)| format!("let {ident} = {owned}; "))
                        .collect::<String>();
                    format!(
                        "{{ let {scope_binding} = {component_scope}; {callback_lets}{value_lets}{body} }}"
                    )
                },
            )
        }
        ResolvedViewKind::Slot {
            slot: slot_id,
            name,
            optional,
            ..
        } => {
            let slot = slot.ok_or_else(|| {
                document.invariant_at_origin(
                    view.origin,
                    "slot reached codegen without component content",
                )
            })?;
            let content = slot
                .entries
                .iter()
                .find(|entry| entry.slot == *slot_id)
                .map_or_else(
                    || {
                        if *optional {
                            Ok(None)
                        } else {
                            Err(document.invariant_at_origin(
                                view.origin,
                                format!("slot `{name}` reached codegen without component content"),
                            ))
                        }
                    },
                    |content| Ok(Some(content)),
                )?;
            let Some(content) = content else {
                return Ok(None);
            };
            // The scope the content renders under comes from here, not from
            // the call site, so it is layered ABOVE the recorder: reading it
            // back is not a capture, and an outlined method rewrites the
            // identifier it names along with the rest of its body.
            let captured = SlotRecordingEnv::new(&content.env, content.recorder.as_ref());
            let mut content_env = ScopedBindingEnv::new(&captured);
            content_env.insert(
                RECONCILIATION_SCOPE_BINDING.into(),
                reconciliation_scope_binding(scope.to_owned()),
            );
            let rendered = render_node(
                content.view,
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
        ResolvedViewKind::ExternComponent => render_extern_component(
            document.resolved_extern_component(node)?,
            document,
            message,
            env,
        ),
        ResolvedViewKind::Themer => {
            render_themer(document.resolved_themer(node)?, document, message, env)
        }
        ResolvedViewKind::Shader => {
            render_shader(document.resolved_shader(node)?, document, message, env)
        }
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, identity, message, env, document, scope)?;
    Ok(Some(rendered))
}

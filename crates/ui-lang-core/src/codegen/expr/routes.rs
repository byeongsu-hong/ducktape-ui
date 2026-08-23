use super::*;

pub(in crate::codegen) fn resolved_route_code(
    route: &crate::lower::ResolvedRoute,
    payloads: &[&str],
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
) -> Result<String, Error> {
    resolved_route_code_impl(route, payloads, None, env, program, message)
}

pub(in crate::codegen) fn resolved_route_code_with_snapshots(
    route: &crate::lower::ResolvedRoute,
    payloads: &[&str],
    snapshots: &[String],
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
) -> Result<String, Error> {
    resolved_route_code_impl(route, payloads, Some(snapshots), env, program, message)
}

fn resolved_route_code_impl(
    route: &crate::lower::ResolvedRoute,
    payloads: &[&str],
    snapshots: Option<&[String]>,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
) -> Result<String, Error> {
    let invariant = |message| program.invariant_at_origin(route.origin, message);
    let (variant, component) = match &route.target {
        crate::lower::ResolvedRouteTarget::App { handler, name } => {
            let resolved = program.try_handler(*handler).ok_or_else(|| {
                invariant("normalized app route references an invalid handler ID")
            })?;
            if resolved.owner != HandlerOwner::App || resolved.name != *name {
                return Err(invariant(
                    "normalized app route target does not match its handler contract",
                ));
            }
            (handler_variant(&resolved.name), None)
        }
        crate::lower::ResolvedRouteTarget::Component {
            component,
            handler,
            name,
        } => {
            let component_contract = program.try_component(*component).ok_or_else(|| {
                invariant("normalized component route references an invalid component ID")
            })?;
            let resolved = program.try_handler(*handler).ok_or_else(|| {
                invariant("normalized component route references an invalid handler ID")
            })?;
            if resolved.owner != HandlerOwner::Component(*component) || resolved.name != *name {
                return Err(invariant(
                    "normalized component route target does not match its handler contract",
                ));
            }
            (
                component_handler_variant(&component_contract.name, &resolved.name),
                Some(component_contract.name.as_str()),
            )
        }
    };
    let mut snapshot = 0;
    let mut args = Vec::with_capacity(route.args.len());
    for arg in &route.args {
        args.push(match arg {
            crate::lower::ResolvedRouteArg::Payload { index, .. } => payloads
                .get(*index as usize)
                .map(|payload| (*payload).to_owned())
                .ok_or_else(|| {
                    invariant("normalized route payload index is outside its payload contract")
                })?,
            crate::lower::ResolvedRouteArg::Expression(expression) => {
                if let Some(snapshots) = snapshots {
                    let code = snapshots.get(snapshot).cloned().ok_or_else(|| {
                        invariant("normalized route has fewer snapshots than expressions")
                    })?;
                    snapshot += 1;
                    code
                } else {
                    resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
                }
            }
        });
    }
    if let Some(snapshots) = snapshots
        && snapshot != snapshots.len()
    {
        return Err(invariant(
            "normalized route has more snapshots than expressions",
        ));
    }
    if let Some(component_name) = component {
        let (active, context) = env.component_context().ok_or_else(|| {
            invariant("normalized component route has no component emission scope")
        })?;
        if active != component_name {
            return Err(invariant(
                "normalized component route owner does not match emission scope",
            ));
        }
        args.insert(0, format!("({}).clone()", context.code));
    }
    if args.is_empty() {
        Ok(format!("{message}::{variant}"))
    } else {
        Ok(format!("{message}::{variant}({})", args.join(", ")))
    }
}

pub(in crate::codegen) fn resolved_interaction_route_code(
    route: &ResolvedInteractionRoute,
    payloads: &[&str],
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
) -> Result<String, Error> {
    let invariant = |message| program.invariant_at_origin(route.origin, message);
    let mut args = route
        .args
        .iter()
        .map(|arg| match arg {
            ResolvedInteractionRouteArg::Expression(expression) => {
                resolved_expr_use_code(program, *expression, env, ValueMode::Owned)
            }
            ResolvedInteractionRouteArg::Payload { index, .. } => payloads
                .get(*index as usize)
                .map(|payload| (*payload).to_owned())
                .ok_or_else(|| {
                    invariant("interaction route payload index is outside its contract")
                }),
        })
        .collect::<Result<Vec<_>, _>>()?;
    match &route.target {
        ResolvedInteractionRouteTarget::TargetHandler(handler) => {
            let target = program.try_handler(*handler).ok_or_else(|| {
                invariant("interaction route references an invalid normalized handler")
            })?;
            let variant = match target.owner {
                HandlerOwner::App => handler_variant(&target.name),
                HandlerOwner::Component(component) => {
                    let contract = program.try_component(component).ok_or_else(|| {
                        invariant("interaction route component handler has no component contract")
                    })?;
                    let (active, context) = component_context(env).ok_or_else(|| {
                        invariant("interaction component route has no component context")
                    })?;
                    if active != contract.name {
                        return Err(invariant("interaction component route context diverged"));
                    }
                    args.insert(0, format!("({}).clone()", context.code));
                    component_handler_variant(&contract.name, &target.name)
                }
                HandlerOwner::Preset(_) => {
                    return Err(invariant(
                        "interaction route cannot target a preset handler",
                    ));
                }
            };
            if args.is_empty() {
                Ok(format!("{message}::{variant}"))
            } else {
                Ok(format!("{message}::{variant}({})", args.join(", ")))
            }
        }
        ResolvedInteractionRouteTarget::OutputCallback { component, .. } => {
            let contract = program.try_component(*component).ok_or_else(|| {
                invariant("interaction component output route has no component contract")
            })?;
            let output = env
                .get(&component_output_key(&contract.name))
                .ok_or_else(|| {
                    invariant("interaction component output route has no output callback")
                })?;
            if args.len() != 1 {
                return Err(invariant(
                    "interaction component output route contract diverged",
                ));
            }
            Ok(format!("({})({})", output.code, args[0]))
        }
        ResolvedInteractionRouteTarget::NamedEvent {
            event,
            name,
            payloads: expected,
        } => {
            let contract = program.try_component(event.component).ok_or_else(|| {
                invariant("interaction named event route has no component contract")
            })?;
            if !program.component_event_matches(*event, name, expected) {
                return Err(invariant("interaction named event route contract diverged"));
            }
            let callback = component_event(env, &contract.name, name).ok_or_else(|| {
                invariant("interaction named event route has no normalized callback")
            })?;
            if args.len() != expected.len() {
                return Err(invariant("interaction named event route arity diverged"));
            }
            Ok(format!("({})({})", callback.code, args.join(", ")))
        }
    }
}

pub(in crate::codegen) fn resolved_interaction_route_callback_with_code(
    route: &ResolvedInteractionRoute,
    pattern: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    render: impl FnOnce(&dyn BindingEnvironment) -> Result<String, Error>,
) -> Result<String, Error> {
    let invariant = |message| program.invariant_at_origin(route.origin, message);
    let (component, captures_context) = match &route.target {
        ResolvedInteractionRouteTarget::TargetHandler(handler) => {
            let target = program.try_handler(*handler).ok_or_else(|| {
                invariant("interaction callback references an invalid handler ID")
            })?;
            match target.owner {
                HandlerOwner::App => (None, false),
                HandlerOwner::Component(component) => (Some(component), true),
                HandlerOwner::Preset(_) => {
                    return Err(invariant(
                        "interaction callback cannot target a preset handler",
                    ));
                }
            }
        }
        ResolvedInteractionRouteTarget::OutputCallback { .. }
        | ResolvedInteractionRouteTarget::NamedEvent { .. } => (None, false),
    };
    let component_context = component
        .map(|component| {
            let contract = program.try_component(component).ok_or_else(|| {
                invariant("interaction callback component ID is outside its arena")
            })?;
            let (active, context) = component_context(env).ok_or_else(|| {
                invariant("interaction callback has no component emission context")
            })?;
            if active != contract.name {
                return Err(invariant("interaction callback component context diverged"));
            }
            Ok((contract.name.clone(), context.code.clone()))
        })
        .transpose()?;
    let local = captures_context.then_some(component_context).flatten();
    let mut captures = Vec::<(String, String)>::new();
    if let Some((_, scope)) = &local {
        captures.push((scope.clone(), "__route_scope".into()));
    }
    let mut state_scopes = component_state_scopes(env);
    state_scopes.sort();
    state_scopes.dedup();
    for scope in state_scopes {
        if !captures.iter().any(|(captured, _)| captured == &scope) {
            captures.push((scope, format!("__route_state_scope_{}", captures.len())));
        }
    }
    // A route that calls a component callback hoists it out of the `move`
    // closure: the callback may be an outlined method's parameter, and a
    // `move` closure would move the parameter itself out of the render —
    // `(cb).clone()` outside the closure borrows instead, and the closure
    // moves its private clone.
    let route_callback = match &route.target {
        ResolvedInteractionRouteTarget::OutputCallback { component, .. } => {
            let contract = program.try_component(*component).ok_or_else(|| {
                invariant("interaction callback output route has no component contract")
            })?;
            Some(component_output_key(&contract.name))
        }
        ResolvedInteractionRouteTarget::NamedEvent { event, name, .. } => {
            let contract = program.try_component(event.component).ok_or_else(|| {
                invariant("interaction callback event route has no component contract")
            })?;
            Some(component_event_key(&contract.name, name))
        }
        ResolvedInteractionRouteTarget::TargetHandler(_) => None,
    };
    let route_callback =
        route_callback.and_then(|key| env.get(&key).map(|cb| (key, cb.code.clone())));
    if captures.is_empty() && route_callback.is_none() {
        let body = render(env)?;
        return Ok(format!("move |{pattern}| {body}"));
    }
    // Overlay only the bindings the capture aliasing actually rewrites, and
    // let every other lookup fall through to `env`: a recording environment
    // must keep observing what the route arguments resolve, and the aliased
    // codes themselves are callback-internal (`__route_scope`/`__route_state_
    // scope_*` are bound right before the closure).
    let snapshot = env.snapshot();
    let mut callback_env = ScopedBindingEnv::new(env);
    for (name, entry) in &snapshot {
        let mut rewritten = entry.clone();
        let mut changed = false;
        for (scope, alias) in &captures {
            if rewritten.code.contains(scope.as_str()) {
                rewritten.code = rewritten.code.replace(scope.as_str(), alias);
                changed = true;
            }
            if let Some(StateBinding::Component {
                scope: state_scope, ..
            }) = &mut rewritten.state
                && state_scope == scope
            {
                *state_scope = alias.clone();
                changed = true;
            }
        }
        if changed {
            callback_env.insert(name.clone(), rewritten);
        }
    }
    if let Some((component, _)) = &local {
        let key = component_context_key(component);
        let mut context = snapshot
            .get(&key)
            .ok_or_else(|| invariant("interaction callback lost its component context binding"))?
            .clone();
        context.code = "__route_scope".into();
        callback_env.insert(key, context);
    }
    let mut hoists = captures
        .iter()
        .map(|(scope, alias)| format!("let {alias} = ({}).clone();", borrowed_scope(scope)))
        .collect::<String>();
    if let Some((key, code)) = &route_callback {
        let mut callback = snapshot
            .get(key)
            .ok_or_else(|| invariant("interaction route callback binding vanished"))?
            .clone();
        callback.code = "__route_callback".into();
        callback_env.insert(key.clone(), callback);
        hoists.push_str(&format!("let __route_callback = ({code}).clone();"));
    }
    let body = render(&callback_env)?;
    Ok(format!("{{ {hoists} move |{pattern}| {body} }}"))
}

pub(in crate::codegen) fn resolved_interaction_route_callback_code(
    route: &ResolvedInteractionRoute,
    pattern: &str,
    payloads: &[&str],
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
) -> Result<String, Error> {
    resolved_interaction_route_callback_with_code(route, pattern, env, program, |callback_env| {
        resolved_interaction_route_code(route, payloads, callback_env, program, message)
    })
}

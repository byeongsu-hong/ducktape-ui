use super::*;

pub(in crate::codegen) fn resolved_route_code(
    route: &crate::lower::ResolvedRoute,
    payloads: &[&str],
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
    let mut args = route
        .args
        .iter()
        .map(|arg| match arg {
            crate::lower::ResolvedRouteArg::Payload { index, .. } => payloads
                .get(*index as usize)
                .map(|payload| (*payload).to_owned())
                .ok_or_else(|| {
                    invariant("normalized route payload index is outside its payload contract")
                }),
            crate::lower::ResolvedRouteArg::Expression(expression) => {
                checked_expr_use_code(program, *expression, env, ValueMode::Owned)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
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
                checked_expr_use_code(program, *expression, env, ValueMode::Owned)
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
            let output = component_output(env).ok_or_else(|| {
                invariant("interaction component output route has no output callback")
            })?;
            if program.try_component(*component).is_none() || args.len() != 1 {
                return Err(invariant(
                    "interaction component output route contract diverged",
                ));
            }
            Ok(format!("({})({})", output.code, args[0]))
        }
        ResolvedInteractionRouteTarget::NamedEvent {
            event: _,
            name,
            payloads: expected,
        } => {
            let (component, _) = component_context(env).ok_or_else(|| {
                invariant("interaction named event route has no component context")
            })?;
            let callback = component_event(env, component, name).ok_or_else(|| {
                invariant("interaction named event route has no normalized callback")
            })?;
            if args.len() != expected.len() {
                return Err(invariant("interaction named event route arity diverged"));
            }
            Ok(format!("({})({})", callback.code, args.join(", ")))
        }
    }
}

pub(in crate::codegen) fn widget_target_field_type(field: &str) -> Option<Type> {
    match field {
        "kind" => Some(Type::Str),
        "id" => Some(Type::Option(Box::new(Type::WidgetId))),
        "x" | "y" | "width" | "height" => Some(Type::F64),
        "visible_x" | "visible_y" | "visible_width" | "visible_height" | "content_x"
        | "content_y" | "content_width" | "content_height" | "translation_x" | "translation_y" => {
            Some(Type::Option(Box::new(Type::F64)))
        }
        "content" => Some(Type::Option(Box::new(Type::Str))),
        _ => None,
    }
}

pub(in crate::codegen) fn route_code(
    route: &Route,
    payload: &str,
    env: &dyn BindingEnvironment,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    if let Some(code) = named_component_emission_code(route, &[payload], false, env, document) {
        return code;
    }
    if route.handler == "emit"
        && let Some(output) = component_output(env)
    {
        let [arg] = route.args.as_slice() else {
            unreachable!("checker requires one component output");
        };
        let value = match arg {
            RouteArg::Payload => payload.to_owned(),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned)?,
        };
        return Ok(format!("({})({value})", output.code));
    }
    let local = local_route(route, env, document);
    let variant = local.map_or_else(
        || handler_variant(&route.handler),
        |(component, _)| component_handler_variant(component, &route.handler),
    );
    if route.args.is_empty() && local.is_none() {
        return Ok(format!("{message}::{variant}"));
    }
    let mut args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok(payload.into()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?;
    if let Some((_, context)) = local {
        args.insert(0, format!("({}).clone()", context.code));
    }
    Ok(format!("{message}::{variant}({})", args.join(", ")))
}

pub(in crate::codegen) fn ordered_route_code(
    route: &Route,
    payloads: &[&str],
    env: &dyn BindingEnvironment,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    if let Some(code) = named_component_emission_code(route, payloads, true, env, document) {
        return code;
    }
    if route.handler == "emit" && component_output(env).is_some() {
        return route_code(route, payloads[0], env, document, message);
    }
    let local = local_route(route, env, document);
    let variant = local.map_or_else(
        || handler_variant(&route.handler),
        |(component, _)| component_handler_variant(component, &route.handler),
    );
    if route.args.is_empty() && local.is_none() {
        return Ok(format!("{message}::{variant}"));
    }
    let mut payload = payloads.iter();
    let mut args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok((*payload.next().expect("checked payload count")).to_owned()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?;
    if let Some((_, context)) = local {
        args.insert(0, format!("({}).clone()", context.code));
    }
    Ok(format!("{message}::{variant}({})", args.join(", ")))
}

fn named_component_emission_code(
    route: &Route,
    payloads: &[&str],
    ordered: bool,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Option<Result<String, Error>> {
    if route.handler != "emit" {
        return None;
    }
    let (component, _) = component_context(env)?;
    let (name, args) = route.args.split_first()?;
    let RouteArg::Expr(Expr::Path(path)) = name else {
        return None;
    };
    let [name] = path.as_slice() else {
        return None;
    };
    let callback = component_event(env, component, name)?;
    let mut payload_index = 0;
    let values = args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => {
                let index = if ordered { payload_index } else { 0 };
                payload_index += 1;
                Ok(payloads[index].to_owned())
            }
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>();
    Some(values.map(|values| format!("({})({})", callback.code, values.join(", "))))
}

fn local_route<'a>(
    route: &Route,
    env: &'a dyn BindingEnvironment,
    document: &Document,
) -> Option<(&'a str, &'a Binding)> {
    component_context(env).filter(|(component, _)| {
        document.components.iter().any(|item| {
            item.name == *component
                && item
                    .handlers
                    .iter()
                    .any(|handler| handler.name == route.handler)
        })
    })
}

pub(in crate::codegen) fn route_callback_with_code(
    route: &Route,
    pattern: &str,
    env: &dyn BindingEnvironment,
    document: &Document,
    render: impl FnOnce(&HashMap<String, Binding>) -> Result<String, Error>,
) -> Result<String, Error> {
    let local = local_route(route, env, document)
        .map(|(component, binding)| (component.to_owned(), binding.code.clone()));
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
    let mut callback_env = env.snapshot();
    if let Some((component, _)) = &local {
        callback_env
            .get_mut(&component_context_key(component))
            .expect("component context")
            .code = "__route_scope".into();
    }
    for binding in callback_env.values_mut() {
        for (scope, alias) in &captures {
            binding.code = binding.code.replace(scope, alias);
            if let Some(StateBinding::Component {
                scope: state_scope, ..
            }) = &mut binding.state
                && state_scope == scope
            {
                *state_scope = alias.clone();
            }
        }
    }
    let body = render(&callback_env)?;
    if captures.is_empty() {
        Ok(format!("move |{pattern}| {body}"))
    } else {
        let captures = captures
            .iter()
            .map(|(scope, alias)| format!("let {alias} = ({scope}).clone();"))
            .collect::<String>();
        Ok(format!("{{ {captures} move |{pattern}| {body} }}"))
    }
}

pub(in crate::codegen) fn resolved_interaction_route_callback_with_code(
    route: &ResolvedInteractionRoute,
    pattern: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    render: impl FnOnce(&HashMap<String, Binding>) -> Result<String, Error>,
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
        ResolvedInteractionRouteTarget::OutputCallback { component, .. } => {
            (Some(*component), false)
        }
        ResolvedInteractionRouteTarget::NamedEvent { event, .. } => (Some(event.component), false),
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
    let mut callback_env = env.snapshot();
    if let Some((component, _)) = &local {
        callback_env
            .get_mut(&component_context_key(component))
            .ok_or_else(|| invariant("interaction callback lost its component context binding"))?
            .code = "__route_scope".into();
    }
    for entry in callback_env.values_mut() {
        for (scope, alias) in &captures {
            entry.code = entry.code.replace(scope, alias);
            if let Some(StateBinding::Component {
                scope: state_scope, ..
            }) = &mut entry.state
                && state_scope == scope
            {
                *state_scope = alias.clone();
            }
        }
    }
    let body = render(&callback_env)?;
    if captures.is_empty() {
        Ok(format!("move |{pattern}| {body}"))
    } else {
        let captures = captures
            .iter()
            .map(|(scope, alias)| format!("let {alias} = ({scope}).clone();"))
            .collect::<String>();
        Ok(format!("{{ {captures} move |{pattern}| {body} }}"))
    }
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

pub(in crate::codegen) fn route_callback_code(
    route: &Route,
    pattern: &str,
    payload: &str,
    env: &dyn BindingEnvironment,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    route_callback_with_code(route, pattern, env, document, |callback_env| {
        route_code(route, payload, callback_env, document, message)
    })
}

pub(in crate::codegen) fn ordered_route_callback_code(
    route: &Route,
    pattern: &str,
    payloads: &[&str],
    env: &dyn BindingEnvironment,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    route_callback_with_code(route, pattern, env, document, |callback_env| {
        ordered_route_code(route, payloads, callback_env, document, message)
    })
}

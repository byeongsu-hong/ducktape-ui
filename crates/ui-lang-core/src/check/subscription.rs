use super::*;

fn require_single_payload_routes<'a>(
    routes: impl IntoIterator<Item = &'a Route>,
    span: &Span,
    message: &'static str,
) -> Result<(), Error> {
    if routes.into_iter().any(|route| {
        route.args.len() > 1
            || route
                .args
                .iter()
                .any(|arg| !matches!(arg, RouteArg::Payload))
    }) {
        return Err(Error::new("E127", span, message));
    }
    Ok(())
}

pub(in crate::check) fn native_subscription_payloads(
    source: &SubscriptionSource,
    window_id: bool,
) -> Option<Vec<Type>> {
    let mut payloads = match source {
        SubscriptionSource::Every { .. } => vec![Type::Instant],
        SubscriptionSource::Event { .. } => vec![Type::Event],
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => Vec::new(),
            InputMethodEvent::Preedit => vec![
                Type::Str,
                Type::Option(Box::new(Type::I64)),
                Type::Option(Box::new(Type::I64)),
            ],
            InputMethodEvent::Commit => vec![Type::Str],
        },
        SubscriptionSource::Keyboard(KeyboardEvent::Press) => vec![Type::KeyPress],
        SubscriptionSource::Keyboard(KeyboardEvent::Release) => vec![Type::KeyRelease],
        SubscriptionSource::Keyboard(KeyboardEvent::Modifiers) => vec![Type::KeyModifiers],
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => Vec::new(),
            MouseEvent::Moved => vec![Type::F64, Type::F64],
            MouseEvent::Pressed | MouseEvent::Released => vec![Type::MouseButton],
            MouseEvent::Wheel => vec![Type::F64, Type::F64, Type::Bool],
        },
        SubscriptionSource::SystemTheme => vec![Type::Str],
        SubscriptionSource::Touch(_) => vec![Type::TouchFinger, Type::F64, Type::F64],
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => Vec::new(),
            WindowEvent::Opened => vec![
                Type::Option(Box::new(Type::F64)),
                Type::Option(Box::new(Type::F64)),
                Type::F64,
                Type::F64,
            ],
            WindowEvent::Moved | WindowEvent::Resized => vec![Type::F64, Type::F64],
            WindowEvent::Rescaled => vec![Type::F64],
            WindowEvent::FileHovered | WindowEvent::FileDropped => vec![Type::Str],
        },
        SubscriptionSource::Repeat { .. }
        | SubscriptionSource::Run { .. }
        | SubscriptionSource::Recipe { .. }
        | SubscriptionSource::Events { .. }
        | SubscriptionSource::Extern { .. } => return None,
    };
    if window_id {
        payloads.insert(0, Type::WindowId);
    }
    Some(payloads)
}

pub(in crate::check) fn canvas_event_name(source: &SubscriptionSource) -> Option<&'static str> {
    Some(match source {
        SubscriptionSource::InputMethod(InputMethodEvent::Opened) => "input-method opened",
        SubscriptionSource::InputMethod(InputMethodEvent::Preedit) => "input-method preedit",
        SubscriptionSource::InputMethod(InputMethodEvent::Commit) => "input-method commit",
        SubscriptionSource::InputMethod(InputMethodEvent::Closed) => "input-method closed",
        SubscriptionSource::Keyboard(KeyboardEvent::Press) => "keyboard press",
        SubscriptionSource::Keyboard(KeyboardEvent::Release) => "keyboard release",
        SubscriptionSource::Keyboard(KeyboardEvent::Modifiers) => "keyboard modifiers",
        SubscriptionSource::Mouse(MouseEvent::Entered) => "mouse entered",
        SubscriptionSource::Mouse(MouseEvent::Left) => "mouse left",
        SubscriptionSource::Mouse(MouseEvent::Moved) => "mouse moved",
        SubscriptionSource::Mouse(MouseEvent::Pressed) => "mouse pressed",
        SubscriptionSource::Mouse(MouseEvent::Released) => "mouse released",
        SubscriptionSource::Mouse(MouseEvent::Wheel) => "mouse wheel",
        SubscriptionSource::Touch(TouchEvent::Pressed) => "touch pressed",
        SubscriptionSource::Touch(TouchEvent::Moved) => "touch moved",
        SubscriptionSource::Touch(TouchEvent::Lifted) => "touch lifted",
        SubscriptionSource::Touch(TouchEvent::Lost) => "touch lost",
        SubscriptionSource::Window(WindowEvent::Frame) => "window frame",
        SubscriptionSource::Window(WindowEvent::Opened) => "window opened",
        SubscriptionSource::Window(WindowEvent::Closed) => "window closed",
        SubscriptionSource::Window(WindowEvent::Moved) => "window moved",
        SubscriptionSource::Window(WindowEvent::Resized) => "window resized",
        SubscriptionSource::Window(WindowEvent::Rescaled) => "window rescaled",
        SubscriptionSource::Window(WindowEvent::CloseRequested) => "window close-request",
        SubscriptionSource::Window(WindowEvent::Focused) => "window focused",
        SubscriptionSource::Window(WindowEvent::Unfocused) => "window unfocused",
        SubscriptionSource::Window(WindowEvent::FileHovered) => "window file-hovered",
        SubscriptionSource::Window(WindowEvent::FileDropped) => "window file-dropped",
        SubscriptionSource::Window(WindowEvent::FilesHoveredLeft) => "window files-hovered-left",
        _ => return None,
    })
}

pub(in crate::check) fn valid_canvas_cursor(value: &str) -> bool {
    matches!(
        value,
        "none"
            | "hidden"
            | "idle"
            | "context-menu"
            | "help"
            | "pointer"
            | "progress"
            | "wait"
            | "cell"
            | "crosshair"
            | "text"
            | "alias"
            | "copy"
            | "move"
            | "no-drop"
            | "not-allowed"
            | "grab"
            | "grabbing"
            | "resize-horizontal"
            | "resize-vertical"
            | "resize-diagonal-up"
            | "resize-diagonal-down"
            | "resize-column"
            | "resize-row"
            | "all-scroll"
            | "zoom-in"
            | "zoom-out"
    )
}

pub(in crate::check) fn infer_subscriptions(
    document: &Document,
    states: &HashMap<String, Type>,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    for subscription in &document.subscriptions {
        if let Some(condition) = &subscription.condition {
            require_type(
                &expr_type(condition, states, document, &subscription.span)?,
                &Type::Bool,
                &subscription.span,
            )?;
        }
        let mut payloads = match &subscription.source {
            SubscriptionSource::Repeat { function, .. } => {
                let source =
                    extern_function(document, function, ExternKind::Future, &subscription.span)?;
                check_call_args(source, &[], states, document, &subscription.span)?;
                vec![source.error.as_ref().map_or_else(
                    || source.output.clone(),
                    |error| Type::Result(Box::new(source.output.clone()), Box::new(error.clone())),
                )]
            }
            SubscriptionSource::Run { function, args } => {
                let source =
                    extern_function(document, function, ExternKind::Stream, &subscription.span)?;
                check_call_args(source, args, states, document, &subscription.span)?;
                for arg in args {
                    let ty = expr_type(arg, states, document, &subscription.span)?;
                    if !lazy_hashable(&ty) {
                        return Err(Error::new(
                            "E129",
                            &subscription.span,
                            format!(
                                "subscription run data must be hashable, got `{}`",
                                ty.display()
                            ),
                        ));
                    }
                }
                vec![source.error.as_ref().map_or_else(
                    || source.output.clone(),
                    |error| Type::Result(Box::new(source.output.clone()), Box::new(error.clone())),
                )]
            }
            SubscriptionSource::Recipe { function, args } => {
                let source =
                    extern_function(document, function, ExternKind::Recipe, &subscription.span)?;
                check_call_args(source, args, states, document, &subscription.span)?;
                vec![source.output.clone()]
            }
            SubscriptionSource::Events { id, filter } => {
                let source = extern_function(
                    document,
                    filter,
                    ExternKind::EventFilter,
                    &subscription.span,
                )?;
                let id = expr_type(id, states, document, &subscription.span)?;
                if !lazy_hashable(&id) {
                    return Err(Error::new(
                        "E129",
                        &subscription.span,
                        format!(
                            "raw event identity must be hashable, got `{}`",
                            id.display()
                        ),
                    ));
                }
                vec![source.output.clone()]
            }
            SubscriptionSource::Extern { function, args } => {
                let source = extern_function(
                    document,
                    function,
                    ExternKind::Subscription,
                    &subscription.span,
                )?;
                check_call_args(source, args, states, document, &subscription.span)?;
                vec![source.output.clone()]
            }
            source => native_subscription_payloads(source, subscription.window_id)
                .expect("native subscription payloads"),
        };
        if let Some(filter) = &subscription.filter {
            let function = extern_function(document, filter, ExternKind::Sync, &subscription.span)?;
            if function.params.len() != payloads.len() {
                return Err(Error::new(
                    "E142",
                    &subscription.span,
                    format!(
                        "subscription filter `{filter}` expects {} payloads, got {}",
                        function.params.len(),
                        payloads.len()
                    ),
                ));
            }
            for (actual, (_, expected)) in payloads.iter().zip(&function.params) {
                require_type(actual, expected, &subscription.span)?;
            }
            let Type::Option(output) = &function.output else {
                return Err(Error::new(
                    "E142",
                    &subscription.span,
                    format!("subscription filter `{filter}` must return an optional value"),
                ));
            };
            payloads = vec![(**output).clone()];
        }
        if let Some(context) = &subscription.context {
            let context = expr_type(context, states, document, &subscription.span)?;
            if !lazy_hashable(&context) {
                return Err(Error::new(
                    "E129",
                    &subscription.span,
                    format!(
                        "subscription context must be hashable, got `{}`",
                        context.display()
                    ),
                ));
            }
            payloads.insert(0, context);
        }
        if subscription
            .route
            .args
            .iter()
            .any(|arg| !matches!(arg, RouteArg::Payload))
        {
            return Err(Error::new(
                "E127",
                &subscription.span,
                "subscription routes only accept `_`; read other state in the handler",
            ));
        }
        if subscription.route.args.is_empty() {
            infer_route(&subscription.route, None, states, document, signatures)?;
        } else {
            infer_ordered_payload_route(
                &subscription.route,
                &payloads,
                states,
                document,
                signatures,
                "subscription",
            )?;
        }
    }
    Ok(())
}

pub(in crate::check) fn infer_runs(
    handler: &Handler,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    value_env: &dyn ExprTypeEnv,
    route_env: &dyn ExprTypeEnv,
) -> Result<(), Error> {
    let params = handler
        .params
        .iter()
        .map(|param| (param.name.as_str(), Type::Unknown));
    infer_run_statements(
        &handler.statements,
        params,
        document,
        signatures,
        value_env,
        route_env,
    )
}

fn infer_run_statements<'a>(
    statements: &[Statement],
    params: impl Iterator<Item = (&'a str, Type)>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    value_env: &dyn ExprTypeEnv,
    route_env: &dyn ExprTypeEnv,
) -> Result<(), Error> {
    let mut unknown_env = ScopedTypeEnv::new(route_env);
    let mut local_env = ScopedTypeEnv::new(value_env);
    for (name, ty) in params {
        unknown_env.insert(name.to_owned(), ty.clone());
        local_env.insert(name.to_owned(), ty);
    }
    for statement in statements {
        if let Statement::Let { name, value, span } = statement {
            let ty = expr_type(value, &local_env, document, span)?;
            unknown_env.insert(name.clone(), ty.clone());
            local_env.insert(name.clone(), ty);
            continue;
        }
        let nested: Option<&[Statement]> = match statement {
            Statement::TaskGroup { statements, .. } => Some(statements),
            Statement::Abortable { task, .. } => Some(::std::slice::from_ref(task.as_ref())),
            _ => None,
        };
        if let Some(statements) = nested {
            infer_run_statements(
                statements,
                ::std::iter::empty(),
                document,
                signatures,
                &local_env,
                &unknown_env,
            )?;
            continue;
        }
        if let Statement::WidgetOperation {
            operation: WidgetOperation::Focused { .. },
            route: Some(route),
            ..
        } = statement
        {
            infer_route(route, Some(Type::Bool), &unknown_env, document, signatures)?;
        }
        if let Statement::WidgetOperation {
            operation: WidgetOperation::Find { selector, all },
            route: Some(route),
            span,
        } = statement
        {
            let output = widget_selector_output(selector, document, span)?;
            infer_route(
                route,
                Some(if *all {
                    Type::List(Box::new(output))
                } else {
                    Type::Option(Box::new(output))
                }),
                &unknown_env,
                document,
                signatures,
            )?;
        }
        if let Statement::PaneOperation {
            operation: PaneOperation::Maximized | PaneOperation::Adjacent { .. },
            route: Some(route),
            ..
        } = statement
        {
            infer_route(
                route,
                Some(Type::Option(Box::new(Type::Str))),
                &unknown_env,
                document,
                signatures,
            )?;
        }
        if let Statement::WindowOperation {
            operation,
            route: Some(route),
            span,
            ..
        } = statement
        {
            match operation {
                WindowOperation::Open(_) => infer_route(
                    route,
                    Some(Type::WindowId),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                WindowOperation::Oldest | WindowOperation::Latest => infer_route(
                    route,
                    Some(Type::Option(Box::new(Type::WindowId))),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                WindowOperation::RawId => {
                    infer_route(route, Some(Type::Str), &unknown_env, document, signatures)?
                }
                WindowOperation::Screenshot => infer_route(
                    route,
                    Some(Type::WindowScreenshot),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                WindowOperation::Size => infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64],
                    &unknown_env,
                    document,
                    signatures,
                    "window size",
                )?,
                WindowOperation::Position | WindowOperation::MonitorSize => {
                    infer_ordered_payload_route(
                        route,
                        &[
                            Type::Option(Box::new(Type::F64)),
                            Type::Option(Box::new(Type::F64)),
                        ],
                        &unknown_env,
                        document,
                        signatures,
                        "optional window coordinates",
                    )?
                }
                WindowOperation::IsMaximized => {
                    infer_route(route, Some(Type::Bool), &unknown_env, document, signatures)?
                }
                WindowOperation::IsMinimized => infer_route(
                    route,
                    Some(Type::Option(Box::new(Type::Bool))),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                WindowOperation::ScaleFactor => {
                    infer_route(route, Some(Type::F64), &unknown_env, document, signatures)?
                }
                WindowOperation::Mode => {
                    infer_route(route, Some(Type::Str), &unknown_env, document, signatures)?
                }
                WindowOperation::Callback { function, .. } => {
                    let callback = extern_function(document, function, ExternKind::Window, span)?;
                    infer_route(
                        route,
                        Some(callback.output.clone()),
                        &unknown_env,
                        document,
                        signatures,
                    )?
                }
                _ => {}
            }
        }
        if let Statement::Run {
            kind,
            function,
            args,
            success,
            error,
            span,
            ..
        } = statement
        {
            if component_context(route_env).is_some()
                && let Some(route) = std::iter::once(success)
                    .chain(error.iter())
                    .find(|route| route.handler == "emit")
            {
                return Err(Error::new(
                    "E135",
                    &route.span,
                    "component outputs can only be emitted from the component view",
                ));
            }
            if *kind == EffectKind::Stream {
                require_single_payload_routes(
                    std::iter::once(success).chain(error.iter()),
                    span,
                    "stream routes accept at most one `_`; read other state in the handler",
                )?;
            }
            if let Some((output, builtin_error)) = builtin_task_type(*kind, function, args, span)? {
                infer_route(success, Some(output), &unknown_env, document, signatures)?;
                match (builtin_error, error) {
                    (Some(error_ty), Some(route)) => {
                        infer_route(route, Some(error_ty), &unknown_env, document, signatures)?
                    }
                    (Some(_), None) => {
                        return Err(Error::new(
                            "E131",
                            span,
                            "fallible built-in task requires an error route",
                        ));
                    }
                    (None, Some(_)) => {
                        return Err(Error::new(
                            "E131",
                            span,
                            "infallible built-in task cannot have an error route",
                        ));
                    }
                    (None, None) => {}
                }
                continue;
            }
            let action = extern_function(document, function, (*kind).into(), span)?;
            infer_route(
                success,
                Some(action.output.clone()),
                &unknown_env,
                document,
                signatures,
            )?;
            match (&action.error, error) {
                (Some(error_ty), Some(route)) => infer_route(
                    route,
                    Some(error_ty.clone()),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                (Some(_), None) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "fallible extern fn requires an error route",
                    ));
                }
                (None, Some(_)) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "infallible extern fn cannot have an error route",
                    ));
                }
                (None, None) => {}
            }
        }
        if let Statement::Sip {
            function,
            progress,
            success,
            error,
            span,
            ..
        } = statement
        {
            require_single_payload_routes(
                std::iter::once(progress)
                    .chain(std::iter::once(success))
                    .chain(error.iter()),
                span,
                "sip routes accept at most one `_`; read other state in the handler",
            )?;
            let action = extern_function(document, function, ExternKind::Sip, span)?;
            let progress_ty = action
                .progress
                .clone()
                .expect("sip extern has a progress type");
            infer_route(
                progress,
                Some(progress_ty),
                &unknown_env,
                document,
                signatures,
            )?;
            infer_route(
                success,
                Some(action.output.clone()),
                &unknown_env,
                document,
                signatures,
            )?;
            match (&action.error, error) {
                (Some(error_ty), Some(route)) => infer_route(
                    route,
                    Some(error_ty.clone()),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                (Some(_), None) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "fallible extern sip requires an error route",
                    ));
                }
                (None, Some(_)) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "infallible extern sip cannot have an error route",
                    ));
                }
                (None, None) => {}
            }
        }
        if let Statement::TaskFlow {
            source,
            transforms,
            success,
            error,
            units,
            span,
        } = statement
        {
            require_single_payload_routes(
                success.iter().chain(error.iter()).chain(units.iter()),
                span,
                "flow routes accept at most one `_`; read other state in the handler",
            )?;
            let (output, error_ty) = task_flow_type(source, transforms, document, &local_env)?;
            if let Some(route) = units {
                infer_route(route, Some(Type::I64), &unknown_env, document, signatures)?;
            }
            match (output, success) {
                (Some(output), Some(route)) => {
                    infer_route(route, Some(output), &unknown_env, document, signatures)?
                }
                (Some(_), None) => {
                    return Err(Error::new("E131", span, "flow requires a done route"));
                }
                (None, Some(_)) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "discarded flow cannot have a done route",
                    ));
                }
                (None, None) => {}
            }
            match (error_ty, error) {
                (Some(error_ty), Some(route)) => {
                    infer_route(route, Some(error_ty), &unknown_env, document, signatures)?
                }
                (Some(_), None) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "fallible flow requires an error route",
                    ));
                }
                (None, Some(_)) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "infallible or discarded flow cannot have an error route",
                    ));
                }
                (None, None) => {}
            }
        }
    }
    Ok(())
}

pub(in crate::check) fn infer_route(
    route: &Route,
    payload: Option<Type>,
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    infer_route_with_payloads(
        route,
        RoutePayloads::Single(payload.as_ref()),
        env,
        document,
        signatures,
    )
}

pub(in crate::check) fn infer_component_event_route(
    route: &Route,
    payloads: &[Type],
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    infer_route_with_payloads(
        route,
        RoutePayloads::Ordered(payloads),
        env,
        document,
        signatures,
    )
}

#[derive(Clone, Copy)]
enum RoutePayloads<'a> {
    Single(Option<&'a Type>),
    Ordered(&'a [Type]),
}

impl RoutePayloads<'_> {
    fn get(self, index: usize, span: &Span) -> Result<Type, Error> {
        match self {
            Self::Single(payload) => payload.cloned(),
            Self::Ordered(payloads) => payloads.get(index).cloned(),
        }
        .ok_or_else(|| Error::new("E134", span, "this route has no `_` payload"))
    }
}

fn infer_route_with_payloads(
    route: &Route,
    payloads: RoutePayloads<'_>,
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    let (captured_payloads, captured_ordered) = match payloads {
        RoutePayloads::Single(payload) => (payload.cloned().into_iter().collect(), false),
        RoutePayloads::Ordered(payloads) => (payloads.to_vec(), true),
    };
    super::expr::capture_handler_route_inputs(route, captured_payloads, captured_ordered);
    if route.handler == "emit"
        && let Some(output) = component_output(env)
    {
        let component_name = component_context(env).expect("component output has a context");
        let component = document
            .components
            .iter()
            .find(|component| component.name == component_name)
            .expect("component context names a declared component");
        let named = route.args.split_first().and_then(|(name, args)| {
            let RouteArg::Expr(Expr::Path(path)) = name else {
                return None;
            };
            let [name] = path.as_slice() else {
                return None;
            };
            component
                .events
                .iter()
                .find(|event| event.name == *name)
                .map(|event| (event, args))
        });
        if let Some((event, args)) = named {
            if args.len() != event.payloads.len() {
                return Err(Error::new(
                    "E133",
                    &route.span,
                    format!(
                        "component event `{}` expects {} values, got {}",
                        event.name,
                        event.payloads.len(),
                        args.len()
                    ),
                ));
            }
            let mut payload_index = 0;
            for (arg, expected) in args.iter().zip(&event.payloads) {
                let actual = match arg {
                    RouteArg::Payload => {
                        let actual = payloads.get(payload_index, &route.span)?;
                        payload_index += 1;
                        actual
                    }
                    RouteArg::Expr(expr) => expr_type(expr, env, document, &route.span)?,
                };
                require_type(&actual, expected, &route.span)?;
            }
            return Ok(());
        }
        if *output == Type::Unit {
            let candidate = route.args.first().and_then(|arg| match arg {
                RouteArg::Expr(Expr::Path(path)) if path.len() == 1 => Some(path[0].as_str()),
                _ => None,
            });
            return Err(Error::new(
                "E135",
                &route.span,
                candidate.map_or_else(
                    || format!("component `{component_name}` does not declare an output"),
                    |name| format!("component `{component_name}` does not declare event `{name}`"),
                ),
            ));
        }
        let [arg] = route.args.as_slice() else {
            return Err(Error::new(
                "E133",
                &route.span,
                "`emit` expects exactly one value",
            ));
        };
        let actual = match arg {
            RouteArg::Payload => payloads.get(0, &route.span)?,
            RouteArg::Expr(expr) => expr_type(expr, env, document, &route.span)?,
        };
        return require_type(&actual, output, &route.span);
    }
    if route.handler == "emit" && component_context(env).is_some() {
        return Err(Error::new(
            "E135",
            &route.span,
            "component outputs can only be emitted from the component view",
        ));
    }
    if route.handler == "mount" {
        return Err(Error::new(
            "E135",
            &route.span,
            "`mount` is initialization-only and cannot receive events",
        ));
    }
    let local_key = component_context(env)
        .map(|component| component_handler_key(component, &route.handler))
        .filter(|key| signatures.contains_key(key));
    if let Some(component) = component_context(env)
        && local_key.is_none()
    {
        return Err(Error::new(
            "E132",
            &route.span,
            format!(
                "component `{component}` cannot reference app handler `{}`",
                route.handler
            ),
        )
        .hint("declare a component event and route it at the call site"));
    }
    let key = local_key.unwrap_or_else(|| route.handler.clone());
    let signature = signatures.get_mut(&key).ok_or_else(|| {
        Error::new(
            "E132",
            &route.span,
            format!("unknown handler `{}`", route.handler),
        )
    })?;
    if signature.len() != route.args.len() {
        return Err(Error::new(
            "E133",
            &route.span,
            format!(
                "handler `{}` expects {} arguments, got {}",
                route.handler,
                signature.len(),
                route.args.len()
            ),
        ));
    }
    let mut payload_index = 0;
    for (slot, arg) in signature.iter_mut().zip(&route.args) {
        let ty = match arg {
            RouteArg::Payload => {
                let ty = payloads.get(payload_index, &route.span)?;
                payload_index += 1;
                ty
            }
            RouteArg::Expr(expr) => expr_type(expr, env, document, &route.span)?,
        };
        if contains_debug_span(&ty) {
            return Err(Error::new(
                "E135",
                &route.span,
                "debug spans cannot cross a handler route; use `debug.active(state)` for status",
            ));
        }
        if ty == Type::Unknown {
            continue;
        }
        if let Some(existing) = slot {
            if !compatible(existing, &ty) {
                return Err(type_error(&route.span, existing, &ty));
            }
        } else {
            *slot = Some(ty);
        }
    }
    Ok(())
}

pub(in crate::check) fn infer_ordered_payload_route(
    route: &Route,
    payloads: &[Type],
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    label: &str,
) -> Result<(), Error> {
    if route.handler == "emit"
        && component_output(env).is_some()
        && route
            .args
            .first()
            .is_some_and(|arg| matches!(arg, RouteArg::Expr(Expr::Path(path)) if path.len() == 1))
    {
        return infer_component_event_route(route, payloads, env, document, signatures);
    }
    if route.args.len() != payloads.len()
        || route
            .args
            .iter()
            .any(|arg| !matches!(arg, RouteArg::Payload))
    {
        return Err(Error::new(
            "E129",
            &route.span,
            format!("{label} route expects {} payloads", payloads.len()),
        ));
    }
    if route.handler == "emit" && component_output(env).is_some() {
        return infer_route(route, payloads.first().cloned(), env, document, signatures);
    }
    infer_route(route, Some(Type::Unknown), env, document, signatures)?;
    super::expr::capture_handler_route_inputs(route, payloads.to_vec(), true);
    let key = component_context(env)
        .map(|component| component_handler_key(component, &route.handler))
        .filter(|key| signatures.contains_key(key))
        .unwrap_or_else(|| route.handler.clone());
    let signature = signatures.get_mut(&key).expect("route signature");
    for (slot, ty) in signature.iter_mut().zip(payloads) {
        if let Some(existing) = slot {
            if !compatible(existing, ty) {
                return Err(type_error(&route.span, existing, ty));
            }
        } else {
            *slot = Some(ty.clone());
        }
    }
    Ok(())
}

use super::*;
use crate::lower::{
    CheckedExprUseId, CheckedSubscription, CheckedSubscriptionRoute, CheckedSubscriptionSource,
    ExternRef,
};

fn checked_subscription_extern<'a>(
    program: &'a LoweredProgram,
    reference: &ExternRef,
    kind: ExternKind,
    span: &Span,
) -> Result<&'a crate::hir::ExternDeclaration, Error> {
    let declaration = program
        .declarations()
        .checked_extern_decl(reference.id, span)?;
    if declaration.name != reference.name || declaration.kind != kind {
        return Err(Error::new(
            "E196",
            span,
            "checked subscription extern reference has a mismatched declaration contract",
        ));
    }
    Ok(declaration)
}

fn checked_subscription_expression_type(
    program: &LoweredProgram,
    id: CheckedExprUseId,
    span: &Span,
) -> Result<Type, Error> {
    let facts = program.checked_facts();
    facts.validate_expression_use(id, program.declarations(), span)?;
    Ok(facts.checked_expression_use(id, span)?.destination.clone())
}

fn validate_subscription_arguments(
    program: &LoweredProgram,
    arguments: &[CheckedExprUseId],
    function: &crate::hir::ExternDeclaration,
    span: &Span,
) -> Result<(), Error> {
    if arguments.len() != function.params.len() {
        return Err(Error::new(
            "E196",
            span,
            "checked subscription extern arguments have a mismatched arity",
        ));
    }
    for (argument, (_, expected)) in arguments.iter().zip(&function.params) {
        if checked_subscription_expression_type(program, *argument, span)? != *expected {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription extern argument has a mismatched parameter type",
            ));
        }
    }
    Ok(())
}

fn extern_payload(function: &crate::hir::ExternDeclaration) -> Type {
    function.error.as_ref().map_or_else(
        || function.output.clone(),
        |error| Type::Result(Box::new(function.output.clone()), Box::new(error.clone())),
    )
}

fn validate_subscription_contract(
    program: &LoweredProgram,
    subscription: &CheckedSubscription,
) -> Result<(), Error> {
    let span = &subscription.span;
    match &subscription.source {
        CheckedSubscriptionSource::Repeat { function, .. } => {
            let function =
                checked_subscription_extern(program, function, ExternKind::Future, span)?;
            validate_subscription_arguments(program, &[], function, span)?;
            if subscription.source_payloads != [extern_payload(function)] {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked repeat subscription has a mismatched output contract",
                ));
            }
        }
        CheckedSubscriptionSource::Run {
            function,
            arguments,
        } => {
            let function =
                checked_subscription_extern(program, function, ExternKind::Stream, span)?;
            validate_subscription_arguments(program, arguments, function, span)?;
            if subscription.source_payloads != [extern_payload(function)] {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked run subscription has a mismatched output contract",
                ));
            }
        }
        CheckedSubscriptionSource::Recipe {
            function,
            arguments,
        } => {
            let function =
                checked_subscription_extern(program, function, ExternKind::Recipe, span)?;
            validate_subscription_arguments(program, arguments, function, span)?;
            if subscription.source_payloads != [function.output.clone()] {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked recipe subscription has a mismatched output contract",
                ));
            }
        }
        CheckedSubscriptionSource::Events { identity, filter } => {
            checked_subscription_expression_type(program, *identity, span)?;
            let function =
                checked_subscription_extern(program, filter, ExternKind::EventFilter, span)?;
            validate_subscription_arguments(program, &[], function, span)?;
            if subscription.source_payloads != [function.output.clone()] {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked event-filter subscription has a mismatched output contract",
                ));
            }
        }
        CheckedSubscriptionSource::Extern {
            function,
            arguments,
        } => {
            let function =
                checked_subscription_extern(program, function, ExternKind::Subscription, span)?;
            validate_subscription_arguments(program, arguments, function, span)?;
            if subscription.source_payloads != [function.output.clone()] {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked custom subscription has a mismatched output contract",
                ));
            }
        }
        _ => {}
    }

    if let Some(condition) = subscription.condition
        && checked_subscription_expression_type(program, condition, span)? != Type::Bool
    {
        return Err(Error::new(
            "E196",
            span,
            "checked subscription condition has a non-boolean type",
        ));
    }

    let mut delivered = if let Some(reference) = &subscription.filter {
        let function = checked_subscription_extern(program, reference, ExternKind::Sync, span)?;
        if function
            .params
            .iter()
            .map(|(_, ty)| ty)
            .ne(&subscription.source_payloads)
        {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription filter has mismatched parameter types",
            ));
        }
        let Type::Option(output) = &function.output else {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription filter has a non-optional output",
            ));
        };
        vec![(**output).clone()]
    } else {
        subscription.source_payloads.clone()
    };
    if let Some(context) = subscription.context {
        delivered.insert(
            0,
            checked_subscription_expression_type(program, context, span)?,
        );
    }
    if delivered != subscription.delivered_payloads {
        return Err(Error::new(
            "E196",
            span,
            "checked subscription transforms have a mismatched delivered payload contract",
        ));
    }

    let handler = program
        .declarations()
        .checked_handler(subscription.route.handler, span)?;
    if handler.name != subscription.route.handler_name {
        return Err(Error::new(
            "E196",
            span,
            "checked subscription route has a mismatched handler identity",
        ));
    }
    let route_payloads = subscription
        .route
        .payloads
        .iter()
        .map(|index| {
            subscription
                .delivered_payloads
                .get(*index as usize)
                .cloned()
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        span,
                        "checked subscription route references an invalid payload index",
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if route_payloads != handler.payloads {
        return Err(Error::new(
            "E196",
            span,
            "checked subscription route has a mismatched handler payload contract",
        ));
    }
    Ok(())
}

pub(in crate::codegen) fn identified_window_filter(filter: &str, arity: usize) -> String {
    match arity {
        0 => format!("({filter}).map(|_| __id)"),
        1 => format!("({filter}).map(|__value| (__id, __value))"),
        count => format!(
            "({filter}).map(|__value| (__id, {}))",
            (0..count)
                .map(|index| format!("__value.{index}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub(in crate::codegen) fn generate_subscription(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
) -> Result<(), Error> {
    let document = program.document();
    let settings = program.settings();
    let animations = has_animations(program);
    let env = checked_state_env(program, "self");
    writeln!(
        out,
        "fn __subscription(&self) -> ::iced::Subscription<{message}> {{"
    )
    .unwrap();
    writeln!(out, "::iced::Subscription::batch([").unwrap();
    if settings.kind == ProgramKind::Application {
        writeln!(
            out,
            "self.__ice_accessibility.subscription().map({message}::__AccessibilityAction),"
        )
        .unwrap();
        writeln!(
            out,
            "::iced::window::events().map(|(__id, __event)| {message}::__AccessibilityWindow(__id, __event)),"
        )
        .unwrap();
    }
    for subscription in program.subscriptions() {
        validate_subscription_contract(program, subscription)?;
        writeln!(out, "{}", source_marker(&subscription.span)).unwrap();
        let source_arity = subscription.source_payloads.len();
        let filter = subscription
            .filter
            .as_ref()
            .map(|filter| {
                let function = checked_subscription_extern(
                    program,
                    filter,
                    ExternKind::Sync,
                    &subscription.span,
                )?;
                let args = match source_arity {
                    0 => String::new(),
                    1 => "__value".into(),
                    count => (0..count)
                        .map(|index| format!("__value.{index}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                };
                Ok(format!(
                    ".filter_map(|{}| {}({args}))",
                    if source_arity == 0 { "_" } else { "__value" },
                    function.rust_path
                ))
            })
            .transpose()?
            .unwrap_or_default();
        let context = subscription
            .context
            .map(|context| {
                checked_expr_use_code_at(
                    program,
                    context,
                    &env,
                    ValueMode::Owned,
                    &subscription.span,
                )
            })
            .transpose()?
            .map(|context| format!(".with({context})"))
            .unwrap_or_default();
        let output_arity = if subscription.filter.is_some() {
            1
        } else {
            source_arity
        };
        let mut payloads = Vec::new();
        if subscription.context.is_some() {
            payloads.push("__value.0".to_owned());
        }
        match output_arity {
            0 => {}
            1 => payloads.push(if subscription.context.is_some() {
                "__value.1".into()
            } else {
                "__value".into()
            }),
            count => payloads.extend((0..count).map(|index| {
                if subscription.context.is_some() {
                    format!("__value.1.{index}")
                } else {
                    format!("__value.{index}")
                }
            })),
        }
        if payloads.len() != subscription.delivered_payloads.len() {
            return Err(Error::new(
                "E196",
                &subscription.span,
                "subscription transform payload shape disagrees with checked HIR",
            ));
        }
        let route = checked_subscription_route_code(program, subscription, &payloads, message)?;
        let transforms = format!("{filter}{context}");
        let condition = subscription
            .condition
            .map(|condition| {
                checked_expr_use_code_at(
                    program,
                    condition,
                    &env,
                    ValueMode::Owned,
                    &subscription.span,
                )
            })
            .transpose()?;
        if let Some(condition) = &condition {
            write!(out, "if {condition} {{ ::iced::Subscription::batch([").unwrap();
        }
        match &subscription.source {
            CheckedSubscriptionSource::Every { milliseconds } => {
                writeln!(out, "::iced::time::every(::std::time::Duration::from_millis({milliseconds})){transforms}.map(move |__value| {route}),").unwrap();
            }
            CheckedSubscriptionSource::Repeat {
                function,
                milliseconds,
            } => {
                let source = checked_subscription_extern(
                    program,
                    function,
                    ExternKind::Future,
                    &subscription.span,
                )?;
                writeln!(out, "::iced::time::repeat({}, ::std::time::Duration::from_millis({milliseconds})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
            }
            CheckedSubscriptionSource::Run {
                function,
                arguments,
            } => {
                let source = checked_subscription_extern(
                    program,
                    function,
                    ExternKind::Stream,
                    &subscription.span,
                )?;
                if arguments.is_empty() {
                    writeln!(
                        out,
                        "::iced::Subscription::run({}){transforms}.map(move |__value| {route}),",
                        source.rust_path
                    )
                    .unwrap();
                } else {
                    let data = arguments
                        .iter()
                        .map(|argument| {
                            checked_expr_use_code_at(
                                program,
                                *argument,
                                &env,
                                ValueMode::Owned,
                                &subscription.span,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let types = source
                        .params
                        .iter()
                        .map(|(_, ty)| program.declarations().rust_type(ty, &subscription.span))
                        .collect::<Result<Vec<_>, _>>()?;
                    let (data, data_type, builder_args) = if arguments.len() == 1 {
                        (data[0].clone(), types[0].clone(), "__data.clone()".into())
                    } else {
                        (
                            format!("({},)", data.join(", ")),
                            format!("({},)", types.join(", ")),
                            (0..arguments.len())
                                .map(|index| format!("__data.{index}.clone()"))
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    };
                    writeln!(out, "::iced::Subscription::run_with({data}, |__data: &{data_type}| {}({builder_args})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
                }
            }
            CheckedSubscriptionSource::Recipe {
                function,
                arguments,
            } => {
                let source = checked_subscription_extern(
                    program,
                    function,
                    ExternKind::Recipe,
                    &subscription.span,
                )?;
                let args =
                    checked_subscription_arguments(program, arguments, &env, &subscription.span)?;
                writeln!(out, "::iced::advanced::subscription::from_recipe({}({args})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
            }
            CheckedSubscriptionSource::Events { identity, filter } => {
                let source = checked_subscription_extern(
                    program,
                    filter,
                    ExternKind::EventFilter,
                    &subscription.span,
                )?;
                let id = checked_expr_use_code_at(
                    program,
                    *identity,
                    &env,
                    ValueMode::Owned,
                    &subscription.span,
                )?;
                let recipe = event_filter_type(&source.name);
                writeln!(out, "::iced::advanced::subscription::from_recipe({recipe} {{ id: {id} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            CheckedSubscriptionSource::Event { raw } => {
                let value = if subscription.window_id {
                    "::std::option::Option::Some((__id, __event))"
                } else {
                    "::std::option::Option::Some(__event)"
                };
                let (filter, status) = event_status_filter(value, subscription.status);
                let listen = if *raw { "listen_raw" } else { "listen_with" };
                writeln!(out, "::iced::event::{listen}(|__event, {status}, __id| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            CheckedSubscriptionSource::Extern {
                function,
                arguments,
            } => {
                let source = checked_subscription_extern(
                    program,
                    function,
                    ExternKind::Subscription,
                    &subscription.span,
                )?;
                let args =
                    checked_subscription_arguments(program, arguments, &env, &subscription.span)?;
                writeln!(
                    out,
                    "{}({args}){transforms}.map(move |__value| {route}),",
                    source.rust_path
                )
                .unwrap();
            }
            CheckedSubscriptionSource::InputMethod(event) => {
                let filter = match event {
                    InputMethodEvent::Opened => {
                        "matches!(__event, ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Opened)).then_some(())"
                    }
                    InputMethodEvent::Preedit => {
                        "match __event { ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Preedit(content, range)) => { let (start, end) = range.map_or((::std::option::Option::None, ::std::option::Option::None), |range| (::std::option::Option::Some(i64::try_from(range.start).unwrap_or(i64::MAX)), ::std::option::Option::Some(i64::try_from(range.end).unwrap_or(i64::MAX)))); ::std::option::Option::Some((content, start, end)) }, _ => ::std::option::Option::None }"
                    }
                    InputMethodEvent::Commit => {
                        "match __event { ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Commit(content)) => ::std::option::Option::Some(content), _ => ::std::option::Option::None }"
                    }
                    InputMethodEvent::Closed => {
                        "matches!(__event, ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Closed)).then_some(())"
                    }
                };
                let (filter, status) = event_status_filter(filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            CheckedSubscriptionSource::Keyboard(event) => {
                let filter = match event {
                    KeyboardEvent::Press => {
                        "match __event { ::iced::keyboard::Event::KeyPressed { key, modified_key, physical_key, location, modifiers, text, repeat } => ::std::option::Option::Some(__IceKeyPress { key, modified_key, physical_key, location, modifiers, text: text.map(|value| value.to_string()), repeat }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Release => {
                        "match __event { ::iced::keyboard::Event::KeyReleased { key, modified_key, physical_key, location, modifiers } => ::std::option::Option::Some(__IceKeyRelease { key, modified_key, physical_key, location, modifiers }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Modifiers => {
                        "match __event { ::iced::keyboard::Event::ModifiersChanged(modifiers) => ::std::option::Option::Some(modifiers), _ => ::std::option::Option::None }"
                    }
                };
                let filter = format!(
                    "match __event {{ ::iced::Event::Keyboard(__event) => {{ {filter} }}, _ => ::std::option::Option::None }}"
                );
                let (filter, status) = event_status_filter(&filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            CheckedSubscriptionSource::Mouse(event) => {
                let filter = match event {
                    MouseEvent::Entered => {
                        "matches!(__event, ::iced::Event::Mouse(::iced::mouse::Event::CursorEntered)).then_some(())"
                    }
                    MouseEvent::Left => {
                        "matches!(__event, ::iced::Event::Mouse(::iced::mouse::Event::CursorLeft)).then_some(())"
                    }
                    MouseEvent::Moved => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::CursorMoved { position }) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Pressed => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::ButtonPressed(button)) => ::std::option::Option::Some(button), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Released => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::ButtonReleased(button)) => ::std::option::Option::Some(button), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Wheel => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::WheelScrolled { delta }) => { let (x, y, pixels) = match delta { ::iced::mouse::ScrollDelta::Lines { x, y } => (x as f64, y as f64, false), ::iced::mouse::ScrollDelta::Pixels { x, y } => (x as f64, y as f64, true) }; ::std::option::Option::Some((x, y, pixels)) }, _ => ::std::option::Option::None }"
                    }
                };
                let (filter, status) = event_status_filter(filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            CheckedSubscriptionSource::SystemTheme => {
                writeln!(out, "::iced::system::theme_changes().map(__ice_system_theme){transforms}.map(move |__value| {route}),").unwrap();
            }
            CheckedSubscriptionSource::Touch(event) => {
                let variant = match event {
                    TouchEvent::Pressed => "FingerPressed",
                    TouchEvent::Moved => "FingerMoved",
                    TouchEvent::Lifted => "FingerLifted",
                    TouchEvent::Lost => "FingerLost",
                };
                let filter = format!(
                    "match __event {{ ::iced::Event::Touch(::iced::touch::Event::{variant} {{ id, position }}) => ::std::option::Option::Some((id, position.x as f64, position.y as f64)), _ => ::std::option::Option::None }}"
                );
                let (filter, status) = event_status_filter(&filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            CheckedSubscriptionSource::Window(event) => {
                if *event == WindowEvent::Frame {
                    writeln!(
                        out,
                        "::iced::window::frames(){transforms}.map(move |__value| {route}),"
                    )
                    .unwrap();
                    if condition.is_some() {
                        writeln!(out, "]) }} else {{ ::iced::Subscription::none() }},").unwrap();
                    }
                    writeln!(out, "{SOURCE_MARKER_END}").unwrap();
                    continue;
                }
                let filter = match event {
                    WindowEvent::Opened => {
                        "match __event { ::iced::window::Event::Opened { position, size } => { let (x, y) = position.map_or((::std::option::Option::None, ::std::option::Option::None), |position| (::std::option::Option::Some(position.x as f64), ::std::option::Option::Some(position.y as f64))); ::std::option::Option::Some((x, y, size.width as f64, size.height as f64)) }, _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Closed => {
                        "matches!(__event, ::iced::window::Event::Closed).then_some(())"
                    }
                    WindowEvent::Moved => {
                        "match __event { ::iced::window::Event::Moved(position) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Resized => {
                        "match __event { ::iced::window::Event::Resized(size) => ::std::option::Option::Some((size.width as f64, size.height as f64)), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Rescaled => {
                        "match __event { ::iced::window::Event::Rescaled(scale) => ::std::option::Option::Some(scale as f64), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::CloseRequested => {
                        "matches!(__event, ::iced::window::Event::CloseRequested).then_some(())"
                    }
                    WindowEvent::Focused => {
                        "matches!(__event, ::iced::window::Event::Focused).then_some(())"
                    }
                    WindowEvent::Unfocused => {
                        "matches!(__event, ::iced::window::Event::Unfocused).then_some(())"
                    }
                    WindowEvent::FileHovered => {
                        "match __event { ::iced::window::Event::FileHovered(path) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::FileDropped => {
                        "match __event { ::iced::window::Event::FileDropped(path) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::FilesHoveredLeft => {
                        "matches!(__event, ::iced::window::Event::FilesHoveredLeft).then_some(())"
                    }
                    WindowEvent::Frame => unreachable!("handled above"),
                };
                let filter = if subscription.window_id {
                    identified_window_filter(
                        filter,
                        source_arity.checked_sub(1).ok_or_else(|| {
                            Error::new(
                                "E196",
                                &subscription.span,
                                "window-id subscription retained no window payload",
                            )
                        })?,
                    )
                } else {
                    filter.to_owned()
                };
                let filter = format!(
                    "match __event {{ ::iced::Event::Window(__event) => {{ {filter} }}, _ => ::std::option::Option::None }}"
                );
                let (filter, status) = event_status_filter(&filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, __id| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
        }
        if condition.is_some() {
            writeln!(out, "]) }} else {{ ::iced::Subscription::none() }},").unwrap();
        }
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
    if animations {
        let active = document
            .states
            .iter()
            .filter(|state| matches!(state.ty, Type::Animation(_)))
            .map(|state| {
                format!(
                    "self.{}.is_animating(::iced::time::Instant::now())",
                    state.name
                )
            })
            .collect::<Vec<_>>()
            .join(" || ");
        writeln!(
            out,
            "if {active} {{ ::iced::window::frames().map(|_| {message}::__AnimationFrame) }} else {{ ::iced::Subscription::none() }},"
        )
        .unwrap();
    }
    writeln!(out, "])\n}}").unwrap();
    Ok(())
}

fn checked_subscription_arguments(
    program: &LoweredProgram,
    arguments: &[CheckedExprUseId],
    env: &dyn BindingEnvironment,
    span: &Span,
) -> Result<String, Error> {
    arguments
        .iter()
        .map(|argument| checked_expr_use_code_at(program, *argument, env, ValueMode::Owned, span))
        .collect::<Result<Vec<_>, _>>()
        .map(|arguments| arguments.join(", "))
}

fn checked_subscription_route_code(
    program: &LoweredProgram,
    subscription: &CheckedSubscription,
    payloads: &[String],
    message: &str,
) -> Result<String, Error> {
    let CheckedSubscriptionRoute {
        handler,
        payloads: route_payloads,
        ..
    } = &subscription.route;
    let handler = program
        .declarations()
        .checked_handler(*handler, &subscription.span)?;
    let variant = handler_variant(&handler.name);
    if route_payloads.is_empty() {
        return Ok(format!("{message}::{variant}"));
    }
    let arguments = route_payloads
        .iter()
        .map(|index| {
            payloads.get(*index as usize).cloned().ok_or_else(|| {
                Error::new(
                    "E196",
                    &subscription.span,
                    "subscription route references an invalid checked payload",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("{message}::{variant}({})", arguments.join(", ")))
}

pub(in crate::codegen) fn event_status_filter(
    filter: &str,
    status: Option<EventStatus>,
) -> (String, &'static str) {
    match status {
        None | Some(EventStatus::Any) => (filter.to_owned(), "_"),
        Some(EventStatus::Captured) => (
            format!(
                "if matches!(__status, ::iced::event::Status::Captured) {{ {filter} }} else {{ ::std::option::Option::None }}"
            ),
            "__status",
        ),
        Some(EventStatus::Ignored) => (
            format!(
                "if matches!(__status, ::iced::event::Status::Ignored) {{ {filter} }} else {{ ::std::option::Option::None }}"
            ),
            "__status",
        ),
    }
}

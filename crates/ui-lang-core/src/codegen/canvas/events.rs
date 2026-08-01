use super::*;

pub(in crate::codegen) fn canvas_update_code(
    options: &ResolvedCanvasOptions,
    events: &[ResolvedCanvasEvent],
    env: &dyn BindingEnvironment,
    canvas_env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
    use_cache: bool,
) -> Result<String, Error> {
    let capture = options
        .capture
        .as_ref()
        .map(|value| checked_expr_use_code(program, *value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let action = |message: String, capture: &str| {
        format!(
            "::std::option::Option::Some(if {capture} {{ ::iced::widget::canvas::Action::publish({message}).and_capture() }} else {{ ::iced::widget::canvas::Action::publish({message}) }})"
        )
    };
    let mut code = format!(
        "move |__state: &mut __IceCanvasState, __event: &::iced::widget::canvas::Event, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ let __capture = {capture};"
    );
    if options.enter.is_some() || options.exit.is_some() {
        code.push_str(" let __inside = __cursor.is_over(__bounds); if __inside != __state.inside { __state.inside = __inside;");
        if let Some(route) = &options.enter {
            let route = resolved_canvas_route_code(route, &[], env, program, message)?;
            write!(
                code,
                " if __inside {{ return {}; }}",
                action(route, "__capture")
            )
            .unwrap();
        }
        if let Some(route) = &options.exit {
            let route = resolved_canvas_route_code(route, &[], env, program, message)?;
            write!(
                code,
                " if !__inside {{ return {}; }}",
                action(route, "__capture")
            )
            .unwrap();
        }
        code.push_str(" }");
    }
    let has_pointer_routes = options.press.is_some()
        || options.release.is_some()
        || options.right_press.is_some()
        || options.right_release.is_some()
        || options.middle_press.is_some()
        || options.middle_release.is_some()
        || options.move_route.is_some()
        || options.scroll.is_some();
    if has_pointer_routes {
        code.push_str(
            " if let ::std::option::Option::Some(__point) = __cursor.position_in(__bounds) { match __event {",
        );
        for (route, event) in [
            (
                &options.press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Left))",
            ),
            (
                &options.release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Left))",
            ),
            (
                &options.right_press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Right))",
            ),
            (
                &options.right_release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Right))",
            ),
            (
                &options.middle_press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Middle))",
            ),
            (
                &options.middle_release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Middle))",
            ),
        ] {
            if let Some(route) = route {
                let route = resolved_canvas_route_code(
                    route,
                    &["__point.x as f64", "__point.y as f64"],
                    env,
                    program,
                    message,
                )?;
                write!(code, " {event} => return {},", action(route, "__capture")).unwrap();
            }
        }
        if let Some(route) = &options.move_route {
            let route = resolved_canvas_route_code(
                route,
                &["__point.x as f64", "__point.y as f64"],
                env,
                program,
                message,
            )?;
            write!(
                code,
                " ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorMoved {{ .. }}) => return {},",
                action(route, "__capture")
            )
            .unwrap();
        }
        if let Some(route) = &options.scroll {
            let lines = resolved_canvas_route_code(
                route,
                &["__x as f64", "__y as f64", "false"],
                env,
                program,
                message,
            )?;
            let pixels = resolved_canvas_route_code(
                route,
                &["__x as f64", "__y as f64", "true"],
                env,
                program,
                message,
            )?;
            write!(
                code,
                " ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::WheelScrolled {{ delta }}) => return match delta {{ ::iced::mouse::ScrollDelta::Lines {{ x: __x, y: __y }} => {}, ::iced::mouse::ScrollDelta::Pixels {{ x: __x, y: __y }} => {} }},",
                action(lines, "__capture"),
                action(pixels, "__capture")
            )
            .unwrap();
        }
        code.push_str(" _ => {} } }");
    }
    for event in events {
        let filter = canvas_event_filter(&event.source);
        let pointer_guard = matches!(
            event.source,
            SubscriptionSource::Mouse(
                MouseEvent::Pressed | MouseEvent::Released | MouseEvent::Moved | MouseEvent::Wheel
            )
        )
        .then_some("__cursor.is_over(__bounds) && ")
        .unwrap_or_default();
        let mut event_env = ScopedBindingEnv::new(canvas_env);
        for binding in &event.bindings {
            event_env.insert(
                binding.name.clone(),
                Binding {
                    code: binding.name.clone(),
                    ty: binding.ty.clone(),
                    local: false,
                    state: None,
                    owner: Some(BindingOwner::Local(binding.local)),
                },
            );
        }
        let binding_names = event
            .bindings
            .iter()
            .map(|binding| binding.name.as_str())
            .collect::<Vec<_>>();
        let bindings = match binding_names.as_slice() {
            [] => String::new(),
            [binding] => format!("let {binding} = __value;"),
            bindings => format!("let ({}) = __value;", bindings.join(", ")),
        };
        let consumed_bindings = event
            .bindings
            .iter()
            .map(|binding| format!("let _ = &{};", binding.name))
            .collect::<String>();
        let mut updates = event
            .updates
            .iter()
            .enumerate()
            .map(|(index, update)| {
                Ok(format!(
                    "let __next_canvas_state_{index} = {}; __state.{} = __next_canvas_state_{index};",
                    checked_expr_use_code(program, update.value, &event_env, ValueMode::Owned)?,
                    update.name,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?
            .join(" ");
        if use_cache && !event.updates.is_empty() {
            updates.push_str(
                " if let ::std::option::Option::Some(__cache) = __state.cache.get() { __cache.clear(); }",
            );
        }
        let event_capture = if event.capture { "true" } else { "__capture" };
        let result = match &event.action {
            Some(ResolvedCanvasEventAction::Route(route)) => {
                let route = if event.route_payload {
                    canvas_event_route_code(&event.source, route, env, program, message)?
                } else {
                    resolved_canvas_route_code(route, &[], &event_env, program, message)?
                };
                action(route, event_capture)
            }
            Some(ResolvedCanvasEventAction::Redraw { after_ms }) => {
                let redraw = after_ms.map_or_else(
                    || "::iced::widget::canvas::Action::request_redraw()".into(),
                    |milliseconds| {
                        format!(
                            "{{ let __now = ::iced::time::Instant::now(); match __now.checked_add(::iced::time::Duration::from_millis({milliseconds})) {{ ::std::option::Option::Some(__at) => ::iced::widget::canvas::Action::request_redraw_at(__at), ::std::option::Option::None => ::iced::widget::canvas::Action::request_redraw(), }} }}"
                        )
                    },
                );
                format!(
                    "::std::option::Option::Some(if {event_capture} {{ {redraw}.and_capture() }} else {{ {redraw} }})"
                )
            }
            None => format!(
                "if {event_capture} {{ ::std::option::Option::Some(::iced::widget::canvas::Action::capture()) }} else {{ ::std::option::Option::None }}"
            ),
        };
        write!(
            code,
            " if {pointer_guard}let ::std::option::Option::Some(__value) = {filter} {{ let _ = &__value; {bindings} {consumed_bindings} {updates} return {result}; }}"
        )
        .unwrap();
    }
    code.push_str(" ::std::option::Option::None }");
    Ok(code)
}

pub(in crate::codegen) fn canvas_event_filter(source: &SubscriptionSource) -> String {
    match source {
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened => "matches!(__event, ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Opened)).then_some(())".into(),
            InputMethodEvent::Preedit => "match __event { ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Preedit(content, range)) => { let (start, end) = range.as_ref().map_or((::std::option::Option::None, ::std::option::Option::None), |range| (::std::option::Option::Some(i64::try_from(range.start).unwrap_or(i64::MAX)), ::std::option::Option::Some(i64::try_from(range.end).unwrap_or(i64::MAX)))); ::std::option::Option::Some((content.clone(), start, end)) }, _ => ::std::option::Option::None }".into(),
            InputMethodEvent::Commit => "match __event { ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Commit(content)) => ::std::option::Option::Some(content.clone()), _ => ::std::option::Option::None }".into(),
            InputMethodEvent::Closed => "matches!(__event, ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Closed)).then_some(())".into(),
        },
        SubscriptionSource::Keyboard(event) => match event {
            KeyboardEvent::Press => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::KeyPressed { key, modified_key, physical_key, location, modifiers, text, repeat }) => ::std::option::Option::Some(__IceKeyPress { key: key.clone(), modified_key: modified_key.clone(), physical_key: *physical_key, location: *location, modifiers: *modifiers, text: text.as_ref().map(::std::string::ToString::to_string), repeat: *repeat }), _ => ::std::option::Option::None }".into(),
            KeyboardEvent::Release => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::KeyReleased { key, modified_key, physical_key, location, modifiers }) => ::std::option::Option::Some(__IceKeyRelease { key: key.clone(), modified_key: modified_key.clone(), physical_key: *physical_key, location: *location, modifiers: *modifiers }), _ => ::std::option::Option::None }".into(),
            KeyboardEvent::Modifiers => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::ModifiersChanged(modifiers)) => ::std::option::Option::Some(*modifiers), _ => ::std::option::Option::None }".into(),
        },
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered => "matches!(__event, ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorEntered)).then_some(())".into(),
            MouseEvent::Left => "matches!(__event, ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorLeft)).then_some(())".into(),
            MouseEvent::Moved => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorMoved { position }) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }".into(),
            MouseEvent::Pressed => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(button)) => ::std::option::Option::Some(*button), _ => ::std::option::Option::None }".into(),
            MouseEvent::Released => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(button)) => ::std::option::Option::Some(*button), _ => ::std::option::Option::None }".into(),
            MouseEvent::Wheel => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::WheelScrolled { delta }) => { let (x, y, pixels) = match delta { ::iced::mouse::ScrollDelta::Lines { x, y } => (*x as f64, *y as f64, false), ::iced::mouse::ScrollDelta::Pixels { x, y } => (*x as f64, *y as f64, true) }; ::std::option::Option::Some((x, y, pixels)) }, _ => ::std::option::Option::None }".into(),
        },
        SubscriptionSource::Touch(event) => {
            let variant = match event {
                TouchEvent::Pressed => "FingerPressed",
                TouchEvent::Moved => "FingerMoved",
                TouchEvent::Lifted => "FingerLifted",
                TouchEvent::Lost => "FingerLost",
            };
            format!("match __event {{ ::iced::widget::canvas::Event::Touch(::iced::touch::Event::{variant} {{ id, position }}) => ::std::option::Option::Some((*id, position.x as f64, position.y as f64)), _ => ::std::option::Option::None }}")
        }
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Frame => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::RedrawRequested(_))).then_some(())".into(),
            WindowEvent::Opened => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Opened { position, size }) => { let (x, y) = position.as_ref().map_or((::std::option::Option::None, ::std::option::Option::None), |position| (::std::option::Option::Some(position.x as f64), ::std::option::Option::Some(position.y as f64))); ::std::option::Option::Some((x, y, size.width as f64, size.height as f64)) }, _ => ::std::option::Option::None }".into(),
            WindowEvent::Closed => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Closed)).then_some(())".into(),
            WindowEvent::Moved => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Moved(position)) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }".into(),
            WindowEvent::Resized => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Resized(size)) => ::std::option::Option::Some((size.width as f64, size.height as f64)), _ => ::std::option::Option::None }".into(),
            WindowEvent::Rescaled => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Rescaled(scale)) => ::std::option::Option::Some(*scale as f64), _ => ::std::option::Option::None }".into(),
            WindowEvent::CloseRequested => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::CloseRequested)).then_some(())".into(),
            WindowEvent::Focused => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Focused)).then_some(())".into(),
            WindowEvent::Unfocused => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Unfocused)).then_some(())".into(),
            WindowEvent::FileHovered => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::FileHovered(path)) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }".into(),
            WindowEvent::FileDropped => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::FileDropped(path)) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }".into(),
            WindowEvent::FilesHoveredLeft => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::FilesHoveredLeft)).then_some(())".into(),
        },
        _ => unreachable!("parser rejects non-event canvas sources"),
    }
}

pub(in crate::codegen) fn canvas_event_route_code(
    source: &SubscriptionSource,
    route: &ResolvedCanvasRoute,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
) -> Result<String, Error> {
    match source {
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => {
                resolved_canvas_route_code(route, &[], env, program, message)
            }
            InputMethodEvent::Preedit => resolved_canvas_route_code(
                route,
                &["__value.0", "__value.1", "__value.2"],
                env,
                program,
                message,
            ),
            InputMethodEvent::Commit => {
                resolved_canvas_route_code(route, &["__value"], env, program, message)
            }
        },
        SubscriptionSource::Keyboard(_) => {
            resolved_canvas_route_code(route, &["__value"], env, program, message)
        }
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => {
                resolved_canvas_route_code(route, &[], env, program, message)
            }
            MouseEvent::Moved => resolved_canvas_route_code(
                route,
                &["__value.0", "__value.1"],
                env,
                program,
                message,
            ),
            MouseEvent::Pressed | MouseEvent::Released => {
                resolved_canvas_route_code(route, &["__value"], env, program, message)
            }
            MouseEvent::Wheel => resolved_canvas_route_code(
                route,
                &["__value.0", "__value.1", "__value.2"],
                env,
                program,
                message,
            ),
        },
        SubscriptionSource::Touch(_) => resolved_canvas_route_code(
            route,
            &["__value.0", "__value.1", "__value.2"],
            env,
            program,
            message,
        ),
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Opened => resolved_canvas_route_code(
                route,
                &["__value.0", "__value.1", "__value.2", "__value.3"],
                env,
                program,
                message,
            ),
            WindowEvent::Moved | WindowEvent::Resized => resolved_canvas_route_code(
                route,
                &["__value.0", "__value.1"],
                env,
                program,
                message,
            ),
            WindowEvent::Rescaled | WindowEvent::FileHovered | WindowEvent::FileDropped => {
                resolved_canvas_route_code(route, &["__value"], env, program, message)
            }
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => {
                resolved_canvas_route_code(route, &[], env, program, message)
            }
        },
        _ => unreachable!("parser rejects non-event canvas sources"),
    }
}

fn resolved_canvas_route_code(
    route: &ResolvedCanvasRoute,
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
            ResolvedCanvasRouteArg::Expression(expression) => {
                checked_expr_use_code(program, *expression, env, ValueMode::Owned)
            }
            ResolvedCanvasRouteArg::Payload { index, .. } => payloads
                .get(*index as usize)
                .map(|payload| (*payload).to_owned())
                .ok_or_else(|| invariant("canvas route payload index is outside its contract")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    match &route.target {
        ResolvedCanvasRouteTarget::Handler(handler) => {
            let target = program.try_handler(*handler).ok_or_else(|| {
                invariant("canvas route references an invalid normalized handler")
            })?;
            let variant = match target.owner {
                HandlerOwner::App => handler_variant(&target.name),
                HandlerOwner::Component(component) => {
                    let contract = program.try_component(component).ok_or_else(|| {
                        invariant("canvas route component handler has no component contract")
                    })?;
                    let (active, context) = component_context(env).ok_or_else(|| {
                        invariant("canvas component route has no component context")
                    })?;
                    if active != contract.name {
                        return Err(invariant("canvas component route context diverged"));
                    }
                    args.insert(0, format!("({}).clone()", context.code));
                    component_handler_variant(&contract.name, &target.name)
                }
                HandlerOwner::Preset(_) => {
                    return Err(invariant("canvas route cannot target a preset handler"));
                }
            };
            if args.is_empty() {
                Ok(format!("{message}::{variant}"))
            } else {
                Ok(format!("{message}::{variant}({})", args.join(", ")))
            }
        }
        ResolvedCanvasRouteTarget::ComponentOutput { component, .. } => {
            let output = component_output(env)
                .ok_or_else(|| invariant("canvas component output route has no output callback"))?;
            if program.try_component(*component).is_none() || args.len() != 1 {
                return Err(invariant("canvas component output route contract diverged"));
            }
            Ok(format!("({})({})", output.code, args[0]))
        }
        ResolvedCanvasRouteTarget::ComponentEvent {
            event: _,
            name,
            payloads: expected,
        } => {
            let (component, _) = component_context(env)
                .ok_or_else(|| invariant("canvas named event route has no component context"))?;
            let callback = component_event(env, component, name)
                .ok_or_else(|| invariant("canvas named event route has no normalized callback"))?;
            if args.len() != expected.len() {
                return Err(invariant("canvas named event route arity diverged"));
            }
            Ok(format!("({})({})", callback.code, args.join(", ")))
        }
    }
}

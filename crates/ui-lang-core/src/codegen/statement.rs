use super::*;

pub(in crate::codegen) fn generate_statements(
    out: &mut String,
    statements: &[Statement],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    state: &str,
    return_task: bool,
) -> Result<bool, Error> {
    let mut has_task = false;
    let (task_prefix, task_suffix) = if return_task {
        ("return ", ";")
    } else {
        ("", "")
    };
    for statement in statements {
        match statement {
            Statement::Assign {
                target, value, at, ..
            } => {
                let target_state = document.states.iter().find(|item| item.name == *target);
                let code = expr_code(value, env, document, ValueMode::Owned)?;
                if target_state.is_some_and(|item| matches!(item.ty, Type::Combo(_))) {
                    writeln!(
                        out,
                        "{state}.{target} = ::iced::widget::combo_box::State::new({code});"
                    )
                    .unwrap();
                } else if let Some(State {
                    ty: Type::Animation(inner),
                    ..
                }) = target_state
                {
                    let code = if **inner == Type::F64 {
                        format!("({code}) as f32")
                    } else {
                        code
                    };
                    let at = at
                        .as_ref()
                        .map(|at| expr_code(at, env, document, ValueMode::Owned))
                        .transpose()?
                        .unwrap_or_else(|| "::iced::time::Instant::now()".into());
                    writeln!(out, "{state}.{target}.go_mut({code}, {at});").unwrap();
                } else {
                    writeln!(out, "{state}.{target} = {code};").unwrap();
                }
            }
            Statement::MarkdownAppend { target, value, .. } => {
                let code = expr_code(value, env, document, ValueMode::Owned)?;
                writeln!(out, "{state}.{target}.push_str(&{code});").unwrap();
            }
            Statement::ComboPush { target, value, .. } => {
                let code = expr_code(value, env, document, ValueMode::Owned)?;
                writeln!(out, "{state}.{target}.push({code});").unwrap();
            }
            Statement::ReturnIf { condition, .. } => {
                let code = expr_code(condition, env, document, ValueMode::Owned)?;
                writeln!(out, "if {code} {{ return ::iced::Task::none(); }}").unwrap();
            }
            Statement::Exit { .. } => {
                has_task = true;
                writeln!(
                    out,
                    "{}::iced::exit::<{message}>(){}",
                    task_prefix, task_suffix
                )
                .unwrap();
            }
            Statement::Run {
                kind,
                function,
                args,
                success,
                error,
                span,
                ..
            } => {
                has_task = true;
                let mapper = if component_context(env).is_some() {
                    "move "
                } else {
                    ""
                };
                if *kind == EffectKind::Task
                    && matches!(
                        function.as_str(),
                        "__ice_system_info"
                            | "__ice_system_theme"
                            | "__ice_time_now"
                            | "__ice_clipboard_read"
                            | "__ice_clipboard_read_primary"
                            | "__ice_font_load"
                            | "__ice_image_allocate"
                    )
                {
                    if function == "__ice_font_load" {
                        let bytes = expr_code(&args[0], env, document, ValueMode::Owned)?;
                        let success_message = route_code(success, "value", env, document, message)?;
                        writeln!(
                            out,
                            "{}::iced::font::load({bytes}).map(move |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => match error {{}} }}){}",
                            task_prefix, task_suffix
                        )
                        .unwrap();
                        continue;
                    }
                    if function == "__ice_image_allocate" {
                        let handle = expr_code(&args[0], env, document, ValueMode::Owned)?;
                        let success_message = route_code(success, "value", env, document, message)?;
                        let error_message = route_code(
                            error.as_ref().expect("checker requires image error route"),
                            "error",
                            env,
                            document,
                            message,
                        )?;
                        writeln!(
                            out,
                            "{}::iced::widget::image::allocate({handle}).map(move |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){}",
                            task_prefix, task_suffix
                        )
                        .unwrap();
                        continue;
                    }
                    let task = match function.as_str() {
                        "__ice_system_info" => {
                            "::iced::system::information().map(__ice_system_info)"
                        }
                        "__ice_system_theme" => "::iced::system::theme().map(__ice_system_theme)",
                        "__ice_time_now" => "::iced::time::now()",
                        "__ice_clipboard_read" => "::iced::clipboard::read()",
                        "__ice_clipboard_read_primary" => "::iced::clipboard::read_primary()",
                        _ => unreachable!(),
                    };
                    let success_message = route_code(success, "value", env, document, message)?;
                    writeln!(
                        out,
                        "{}{task}.map(move |value| {success_message}){}",
                        task_prefix, task_suffix
                    )
                    .unwrap();
                    continue;
                }
                let extern_kind = match kind {
                    EffectKind::Future => ExternKind::Future,
                    EffectKind::Task => ExternKind::Task,
                    EffectKind::Stream => ExternKind::Stream,
                };
                let action =
                    find_extern_function(document, function, extern_kind).ok_or_else(|| {
                        Error::new("E130", span, format!("unknown extern fn `{function}`"))
                    })?;
                let args = expr_list_code(args, env, document)?;
                let success_message = route_code(success, "value", env, document, message)?;
                if let (Some(error_route), Some(_)) = (error, &action.error) {
                    let error_message = route_code(error_route, "error", env, document, message)?;
                    match kind {
                        EffectKind::Future => writeln!(out, "{task_prefix}::iced::Task::perform({}({args}), {mapper}|result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){task_suffix}", action.rust_path).unwrap(),
                        EffectKind::Task => writeln!(out, "{task_prefix}{}({args}).map(|result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){task_suffix}", action.rust_path).unwrap(),
                        EffectKind::Stream => writeln!(out, "{task_prefix}::iced::Task::run({}({args}), |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){task_suffix}", action.rust_path).unwrap(),
                    }
                } else {
                    match kind {
                        EffectKind::Future => writeln!(
                            out,
                            "{task_prefix}::iced::Task::perform({}({args}), {mapper}|value| {success_message}){task_suffix}",
                            action.rust_path
                        )
                        .unwrap(),
                        EffectKind::Task => writeln!(
                            out,
                            "{task_prefix}{}({args}).map(|value| {success_message}){task_suffix}",
                            action.rust_path
                        )
                        .unwrap(),
                        EffectKind::Stream => writeln!(
                            out,
                            "{task_prefix}::iced::Task::run({}({args}), |value| {success_message}){task_suffix}",
                            action.rust_path
                        )
                        .unwrap(),
                    }
                }
            }
            Statement::Sip {
                function,
                args,
                progress,
                success,
                error,
                span,
            } => {
                has_task = true;
                let action =
                    find_extern_function(document, function, ExternKind::Sip).ok_or_else(|| {
                        Error::new("E130", span, format!("unknown extern sip `{function}`"))
                    })?;
                let args = expr_list_code(args, env, document)?;
                let progress_message = route_code(progress, "value", env, document, message)?;
                let success_message = route_code(success, "value", env, document, message)?;
                if let (Some(error_route), Some(_)) = (error, &action.error) {
                    let error_message = route_code(error_route, "error", env, document, message)?;
                    writeln!(out, "{task_prefix}::iced::Task::sip({}({args}), |value| {progress_message}, |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){task_suffix}", action.rust_path).unwrap();
                } else {
                    writeln!(out, "{task_prefix}::iced::Task::sip({}({args}), |value| {progress_message}, |value| {success_message}){task_suffix}", action.rust_path).unwrap();
                }
            }
            Statement::TaskFlow {
                source,
                transforms,
                success,
                error,
                units,
                ..
            } => {
                has_task = true;
                let type_env = env
                    .iter()
                    .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                    .collect::<HashMap<_, _>>();
                let (output, error_ty) =
                    crate::check::task_flow_type(source, transforms, document, &type_env)?;
                let task = task_flow_code(source, transforms, document, message, env)?;
                let mapped = if output.is_none() {
                    task
                } else {
                    let success = success.as_ref().expect("checked flow done route");
                    let success_message = route_code(success, "value", env, document, message)?;
                    if error_ty.is_some() {
                        let error = error.as_ref().expect("checked flow error route");
                        let error_message = route_code(error, "error", env, document, message)?;
                        format!(
                            "({task}).map(|result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }})"
                        )
                    } else {
                        format!("({task}).map(|value| {success_message})")
                    }
                };
                let task = if let Some(units) = units {
                    let units_message = route_code(units, "__units", env, document, message)?;
                    format!(
                        "{{ let __task = {mapped}; let __units = i64::try_from(__task.units()).unwrap_or(i64::MAX); ::iced::Task::batch([__task, ::iced::Task::done({units_message})]) }}"
                    )
                } else {
                    mapped
                };
                writeln!(out, "{}{task}{}", task_prefix, task_suffix).unwrap();
            }
            Statement::TaskGroup {
                kind, statements, ..
            } => {
                has_task = true;
                let mut task_env = env.clone();
                for binding in task_env.values_mut() {
                    binding.local = false;
                }
                if return_task {
                    write!(out, "return ").unwrap();
                }
                match kind {
                    TaskGroupKind::Parallel => {
                        writeln!(out, "::iced::Task::batch([").unwrap();
                        for statement in statements {
                            write!(out, "{{ ").unwrap();
                            generate_statements(
                                out,
                                ::std::slice::from_ref(statement),
                                document,
                                message,
                                &task_env,
                                state,
                                false,
                            )?;
                            writeln!(out, "}},").unwrap();
                        }
                        write!(out, "])").unwrap();
                    }
                    TaskGroupKind::Sequential => {
                        write!(out, "::iced::Task::none()").unwrap();
                        for statement in statements {
                            write!(out, ".chain({{ ").unwrap();
                            generate_statements(
                                out,
                                ::std::slice::from_ref(statement),
                                document,
                                message,
                                &task_env,
                                state,
                                false,
                            )?;
                            write!(out, "}})").unwrap();
                        }
                    }
                }
                writeln!(out, "{task_suffix}").unwrap();
            }
            Statement::Abortable {
                handle,
                abort_on_drop,
                task,
                ..
            } => {
                has_task = true;
                let mut task_env = env.clone();
                for binding in task_env.values_mut() {
                    binding.local = false;
                }
                if return_task {
                    write!(out, "return ").unwrap();
                }
                writeln!(out, "{{ let (__task, __handle) = ({{").unwrap();
                generate_statements(
                    out,
                    ::std::slice::from_ref(task),
                    document,
                    message,
                    &task_env,
                    state,
                    false,
                )?;
                writeln!(out, "}}).abortable();").unwrap();
                writeln!(
                    out,
                    "{state}.{handle} = ::std::option::Option::Some(__handle{}); __task }}{}",
                    if *abort_on_drop {
                        ".abort_on_drop()"
                    } else {
                        ""
                    },
                    task_suffix
                )
                .unwrap();
            }
            Statement::Abort { handle, .. } => {
                writeln!(out, "if let ::std::option::Option::Some(__handle) = &{state}.{handle} {{ __handle.abort(); }}").unwrap();
            }
            Statement::DebugStart { name, target, .. } => {
                let name = expr_code(name, env, document, ValueMode::Owned)?;
                writeln!(out, "if let ::std::option::Option::Some(__span) = {state}.{target}.take() {{ __span.finish(); }}").unwrap();
                writeln!(
                    out,
                    "{state}.{target} = ::std::option::Option::Some(::iced::debug::time({name}));"
                )
                .unwrap();
            }
            Statement::DebugFinish { target, .. } => {
                writeln!(out, "if let ::std::option::Option::Some(__span) = {state}.{target}.take() {{ __span.finish(); }}").unwrap();
            }
            Statement::ClipboardWrite { primary, value, .. } => {
                has_task = true;
                let value = expr_code(value, env, document, ValueMode::Owned)?;
                let function = if *primary { "write_primary" } else { "write" };
                writeln!(
                    out,
                    "{}::iced::clipboard::{function}::<{message}>({value}){}",
                    task_prefix, task_suffix
                )
                .unwrap();
            }
            Statement::WidgetOperation {
                operation, route, ..
            } => {
                has_task = true;
                let id = |target: &WidgetTarget| widget_target_code(target, env, document);
                let value = |value: &Expr, cast: &str| {
                    let code = expr_code(value, env, document, ValueMode::Owned)?;
                    Ok::<_, Error>(if cast == "usize" {
                        format!("usize::try_from({code}).unwrap_or(0)")
                    } else {
                        format!("({code}) as {cast}")
                    })
                };
                let task = match operation {
                    WidgetOperation::FocusPrevious => {
                        format!("::iced::widget::operation::focus_previous::<{message}>()")
                    }
                    WidgetOperation::FocusNext => {
                        format!("::iced::widget::operation::focus_next::<{message}>()")
                    }
                    WidgetOperation::Focus { target } => format!(
                        "::iced::widget::operation::focus::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::Focused { target } => {
                        let route = route.as_ref().expect("checker requires focused route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::widget::operation::is_focused({}).map(move |value| {message_code})",
                            id(target)?
                        )
                    }
                    WidgetOperation::CursorFront { target } => format!(
                        "::iced::widget::operation::move_cursor_to_front::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::CursorEnd { target } => format!(
                        "::iced::widget::operation::move_cursor_to_end::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::Cursor { target, position } => format!(
                        "::iced::widget::operation::move_cursor_to::<{message}>({}, {})",
                        id(target)?,
                        value(position, "usize")?
                    ),
                    WidgetOperation::SelectAll { target } => format!(
                        "::iced::widget::operation::select_all::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::Select { target, start, end } => format!(
                        "::iced::widget::operation::select_range::<{message}>({}, {}, {})",
                        id(target)?,
                        value(start, "usize")?,
                        value(end, "usize")?
                    ),
                    WidgetOperation::Snap { target, x, y } => format!(
                        "::iced::widget::operation::snap_to::<{message}>({}, ::iced::widget::operation::RelativeOffset {{ x: {}, y: {} }})",
                        id(target)?,
                        clamped_f32_code(x, "0.0", "1.0", env, document)?,
                        clamped_f32_code(y, "0.0", "1.0", env, document)?
                    ),
                    WidgetOperation::SnapEnd { target } => format!(
                        "::iced::widget::operation::snap_to_end::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::ScrollTo { target, x, y } => format!(
                        "::iced::widget::operation::scroll_to::<{message}>({}, ::iced::widget::operation::AbsoluteOffset {{ x: {}, y: {} }})",
                        id(target)?,
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WidgetOperation::ScrollBy { target, x, y } => format!(
                        "::iced::widget::operation::scroll_by::<{message}>({}, ::iced::widget::operation::AbsoluteOffset {{ x: {}, y: {} }})",
                        id(target)?,
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WidgetOperation::Find { selector, all } => {
                        let route = route.as_ref().expect("checker requires selector route");
                        let (selector, conversion) = widget_selector_code(selector, env, document)?;
                        let function = if *all { "find_all" } else { "find" };
                        let mut task = format!("::iced::widget::selector::{function}({selector})");
                        if let Some(conversion) = conversion {
                            if *all {
                                write!(task, ".map(|values| values.into_iter().map({conversion}).collect::<::std::vec::Vec<_>>())").unwrap();
                            } else {
                                write!(task, ".map(|value| value.map({conversion}))").unwrap();
                            }
                        }
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!("{task}.map(move |value| {message_code})")
                    }
                };
                writeln!(out, "{}{task}{}", task_prefix, task_suffix).unwrap();
            }
            Statement::PaneOperation {
                grid,
                operation,
                route,
                ..
            } => {
                let field = pane_field(grid);
                let dynamic = document_pane_grids(document).into_iter().any(|(node, _)| {
                    matches!(node, ViewNode::PaneGrid { name, templates, .. } if name == grid && !templates.is_empty())
                });
                let pane = |reference: &PaneReference| {
                    pane_reference_find_code(reference, grid, state, dynamic, env, document)
                };
                let edge = |edge: &PaneEdge| match edge {
                    PaneEdge::Top => "Top",
                    PaneEdge::Left => "Left",
                    PaneEdge::Right => "Right",
                    PaneEdge::Bottom => "Bottom",
                };
                let axis = |axis: &PaneAxis| match axis {
                    PaneAxis::Horizontal => "Horizontal",
                    PaneAxis::Vertical => "Vertical",
                };
                match operation {
                    PaneOperation::Maximize { pane: name } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ {state}.{field}.maximize(__pane); }} }}",
                        pane(name)?
                    )
                    .unwrap(),
                    PaneOperation::Restore => {
                        writeln!(out, "{state}.{field}.restore();").unwrap()
                    }
                    PaneOperation::Swap { first, second } => writeln!(
                        out,
                        "{{ let __first = {}; let __second = {}; if let (::std::option::Option::Some(__first), ::std::option::Option::Some(__second)) = (__first, __second) && __first != __second {{ {state}.{field}.swap(__first, __second); }} }}",
                        pane(first)?,
                        pane(second)?
                    )
                    .unwrap(),
                    PaneOperation::Close { pane: name } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ let _ = {state}.{field}.close(__pane); }} }}",
                        pane(name)?
                    )
                    .unwrap(),
                    PaneOperation::Move { pane: name, edge: side } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ {state}.{field}.move_to_edge(__pane, ::iced::widget::pane_grid::Edge::{}); }} }}",
                        pane(name)?,
                        edge(side)
                    )
                    .unwrap(),
                    PaneOperation::Resize { split, ratio } => {
                        let split = split.as_ref().map_or_else(
                            || format!("{state}.{field}.layout().splits().next().copied()"),
                            |name| {
                                format!(
                                    "{state}.{}.get({}).copied()",
                                    pane_splits_field(grid),
                                    rust_string(name)
                                )
                            },
                        );
                        writeln!(
                            out,
                            "{{ let __split = {split}; if let ::std::option::Option::Some(__split) = __split {{ {state}.{field}.resize(__split, (({}) as f32).max(0.0).min(1.0)); }} }}",
                            expr_code(ratio, env, document, ValueMode::Owned)?
                        )
                        .unwrap();
                    }
                    PaneOperation::Drop {
                        pane: name,
                        target,
                        edge: side,
                    } => {
                        let region = side.as_ref().map_or_else(
                            || "::iced::widget::pane_grid::Region::Center".into(),
                            |side| {
                                format!(
                                    "::iced::widget::pane_grid::Region::Edge(::iced::widget::pane_grid::Edge::{})",
                                    edge(side)
                                )
                            },
                        );
                        writeln!(
                            out,
                            "{{ let __pane = {}; let __target = {}; if let (::std::option::Option::Some(__pane), ::std::option::Option::Some(__target)) = (__pane, __target) && __pane != __target {{ {state}.{field}.drop(__pane, ::iced::widget::pane_grid::Target::Pane(__target, {region})); }} }}",
                            pane(name)?,
                            pane(target)?
                        )
                        .unwrap();
                    }
                    PaneOperation::Split {
                        target,
                        pane: name,
                        axis: direction,
                        ratio,
                    } => {
                        let target = pane(target)?;
                        let value = pane_reference_value_code(
                            name, grid, dynamic, env, document,
                        )?;
                        let ratio = expr_code(ratio, env, document, ValueMode::Owned)?;
                        if dynamic {
                            writeln!(
                                out,
                                "{{ let __target = {target}; let __pane_value = {value}; let __pane = {state}.{field}.iter().find_map(|(__pane, __value)| (__value == &__pane_value).then_some(*__pane)); if let (::std::option::Option::Some(__target), ::std::option::Option::None) = (__target, __pane) {{ if let ::std::option::Option::Some((_, __split)) = {state}.{field}.split(::iced::widget::pane_grid::Axis::{}, __target, __pane_value) {{ {state}.{field}.resize(__split, (({ratio}) as f32).max(0.0).min(1.0)); }} }} }}",
                                axis(direction),
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                out,
                                "{{ let __target = {target}; let __pane = {}; if let (::std::option::Option::Some(__target), ::std::option::Option::None) = (__target, __pane) {{ if let ::std::option::Option::Some((_, __split)) = {state}.{field}.split(::iced::widget::pane_grid::Axis::{}, __target, {value}) {{ {state}.{field}.resize(__split, (({ratio}) as f32).max(0.0).min(1.0)); }} }} }}",
                                pane(name)?,
                                axis(direction),
                            )
                            .unwrap();
                        }
                    }
                    PaneOperation::Maximized | PaneOperation::Adjacent { .. } => {
                        has_task = true;
                        let value = match operation {
                            PaneOperation::Maximized => if dynamic {
                                format!("{state}.{field}.maximized().and_then(|__pane| {state}.{field}.get(__pane)).map(|__pane| __pane.__name())")
                            } else {
                                format!("{state}.{field}.maximized().and_then(|__pane| {state}.{field}.get(__pane)).map(|__name| (*__name).to_owned())")
                            },
                            PaneOperation::Adjacent { pane: name, edge: side } => {
                                let direction = match side {
                                    PaneEdge::Top => "Up",
                                    PaneEdge::Left => "Left",
                                    PaneEdge::Right => "Right",
                                    PaneEdge::Bottom => "Down",
                                };
                                let value = pane(name)?;
                                if dynamic {
                                    format!("{value}.and_then(|__pane| {state}.{field}.adjacent(__pane, ::iced::widget::pane_grid::Direction::{direction})).and_then(|__pane| {state}.{field}.get(__pane)).map(|__pane| __pane.__name())")
                                } else {
                                    format!("{value}.and_then(|__pane| {state}.{field}.adjacent(__pane, ::iced::widget::pane_grid::Direction::{direction})).and_then(|__pane| {state}.{field}.get(__pane)).map(|__name| (*__name).to_owned())")
                                }
                            }
                            _ => unreachable!(),
                        };
                        let route = route.as_ref().expect("checker requires pane query route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        let task = format!(
                            "{{ let value = {value}; ::iced::Task::done({message_code}) }}"
                        );
                        writeln!(
                            out,
                            "{}{task}{}",
                            task_prefix, task_suffix
                        )
                        .unwrap();
                    }
                }
            }
            Statement::WindowOperation {
                operation,
                target,
                route,
                ..
            } => {
                has_task = true;
                let target = target
                    .as_ref()
                    .map(|target| expr_code(target, env, document, ValueMode::Owned))
                    .transpose()?;
                let id = target.as_deref().unwrap_or("__window");
                let value = |value: &Expr, cast: &str| {
                    Ok::<_, Error>(format!(
                        "({}) as {cast}",
                        expr_code(value, env, document, ValueMode::Owned)?
                    ))
                };
                let size = |width: &Expr, height: &Expr| {
                    let positive = |value| {
                        Ok::<_, Error>(format!(
                            "(({}) as f32).max(f32::EPSILON).min(f32::MAX)",
                            expr_code(value, env, document, ValueMode::Owned)?
                        ))
                    };
                    Ok::<_, Error>(format!(
                        "::iced::Size::new({}, {})",
                        positive(width)?,
                        positive(height)?
                    ))
                };
                let optional_size = |size_value: &Option<(Expr, Expr)>| {
                    Ok::<_, Error>(match size_value {
                        Some((width, height)) => {
                            format!("::std::option::Option::Some({})", size(width, height)?)
                        }
                        None => "::std::option::Option::None".into(),
                    })
                };
                let bool_value = |value: &Expr| expr_code(value, env, document, ValueMode::Owned);
                let task = match operation {
                    WindowOperation::Open(name) => {
                        let settings = name.as_ref().map_or_else(
                            || "::std::default::Default::default()".into(),
                            |name| {
                                let index = document
                                    .settings
                                    .windows
                                    .iter()
                                    .position(|window| window.name == *name)
                                    .expect("checker validates named windows");
                                format!("Self::__window_{index}()")
                            },
                        );
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "{{ let (_, __task) = ::iced::window::open({settings}); __task.map(move |value| {message_code}) }}"
                        )
                    }
                    WindowOperation::Oldest | WindowOperation::Latest => {
                        let function = if matches!(operation, WindowOperation::Oldest) {
                            "oldest"
                        } else {
                            "latest"
                        };
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!("::iced::window::{function}().map(move |value| {message_code})")
                    }
                    WindowOperation::Close => {
                        format!("::iced::window::close::<{message}>({id})")
                    }
                    WindowOperation::Drag => {
                        format!("::iced::window::drag::<{message}>({id})")
                    }
                    WindowOperation::DragResize(direction) => {
                        let direction = match direction {
                            WindowDirection::North => "North",
                            WindowDirection::South => "South",
                            WindowDirection::East => "East",
                            WindowDirection::West => "West",
                            WindowDirection::NorthEast => "NorthEast",
                            WindowDirection::NorthWest => "NorthWest",
                            WindowDirection::SouthEast => "SouthEast",
                            WindowDirection::SouthWest => "SouthWest",
                        };
                        format!(
                            "::iced::window::drag_resize::<{message}>({id}, ::iced::window::Direction::{direction})"
                        )
                    }
                    WindowOperation::Resize(width, height) => format!(
                        "::iced::window::resize::<{message}>({id}, {})",
                        size(width, height)?
                    ),
                    WindowOperation::Resizable(enabled) => format!(
                        "::iced::window::set_resizable::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::MinSize(size) => format!(
                        "::iced::window::set_min_size::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::MaxSize(size) => format!(
                        "::iced::window::set_max_size::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::ResizeIncrements(size) => format!(
                        "::iced::window::set_resize_increments::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::Size => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = ordered_route_code(
                            route,
                            &["value.width as f64", "value.height as f64"],
                            env,
                            document,
                            message,
                        )?;
                        format!("::iced::window::size({id}).map(move |value| {message_code})")
                    }
                    WindowOperation::IsMaximized => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::is_maximized({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Maximize(enabled) => format!(
                        "::iced::window::maximize::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::IsMinimized => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::is_minimized({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Minimize(enabled) => format!(
                        "::iced::window::minimize::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::Position => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            ordered_route_code(route, &["x", "y"], env, document, message)?;
                        format!(
                            "::iced::window::position({id}).map(move |value| {{ let (x, y) = value.map_or((::std::option::Option::None, ::std::option::Option::None), |value| (::std::option::Option::Some(value.x as f64), ::std::option::Option::Some(value.y as f64))); {message_code} }})"
                        )
                    }
                    WindowOperation::ScaleFactor => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            route_code(route, "value as f64", env, document, message)?;
                        format!(
                            "::iced::window::scale_factor({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Move(x, y) => format!(
                        "::iced::window::move_to::<{message}>({id}, ::iced::Point::new({}, {}))",
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WindowOperation::Mode => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::mode({id}).map(move |value| {{ let value = match value {{ ::iced::window::Mode::Windowed => \"windowed\", ::iced::window::Mode::Fullscreen => \"fullscreen\", ::iced::window::Mode::Hidden => \"hidden\" }}.to_owned(); {message_code} }})"
                        )
                    }
                    WindowOperation::SetMode(mode) => {
                        let mode = match mode {
                            WindowMode::Windowed => "Windowed",
                            WindowMode::Fullscreen => "Fullscreen",
                            WindowMode::Hidden => "Hidden",
                        };
                        format!(
                            "::iced::window::set_mode::<{message}>({id}, ::iced::window::Mode::{mode})"
                        )
                    }
                    WindowOperation::ToggleMaximize => {
                        format!("::iced::window::toggle_maximize::<{message}>({id})")
                    }
                    WindowOperation::ToggleDecorations => {
                        format!("::iced::window::toggle_decorations::<{message}>({id})")
                    }
                    WindowOperation::Attention(attention) => {
                        let attention: String = match attention {
                            None => "::std::option::Option::None".into(),
                            Some(WindowAttention::Critical) => "::std::option::Option::Some(::iced::window::UserAttention::Critical)".into(),
                            Some(WindowAttention::Informational) => "::std::option::Option::Some(::iced::window::UserAttention::Informational)".into(),
                        };
                        format!(
                            "::iced::window::request_user_attention::<{message}>({id}, {attention})"
                        )
                    }
                    WindowOperation::Focus => {
                        format!("::iced::window::gain_focus::<{message}>({id})")
                    }
                    WindowOperation::SetLevel(level) => {
                        let level = match level {
                            WindowLevel::Normal => "Normal",
                            WindowLevel::AlwaysOnBottom => "AlwaysOnBottom",
                            WindowLevel::AlwaysOnTop => "AlwaysOnTop",
                        };
                        format!(
                            "::iced::window::set_level::<{message}>({id}, ::iced::window::Level::{level})"
                        )
                    }
                    WindowOperation::SystemMenu => {
                        format!("::iced::window::show_system_menu::<{message}>({id})")
                    }
                    WindowOperation::RawId => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            route_code(route, "value.to_string()", env, document, message)?;
                        format!(
                            "::iced::window::raw_id::<{message}>({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Screenshot => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!("::iced::window::screenshot({id}).map(move |value| {message_code})")
                    }
                    WindowOperation::MousePassthrough(enabled) => {
                        let enabled = bool_value(enabled)?;
                        format!(
                            "if {enabled} {{ ::iced::window::enable_mouse_passthrough::<{message}>({id}) }} else {{ ::iced::window::disable_mouse_passthrough::<{message}>({id}) }}"
                        )
                    }
                    WindowOperation::MonitorSize => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = ordered_route_code(
                            route,
                            &["width", "height"],
                            env,
                            document,
                            message,
                        )?;
                        format!(
                            "::iced::window::monitor_size({id}).map(move |value| {{ let (width, height) = value.map_or((::std::option::Option::None, ::std::option::Option::None), |value| (::std::option::Option::Some(value.width as f64), ::std::option::Option::Some(value.height as f64))); {message_code} }})"
                        )
                    }
                    WindowOperation::AutomaticTabbing(enabled) => format!(
                        "::iced::window::allow_automatic_tabbing::<{message}>({})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::Icon {
                        pixels,
                        width,
                        height,
                    } => {
                        let pixels = expr_code(pixels, env, document, ValueMode::Owned)?;
                        let width = expr_code(width, env, document, ValueMode::Owned)?;
                        let height = expr_code(height, env, document, ValueMode::Owned)?;
                        format!(
                            "{{ let __pixels = {pixels}; let __width = {width}; let __height = {height}; match (::std::primitive::u32::try_from(__width), ::std::primitive::u32::try_from(__height)) {{ (::std::result::Result::Ok(__width), ::std::result::Result::Ok(__height)) if __width > 0 && __height > 0 && __width.checked_mul(__height).is_some() => ::iced::window::icon::from_rgba(__pixels, __width, __height).map_or_else(|_| ::iced::Task::none(), |__icon| ::iced::window::set_icon::<{message}>({id}, __icon)), _ => ::iced::Task::none(), }} }}"
                        )
                    }
                    WindowOperation::Callback { function, args } => {
                        let callback = find_extern_function(document, function, ExternKind::Window)
                            .expect("checker validates window callback");
                        let args = expr_args_suffix_code(args, env, document)?;
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::run({id}, move |__window| {}(__window{args})).map(move |value| {message_code})",
                            callback.rust_path
                        )
                    }
                };
                let task = if target.is_some()
                    || matches!(
                        operation,
                        WindowOperation::Open(_)
                            | WindowOperation::Oldest
                            | WindowOperation::Latest
                            | WindowOperation::AutomaticTabbing(_)
                    ) {
                    task
                } else {
                    format!("::iced::window::oldest().and_then(move |__window| {task})")
                };
                writeln!(out, "{}{task}{}", task_prefix, task_suffix).unwrap();
            }
        }
    }
    Ok(has_task)
}

mod task;
mod view_fn;

pub(super) use task::*;
pub(super) use view_fn::*;

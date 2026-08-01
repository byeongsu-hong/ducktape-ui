use super::*;

fn route_result_code(route: &ResolvedRoute, binding: &str, expression: String) -> String {
    if route
        .args
        .iter()
        .any(|arg| matches!(arg, ResolvedRouteArg::Payload { .. }))
    {
        expression
    } else {
        format!("{{ let _ = &{binding}; {expression} }}")
    }
}

fn editor_self_assignment_code(
    target: &ResolvedWritableState,
    value: CheckedExprUseId,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    state: &str,
) -> Result<String, Error> {
    let binding = Binding {
        code: format!("::std::mem::take(&mut {state}.{})", target.name),
        ty: target.ty.clone(),
        local: true,
        state: None,
        owner: Some(BindingOwner::Value(target.value)),
    };
    let moved = LayeredBindingEnv::new(env, &target.name, binding);
    checked_expr_use_code(program, value, &moved, ValueMode::Owned)
}

fn resolved_widget_target_code(
    target: &ResolvedWidgetTarget,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let mut scope = env
        .component_context()
        .map(|(_, binding)| binding.code.clone())
        .unwrap_or_else(|| rust_string(program.app_name()));
    for segment in &target.segments {
        scope = if let Some(key) = segment.key {
            let key = checked_expr_use_code(program, key, env, ValueMode::Borrowed)?;
            format!("format!(\"{{}}/{}({{}})\", {scope}, {key})", segment.name)
        } else {
            format!("format!(\"{{}}/{}\", {scope})", segment.name)
        };
    }
    let constructor = if env.component_context().is_none()
        && target.segments.iter().all(|segment| segment.key.is_none())
    {
        "new"
    } else {
        "from"
    };
    let path = if constructor == "new" {
        rust_string(&format!(
            "{}/{}",
            program.app_name(),
            target
                .segments
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>()
                .join("/")
        ))
    } else {
        scope
    };
    Ok(format!("::iced::widget::Id::{constructor}({path})"))
}

fn resolved_widget_selector_code(
    selector: &ResolvedWidgetSelector,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<(String, Option<&'static str>), Error> {
    Ok(match selector {
        ResolvedWidgetSelector::Id(target) => (
            format!(
                "::iced::widget::selector::id({})",
                resolved_widget_target_code(target, env, program)?
            ),
            Some("__ice_widget_target_from_target"),
        ),
        ResolvedWidgetSelector::Text(value) => (
            checked_expr_use_code(program, *value, env, ValueMode::Owned)?,
            Some("__ice_widget_target_from_text"),
        ),
        ResolvedWidgetSelector::Point { x, y } => (
            format!(
                "::iced::Point::new(({}) as f32, ({}) as f32)",
                checked_expr_use_code(program, *x, env, ValueMode::Owned)?,
                checked_expr_use_code(program, *y, env, ValueMode::Owned)?
            ),
            Some("__ice_widget_target_from_target"),
        ),
        ResolvedWidgetSelector::Focused => (
            "::iced::widget::selector::is_focused()".into(),
            Some("__ice_widget_target_from_target"),
        ),
        ResolvedWidgetSelector::Extern { target, args } => {
            let function = program.extern_function(*target);
            let args = args
                .iter()
                .map(|arg| checked_expr_use_code(program, *arg, env, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            (format!("{}({args})", function.rust_path), None)
        }
    })
}

fn resolved_pane_value_code(
    reference: &ResolvedPaneReference,
    grid: &str,
    dynamic: bool,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    Ok(match reference {
        ResolvedPaneReference::Static(name) if dynamic => {
            format!("{}::__Static({})", pane_type(grid), rust_string(name))
        }
        ResolvedPaneReference::Static(name) => rust_string(name),
        ResolvedPaneReference::Dynamic { template, key } => format!(
            "{}::{}({})",
            pane_type(grid),
            pane_template_variant(template),
            checked_expr_use_code(program, *key, env, ValueMode::Owned)?
        ),
    })
}

fn resolved_pane_find_code(
    reference: &ResolvedPaneReference,
    grid: &str,
    state: &str,
    dynamic: bool,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let field = pane_field(grid);
    if dynamic {
        let value = resolved_pane_value_code(reference, grid, true, env, program)?;
        return Ok(format!(
            "{{ let __value = {value}; {state}.{field}.iter().find_map(|(__pane, __pane_value)| (__pane_value == &__value).then_some(*__pane)) }}"
        ));
    }
    match reference {
        ResolvedPaneReference::Static(name) => Ok(format!(
            "{state}.{field}.iter().find_map(|(__pane, __name)| (*__name == {}).then_some(*__pane))",
            rust_string(name)
        )),
        ResolvedPaneReference::Dynamic { .. } => Err(Error::new(
            "E196",
            &Span::line(1),
            "normalized dynamic pane reference has no dynamic pane grid",
        )),
    }
}

pub(in crate::codegen) fn generate_statements(
    out: &mut String,
    statements: &[ResolvedStatement],
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    state: &str,
    return_task: bool,
) -> Result<bool, Error> {
    let mut local_env = ScopedBindingEnv::new(env);
    let env = &mut local_env;
    let mut has_task = false;
    let (task_prefix, task_suffix) = if return_task {
        ("return ", ";")
    } else {
        ("", "")
    };
    for statement in statements {
        has_task |= statement.task.is_some();
        writeln!(out, "{}", source_marker_origin(program, statement.origin)).unwrap();
        match &statement.kind {
            ResolvedStatementKind::Let { local, name, value } => {
                let code = checked_expr_use_code(program, *value, env, ValueMode::Owned)?;
                writeln!(out, "let {name} = {code};").unwrap();
                let ty = program
                    .checked_facts()
                    .expression_use(*value)
                    .destination
                    .clone();
                env.insert(
                    name.clone(),
                    Binding {
                        code: name.clone(),
                        ty,
                        local: false,
                        state: None,
                        owner: Some(BindingOwner::Local(*local)),
                    },
                );
            }
            ResolvedStatementKind::Assign {
                target,
                value,
                at,
                move_self,
            } => {
                let code = if *move_self {
                    editor_self_assignment_code(target, *value, env, program, state)?
                } else {
                    checked_expr_use_code(program, *value, env, ValueMode::Owned)?
                };
                if matches!(target.ty, Type::Combo(_)) {
                    writeln!(
                        out,
                        "{state}.{} = ::iced::widget::combo_box::State::new({code});",
                        target.name
                    )
                    .unwrap();
                } else if let Type::Animation(inner) = &target.ty {
                    let code = if **inner == Type::F64 {
                        format!("({code}) as f32")
                    } else {
                        code
                    };
                    let at = at
                        .as_ref()
                        .map(|at| checked_expr_use_code(program, *at, env, ValueMode::Owned))
                        .transpose()?
                        .unwrap_or_else(|| "::iced::time::Instant::now()".into());
                    writeln!(out, "{state}.{}.go_mut({code}, {at});", target.name).unwrap();
                } else {
                    writeln!(out, "{state}.{} = {code};", target.name).unwrap();
                }
            }
            ResolvedStatementKind::MarkdownAppend { target, value } => {
                let code = checked_expr_use_code(program, *value, env, ValueMode::Owned)?;
                writeln!(out, "{state}.{}.push_str(&{code});", target.name).unwrap();
            }
            ResolvedStatementKind::ComboPush { target, value } => {
                let code = checked_expr_use_code(program, *value, env, ValueMode::Owned)?;
                writeln!(out, "{state}.{}.push({code});", target.name).unwrap();
            }
            ResolvedStatementKind::ReturnIf { condition } => {
                let code = checked_expr_use_code(program, *condition, env, ValueMode::Owned)?;
                writeln!(out, "if {code} {{ return ::iced::Task::none(); }}").unwrap();
            }
            ResolvedStatementKind::Exit => {
                writeln!(
                    out,
                    "{}::iced::exit::<{message}>(){}",
                    task_prefix, task_suffix
                )
                .unwrap();
            }
            ResolvedStatementKind::Run(run) => {
                let ResolvedRun {
                    kind,
                    target,
                    args,
                    success,
                    error,
                    ..
                } = run;
                let mapper = if env.component_context().is_some() {
                    "move "
                } else {
                    ""
                };
                if let ResolvedEffectTarget::Builtin(function) = target {
                    if function == "__ice_font_load" {
                        let bytes = checked_expr_use_code(program, args[0], env, ValueMode::Owned)?;
                        let success_message = route_result_code(
                            success,
                            "value",
                            resolved_route_code(success, &["value"], env, program, message)?,
                        );
                        writeln!(
                            out,
                            "{}::iced::font::load({bytes}).map(move |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => match error {{}} }}){}",
                            task_prefix, task_suffix
                        )
                        .unwrap();
                        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
                        continue;
                    }
                    if function == "__ice_image_allocate" {
                        let handle =
                            checked_expr_use_code(program, args[0], env, ValueMode::Owned)?;
                        let success_message = route_result_code(
                            success,
                            "value",
                            resolved_route_code(success, &["value"], env, program, message)?,
                        );
                        let error_message = resolved_route_code(
                            error.as_ref().expect("checker requires image error route"),
                            &["error"],
                            env,
                            program,
                            message,
                        )?;
                        let error_message = route_result_code(
                            error.as_ref().expect("checker requires image error route"),
                            "error",
                            error_message,
                        );
                        writeln!(
                            out,
                            "{}::iced::widget::image::allocate({handle}).map(move |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){}",
                            task_prefix, task_suffix
                        )
                        .unwrap();
                        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
                        continue;
                    }
                    let task_code = match function.as_str() {
                        "__ice_system_info" => {
                            "::iced::system::information().map(__ice_system_info)"
                        }
                        "__ice_system_theme" => "::iced::system::theme().map(__ice_system_theme)",
                        "__ice_time_now" => "::iced::time::now()",
                        "__ice_clipboard_read" => "::iced::clipboard::read()",
                        "__ice_clipboard_read_primary" => "::iced::clipboard::read_primary()",
                        _ => unreachable!(),
                    };
                    let success_message = route_result_code(
                        success,
                        "value",
                        resolved_route_code(success, &["value"], env, program, message)?,
                    );
                    writeln!(
                        out,
                        "{}{task_code}.map(move |value| {success_message}){}",
                        task_prefix, task_suffix
                    )
                    .unwrap();
                    writeln!(out, "{SOURCE_MARKER_END}").unwrap();
                    continue;
                }
                let ResolvedEffectTarget::Extern(action) = target else {
                    unreachable!("built-in effects return above")
                };
                let action = program.extern_function(*action);
                let args = args
                    .iter()
                    .map(|arg| checked_expr_use_code(program, *arg, env, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let success_message = route_result_code(
                    success,
                    "value",
                    resolved_route_code(success, &["value"], env, program, message)?,
                );
                if let (Some(error_route), Some(_)) = (error, &action.error) {
                    let error_message = route_result_code(
                        error_route,
                        "error",
                        resolved_route_code(error_route, &["error"], env, program, message)?,
                    );
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
            ResolvedStatementKind::Sip(sip) => {
                let ResolvedSip {
                    target,
                    args,
                    progress,
                    success,
                    error,
                } = sip;
                let action = program.extern_function(*target);
                let args = args
                    .iter()
                    .map(|arg| checked_expr_use_code(program, *arg, env, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let progress_message = route_result_code(
                    progress,
                    "value",
                    resolved_route_code(progress, &["value"], env, program, message)?,
                );
                let success_message = route_result_code(
                    success,
                    "value",
                    resolved_route_code(success, &["value"], env, program, message)?,
                );
                if let (Some(error_route), Some(_)) = (error, &action.error) {
                    let error_message = route_result_code(
                        error_route,
                        "error",
                        resolved_route_code(error_route, &["error"], env, program, message)?,
                    );
                    writeln!(out, "{task_prefix}::iced::Task::sip({}({args}), |value| {progress_message}, |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){task_suffix}", action.rust_path).unwrap();
                } else {
                    writeln!(out, "{task_prefix}::iced::Task::sip({}({args}), |value| {progress_message}, |value| {success_message}){task_suffix}", action.rust_path).unwrap();
                }
            }
            ResolvedStatementKind::TaskFlow(flow) => {
                let task = task_flow_code(flow, program, message, env)?;
                let mapped = if flow.output.is_none() {
                    task
                } else {
                    let success = flow.success.as_ref().expect("checked flow done route");
                    let success_message = route_result_code(
                        success,
                        "value",
                        resolved_route_code(success, &["value"], env, program, message)?,
                    );
                    if flow.error_type.is_some() {
                        let error = flow.error.as_ref().expect("checked flow error route");
                        let error_message = route_result_code(
                            error,
                            "error",
                            resolved_route_code(error, &["error"], env, program, message)?,
                        );
                        format!(
                            "({task}).map(|result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }})"
                        )
                    } else {
                        format!("({task}).map(|value| {success_message})")
                    }
                };
                let task = if let Some(units) = &flow.units {
                    let units_message =
                        resolved_route_code(units, &["__units"], env, program, message)?;
                    format!(
                        "{{ let __task = {mapped}; let __units = i64::try_from(__task.units()).unwrap_or(i64::MAX); ::iced::Task::batch([__task, ::iced::Task::done({units_message})]) }}"
                    )
                } else {
                    mapped
                };
                writeln!(out, "{}{task}{}", task_prefix, task_suffix).unwrap();
            }
            ResolvedStatementKind::TaskGroup {
                kind, statements, ..
            } => {
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
                                program,
                                message,
                                env,
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
                                program,
                                message,
                                env,
                                state,
                                false,
                            )?;
                            write!(out, "}})").unwrap();
                        }
                    }
                }
                writeln!(out, "{task_suffix}").unwrap();
            }
            ResolvedStatementKind::Abortable {
                handle,
                abort_on_drop,
                task,
                ..
            } => {
                if return_task {
                    write!(out, "return ").unwrap();
                }
                writeln!(out, "{{ let (__task, __handle) = ({{").unwrap();
                generate_statements(
                    out,
                    ::std::slice::from_ref(task),
                    program,
                    message,
                    env,
                    state,
                    false,
                )?;
                writeln!(out, "}}).abortable();").unwrap();
                writeln!(
                    out,
                    "{state}.{} = ::std::option::Option::Some(__handle{}); __task }}{}",
                    handle.name,
                    if *abort_on_drop {
                        ".abort_on_drop()"
                    } else {
                        ""
                    },
                    task_suffix
                )
                .unwrap();
            }
            ResolvedStatementKind::Abort { handle } => {
                writeln!(out, "if let ::std::option::Option::Some(__handle) = &{state}.{} {{ __handle.abort(); }}", handle.name).unwrap();
            }
            ResolvedStatementKind::DebugStart { name, target } => {
                let name = checked_expr_use_code(program, *name, env, ValueMode::Owned)?;
                writeln!(out, "if let ::std::option::Option::Some(__span) = {state}.{}.take() {{ __span.finish(); }}", target.name).unwrap();
                writeln!(
                    out,
                    "{state}.{} = ::std::option::Option::Some(::iced::debug::time({name}));",
                    target.name
                )
                .unwrap();
            }
            ResolvedStatementKind::DebugFinish { target } => {
                writeln!(out, "if let ::std::option::Option::Some(__span) = {state}.{}.take() {{ __span.finish(); }}", target.name).unwrap();
            }
            ResolvedStatementKind::ClipboardWrite { primary, value } => {
                let value = checked_expr_use_code(program, *value, env, ValueMode::Owned)?;
                let function = if *primary { "write_primary" } else { "write" };
                writeln!(
                    out,
                    "{}::iced::clipboard::{function}::<{message}>({value}){}",
                    task_prefix, task_suffix
                )
                .unwrap();
            }
            ResolvedStatementKind::WidgetOperation {
                operation, route, ..
            } => {
                let id = |target: &ResolvedWidgetTarget| {
                    resolved_widget_target_code(target, env, program)
                };
                let value = |value: CheckedExprUseId, cast: &str| {
                    let code = checked_expr_use_code(program, value, env, ValueMode::Owned)?;
                    Ok::<_, Error>(if cast == "usize" {
                        format!("usize::try_from({code}).unwrap_or(0)")
                    } else {
                        format!("({code}) as {cast}")
                    })
                };
                let task = match operation {
                    ResolvedWidgetOperation::FocusPrevious => {
                        format!("::iced::widget::operation::focus_previous::<{message}>()")
                    }
                    ResolvedWidgetOperation::FocusNext => {
                        format!("::iced::widget::operation::focus_next::<{message}>()")
                    }
                    ResolvedWidgetOperation::Focus { target } => format!(
                        "::iced::widget::operation::focus::<{message}>({})",
                        id(target)?
                    ),
                    ResolvedWidgetOperation::Focused { target } => {
                        let route = route.as_ref().expect("checker requires focused route");
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!(
                            "::iced::widget::operation::is_focused({}).map(move |value| {message_code})",
                            id(target)?
                        )
                    }
                    ResolvedWidgetOperation::CursorFront { target } => format!(
                        "::iced::widget::operation::move_cursor_to_front::<{message}>({})",
                        id(target)?
                    ),
                    ResolvedWidgetOperation::CursorEnd { target } => format!(
                        "::iced::widget::operation::move_cursor_to_end::<{message}>({})",
                        id(target)?
                    ),
                    ResolvedWidgetOperation::Cursor { target, position } => format!(
                        "::iced::widget::operation::move_cursor_to::<{message}>({}, {})",
                        id(target)?,
                        value(*position, "usize")?
                    ),
                    ResolvedWidgetOperation::SelectAll { target } => format!(
                        "::iced::widget::operation::select_all::<{message}>({})",
                        id(target)?
                    ),
                    ResolvedWidgetOperation::Select { target, start, end } => format!(
                        "::iced::widget::operation::select_range::<{message}>({}, {}, {})",
                        id(target)?,
                        value(*start, "usize")?,
                        value(*end, "usize")?
                    ),
                    ResolvedWidgetOperation::Snap { target, x, y } => {
                        let x = checked_expr_use_code(program, *x, env, ValueMode::Owned)?;
                        let y = checked_expr_use_code(program, *y, env, ValueMode::Owned)?;
                        format!(
                            "::iced::widget::operation::snap_to::<{message}>({}, ::iced::widget::operation::RelativeOffset {{ x: (({x}) as f32).max(0.0).min(1.0), y: (({y}) as f32).max(0.0).min(1.0) }})",
                            id(target)?,
                        )
                    }
                    ResolvedWidgetOperation::SnapEnd { target } => format!(
                        "::iced::widget::operation::snap_to_end::<{message}>({})",
                        id(target)?
                    ),
                    ResolvedWidgetOperation::ScrollTo { target, x, y } => format!(
                        "::iced::widget::operation::scroll_to::<{message}>({}, ::iced::widget::operation::AbsoluteOffset {{ x: {}, y: {} }})",
                        id(target)?,
                        value(*x, "f32")?,
                        value(*y, "f32")?
                    ),
                    ResolvedWidgetOperation::ScrollBy { target, x, y } => format!(
                        "::iced::widget::operation::scroll_by::<{message}>({}, ::iced::widget::operation::AbsoluteOffset {{ x: {}, y: {} }})",
                        id(target)?,
                        value(*x, "f32")?,
                        value(*y, "f32")?
                    ),
                    ResolvedWidgetOperation::Find { selector, all } => {
                        let route = route.as_ref().expect("checker requires selector route");
                        let (selector, conversion) =
                            resolved_widget_selector_code(selector, env, program)?;
                        let function = if *all { "find_all" } else { "find" };
                        let mut task = format!("::iced::widget::selector::{function}({selector})");
                        if let Some(conversion) = conversion {
                            if *all {
                                write!(task, ".map(|values| values.into_iter().map({conversion}).collect::<::std::vec::Vec<_>>())").unwrap();
                            } else {
                                write!(task, ".map(|value| value.map({conversion}))").unwrap();
                            }
                        }
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!("{task}.map(move |value| {message_code})")
                    }
                };
                writeln!(out, "{}{task}{}", task_prefix, task_suffix).unwrap();
            }
            ResolvedStatementKind::PaneOperation {
                grid,
                dynamic,
                operation,
                route,
                ..
            } => {
                let field = pane_field(grid);
                let pane = |reference: &ResolvedPaneReference| {
                    resolved_pane_find_code(reference, grid, state, *dynamic, env, program)
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
                    ResolvedPaneOperation::Maximize { pane: name } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ {state}.{field}.maximize(__pane); }} }}",
                        pane(name)?
                    )
                    .unwrap(),
                    ResolvedPaneOperation::Restore => {
                        writeln!(out, "{state}.{field}.restore();").unwrap()
                    }
                    ResolvedPaneOperation::Swap { first, second } => writeln!(
                        out,
                        "{{ let __first = {}; let __second = {}; if let (::std::option::Option::Some(__first), ::std::option::Option::Some(__second)) = (__first, __second) && __first != __second {{ {state}.{field}.swap(__first, __second); }} }}",
                        pane(first)?,
                        pane(second)?
                    )
                    .unwrap(),
                    ResolvedPaneOperation::Close { pane: name } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ let _ = {state}.{field}.close(__pane); }} }}",
                        pane(name)?
                    )
                    .unwrap(),
                    ResolvedPaneOperation::Move { pane: name, edge: side } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ {state}.{field}.move_to_edge(__pane, ::iced::widget::pane_grid::Edge::{}); }} }}",
                        pane(name)?,
                        edge(side)
                    )
                    .unwrap(),
                    ResolvedPaneOperation::Resize { split, ratio } => {
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
                            checked_expr_use_code(program, *ratio, env, ValueMode::Owned)?
                        )
                        .unwrap();
                    }
                    ResolvedPaneOperation::Drop {
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
                    ResolvedPaneOperation::Split {
                        target,
                        pane: name,
                        axis: direction,
                        ratio,
                    } => {
                        let target = pane(target)?;
                        let value = resolved_pane_value_code(
                            name, grid, *dynamic, env, program,
                        )?;
                        let ratio = checked_expr_use_code(program, *ratio, env, ValueMode::Owned)?;
                        if *dynamic {
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
                    ResolvedPaneOperation::Maximized | ResolvedPaneOperation::Adjacent { .. } => {
                        let value = match operation {
                            ResolvedPaneOperation::Maximized => if *dynamic {
                                format!("{state}.{field}.maximized().and_then(|__pane| {state}.{field}.get(__pane)).map(|__pane| __pane.__name())")
                            } else {
                                format!("{state}.{field}.maximized().and_then(|__pane| {state}.{field}.get(__pane)).map(|__name| (*__name).to_owned())")
                            },
                            ResolvedPaneOperation::Adjacent { pane: name, edge: side } => {
                                let direction = match side {
                                    PaneEdge::Top => "Up",
                                    PaneEdge::Left => "Left",
                                    PaneEdge::Right => "Right",
                                    PaneEdge::Bottom => "Down",
                                };
                                let value = pane(name)?;
                                if *dynamic {
                                    format!("{value}.and_then(|__pane| {state}.{field}.adjacent(__pane, ::iced::widget::pane_grid::Direction::{direction})).and_then(|__pane| {state}.{field}.get(__pane)).map(|__pane| __pane.__name())")
                                } else {
                                    format!("{value}.and_then(|__pane| {state}.{field}.adjacent(__pane, ::iced::widget::pane_grid::Direction::{direction})).and_then(|__pane| {state}.{field}.get(__pane)).map(|__name| (*__name).to_owned())")
                                }
                            }
                            _ => unreachable!(),
                        };
                        let route = route.as_ref().expect("checker requires pane query route");
                        let message_code = resolved_route_code(route, &["value"], env, program, message)?;
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
            ResolvedStatementKind::WindowOperation {
                operation,
                target,
                route,
                ..
            } => {
                let target = target
                    .as_ref()
                    .map(|target| checked_expr_use_code(program, *target, env, ValueMode::Owned))
                    .transpose()?;
                let id = target.as_deref().unwrap_or("__window");
                let value = |value: CheckedExprUseId, cast: &str| {
                    Ok::<_, Error>(format!(
                        "({}) as {cast}",
                        checked_expr_use_code(program, value, env, ValueMode::Owned)?
                    ))
                };
                let size = |width: CheckedExprUseId, height: CheckedExprUseId| {
                    let positive = |value| {
                        Ok::<_, Error>(format!(
                            "(({}) as f32).max(f32::EPSILON).min(f32::MAX)",
                            checked_expr_use_code(program, value, env, ValueMode::Owned)?
                        ))
                    };
                    Ok::<_, Error>(format!(
                        "::iced::Size::new({}, {})",
                        positive(width)?,
                        positive(height)?
                    ))
                };
                let optional_size = |size_value: &Option<(CheckedExprUseId, CheckedExprUseId)>| {
                    Ok::<_, Error>(match size_value {
                        Some((width, height)) => {
                            format!("::std::option::Option::Some({})", size(*width, *height)?)
                        }
                        None => "::std::option::Option::None".into(),
                    })
                };
                let bool_value = |value: CheckedExprUseId| {
                    checked_expr_use_code(program, value, env, ValueMode::Owned)
                };
                let task = match operation {
                    ResolvedWindowOperation::Open(index) => {
                        let settings = index.map_or_else(
                            || "::std::default::Default::default()".into(),
                            |index| format!("Self::__window_{index}()"),
                        );
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!(
                            "{{ let (_, __task) = ::iced::window::open({settings}); __task.map(move |value| {message_code}) }}"
                        )
                    }
                    ResolvedWindowOperation::Oldest | ResolvedWindowOperation::Latest => {
                        let function = if matches!(operation, ResolvedWindowOperation::Oldest) {
                            "oldest"
                        } else {
                            "latest"
                        };
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!("::iced::window::{function}().map(move |value| {message_code})")
                    }
                    ResolvedWindowOperation::Close => {
                        format!("::iced::window::close::<{message}>({id})")
                    }
                    ResolvedWindowOperation::Drag => {
                        format!("::iced::window::drag::<{message}>({id})")
                    }
                    ResolvedWindowOperation::DragResize(direction) => {
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
                    ResolvedWindowOperation::Resize(width, height) => format!(
                        "::iced::window::resize::<{message}>({id}, {})",
                        size(*width, *height)?
                    ),
                    ResolvedWindowOperation::Resizable(enabled) => format!(
                        "::iced::window::set_resizable::<{message}>({id}, {})",
                        bool_value(*enabled)?
                    ),
                    ResolvedWindowOperation::MinSize(size) => format!(
                        "::iced::window::set_min_size::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    ResolvedWindowOperation::MaxSize(size) => format!(
                        "::iced::window::set_max_size::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    ResolvedWindowOperation::ResizeIncrements(size) => format!(
                        "::iced::window::set_resize_increments::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    ResolvedWindowOperation::Size => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = resolved_route_code(
                            route,
                            &["value.width as f64", "value.height as f64"],
                            env,
                            program,
                            message,
                        )?;
                        format!("::iced::window::size({id}).map(move |value| {message_code})")
                    }
                    ResolvedWindowOperation::IsMaximized => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!(
                            "::iced::window::is_maximized({id}).map(move |value| {message_code})"
                        )
                    }
                    ResolvedWindowOperation::Maximize(enabled) => format!(
                        "::iced::window::maximize::<{message}>({id}, {})",
                        bool_value(*enabled)?
                    ),
                    ResolvedWindowOperation::IsMinimized => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!(
                            "::iced::window::is_minimized({id}).map(move |value| {message_code})"
                        )
                    }
                    ResolvedWindowOperation::Minimize(enabled) => format!(
                        "::iced::window::minimize::<{message}>({id}, {})",
                        bool_value(*enabled)?
                    ),
                    ResolvedWindowOperation::Position => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["x", "y"], env, program, message)?;
                        format!(
                            "::iced::window::position({id}).map(move |value| {{ let (x, y) = value.map_or((::std::option::Option::None, ::std::option::Option::None), |value| (::std::option::Option::Some(value.x as f64), ::std::option::Option::Some(value.y as f64))); {message_code} }})"
                        )
                    }
                    ResolvedWindowOperation::ScaleFactor => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["value as f64"], env, program, message)?;
                        format!(
                            "::iced::window::scale_factor({id}).map(move |value| {message_code})"
                        )
                    }
                    ResolvedWindowOperation::Move(x, y) => format!(
                        "::iced::window::move_to::<{message}>({id}, ::iced::Point::new({}, {}))",
                        value(*x, "f32")?,
                        value(*y, "f32")?
                    ),
                    ResolvedWindowOperation::Mode => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!(
                            "::iced::window::mode({id}).map(move |value| {{ let value = match value {{ ::iced::window::Mode::Windowed => \"windowed\", ::iced::window::Mode::Fullscreen => \"fullscreen\", ::iced::window::Mode::Hidden => \"hidden\" }}.to_owned(); {message_code} }})"
                        )
                    }
                    ResolvedWindowOperation::SetMode(mode) => {
                        let mode = match mode {
                            WindowMode::Windowed => "Windowed",
                            WindowMode::Fullscreen => "Fullscreen",
                            WindowMode::Hidden => "Hidden",
                        };
                        format!(
                            "::iced::window::set_mode::<{message}>({id}, ::iced::window::Mode::{mode})"
                        )
                    }
                    ResolvedWindowOperation::ToggleMaximize => {
                        format!("::iced::window::toggle_maximize::<{message}>({id})")
                    }
                    ResolvedWindowOperation::ToggleDecorations => {
                        format!("::iced::window::toggle_decorations::<{message}>({id})")
                    }
                    ResolvedWindowOperation::Attention(attention) => {
                        let attention: String = match attention {
                            None => "::std::option::Option::None".into(),
                            Some(WindowAttention::Critical) => "::std::option::Option::Some(::iced::window::UserAttention::Critical)".into(),
                            Some(WindowAttention::Informational) => "::std::option::Option::Some(::iced::window::UserAttention::Informational)".into(),
                        };
                        format!(
                            "::iced::window::request_user_attention::<{message}>({id}, {attention})"
                        )
                    }
                    ResolvedWindowOperation::Focus => {
                        format!("::iced::window::gain_focus::<{message}>({id})")
                    }
                    ResolvedWindowOperation::SetLevel(level) => {
                        let level = match level {
                            WindowLevel::Normal => "Normal",
                            WindowLevel::AlwaysOnBottom => "AlwaysOnBottom",
                            WindowLevel::AlwaysOnTop => "AlwaysOnTop",
                        };
                        format!(
                            "::iced::window::set_level::<{message}>({id}, ::iced::window::Level::{level})"
                        )
                    }
                    ResolvedWindowOperation::SystemMenu => {
                        format!("::iced::window::show_system_menu::<{message}>({id})")
                    }
                    ResolvedWindowOperation::RawId => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = resolved_route_code(
                            route,
                            &["value.to_string()"],
                            env,
                            program,
                            message,
                        )?;
                        format!(
                            "::iced::window::raw_id::<{message}>({id}).map(move |value| {message_code})"
                        )
                    }
                    ResolvedWindowOperation::Screenshot => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!("::iced::window::screenshot({id}).map(move |value| {message_code})")
                    }
                    ResolvedWindowOperation::MousePassthrough(enabled) => {
                        let enabled = bool_value(*enabled)?;
                        format!(
                            "if {enabled} {{ ::iced::window::enable_mouse_passthrough::<{message}>({id}) }} else {{ ::iced::window::disable_mouse_passthrough::<{message}>({id}) }}"
                        )
                    }
                    ResolvedWindowOperation::MonitorSize => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = resolved_route_code(
                            route,
                            &["width", "height"],
                            env,
                            program,
                            message,
                        )?;
                        format!(
                            "::iced::window::monitor_size({id}).map(move |value| {{ let (width, height) = value.map_or((::std::option::Option::None, ::std::option::Option::None), |value| (::std::option::Option::Some(value.width as f64), ::std::option::Option::Some(value.height as f64))); {message_code} }})"
                        )
                    }
                    ResolvedWindowOperation::AutomaticTabbing(enabled) => format!(
                        "::iced::window::allow_automatic_tabbing::<{message}>({})",
                        bool_value(*enabled)?
                    ),
                    ResolvedWindowOperation::Icon {
                        pixels,
                        width,
                        height,
                    } => {
                        let pixels =
                            checked_expr_use_code(program, *pixels, env, ValueMode::Owned)?;
                        let width = checked_expr_use_code(program, *width, env, ValueMode::Owned)?;
                        let height =
                            checked_expr_use_code(program, *height, env, ValueMode::Owned)?;
                        format!(
                            "{{ let __pixels = {pixels}; let __width = {width}; let __height = {height}; match (::std::primitive::u32::try_from(__width), ::std::primitive::u32::try_from(__height)) {{ (::std::result::Result::Ok(__width), ::std::result::Result::Ok(__height)) if __width > 0 && __height > 0 && __width.checked_mul(__height).is_some() => ::iced::window::icon::from_rgba(__pixels, __width, __height).map_or_else(|_| ::iced::Task::none(), |__icon| ::iced::window::set_icon::<{message}>({id}, __icon)), _ => ::iced::Task::none(), }} }}"
                        )
                    }
                    ResolvedWindowOperation::Callback {
                        target: callback,
                        args,
                    } => {
                        let callback = program.extern_function(*callback);
                        let args = args
                            .iter()
                            .map(|arg| {
                                checked_expr_use_code(program, *arg, env, ValueMode::Owned)
                                    .map(|arg| format!(", {arg}"))
                            })
                            .collect::<Result<String, _>>()?;
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            resolved_route_code(route, &["value"], env, program, message)?;
                        format!(
                            "::iced::window::run({id}, move |__window| {}(__window{args})).map(move |value| {message_code})",
                            callback.rust_path
                        )
                    }
                };
                let task = if target.is_some()
                    || matches!(
                        operation,
                        ResolvedWindowOperation::Open(_)
                            | ResolvedWindowOperation::Oldest
                            | ResolvedWindowOperation::Latest
                            | ResolvedWindowOperation::AutomaticTabbing(_)
                    ) {
                    task
                } else {
                    format!("::iced::window::oldest().and_then(move |__window| {task})")
                };
                writeln!(out, "{}{task}{}", task_prefix, task_suffix).unwrap();
            }
        }
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
    Ok(has_task)
}

mod task;
mod view_fn;

pub(super) use task::*;
pub(super) use view_fn::*;

use super::*;

pub(in crate::check) fn check_handler(
    handler: &Handler,
    states: &HashMap<String, Type>,
    readables: &HashMap<String, Type>,
    document: &Document,
    operation_ids: &[WidgetIdPath],
    pane_grids: &HashMap<String, PaneGridNames>,
) -> Result<(), Error> {
    let mut env = readables.clone();
    env.extend(
        handler
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone())),
    );
    for (index, statement) in handler.statements.iter().enumerate() {
        match statement {
            Statement::Let { name, value, span } => {
                if env.contains_key(name) {
                    return Err(Error::new(
                        "E140",
                        span,
                        format!("local binding `{name}` shadows an existing value"),
                    ));
                }
                let ty = expr_type(value, &env, document, span)?;
                if ty == Type::Unknown {
                    return Err(Error::new(
                        "E140",
                        span,
                        format!("cannot infer type of local binding `{name}`"),
                    ));
                }
                env.insert(name.clone(), ty);
            }
            Statement::Assign {
                target,
                value,
                at,
                span,
            } => {
                let expected = states.get(target).ok_or_else(|| {
                    Error::new("E140", span, format!("`{target}` is not writable state"))
                })?;
                if contains_debug_span(expected) {
                    return Err(Error::new(
                        "E144",
                        span,
                        "debug span state is owned by `debug start` and `debug finish`",
                    ));
                }
                let actual = expr_type(value, &env, document, span)?;
                if let Type::Combo(inner) = expected {
                    require_type(&actual, &Type::List(inner.clone()), span)?;
                } else if let Type::Animation(inner) = expected {
                    require_type(&actual, inner, span)?;
                    if **inner == Type::F64 {
                        require_f32_literal_range(
                            value,
                            f64::NEG_INFINITY,
                            None,
                            "animation value",
                            span,
                        )?;
                    }
                } else {
                    require_type(&actual, expected, span)?;
                }
                if let Some(at) = at {
                    if !matches!(expected, Type::Animation(_)) {
                        return Err(Error::new(
                            "E140",
                            span,
                            "`at` is only valid when assigning animation state",
                        ));
                    }
                    require_type(&expr_type(at, &env, document, span)?, &Type::Instant, span)?;
                }
            }
            Statement::MarkdownAppend {
                target,
                value,
                span,
            } => {
                let expected = states.get(target).ok_or_else(|| {
                    Error::new("E140", span, format!("unknown markdown state `{target}`"))
                })?;
                require_type(expected, &Type::Markdown, span)?;
                require_type(&expr_type(value, &env, document, span)?, &Type::Str, span)?;
            }
            Statement::ComboPush {
                target,
                value,
                span,
            } => {
                let actual = states.get(target).ok_or_else(|| {
                    Error::new("E140", span, format!("unknown combo state `{target}`"))
                })?;
                let Type::Combo(expected) = actual else {
                    return Err(Error::new(
                        "E140",
                        span,
                        format!("`{target}` is not combo state"),
                    ));
                };
                require_type(&expr_type(value, &env, document, span)?, expected, span)?;
            }
            Statement::ReturnIf { condition, span } => {
                require_type(
                    &expr_type(condition, &env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            Statement::Exit { span } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "exit must be the final statement in a handler",
                    ));
                }
            }
            Statement::Run {
                kind,
                function,
                args,
                span,
                ..
            } => {
                if index + 1 != handler.statements.len() {
                    let effect = match kind {
                        EffectKind::Future => "run",
                        EffectKind::Task => "task",
                        EffectKind::Stream => "stream",
                    };
                    return Err(Error::new(
                        "E141",
                        span,
                        format!("{effect} must be the final statement in a handler"),
                    ));
                }
                if builtin_task_type(*kind, function, args, span)?.is_some() {
                    if function == "__ice_font_load" {
                        require_type(
                            &expr_type(&args[0], &env, document, span)?,
                            &Type::Bytes,
                            span,
                        )?;
                    } else if function == "__ice_image_allocate" {
                        require_type(
                            &expr_type(&args[0], &env, document, span)?,
                            &Type::Image,
                            span,
                        )?;
                    }
                    continue;
                }
                let action = extern_function(document, function, (*kind).into(), span)?;
                check_call_args(action, args, &env, document, span)?;
            }
            Statement::Sip {
                function,
                args,
                span,
                ..
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "sip must be the final statement in a handler",
                    ));
                }
                let action = extern_function(document, function, ExternKind::Sip, span)?;
                check_call_args(action, args, &env, document, span)?;
            }
            Statement::TaskFlow {
                source,
                transforms,
                span,
                ..
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "flow must be the final statement in a handler",
                    ));
                }
                task_flow_type(source, transforms, document, &env)?;
            }
            Statement::TaskGroup { statements, .. } => {
                for statement in statements {
                    check_handler(
                        &Handler {
                            statements: vec![statement.clone()],
                            ..handler.clone()
                        },
                        states,
                        &env,
                        document,
                        operation_ids,
                        pane_grids,
                    )?;
                }
            }
            Statement::Abortable {
                handle, task, span, ..
            } => {
                require_task_handle_state(handle, states, span)?;
                check_handler(
                    &Handler {
                        statements: vec![(**task).clone()],
                        ..handler.clone()
                    },
                    states,
                    &env,
                    document,
                    operation_ids,
                    pane_grids,
                )?;
            }
            Statement::Abort { handle, span } => {
                require_task_handle_state(handle, states, span)?;
            }
            Statement::DebugStart { name, target, span } => {
                require_debug_span_state(target, states, span)?;
                require_type(&expr_type(name, &env, document, span)?, &Type::Str, span)?;
            }
            Statement::DebugFinish { target, span } => {
                require_debug_span_state(target, states, span)?;
            }
            Statement::ClipboardWrite { value, span, .. } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "clipboard write must be the final statement in a handler",
                    ));
                }
                require_type(&expr_type(value, &env, document, span)?, &Type::Str, span)?;
            }
            Statement::WidgetOperation {
                operation,
                route,
                span,
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E172",
                        span,
                        "widget operation must be the final statement in a handler",
                    ));
                }
                let target = match operation {
                    WidgetOperation::FocusPrevious
                    | WidgetOperation::FocusNext
                    | WidgetOperation::Find { .. } => None,
                    WidgetOperation::Focus { target }
                    | WidgetOperation::Focused { target }
                    | WidgetOperation::CursorFront { target }
                    | WidgetOperation::CursorEnd { target }
                    | WidgetOperation::Cursor { target, .. }
                    | WidgetOperation::SelectAll { target }
                    | WidgetOperation::Select { target, .. }
                    | WidgetOperation::Snap { target, .. }
                    | WidgetOperation::SnapEnd { target }
                    | WidgetOperation::ScrollTo { target, .. }
                    | WidgetOperation::ScrollBy { target, .. } => Some(target),
                };
                if let Some(target) = target {
                    check_widget_target(target, &env, document, operation_ids, span)?;
                }
                if let WidgetOperation::Find { selector, .. } = operation {
                    check_widget_selector(selector, &env, document, operation_ids, span)?;
                }
                match (operation, route) {
                    (WidgetOperation::Focused { .. }, None) => {
                        return Err(Error::new(
                            "E172",
                            span,
                            "widget focused requires `-> handler _`",
                        ));
                    }
                    (WidgetOperation::Focused { .. }, Some(_)) => {}
                    (WidgetOperation::Find { .. }, None) => {
                        return Err(Error::new(
                            "E172",
                            span,
                            "widget selector requires `-> handler _`",
                        ));
                    }
                    (WidgetOperation::Find { .. }, Some(_)) => {}
                    (_, Some(_)) => {
                        return Err(Error::new(
                            "E172",
                            span,
                            "widget effects do not produce a route",
                        ));
                    }
                    (_, None) => {}
                }
                for value in match operation {
                    WidgetOperation::Cursor { position, .. } => vec![(position, "cursor position")],
                    WidgetOperation::Select { start, end, .. } => {
                        vec![(start, "selection start"), (end, "selection end")]
                    }
                    _ => Vec::new(),
                } {
                    require_type(&expr_type(value.0, &env, document, span)?, &Type::I64, span)?;
                    if matches!(value.0, Expr::I64(number) if *number < 0) {
                        return Err(Error::new(
                            "E172",
                            span,
                            format!("{} cannot be negative", value.1),
                        ));
                    }
                }
                if let WidgetOperation::Select {
                    start: Expr::I64(start),
                    end: Expr::I64(end),
                    ..
                } = operation
                    && start > end
                {
                    return Err(Error::new(
                        "E172",
                        span,
                        "selection start cannot exceed end",
                    ));
                }
                for (value, relative) in match operation {
                    WidgetOperation::Snap { x, y, .. } => vec![(x, true), (y, true)],
                    WidgetOperation::ScrollTo { x, y, .. }
                    | WidgetOperation::ScrollBy { x, y, .. } => vec![(x, false), (y, false)],
                    _ => Vec::new(),
                } {
                    require_f32_value(value, &env, document, "scroll offset", span)?;
                    if relative {
                        require_literal_range(
                            value,
                            0.0,
                            Some(1.0),
                            "relative scroll offset",
                            span,
                        )?;
                    }
                }
            }
            Statement::PaneOperation {
                grid,
                operation,
                route,
                span,
            } => {
                let names = pane_grids
                    .get(grid)
                    .ok_or_else(|| Error::new("E188", span, format!("unknown panes `#{grid}`")))?;
                let referenced = match operation {
                    PaneOperation::Maximize { pane }
                    | PaneOperation::Adjacent { pane, .. }
                    | PaneOperation::Close { pane }
                    | PaneOperation::Move { pane, .. } => vec![pane],
                    PaneOperation::Swap { first, second } => vec![first, second],
                    PaneOperation::Drop { pane, target, .. } => vec![pane, target],
                    PaneOperation::Split { target, pane, .. } => vec![target, pane],
                    PaneOperation::Restore
                    | PaneOperation::Maximized
                    | PaneOperation::Resize { .. } => Vec::new(),
                };
                for pane in referenced {
                    match pane {
                        PaneReference::Static(pane) => {
                            if !names.panes.contains(pane) {
                                return Err(Error::new(
                                    "E188",
                                    span,
                                    format!("panes `#{grid}` has no pane `{pane}`"),
                                ));
                            }
                        }
                        PaneReference::Dynamic { template, key } => {
                            let expected = names.templates.get(template).ok_or_else(|| {
                                Error::new(
                                    "E188",
                                    span,
                                    format!(
                                        "panes `#{grid}` has no dynamic pane template `{template}`"
                                    ),
                                )
                            })?;
                            require_type(&expr_type(key, &env, document, span)?, expected, span)?;
                        }
                    }
                }
                if let PaneOperation::Resize {
                    split: Some(split), ..
                } = operation
                    && !names.splits.contains(split)
                {
                    return Err(Error::new(
                        "E188",
                        span,
                        format!("panes `#{grid}` has no split `{split}`"),
                    ));
                }
                let same_static = |first: &PaneReference, second: &PaneReference| matches!((first, second), (PaneReference::Static(first), PaneReference::Static(second)) if first == second);
                if matches!(
                    operation,
                    PaneOperation::Swap { first, second } if same_static(first, second)
                ) || matches!(
                    operation,
                    PaneOperation::Drop { pane, target, .. } if same_static(pane, target)
                ) || matches!(
                    operation,
                    PaneOperation::Split { target, pane, .. } if same_static(target, pane)
                ) {
                    return Err(Error::new(
                        "E188",
                        span,
                        "pane operation requires two different panes",
                    ));
                }
                let query = matches!(
                    operation,
                    PaneOperation::Maximized | PaneOperation::Adjacent { .. }
                );
                if query && index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E188",
                        span,
                        "pane query must be the final statement in a handler",
                    ));
                }
                match (query, route) {
                    (true, None) => {
                        return Err(Error::new("E188", span, "pane query requires a route"));
                    }
                    (false, Some(_)) => {
                        return Err(Error::new(
                            "E188",
                            span,
                            "pane effects do not produce a route",
                        ));
                    }
                    _ => {}
                }
                if let PaneOperation::Resize { ratio, .. } | PaneOperation::Split { ratio, .. } =
                    operation
                {
                    require_type(&expr_type(ratio, &env, document, span)?, &Type::F64, span)?;
                    require_literal_range(ratio, 0.0, Some(1.0), "pane split ratio", span)?;
                }
            }
            Statement::WindowOperation {
                operation,
                target,
                route,
                span,
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E173",
                        span,
                        "window task must be the final statement in a handler",
                    ));
                }
                let query = matches!(
                    operation,
                    WindowOperation::Open(_)
                        | WindowOperation::Oldest
                        | WindowOperation::Latest
                        | WindowOperation::Size
                        | WindowOperation::IsMaximized
                        | WindowOperation::IsMinimized
                        | WindowOperation::Position
                        | WindowOperation::ScaleFactor
                        | WindowOperation::Mode
                        | WindowOperation::RawId
                        | WindowOperation::Screenshot
                        | WindowOperation::MonitorSize
                        | WindowOperation::Callback { .. }
                );
                if let WindowOperation::Open(Some(name)) = operation
                    && !document
                        .settings
                        .windows
                        .iter()
                        .any(|window| window.name == *name)
                {
                    return Err(Error::new(
                        "E173",
                        span,
                        format!("unknown app window `{name}`"),
                    ));
                }
                if let Some(target) = target {
                    if matches!(
                        operation,
                        WindowOperation::Open(_)
                            | WindowOperation::Oldest
                            | WindowOperation::Latest
                            | WindowOperation::AutomaticTabbing(_)
                    ) {
                        return Err(Error::new(
                            "E173",
                            span,
                            "this window task does not accept `target=`",
                        ));
                    }
                    require_type(
                        &expr_type(target, &env, document, span)?,
                        &Type::WindowId,
                        span,
                    )?;
                }
                match (query, route) {
                    (true, None) => {
                        return Err(Error::new("E173", span, "window query requires a route"));
                    }
                    (false, Some(_)) => {
                        return Err(Error::new(
                            "E173",
                            span,
                            "window effects do not produce a route",
                        ));
                    }
                    _ => {}
                }
                for value in match operation {
                    WindowOperation::Resizable(value)
                    | WindowOperation::Maximize(value)
                    | WindowOperation::Minimize(value)
                    | WindowOperation::MousePassthrough(value)
                    | WindowOperation::AutomaticTabbing(value) => vec![value],
                    _ => Vec::new(),
                } {
                    require_type(&expr_type(value, &env, document, span)?, &Type::Bool, span)?;
                }
                for value in match operation {
                    WindowOperation::Resize(width, height) => vec![width, height],
                    WindowOperation::MinSize(Some((width, height)))
                    | WindowOperation::MaxSize(Some((width, height)))
                    | WindowOperation::ResizeIncrements(Some((width, height))) => {
                        vec![width, height]
                    }
                    _ => Vec::new(),
                } {
                    require_type(&expr_type(value, &env, document, span)?, &Type::F64, span)?;
                    require_f32_literal_range(value, f64::EPSILON, None, "window size", span)?;
                }
                if let WindowOperation::Move(x, y) = operation {
                    require_f32_value(x, &env, document, "window position", span)?;
                    require_f32_value(y, &env, document, "window position", span)?;
                }
                if let WindowOperation::Icon {
                    pixels,
                    width,
                    height,
                } = operation
                {
                    require_type(
                        &expr_type(pixels, &env, document, span)?,
                        &Type::Bytes,
                        span,
                    )?;
                    for (dimension, label) in
                        [(width, "window icon width"), (height, "window icon height")]
                    {
                        require_type(
                            &expr_type(dimension, &env, document, span)?,
                            &Type::I64,
                            span,
                        )?;
                        require_literal_range(dimension, 1.0, Some(u32::MAX as f64), label, span)?;
                    }
                    if let (Expr::I64(width), Expr::I64(height)) = (width, height)
                        && (*width as u128) * (*height as u128) > u32::MAX as u128
                    {
                        return Err(Error::new(
                            "E173",
                            span,
                            "window icon dimensions are too large",
                        ));
                    }
                    if let (Expr::Bytes(pixels), Expr::I64(width), Expr::I64(height)) =
                        (pixels, width, height)
                    {
                        let expected = (*width as u128) * (*height as u128) * 4;
                        if expected != pixels.len() as u128 {
                            return Err(Error::new(
                                "E173",
                                span,
                                "window icon pixels must contain width × height × 4 bytes",
                            ));
                        }
                    }
                }
                if let WindowOperation::Callback { function, args } = operation {
                    let callback = extern_function(document, function, ExternKind::Window, span)?;
                    check_call_args(callback, args, &env, document, span)?;
                }
            }
        }
    }
    Ok(())
}

pub(in crate::check) fn check_structured_tasks(handler: &Handler) -> Result<(), Error> {
    for (index, statement) in handler.statements.iter().enumerate() {
        match statement {
            Statement::TaskGroup {
                statements, span, ..
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "task group must be the final statement in a handler",
                    ));
                }
                if let Some(span) = statements.iter().find_map(invalid_task_producer) {
                    return Err(Error::new(
                        "E143",
                        span,
                        "task groups only accept task-producing statements",
                    ));
                }
            }
            Statement::Abortable { task, span, .. } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "abortable task must be the final statement in a handler",
                    ));
                }
                if let Some(span) = invalid_task_producer(task) {
                    return Err(Error::new(
                        "E143",
                        span,
                        "abortable requires a task-producing statement",
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub(in crate::check) fn invalid_task_producer(statement: &Statement) -> Option<&Span> {
    match statement {
        Statement::Exit { .. }
        | Statement::Run { .. }
        | Statement::Sip { .. }
        | Statement::TaskFlow { .. }
        | Statement::ClipboardWrite { .. }
        | Statement::WidgetOperation { .. }
        | Statement::WindowOperation { .. }
        | Statement::PaneOperation {
            operation: PaneOperation::Maximized | PaneOperation::Adjacent { .. },
            ..
        } => None,
        Statement::Abortable { task, .. } => invalid_task_producer(task),
        Statement::TaskGroup { statements, .. } => {
            statements.iter().find_map(invalid_task_producer)
        }
        Statement::Assign { .. }
        | Statement::Let { .. }
        | Statement::MarkdownAppend { .. }
        | Statement::ComboPush { .. }
        | Statement::ReturnIf { .. }
        | Statement::Abort { .. }
        | Statement::DebugStart { .. }
        | Statement::DebugFinish { .. }
        | Statement::PaneOperation { .. } => Some(statement_span(statement)),
    }
}

pub(in crate::check) fn require_task_handle_state(
    handle: &str,
    states: &HashMap<String, Type>,
    span: &Span,
) -> Result<(), Error> {
    let Some(actual) = states.get(handle) else {
        return Err(Error::new(
            "E143",
            span,
            format!("unknown task handle state `{handle}`"),
        )
        .hint(format!("declare `{handle}:task-handle? = none` in state")));
    };
    require_type(actual, &Type::Option(Box::new(Type::TaskHandle)), span)
}

pub(in crate::check) fn require_debug_span_state(
    target: &str,
    states: &HashMap<String, Type>,
    span: &Span,
) -> Result<(), Error> {
    let Some(actual) = states.get(target) else {
        return Err(
            Error::new("E144", span, format!("unknown debug span state `{target}`"))
                .hint(format!("declare `{target}:debug-span? = none` in state")),
        );
    };
    require_type(actual, &Type::Option(Box::new(Type::DebugSpan)), span)
}

pub(in crate::check) fn statement_span(statement: &Statement) -> &Span {
    match statement {
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::MarkdownAppend { span, .. }
        | Statement::ComboPush { span, .. }
        | Statement::ReturnIf { span, .. }
        | Statement::Exit { span }
        | Statement::Run { span, .. }
        | Statement::Sip { span, .. }
        | Statement::TaskFlow { span, .. }
        | Statement::TaskGroup { span, .. }
        | Statement::Abortable { span, .. }
        | Statement::Abort { span, .. }
        | Statement::DebugStart { span, .. }
        | Statement::DebugFinish { span, .. }
        | Statement::ClipboardWrite { span, .. }
        | Statement::WidgetOperation { span, .. }
        | Statement::WindowOperation { span, .. }
        | Statement::PaneOperation { span, .. } => span,
    }
}

impl From<EffectKind> for ExternKind {
    fn from(value: EffectKind) -> Self {
        match value {
            EffectKind::Future => Self::Future,
            EffectKind::Task => Self::Task,
            EffectKind::Stream => Self::Stream,
        }
    }
}

pub(in crate::check) fn extern_function<'a>(
    document: &'a Document,
    name: &str,
    kind: ExternKind,
    span: &Span,
) -> Result<&'a ExternFn, Error> {
    document
        .functions
        .iter()
        .find(|item| item.name == name && item.kind == kind)
        .ok_or_else(|| {
            let label = match kind {
                ExternKind::Future => "function",
                ExternKind::Component => "component",
                ExternKind::Shader => "shader",
                ExternKind::Task => "task",
                ExternKind::Stream => "stream",
                ExternKind::Sip => "sip",
                ExternKind::Recipe => "recipe",
                ExternKind::Selector => "selector",
                ExternKind::EventFilter => "event filter",
                ExternKind::Sync => "sync function",
                ExternKind::Subscription => "subscription",
                ExternKind::Theme => "theme factory",
                ExternKind::Themer => "themer",
                ExternKind::Window => "window callback",
                ExternKind::MarkdownViewer => "markdown viewer",
                ExternKind::EditorBinding => "editor binding",
                ExternKind::EditorHighlighter => "editor highlighter",
                ExternKind::EditorStyle => "editor style",
                ExternKind::TextStyle => "text style",
                ExternKind::SliderStyle => "slider style",
                ExternKind::ProgressStyle => "progress style",
                ExternKind::ButtonStyle => "button style",
                ExternKind::CheckboxStyle => "checkbox style",
                ExternKind::TogglerStyle => "toggler style",
                ExternKind::RadioStyle => "radio style",
                ExternKind::ContainerStyle => "box style",
                ExternKind::SvgStyle => "svg style",
                ExternKind::InputStyle => "input style",
                ExternKind::ScrollStyle => "scroll style",
                ExternKind::PickListStyle => "pick-list style",
                ExternKind::MenuStyle => "menu style",
                ExternKind::PaneGridStyle => "panes style",
            };
            Error::new("E130", span, format!("unknown extern {label} `{name}`"))
        })
}

pub(in crate::check) fn check_call_args(
    function: &ExternFn,
    args: &[Expr],
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if args.len() != function.params.len() {
        return Err(Error::new(
            "E142",
            span,
            format!(
                "extern `{}` expects {} arguments, got {}",
                function.name,
                function.params.len(),
                args.len()
            ),
        ));
    }
    for (arg, (_, expected)) in args.iter().zip(&function.params) {
        let actual = expr_type(arg, env, document, span)?;
        require_type(&actual, expected, span)?;
    }
    Ok(())
}

pub(in crate::check) fn builtin_task_type(
    kind: EffectKind,
    function: &str,
    args: &[Expr],
    span: &Span,
) -> Result<Option<(Type, Option<Type>)>, Error> {
    let output = match (kind, function) {
        (EffectKind::Task, "__ice_system_info") => Some((Type::SystemInfo, None)),
        (EffectKind::Task, "__ice_system_theme") => Some((Type::Str, None)),
        (EffectKind::Task, "__ice_time_now") => Some((Type::Instant, None)),
        (EffectKind::Task, "__ice_clipboard_read" | "__ice_clipboard_read_primary") => {
            Some((Type::Option(Box::new(Type::Str)), None))
        }
        (EffectKind::Task, "__ice_font_load") => Some((Type::Unit, None)),
        (EffectKind::Task, "__ice_image_allocate") => {
            Some((Type::ImageAllocation, Some(Type::ImageError)))
        }
        _ => None,
    };
    if matches!(function, "__ice_font_load" | "__ice_image_allocate") && args.len() != 1 {
        return Err(Error::new(
            "E142",
            span,
            "this built-in task expects one argument",
        ));
    }
    if output.is_some()
        && !matches!(function, "__ice_font_load" | "__ice_image_allocate")
        && !args.is_empty()
    {
        return Err(Error::new(
            "E142",
            span,
            "this built-in task takes no arguments",
        ));
    }
    Ok(output)
}

pub(in crate::check) fn task_source_type(
    source: &TaskSource,
    document: &Document,
    env: &HashMap<String, Type>,
) -> Result<(Type, Option<Type>), Error> {
    match source {
        TaskSource::Done { value, span } => Ok((expr_type(value, env, document, span)?, None)),
        TaskSource::None { output, span } => {
            let known = document
                .structs
                .iter()
                .map(|item| item.name.as_str())
                .collect::<HashSet<_>>();
            check_declared_type(output, span, &known)?;
            Ok((output.clone(), None))
        }
        TaskSource::Effect {
            kind,
            function,
            args,
            span,
        } => {
            if let Some((output, error)) = builtin_task_type(*kind, function, args, span)? {
                if function == "__ice_font_load" {
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Bytes,
                        span,
                    )?;
                } else if function == "__ice_image_allocate" {
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Image,
                        span,
                    )?;
                }
                return Ok((output, error));
            }
            let action = extern_function(document, function, (*kind).into(), span)?;
            check_call_args(action, args, env, document, span)?;
            Ok((action.output.clone(), action.error.clone()))
        }
    }
}

pub(crate) fn task_flow_type(
    source: &TaskSource,
    transforms: &[TaskTransform],
    document: &Document,
    root_env: &HashMap<String, Type>,
) -> Result<(Option<Type>, Option<Type>), Error> {
    let (mut output, mut error_ty) = task_source_type(source, document, root_env)?;
    for (index, transform) in transforms.iter().enumerate() {
        match transform {
            TaskTransform::Map {
                binding,
                value,
                span,
            } => {
                let env = HashMap::from([(binding.clone(), output)]);
                output = expr_type(value, &env, document, span).map_err(|error| {
                    if error.code == "E150" {
                        error.hint(format!("map may only read its `{binding}` binding"))
                    } else {
                        error
                    }
                })?;
            }
            TaskTransform::Then {
                binding,
                source,
                span,
            } => {
                if error_ty.is_some() {
                    return Err(Error::new(
                        "E144",
                        span,
                        "then cannot unwrap a fallible task; use try",
                    ));
                }
                let env = HashMap::from([(binding.clone(), output)]);
                let next = task_source_type(source, document, &env).map_err(|error| {
                    if error.code == "E150" {
                        error.hint(format!(
                            "a flow transform may only read its `{binding}` binding"
                        ))
                    } else {
                        error
                    }
                })?;
                output = next.0;
                error_ty = next.1;
            }
            TaskTransform::AndThen {
                binding,
                source,
                span,
            } => {
                let (binding_ty, result_error) = if let Some(error) = &error_ty {
                    (output.clone(), Some(error.clone()))
                } else if let Type::Option(inner) = &output {
                    ((**inner).clone(), None)
                } else {
                    return Err(Error::new(
                        "E144",
                        span,
                        "try requires an optional or fallible task output",
                    ));
                };
                let env = HashMap::from([(binding.clone(), binding_ty)]);
                let next = task_source_type(source, document, &env).map_err(|error| {
                    if error.code == "E150" {
                        error.hint(format!(
                            "a flow transform may only read its `{binding}` binding"
                        ))
                    } else {
                        error
                    }
                })?;
                if let Some(expected) = result_error {
                    let Some(actual) = &next.1 else {
                        return Err(Error::new(
                            "E144",
                            span,
                            "try after a fallible task must return a fallible task",
                        ));
                    };
                    require_type(actual, &expected, span)?;
                    error_ty = Some(expected);
                } else {
                    error_ty = next.1;
                }
                output = next.0;
            }
            TaskTransform::MapError {
                binding,
                value,
                span,
            } => {
                let Some(error) = error_ty.take() else {
                    return Err(Error::new("E144", span, "map-err requires a fallible flow"));
                };
                let env = HashMap::from([(binding.clone(), error)]);
                let mapped = expr_type(value, &env, document, span).map_err(|error| {
                    if error.code == "E150" {
                        error.hint(format!("map-err may only read its `{binding}` binding"))
                    } else {
                        error
                    }
                })?;
                error_ty = Some(mapped);
            }
            TaskTransform::Collect { .. } => {
                let item = match error_ty.take() {
                    Some(error) => Type::Result(Box::new(output), Box::new(error)),
                    None => output,
                };
                output = Type::List(Box::new(item));
            }
            TaskTransform::Discard { span } => {
                if index + 1 != transforms.len() {
                    return Err(Error::new(
                        "E144",
                        span,
                        "discard must be the final flow transform",
                    ));
                }
                return Ok((None, None));
            }
        }
    }
    Ok((Some(output), error_ty))
}

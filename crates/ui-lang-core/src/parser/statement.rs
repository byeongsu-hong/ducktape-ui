use super::*;

pub(in crate::parser) fn parse_statement(line: &Line) -> Result<Statement, Error> {
    let group = match line.text.as_str() {
        "parallel" => Some(TaskGroupKind::Parallel),
        "sequential" => Some(TaskGroupKind::Sequential),
        _ => None,
    };
    if let Some(kind) = group {
        if line.children.is_empty() {
            return Err(error(
                "E050",
                line,
                "task groups require at least one indented task",
            ));
        }
        return Ok(Statement::TaskGroup {
            kind,
            statements: line
                .children
                .iter()
                .map(parse_statement)
                .collect::<Result<_, _>>()?,
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("abortable ") {
        let parts = split_words(source);
        if parts.len() > 2
            || parts.is_empty()
            || parts.get(1).is_some_and(|value| value != "abort-on-drop")
        {
            return Err(error(
                "E050",
                line,
                "abortable uses `abortable handle [abort-on-drop]`",
            ));
        }
        if line.children.len() != 1 {
            return Err(error(
                "E050",
                line,
                "abortable requires exactly one indented task",
            ));
        }
        return Ok(Statement::Abortable {
            handle: identifier(&parts[0], line)?,
            abort_on_drop: parts.len() == 2,
            task: Box::new(parse_statement(&line.children[0])?),
            span: Span::line(line.number),
        });
    }
    if line.text == "abortable" {
        return Err(error("E050", line, "abortable requires a handle state"));
    }
    if line.text == "flow" {
        return parse_task_flow(line);
    }
    if let Some(source) = line.text.strip_prefix("sip ") {
        return parse_sip_statement(source, line);
    }
    if line.text == "sip" {
        return Err(error("E050", line, "sip requires an extern call"));
    }
    ensure_leaf(line)?;
    if line.text == "exit" {
        return Ok(Statement::Exit {
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("let ") {
        let Some((name, value)) = split_top_once(source, '=') else {
            return Err(error(
                "E050",
                line,
                "local bindings use `let name = expression`",
            ));
        };
        return Ok(Statement::Let {
            name: identifier(name.trim(), line)?,
            value: parse_expr(value.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("combo ") {
        let Some((target, value)) = split_top_marker(source, " push ") else {
            return Err(error(
                "E050",
                line,
                "combo mutation uses `combo state push value`",
            ));
        };
        return Ok(Statement::ComboPush {
            target: identifier(target.trim(), line)?,
            value: parse_expr(value.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("markdown ") {
        let Some((target, value)) = split_top_marker(source, " append ") else {
            return Err(error(
                "E050",
                line,
                "markdown mutation uses `markdown state append text`",
            ));
        };
        return Ok(Statement::MarkdownAppend {
            target: identifier(target.trim(), line)?,
            value: parse_expr(value.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(condition) = line.text.strip_prefix("return if ") {
        return Ok(Statement::ReturnIf {
            condition: parse_expr(condition, line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(handle) = line.text.strip_prefix("abort ") {
        return Ok(Statement::Abort {
            handle: identifier(handle.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("debug start ") {
        let Some((name, target)) = split_top_marker(source, "->") else {
            return Err(error(
                "E050",
                line,
                "debug timing starts with `debug start name -> span_state`",
            ));
        };
        return Ok(Statement::DebugStart {
            name: parse_expr(name.trim(), line)?,
            target: identifier(target.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(target) = line.text.strip_prefix("debug finish ") {
        return Ok(Statement::DebugFinish {
            target: identifier(target.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if line.text.starts_with("debug ") {
        return Err(error(
            "E050",
            line,
            "debug timing uses `debug start name -> span_state` or `debug finish span_state`",
        ));
    }
    if let Some(source) = line.text.strip_prefix("pane ") {
        return parse_pane_operation(source, line);
    }
    if let Some(source) = line.text.strip_prefix("task widget ") {
        return parse_widget_operation(source, line);
    }
    if let Some(source) = line.text.strip_prefix("task window ") {
        return parse_window_operation(source, line);
    }
    for (prefix, primary) in [
        ("task clipboard write-primary ", true),
        ("task clipboard write ", false),
    ] {
        if let Some(value) = line.text.strip_prefix(prefix) {
            return Ok(Statement::ClipboardWrite {
                primary,
                value: parse_expr(value, line)?,
                span: Span::line(line.number),
            });
        }
    }
    let effect = line
        .text
        .strip_prefix("run ")
        .map(|source| (EffectKind::Future, source))
        .or_else(|| {
            line.text
                .strip_prefix("task ")
                .map(|source| (EffectKind::Task, source))
        })
        .or_else(|| {
            line.text
                .strip_prefix("stream ")
                .map(|source| (EffectKind::Stream, source))
        });
    if let Some((kind, run)) = effect {
        let (mode, run) = if kind == EffectKind::Future {
            run.strip_prefix("latest ")
                .map(|run| (FutureMode::Latest, run))
                .or_else(|| {
                    run.strip_prefix("replace ")
                        .map(|run| (FutureMode::Replace, run))
                })
                .unwrap_or((FutureMode::Every, run))
        } else {
            (FutureMode::Every, run)
        };
        let Some((call, routes)) = split_top_marker(run, "->") else {
            let keyword = match kind {
                EffectKind::Future => "run",
                EffectKind::Task => "task",
                EffectKind::Stream => "stream",
            };
            return Err(error(
                "E050",
                line,
                format!("{keyword} requires `-> success _ | error _`"),
            ));
        };
        let (function, args) = parse_effect_call(kind, call.trim(), line)?;
        let (success, error_route) = match split_top_once(routes.trim(), '|') {
            Some((success, failure)) => (
                parse_route(success.trim(), line)?,
                Some(parse_route(failure.trim(), line)?),
            ),
            None => (parse_route(routes.trim(), line)?, None),
        };
        return Ok(Statement::Run {
            kind,
            mode,
            function,
            args,
            success,
            error: error_route,
            span: Span::line(line.number),
        });
    }
    if let Some((target, value)) = split_top_once(&line.text, '=') {
        let (value, at) = match split_top_marker(value.trim(), " at ") {
            Some((value, at)) => (value.trim(), Some(parse_expr(at.trim(), line)?)),
            None => (value.trim(), None),
        };
        return Ok(Statement::Assign {
            target: identifier(target.trim(), line)?,
            value: parse_expr(value, line)?,
            at,
            span: Span::line(line.number),
        });
    }
    Err(error(
        "E051",
        line,
        format!("unknown statement `{}`", line.text),
    ))
}

pub(in crate::parser) fn parse_task_flow(line: &Line) -> Result<Statement, Error> {
    let Some(first) = line.children.first() else {
        return Err(error(
            "E050",
            line,
            "flow requires an indented `from run|task|stream ...`, `from done value`, or `from none Type` source",
        ));
    };
    ensure_leaf(first)?;
    let source = first.text.strip_prefix("from ").ok_or_else(|| {
        error(
            "E050",
            first,
            "the first flow line must be `from run|task|stream ...`, `from done value`, or `from none Type`",
        )
    })?;
    let source = parse_task_source(source, first)?;
    let mut transforms = Vec::new();
    let mut success = None;
    let mut failure = None;
    let mut units = None;
    for item in &line.children[1..] {
        ensure_leaf(item)?;
        if item.text == "collect" {
            transforms.push(TaskTransform::Collect {
                span: Span::line(item.number),
            });
            continue;
        }
        if item.text == "discard" {
            transforms.push(TaskTransform::Discard {
                span: Span::line(item.number),
            });
            continue;
        }
        if let Some(source) = item.text.strip_prefix("map ") {
            let Some((binding, value)) = split_top_marker(source, "->") else {
                return Err(error("E050", item, "map uses `map value -> expr`"));
            };
            transforms.push(TaskTransform::Map {
                binding: identifier(binding.trim(), item)?,
                value: parse_expr(value.trim(), item)?,
                span: Span::line(item.number),
            });
            continue;
        }
        if let Some(source) = item.text.strip_prefix("map-err ") {
            let Some((binding, value)) = split_top_marker(source, "->") else {
                return Err(error(
                    "E050",
                    item,
                    "map-err uses `map-err error -> expression`",
                ));
            };
            transforms.push(TaskTransform::MapError {
                binding: identifier(binding.trim(), item)?,
                value: parse_expr(value.trim(), item)?,
                span: Span::line(item.number),
            });
            continue;
        }
        let transform = item
            .text
            .strip_prefix("then ")
            .map(|source| (false, source))
            .or_else(|| item.text.strip_prefix("try ").map(|source| (true, source)));
        if let Some((and_then, source)) = transform {
            let Some((binding, source)) = split_top_marker(source, "->") else {
                return Err(error(
                    "E050",
                    item,
                    "flow transforms use `then value -> task call(...)` or `try value -> task call(...)`",
                ));
            };
            let binding = identifier(binding.trim(), item)?;
            let source = parse_task_source(source.trim(), item)?;
            transforms.push(if and_then {
                TaskTransform::AndThen {
                    binding,
                    source,
                    span: Span::line(item.number),
                }
            } else {
                TaskTransform::Then {
                    binding,
                    source,
                    span: Span::line(item.number),
                }
            });
            continue;
        }
        let Some((kind, route)) = split_top_marker(&item.text, "->") else {
            return Err(error(
                "E050",
                item,
                "flow lines must be map, then, try, map-err, collect, discard, done, error, or units",
            ));
        };
        let slot = match kind.trim() {
            "done" => &mut success,
            "error" => &mut failure,
            "units" => &mut units,
            _ => {
                return Err(error(
                    "E050",
                    item,
                    "flow route must be done, error, or units",
                ));
            }
        };
        if slot.is_some() {
            return Err(error(
                "E050",
                item,
                format!("duplicate flow {} route", kind.trim()),
            ));
        }
        *slot = Some(parse_route(route.trim(), item)?);
    }
    Ok(Statement::TaskFlow {
        source,
        transforms,
        success,
        error: failure,
        units,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_task_source(source: &str, line: &Line) -> Result<TaskSource, Error> {
    if let Some(value) = source.strip_prefix("done ") {
        return Ok(TaskSource::Done {
            value: parse_expr(value.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(output) = source.strip_prefix("none ") {
        return Ok(TaskSource::None {
            output: parse_type(output.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    let (kind, call) = source
        .strip_prefix("run ")
        .map(|call| (EffectKind::Future, call))
        .or_else(|| {
            source
                .strip_prefix("task ")
                .map(|call| (EffectKind::Task, call))
        })
        .or_else(|| {
            source
                .strip_prefix("stream ")
                .map(|call| (EffectKind::Stream, call))
        })
        .ok_or_else(|| {
            error(
                "E050",
                line,
                "task source must start with run, task, or stream",
            )
        })?;
    let (function, args) = parse_effect_call(kind, call.trim(), line)?;
    Ok(TaskSource::Effect {
        kind,
        function,
        args,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_effect_call(
    kind: EffectKind,
    call: &str,
    line: &Line,
) -> Result<(String, Vec<Expr>), Error> {
    if kind == EffectKind::Task && call == "system info" {
        Ok(("__ice_system_info".into(), Vec::new()))
    } else if kind == EffectKind::Task && call == "system theme" {
        Ok(("__ice_system_theme".into(), Vec::new()))
    } else if kind == EffectKind::Task && call == "time now" {
        Ok(("__ice_time_now".into(), Vec::new()))
    } else if kind == EffectKind::Task && call == "clipboard read" {
        Ok(("__ice_clipboard_read".into(), Vec::new()))
    } else if kind == EffectKind::Task && call == "clipboard read-primary" {
        Ok(("__ice_clipboard_read_primary".into(), Vec::new()))
    } else if kind == EffectKind::Task
        && let Some(value) = call.strip_prefix("font load ")
    {
        if value.trim().is_empty() {
            return Err(error("E050", line, "font load requires bytes"));
        }
        Ok((
            "__ice_font_load".into(),
            vec![parse_expr(value.trim(), line)?],
        ))
    } else if kind == EffectKind::Task
        && let Some(value) = call.strip_prefix("image allocate ")
    {
        if value.trim().is_empty() {
            return Err(error("E050", line, "image allocation requires a handle"));
        }
        Ok((
            "__ice_image_allocate".into(),
            vec![parse_expr(value.trim(), line)?],
        ))
    } else if call.starts_with("system ") {
        Err(error(
            "E050",
            line,
            "system task must be `task system info` or `task system theme`",
        ))
    } else if call.starts_with("clipboard ") {
        Err(error(
            "E050",
            line,
            "clipboard task must read, read-primary, write, or write-primary",
        ))
    } else if call.starts_with("font ") {
        Err(error(
            "E050",
            line,
            "font task must be `task font load bytes -> loaded`",
        ))
    } else if call.starts_with("image ") {
        Err(error(
            "E050",
            line,
            "image task must be `task image allocate handle -> ready _ | failed _`",
        ))
    } else if call.starts_with("time ") {
        Err(error("E050", line, "time task must be `task time now`"))
    } else {
        let (function, args_source) = parse_signature(call, line)?;
        Ok((function, parse_expr_list(&args_source, line)?))
    }
}

pub(in crate::parser) fn parse_sip_statement(
    source: &str,
    line: &Line,
) -> Result<Statement, Error> {
    let (function, args) = parse_signature(source.trim(), line)?;
    let args = parse_expr_list(&args, line)?;
    let mut progress = None;
    let mut success = None;
    let mut failure = None;
    for route in &line.children {
        ensure_leaf(route)?;
        let Some((kind, target)) = split_top_marker(&route.text, "->") else {
            return Err(error(
                "E050",
                route,
                "sip routes use `progress -> handler _`, `done -> handler _`, or `error -> handler _`",
            ));
        };
        let slot = match kind.trim() {
            "progress" => &mut progress,
            "done" => &mut success,
            "error" => &mut failure,
            _ => {
                return Err(error(
                    "E050",
                    route,
                    "sip route must be progress, done, or error",
                ));
            }
        };
        if slot.is_some() {
            return Err(error(
                "E050",
                route,
                format!("duplicate sip {} route", kind.trim()),
            ));
        }
        *slot = Some(parse_route(target.trim(), route)?);
    }
    let progress = progress.ok_or_else(|| {
        error(
            "E050",
            line,
            "sip requires an indented `progress -> handler _` route",
        )
    })?;
    let success = success.ok_or_else(|| {
        error(
            "E050",
            line,
            "sip requires an indented `done -> handler _` route",
        )
    })?;
    Ok(Statement::Sip {
        function,
        args,
        progress,
        success,
        error: failure,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_pane_operation(
    source: &str,
    line: &Line,
) -> Result<Statement, Error> {
    let (source, route) = split_top_marker(source, "->")
        .map_or((source, None), |(source, route)| (source, Some(route)));
    let parts = split_words(source);
    let grid = parts
        .first()
        .and_then(|part| part.strip_prefix('#'))
        .ok_or_else(|| error("E188", line, "pane operation target must use `#grid`"))?;
    let grid = identifier(grid, line)?;
    let pane = |index: usize| {
        parse_pane_reference(
            parts
                .get(index)
                .ok_or_else(|| error("E188", line, "pane operation is missing a pane name"))?,
            line,
        )
    };
    let edge = |index: usize| {
        Ok(match parts.get(index).map(String::as_str) {
            Some("top") => PaneEdge::Top,
            Some("left") => PaneEdge::Left,
            Some("right") => PaneEdge::Right,
            Some("bottom") => PaneEdge::Bottom,
            _ => {
                return Err(error(
                    "E188",
                    line,
                    "pane edge must be top, left, right, or bottom",
                ));
            }
        })
    };
    let axis = |index: usize| {
        Ok(match parts.get(index).map(String::as_str) {
            Some("horizontal") => PaneAxis::Horizontal,
            Some("vertical") => PaneAxis::Vertical,
            _ => {
                return Err(error(
                    "E188",
                    line,
                    "pane split axis must be horizontal or vertical",
                ));
            }
        })
    };
    let operation = match parts.get(1).map(String::as_str) {
        Some("maximize") if parts.len() == 3 => PaneOperation::Maximize { pane: pane(2)? },
        Some("restore") if parts.len() == 2 => PaneOperation::Restore,
        Some("maximized") if parts.len() == 2 => PaneOperation::Maximized,
        Some("adjacent") if parts.len() == 4 => PaneOperation::Adjacent {
            pane: pane(2)?,
            edge: edge(3)?,
        },
        Some("swap") if parts.len() == 4 => PaneOperation::Swap {
            first: pane(2)?,
            second: pane(3)?,
        },
        Some("close") if parts.len() == 3 => PaneOperation::Close { pane: pane(2)? },
        Some("move") if parts.len() == 4 => PaneOperation::Move {
            pane: pane(2)?,
            edge: edge(3)?,
        },
        Some("resize") if (3..=4).contains(&parts.len()) => PaneOperation::Resize {
            split: (parts.len() == 4)
                .then(|| identifier(&parts[2], line))
                .transpose()?,
            ratio: parse_expr(
                strip_wrapping_parens(&parts[if parts.len() == 4 { 3 } else { 2 }]),
                line,
            )?,
        },
        Some("drop") if parts.len() == 5 => PaneOperation::Drop {
            pane: pane(2)?,
            target: pane(3)?,
            edge: match parts[4].as_str() {
                "center" => None,
                _ => Some(edge(4)?),
            },
        },
        Some("split") if (5..=6).contains(&parts.len()) => PaneOperation::Split {
            target: pane(2)?,
            pane: pane(3)?,
            axis: axis(4)?,
            ratio: parts.get(5).map_or(Ok(Expr::F64(0.5)), |part| {
                let value = part
                    .strip_prefix("ratio=")
                    .ok_or_else(|| error("E188", line, "pane split ratio uses `ratio=value`"))?;
                parse_expr(strip_wrapping_parens(value), line)
            })?,
        },
        _ => {
            return Err(error(
                "E188",
                line,
                "unknown pane operation or wrong arguments",
            ));
        }
    };
    Ok(Statement::PaneOperation {
        grid,
        operation,
        route: route
            .map(|route| parse_route(route.trim(), line))
            .transpose()?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_pane_reference(
    source: &str,
    line: &Line,
) -> Result<PaneReference, Error> {
    if !source.contains('(') {
        return Ok(PaneReference::Static(identifier(source, line)?));
    }
    let (template, args) = parse_local_signature(source, line)
        .map_err(|_| error("E188", line, "dynamic pane references use `template(key)`"))?;
    let mut args = parse_expr_list(&args, line)?;
    if args.len() != 1 {
        return Err(error(
            "E188",
            line,
            "dynamic pane references require exactly one key",
        ));
    }
    Ok(PaneReference::Dynamic {
        template,
        key: args.remove(0),
    })
}

pub(in crate::parser) fn parse_widget_operation(
    source: &str,
    line: &Line,
) -> Result<Statement, Error> {
    let (source, route) = split_top_marker(source, "->")
        .map_or((source, None), |(source, route)| (source, Some(route)));
    let parts = split_words(source);
    let target = |index: usize| {
        let value = parts
            .get(index)
            .ok_or_else(|| error("E052", line, "widget operation is missing `#id`"))?;
        parse_widget_target(value, line)
    };
    let expr = |index: usize| {
        parse_expr(
            strip_wrapping_parens(
                parts
                    .get(index)
                    .ok_or_else(|| error("E052", line, "widget operation is missing a value"))?,
            ),
            line,
        )
    };
    let operation = if let Some(selector) = source.strip_prefix("find-all ") {
        WidgetOperation::Find {
            selector: parse_widget_selector(selector.trim(), line)?,
            all: true,
        }
    } else if let Some(selector) = source.strip_prefix("find ") {
        WidgetOperation::Find {
            selector: parse_widget_selector(selector.trim(), line)?,
            all: false,
        }
    } else {
        match parts.first().map(String::as_str) {
            Some("focus-prev") if parts.len() == 1 => WidgetOperation::FocusPrevious,
            Some("focus-next") if parts.len() == 1 => WidgetOperation::FocusNext,
            Some("focus") if parts.len() == 2 => WidgetOperation::Focus { target: target(1)? },
            Some("focused") if parts.len() == 2 => WidgetOperation::Focused { target: target(1)? },
            Some("cursor-front") if parts.len() == 2 => {
                WidgetOperation::CursorFront { target: target(1)? }
            }
            Some("cursor-end") if parts.len() == 2 => {
                WidgetOperation::CursorEnd { target: target(1)? }
            }
            Some("cursor") if parts.len() == 3 => WidgetOperation::Cursor {
                target: target(1)?,
                position: expr(2)?,
            },
            Some("select-all") if parts.len() == 2 => {
                WidgetOperation::SelectAll { target: target(1)? }
            }
            Some("select") if parts.len() == 4 => WidgetOperation::Select {
                target: target(1)?,
                start: expr(2)?,
                end: expr(3)?,
            },
            Some("snap") if parts.len() == 4 => WidgetOperation::Snap {
                target: target(1)?,
                x: expr(2)?,
                y: expr(3)?,
            },
            Some("snap-end") if parts.len() == 2 => WidgetOperation::SnapEnd { target: target(1)? },
            Some("scroll-to") if parts.len() == 4 => WidgetOperation::ScrollTo {
                target: target(1)?,
                x: expr(2)?,
                y: expr(3)?,
            },
            Some("scroll-by") if parts.len() == 4 => WidgetOperation::ScrollBy {
                target: target(1)?,
                x: expr(2)?,
                y: expr(3)?,
            },
            _ => {
                return Err(error(
                    "E052",
                    line,
                    "unknown widget operation or wrong arguments",
                ));
            }
        }
    };
    Ok(Statement::WidgetOperation {
        operation,
        route: route
            .map(|route| parse_route(route.trim(), line))
            .transpose()?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_widget_selector(
    source: &str,
    line: &Line,
) -> Result<WidgetSelector, Error> {
    if let Some(target) = source.strip_prefix("id ") {
        Ok(WidgetSelector::Id(parse_widget_target(
            target.trim(),
            line,
        )?))
    } else if let Some(value) = source.strip_prefix("text ") {
        Ok(WidgetSelector::Text(parse_expr(value.trim(), line)?))
    } else if let Some(values) = source.strip_prefix("point ") {
        let values = split_words(values);
        if values.len() != 2 {
            return Err(error(
                "E052",
                line,
                "point selector requires x and y expressions",
            ));
        }
        Ok(WidgetSelector::Point {
            x: parse_expr(&values[0], line)?,
            y: parse_expr(&values[1], line)?,
        })
    } else if source == "focused" {
        Ok(WidgetSelector::Focused)
    } else {
        let (function, args) = parse_signature(source, line)?;
        Ok(WidgetSelector::Extern {
            function,
            args: parse_expr_list(&args, line)?,
        })
    }
}

pub(in crate::parser) fn parse_widget_target(
    source: &str,
    line: &Line,
) -> Result<WidgetTarget, Error> {
    let source = source.strip_prefix('#').ok_or_else(|| {
        error(
            "E052",
            line,
            "widget operation target must use `#id`, `#id(key)`, or `#scope/id`",
        )
    })?;
    let segments = split_top(source, '/')
        .into_iter()
        .map(|segment| {
            let segment = segment.strip_prefix('#').unwrap_or(segment);
            if segment.is_empty() {
                return Err(error("E052", line, "widget target contains an empty scope"));
            }
            if segment.contains('(') {
                parse_id(segment, line)
            } else if kebab_identifier(segment, line).is_ok()
                || component_identifier(segment, line).is_ok()
            {
                Ok(Id {
                    name: segment.into(),
                    key: None,
                })
            } else {
                Err(error(
                    "E052",
                    line,
                    format!("invalid widget target scope `{segment}`"),
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WidgetTarget { segments })
}

pub(in crate::parser) fn parse_window_operation(
    source: &str,
    line: &Line,
) -> Result<Statement, Error> {
    let (source, route) = split_top_marker(source, "->")
        .map_or((source, None), |(source, route)| (source, Some(route)));
    let (source, target) = split_top_marker(source, " target=")
        .map_or((source, None), |(source, target)| {
            (source, Some(parse_expr(target.trim(), line)))
        });
    let target = target.transpose()?;
    let parts = split_words(source);
    let expr = |index: usize| {
        parse_expr(
            strip_wrapping_parens(
                parts
                    .get(index)
                    .ok_or_else(|| error("E053", line, "window task is missing a value"))?,
            ),
            line,
        )
    };
    let size = || match parts.as_slice() {
        [_, value] if value == "none" => Ok(None),
        [_, _, _] => Ok(Some((expr(1)?, expr(2)?))),
        _ => Err(error(
            "E053",
            line,
            "window size task expects `width height` or `none`",
        )),
    };
    let operation = match parts.first().map(String::as_str) {
        Some("open") if parts.len() == 1 => WindowOperation::Open(None),
        Some("open") if parts.len() == 2 => {
            WindowOperation::Open(Some(identifier(&parts[1], line)?))
        }
        Some("oldest") if parts.len() == 1 => WindowOperation::Oldest,
        Some("latest") if parts.len() == 1 => WindowOperation::Latest,
        Some("close") if parts.len() == 1 => WindowOperation::Close,
        Some("drag") if parts.len() == 1 => WindowOperation::Drag,
        Some("drag-resize") if parts.len() == 2 => {
            WindowOperation::DragResize(match parts[1].as_str() {
                "north" => WindowDirection::North,
                "south" => WindowDirection::South,
                "east" => WindowDirection::East,
                "west" => WindowDirection::West,
                "north-east" => WindowDirection::NorthEast,
                "north-west" => WindowDirection::NorthWest,
                "south-east" => WindowDirection::SouthEast,
                "south-west" => WindowDirection::SouthWest,
                _ => return Err(error("E053", line, "unknown window resize direction")),
            })
        }
        Some("resize") if parts.len() == 3 => WindowOperation::Resize(expr(1)?, expr(2)?),
        Some("resizable") if parts.len() == 2 => WindowOperation::Resizable(expr(1)?),
        Some("min-size") => WindowOperation::MinSize(size()?),
        Some("max-size") => WindowOperation::MaxSize(size()?),
        Some("resize-step") => WindowOperation::ResizeIncrements(size()?),
        Some("size") if parts.len() == 1 => WindowOperation::Size,
        Some("maximized") if parts.len() == 1 => WindowOperation::IsMaximized,
        Some("maximize") if parts.len() == 2 => WindowOperation::Maximize(expr(1)?),
        Some("minimized") if parts.len() == 1 => WindowOperation::IsMinimized,
        Some("minimize") if parts.len() == 2 => WindowOperation::Minimize(expr(1)?),
        Some("position") if parts.len() == 1 => WindowOperation::Position,
        Some("scale") if parts.len() == 1 => WindowOperation::ScaleFactor,
        Some("move") if parts.len() == 3 => WindowOperation::Move(expr(1)?, expr(2)?),
        Some("mode") if parts.len() == 1 => WindowOperation::Mode,
        Some("set-mode") if parts.len() == 2 => WindowOperation::SetMode(match parts[1].as_str() {
            "windowed" => WindowMode::Windowed,
            "fullscreen" => WindowMode::Fullscreen,
            "hidden" => WindowMode::Hidden,
            _ => {
                return Err(error(
                    "E053",
                    line,
                    "window mode must be windowed, fullscreen, or hidden",
                ));
            }
        }),
        Some("toggle-maximize") if parts.len() == 1 => WindowOperation::ToggleMaximize,
        Some("toggle-decorations") if parts.len() == 1 => WindowOperation::ToggleDecorations,
        Some("attention") if parts.len() == 2 => {
            WindowOperation::Attention(match parts[1].as_str() {
                "none" => None,
                "critical" => Some(WindowAttention::Critical),
                "informational" => Some(WindowAttention::Informational),
                _ => {
                    return Err(error(
                        "E053",
                        line,
                        "window attention must be none, critical, or informational",
                    ));
                }
            })
        }
        Some("focus") if parts.len() == 1 => WindowOperation::Focus,
        Some("level") if parts.len() == 2 => WindowOperation::SetLevel(match parts[1].as_str() {
            "normal" => WindowLevel::Normal,
            "always-on-bottom" => WindowLevel::AlwaysOnBottom,
            "always-on-top" => WindowLevel::AlwaysOnTop,
            _ => return Err(error("E053", line, "unknown window level")),
        }),
        Some("system-menu") if parts.len() == 1 => WindowOperation::SystemMenu,
        Some("raw-id") if parts.len() == 1 => WindowOperation::RawId,
        Some("screenshot") if parts.len() == 1 => WindowOperation::Screenshot,
        Some("mouse-passthrough") if parts.len() == 2 => {
            WindowOperation::MousePassthrough(expr(1)?)
        }
        Some("monitor-size") if parts.len() == 1 => WindowOperation::MonitorSize,
        Some("auto-tabs") if parts.len() == 2 => WindowOperation::AutomaticTabbing(expr(1)?),
        Some("icon") if parts.len() == 4 => WindowOperation::Icon {
            pixels: expr(1)?,
            width: expr(2)?,
            height: expr(3)?,
        },
        Some(_) if source.contains('(') => {
            let (function, args) = parse_signature(source.trim(), line)?;
            WindowOperation::Callback {
                function,
                args: parse_expr_list(&args, line)?,
            }
        }
        _ => {
            return Err(error(
                "E053",
                line,
                "unknown window task or wrong arguments",
            ));
        }
    };
    Ok(Statement::WindowOperation {
        operation,
        target,
        route: route
            .map(|route| parse_route(route.trim(), line))
            .transpose()?,
        span: Span::line(line.number),
    })
}

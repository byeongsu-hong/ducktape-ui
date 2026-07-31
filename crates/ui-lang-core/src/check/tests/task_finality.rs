use super::*;

fn span(line: usize) -> Span {
    Span::line(line)
}

fn route(line: usize) -> Route {
    Route {
        handler: "done".into(),
        args: Vec::new(),
        span: span(line),
    }
}

fn pane(name: &str) -> PaneReference {
    PaneReference::Static(name.into())
}

fn assignment(line: usize) -> Statement {
    Statement::Assign {
        target: "changed".into(),
        value: Expr::Bool(true),
        at: None,
        span: span(line),
    }
}

#[test]
fn classifies_every_statement_variant_by_immediate_task_semantics() {
    let line = 7;
    let nonterminal = vec![
        Statement::Let {
            name: "local".into(),
            value: Expr::Bool(true),
            span: span(line),
        },
        assignment(line),
        Statement::MarkdownAppend {
            target: "markdown".into(),
            value: Expr::Str("text".into()),
            span: span(line),
        },
        Statement::ComboPush {
            target: "combo".into(),
            value: Expr::Str("item".into()),
            span: span(line),
        },
        Statement::ReturnIf {
            condition: Expr::Bool(true),
            span: span(line),
        },
        Statement::Abort {
            handle: "request".into(),
            span: span(line),
        },
        Statement::DebugStart {
            name: Expr::Str("load".into()),
            target: "timing".into(),
            span: span(line),
        },
        Statement::DebugFinish {
            target: "timing".into(),
            span: span(line),
        },
        Statement::PaneOperation {
            grid: "main".into(),
            operation: PaneOperation::Maximize { pane: pane("left") },
            route: None,
            span: span(line),
        },
        Statement::PaneOperation {
            grid: "main".into(),
            operation: PaneOperation::Restore,
            route: None,
            span: span(line),
        },
        Statement::PaneOperation {
            grid: "main".into(),
            operation: PaneOperation::Swap {
                first: pane("left"),
                second: pane("right"),
            },
            route: None,
            span: span(line),
        },
        Statement::PaneOperation {
            grid: "main".into(),
            operation: PaneOperation::Close { pane: pane("left") },
            route: None,
            span: span(line),
        },
        Statement::PaneOperation {
            grid: "main".into(),
            operation: PaneOperation::Move {
                pane: pane("left"),
                edge: PaneEdge::Right,
            },
            route: None,
            span: span(line),
        },
        Statement::PaneOperation {
            grid: "main".into(),
            operation: PaneOperation::Resize {
                split: None,
                ratio: Expr::F64(0.5),
            },
            route: None,
            span: span(line),
        },
        Statement::PaneOperation {
            grid: "main".into(),
            operation: PaneOperation::Drop {
                pane: pane("left"),
                target: pane("right"),
                edge: None,
            },
            route: None,
            span: span(line),
        },
        Statement::PaneOperation {
            grid: "main".into(),
            operation: PaneOperation::Split {
                target: pane("left"),
                pane: pane("right"),
                axis: PaneAxis::Horizontal,
                ratio: Expr::F64(0.5),
            },
            route: None,
            span: span(line),
        },
    ];
    for statement in nonterminal {
        assert_eq!(statement.immediate_task(), None);
        check_task_finality(&statement, false).unwrap();
    }

    let terminal = vec![
        (Statement::Exit { span: span(line) }, "E141", "exit"),
        (
            Statement::Run {
                kind: EffectKind::Future,
                mode: FutureMode::Every,
                function: "load".into(),
                args: Vec::new(),
                success: route(line),
                error: None,
                span: span(line),
            },
            "E141",
            "run",
        ),
        (
            Statement::Run {
                kind: EffectKind::Task,
                mode: FutureMode::Every,
                function: "load".into(),
                args: Vec::new(),
                success: route(line),
                error: None,
                span: span(line),
            },
            "E141",
            "task",
        ),
        (
            Statement::Run {
                kind: EffectKind::Stream,
                mode: FutureMode::Every,
                function: "load".into(),
                args: Vec::new(),
                success: route(line),
                error: None,
                span: span(line),
            },
            "E141",
            "stream",
        ),
        (
            Statement::Sip {
                function: "load".into(),
                args: Vec::new(),
                progress: route(line),
                success: route(line),
                error: None,
                span: span(line),
            },
            "E141",
            "sip",
        ),
        (
            Statement::TaskFlow {
                source: TaskSource::None {
                    output: Type::Unit,
                    span: span(line),
                },
                transforms: Vec::new(),
                success: None,
                error: None,
                units: None,
                span: span(line),
            },
            "E141",
            "flow",
        ),
        (
            Statement::TaskGroup {
                kind: TaskGroupKind::Parallel,
                statements: vec![Statement::Exit { span: span(line) }],
                span: span(line),
            },
            "E141",
            "task group",
        ),
        (
            Statement::Abortable {
                handle: "request".into(),
                abort_on_drop: false,
                task: Box::new(Statement::Exit { span: span(line) }),
                span: span(line),
            },
            "E141",
            "abortable task",
        ),
        (
            Statement::ClipboardWrite {
                primary: false,
                value: Expr::Str("text".into()),
                span: span(line),
            },
            "E141",
            "clipboard write",
        ),
        (
            Statement::WidgetOperation {
                operation: WidgetOperation::FocusNext,
                route: None,
                span: span(line),
            },
            "E172",
            "widget operation",
        ),
        (
            Statement::WindowOperation {
                operation: WindowOperation::Close,
                target: None,
                route: None,
                span: span(line),
            },
            "E173",
            "window task",
        ),
        (
            Statement::PaneOperation {
                grid: "main".into(),
                operation: PaneOperation::Maximized,
                route: Some(route(line)),
                span: span(line),
            },
            "E188",
            "pane query",
        ),
        (
            Statement::PaneOperation {
                grid: "main".into(),
                operation: PaneOperation::Adjacent {
                    pane: pane("left"),
                    edge: PaneEdge::Right,
                },
                route: Some(route(line)),
                span: span(line),
            },
            "E188",
            "pane query",
        ),
    ];
    for (statement, code, name) in terminal {
        assert!(statement.immediate_task().is_some());
        check_task_finality(&statement, true).unwrap();
        let error = check_task_finality(&statement, false).unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(error.line, line);
        assert!(error.message.starts_with(name), "{}", error.message);
        assert!(error.message.contains("`parallel` or `sequential`"));
    }
}

#[test]
fn reports_each_native_task_family_at_its_source_line() {
    for (source, code, name) in [
        (
            include_str!("../../../tests/cases/diagnostic/native-task-finality/as-is.ice"),
            "E141",
            "task",
        ),
        (
            include_str!("../../../tests/cases/diagnostic/clipboard-task-finality/as-is.ice"),
            "E141",
            "clipboard write",
        ),
        (
            include_str!("../../../tests/cases/diagnostic/widget-task-finality/as-is.ice"),
            "E172",
            "widget operation",
        ),
        (
            include_str!("../../../tests/cases/diagnostic/window-task-finality/as-is.ice"),
            "E173",
            "window task",
        ),
        (
            include_str!("../../../tests/cases/diagnostic/pane-task-finality/as-is.ice"),
            "E188",
            "pane query",
        ),
    ] {
        let error = crate::analyze(source).unwrap_err();

        assert_eq!(error.code, code);
        assert_eq!(error.line, 15);
        assert_eq!(error.column, 1);
        assert!(error.message.starts_with(name), "{}", error.message);
        assert!(error.message.contains("`parallel` or `sequential`"));
    }
}

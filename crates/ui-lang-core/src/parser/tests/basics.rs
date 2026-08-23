use super::*;

#[test]
fn parses_compact_app() {
    let document = parse(SOURCE).unwrap();
    assert_eq!(document.app, "Demo");
    assert!(!document.daemon);
    assert_eq!(document.structs.len(), 1);
    assert_eq!(document.handlers.len(), 3);
}

#[test]
fn parses_semantic_style_recipes() {
    let document = parse(
        "app Demo\nrecipe surface for box\n  @w-full bg-surface\nrecipe panel for box extends surface\n  @px-16px py-11px border border-border rounded-9px\nrecipe label for text\n  @text-12.5px\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\n  surface\n  border\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n  surface #111111\n  border #222222\nview\n  box @panel\n    text \"Panel\" @label\n",
    )
    .unwrap();

    assert_eq!(document.recipes.len(), 3);
    assert_eq!(document.recipes[1].name, "panel");
    assert_eq!(document.recipes[1].target, StyleRecipeTarget::Container);
    assert_eq!(document.recipes[1].base.as_deref(), Some("surface"));
    assert_eq!(
        document.recipes[1].utilities,
        [
            "px-16px",
            "py-11px",
            "border",
            "border-border",
            "rounded-9px"
        ]
    );
    assert_eq!(document.recipes[2].utilities, ["text-12.5px"]);
}

#[test]
fn parses_component_prop_shorthand() {
    let document = parse(
        "app Demo\ncomponent Card(title:str, count:i64)\n  text title\nview\n  Card title count\n",
    )
    .unwrap();
    let ViewNode::Component { args, .. } = &document.view else {
        panic!("expected component call");
    };
    assert_eq!(args.len(), 2);
    assert!(matches!(
        &args[0],
        ComponentArg { name, value: Expr::Path(path), bind: false } if name == "title" && *path == ["title"]
    ));
    assert!(matches!(
        &args[1],
        ComponentArg { name, value: Expr::Path(path), bind: false } if name == "count" && *path == ["count"]
    ));
}

#[test]
fn rejects_malformed_component_props() {
    let error =
        parse("app Demo\ncomponent Card(title:str)\n  text title\nview\n  Card 42\n").unwrap_err();
    assert_eq!(error.code, "E040");
}

#[test]
fn parses_component_prop_defaults() {
    let document = parse(
        "app Demo\ncomponent Badge(count:i64, label:str=\"Untitled\", selected:bool=false)\n  text label\nview\n  Badge count=1\n",
    )
    .unwrap();

    let params = &document.components[0].params;
    assert_eq!(params[0].name, "count");
    assert!(params[0].default.is_none());
    assert_eq!(params[1].name, "label");
    assert_eq!(params[1].ty, Type::Str);
    assert!(matches!(params[1].default, Some(Expr::Str(ref value)) if value == "Untitled"));
    assert!(matches!(params[2].default, Some(Expr::Bool(false))));
}

#[test]
fn rejects_removed_property_spellings() {
    for view in [
        "box background=bg\n    text \"Demo\"",
        "box width=fill\n    text \"Demo\"",
        "box padding=8.0\n    text \"Demo\"",
        "row spacing=8.0\n    text \"Demo\"",
        "grid columns=2\n    text \"Demo\"",
        "flex direction=row\n    text \"Demo\"",
        "flex justify=normal\n    text \"Demo\"",
        "flex items=self-start\n    text \"Demo\"",
    ] {
        let source = format!(
            "app Demo\ntheme contract AppTheme\n  bg\npalette app for AppTheme\n  bg #000000\nview\n  {view}\n"
        );
        parse(&source).unwrap_err();
    }
}

#[test]
fn parses_daemon_root_and_exit() {
    let source = r#"daemon Agent
  window dashboard
on quit
  exit
view
  button "Quit" -> quit
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.app, "Agent");
    assert!(document.daemon);
    assert_eq!(document.settings.windows[0].name, "dashboard");
    assert!(matches!(
        document.handlers[0].statements[0],
        Statement::Exit { .. }
    ));

    let error = parse(&source.replace("window dashboard", "window")).unwrap_err();
    assert_eq!(error.code, "E014");
    assert!(error.message.contains("no initial window"));
    assert!(error.hint.unwrap().contains("window name"));
}

#[test]
fn parses_borrowed_component_parameters() {
    let source = r#"app Borrowed
extern crate::backend
  Item(label:str)
  component native_row(label:&str, items:&[Item], active:&bool) -> bool
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  label = "Borrowed"
  items:[Item] = []
  active = false
on changed(next)
view
  extern native_row(label, items, active) -> changed _
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].borrowed, vec![true, true, true]);
    assert_eq!(document.functions[0].params[0].1, Type::Str);
    assert_eq!(
        document.functions[0].params[1].1,
        Type::List(Box::new(Type::Named("Item".into())))
    );

    for kind in ["pure", "sync"] {
        let document =
            parse(&source.replace("component native_row", &format!("{kind} native_row"))).unwrap();
        assert_eq!(document.functions[0].borrowed, vec![true, true, true]);
    }
    for declaration in [
        "native_row(label:&str, items:&[Item], active:&bool) -> bool ! Item",
        "task native_row(label:&str, items:&[Item], active:&bool) -> bool",
        "stream native_row(label:&str, items:&[Item], active:&bool) -> bool",
        "markdown-viewer native_row(label:&str, items:&[Item], active:&bool) -> bool",
    ] {
        let error = parse(&source.replace(
            "component native_row(label:&str, items:&[Item], active:&bool) -> bool",
            declaration,
        ))
        .unwrap_err();
        assert_eq!(error.code, "E021", "{declaration}");
        assert!(
            error
                .message
                .contains("only extern component, pure, and sync parameters may borrow"),
            "{declaration}: {}",
            error.message
        );
    }
}

#[test]
fn parses_named_component_events_and_route_maps() {
    let document = parse(
        "app Demo\ncomponent Menu()\n  emits\n    close\n    select(str, bool)\n  button \"Close\" -> emit(close)\non closed\non selected(value, active)\nview\n  Menu\n    events\n      close -> closed\n      select -> selected _ _\n",
    )
    .unwrap();
    assert_eq!(document.components[0].events[0].name, "close");
    assert!(document.components[0].events[0].payloads.is_empty());
    assert_eq!(
        document.components[0].events[1].payloads,
        [Type::Str, Type::Bool]
    );
    let ViewNode::Component { events, .. } = &document.view else {
        panic!("expected component call");
    };
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].name, "select");
    assert_eq!(events[1].route.as_ref().unwrap().args.len(), 2);
}

#[test]
fn parses_explicit_component_event_forwarding() {
    let document = parse(
        "app Demo\ncomponent Leaf()\n  emits\n    select(str)\n  button \"Open\" -> emit(select, \"page\")\ncomponent Shell()\n  emits\n    select(str)\n  Leaf\n    forward\n      select\nview\n  Shell\n",
    )
    .unwrap();
    let ViewNode::Component { events, .. } = &document.components[1].root else {
        panic!("expected component call");
    };
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].name, "select");
    assert!(events[0].route.is_none());
}

#[test]
fn rejects_non_function_emit_syntax() {
    let error = parse(
        "app Demo\ncomponent Card() -> bool\n  button \"Go\" -> emit _\nview\n  text \"Demo\"\n",
    )
    .unwrap_err();
    assert_eq!(error.code, "E052");
    assert!(error.message.contains("function syntax"));
}

#[test]
fn parses_component_lifetime_and_replace_futures() {
    let source = r#"app Demo
extern crate::backend
  fetch() -> str
component Search()
  lifetime mounted
  on search
    run replace lane=requests::search fetch() -> loaded _
  button "Search" -> search
on loaded(value)
view
  Search
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.components[0].lifetime, ComponentLifetime::Mounted);
    assert!(matches!(
        document.components[0].handlers[0].statements[0],
        Statement::Run {
            mode: DeliveryMode::Replace,
            lane: Some(ref lane),
            ..
        } if lane == "requests::search"
    ));
    assert_eq!(
        parse(&source.replace("  lifetime mounted\n", ""))
            .unwrap()
            .components[0]
            .lifetime,
        ComponentLifetime::Retained
    );
    assert_eq!(
        parse(&source.replace("lifetime mounted", "lifetime retained"))
            .unwrap()
            .components[0]
            .lifetime,
        ComponentLifetime::Retained
    );

    for (replacement, expected) in [
        (
            "  lifetime mounted\n  lifetime retained",
            "duplicate lifetime",
        ),
        ("  lifetime transient", "must be `retained` or `mounted`"),
    ] {
        let error = parse(&source.replace("  lifetime mounted", replacement)).unwrap_err();
        assert!(error.message.contains(expected), "{}", error.message);
    }
}

#[test]
fn parses_explicit_every_while_replacement_modes_require_named_delivery_lanes() {
    let source = r#"app Demo
extern crate::backend
  fetch() -> str
on search
  run every fetch() -> loaded _
on loaded(value)
view
  text "Demo"
"#;
    let document = parse(source).unwrap();
    assert!(matches!(
        document.handlers[0].statements[0],
        Statement::Run {
            mode: DeliveryMode::Every,
            lane: None,
            ..
        }
    ));

    for mode in ["latest", "replace"] {
        let error =
            parse(&source.replace("run every fetch", &format!("run {mode} fetch"))).unwrap_err();
        assert_eq!(error.code, "E050");
        assert_eq!(
            error.message,
            format!("`run {mode}` requires a named delivery lane")
        );
        assert_eq!(
            error.hint.as_deref(),
            Some(format!("write `run {mode} lane=name call(...) -> ...`").as_str())
        );
    }
}

#[test]
fn parses_explicit_stream_delivery_modes() {
    let source = r#"app Demo
extern crate::backend
  stream watch() -> str
on observe
  stream every watch() -> observed _
on replace
  stream replace lane=feed watch() -> observed _
on observed(value)
view
  text "Demo"
"#;
    let document = parse(source).unwrap();

    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Run {
            kind: EffectKind::Stream,
            mode: DeliveryMode::Every,
            lane: None,
            ..
        }
    ));
    assert!(matches!(
        &document.handlers[1].statements[0],
        Statement::Run {
            kind: EffectKind::Stream,
            mode: DeliveryMode::Replace,
            lane: Some(lane),
            ..
        } if lane == "feed"
    ));

    let error = parse(&source.replace(
        "stream every watch() -> observed _",
        "stream replace watch() -> observed _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E050");
    assert_eq!(
        error.message,
        "`stream replace` requires a named delivery lane"
    );
    assert_eq!(
        error.hint.as_deref(),
        Some("write `stream replace lane=name call(...) -> ...`")
    );
}

#[test]
fn rejects_stream_without_an_explicit_delivery_mode() {
    let error = parse(
        r#"app Demo
extern crate::backend
  stream watch() -> str
on observe
  stream watch() -> observed _
on observed(value)
view
  text "Demo"
"#,
    )
    .expect_err("bare stream must not select a delivery mode implicitly");
    assert_eq!(error.code, "E050");
    assert_eq!(error.message, "`stream` requires an explicit delivery mode");
    assert_eq!(
        error.hint.as_deref(),
        Some(
            "write `stream every call(...) -> ...` to deliver every item; use a named `stream replace` lane when a new stream supersedes the old one"
        )
    );
}

#[test]
fn rejects_stream_latest() {
    let error = parse(
        r#"app Demo
extern crate::backend
  stream watch() -> str
on observe
  stream latest lane=feed watch() -> observed _
on observed(value)
view
  text "Demo"
"#,
    )
    .expect_err("stream latest cannot define a completion-based lane");
    assert_eq!(error.code, "E050");
    assert_eq!(error.message, "`stream latest` is not supported");
    assert_eq!(
        error.hint.as_deref(),
        Some(
            "use `stream replace lane=name call(...) -> ...` to abort and suppress the prior stream"
        )
    );
}

#[test]
fn rejects_run_without_an_explicit_delivery_mode() {
    let error = parse(
        r#"app Demo
extern crate::backend
  fetch() -> str
on search
  run fetch() -> loaded _
on loaded(value)
view
  text "Demo"
"#,
    )
    .expect_err("bare run must not select a delivery mode implicitly");
    assert_eq!(error.code, "E050");
    assert_eq!(error.message, "`run` requires an explicit delivery mode");
    assert_eq!(
        error.hint.as_deref(),
        Some(
            "write `run every call(...) -> ...` to deliver every completion; use a named `run latest` or `run replace` lane when newer work supersedes older work"
        )
    );
}

#[test]
fn parses_only_canonical_qualified_delivery_lane_invalidation() {
    let source = r#"app Demo
extern crate::backend
  fetch() -> str
on stop
  invalidate lane=requests::search
on search
  run latest lane=requests::search fetch() -> loaded _
on loaded(value)
view
  text "Demo"
"#;
    let document = parse(source).unwrap();
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::InvalidateLane { lane, .. } if lane == "requests::search"
    ));
    let namespaces = source
        .lines()
        .map(|line| {
            (line.trim_start().starts_with("invalidate ")
                || line.trim_start().starts_with("run latest "))
            .then(|| "imported".to_owned())
        })
        .collect::<Vec<_>>();
    let (imported, _) = parse_with_symbols_and_namespaces(source, &namespaces).unwrap();
    assert!(matches!(
        &imported.handlers[0].statements[0],
        Statement::InvalidateLane { lane, .. } if lane == "imported::requests::search"
    ));
    assert!(matches!(
        &imported.handlers[1].statements[0],
        Statement::Run { lane: Some(lane), .. } if lane == "imported::requests::search"
    ));

    for invalid in ["invalidate", "invalidate requests::search"] {
        let error =
            parse(&source.replace("invalidate lane=requests::search", invalid)).unwrap_err();
        assert_eq!(error.code, "E050");
        assert_eq!(
            error.message,
            "`invalidate` requires `lane=<qualified-identifier>`"
        );
        assert_eq!(
            error.hint.as_deref(),
            Some("write `invalidate lane=request_name`")
        );
    }

    let error = parse(&source.replace(
        "invalidate lane=requests::search",
        "invalidate lane=requests::search now",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E072");
}

#[test]
fn parses_all_native_time_operations() {
    let source = example!("timer.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.states[0].ty, Type::Option(Box::new(Type::Instant)));
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Run { function, .. } if function == "__ice_time_now"
    ));
    assert!(matches!(
        document.subscriptions[0].source,
        SubscriptionSource::Every { milliseconds: 250 }
    ));
    assert!(matches!(
        &document.subscriptions[1].source,
        SubscriptionSource::Repeat {
            function,
            milliseconds: 1000
        } if function == "refresh_time"
    ));
    assert_eq!(
        document.subscriptions[1].filter.as_deref(),
        Some("even_refresh")
    );
    assert!(matches!(
        document.subscriptions[1].context,
        Some(Expr::I64(7))
    ));
    assert_eq!(
        document.subscriptions[2].filter.as_deref(),
        Some("visible_pointer")
    );
    assert!(document.subscriptions[3].context.is_none());

    let error =
        parse(&source.replace("refresh_time() every", "refresh_time(1) every")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("cannot take arguments"));
}

#[test]
fn parses_structured_task_groups() {
    let source = SOURCE.replace(
            "  run every load() -> loaded _ | failed _",
            "  parallel\n    run every load() -> loaded _ | failed _\n    sequential\n      task clipboard read -> clipboard_read _\n      task system theme -> theme_read _",
        );
    let document = parse(&source).unwrap();
    let Statement::TaskGroup {
        kind, statements, ..
    } = &document.handlers[0].statements[0]
    else {
        panic!("expected task group");
    };
    assert_eq!(*kind, TaskGroupKind::Parallel);
    assert_eq!(statements.len(), 2);
    assert!(matches!(
        &statements[1],
        Statement::TaskGroup {
            kind: TaskGroupKind::Sequential,
            statements,
            ..
        } if statements.len() == 2
    ));

    let error = parse(&SOURCE.replace("  run every load() -> loaded _ | failed _", "  parallel"))
        .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("at least one"));
}

#[test]
fn parses_abortable_tasks_and_handles() {
    let source = SOURCE
        .replace(
            "  query = \"\"",
            "  query = \"\"\n  request:task-handle? = none",
        )
        .replace(
            "  run every load() -> loaded _ | failed _",
            "  abortable request abort-on-drop\n    run every load() -> loaded _ | failed _",
        );
    let document = parse(&source).unwrap();
    assert_eq!(
        document.states[2].ty,
        Type::Option(Box::new(Type::TaskHandle))
    );
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Abortable {
            handle,
            abort_on_drop: true,
            task,
            ..
        } if handle == "request" && matches!(task.as_ref(), Statement::Run { .. })
    ));

    let error = parse(&SOURCE.replace(
        "  run every load() -> loaded _ | failed _",
        "  abortable request later\n    run every load() -> loaded _ | failed _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("abort-on-drop"));

    let error = parse(&SOURCE.replace(
            "  run every load() -> loaded _ | failed _",
            "  abortable request\n    run every load() -> loaded _ | failed _\n    run every load() -> loaded _ | failed _",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("exactly one"));
}

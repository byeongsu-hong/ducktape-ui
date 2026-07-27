use super::*;

#[test]
fn parses_compact_app() {
    let document = parse(SOURCE).unwrap();
    assert_eq!(document.app, "Demo");
    assert!(!document.daemon);
    assert_eq!(document.structs.len(), 1);
    assert_eq!(document.handlers.len(), 3);
    assert_eq!(document.qr_codes.len(), 1);
    assert_eq!(
        document.qr_codes[0].data,
        QrPayload::Text("https://example.com/ice docs".into())
    );
}

#[test]
fn parses_semantic_style_recipes() {
    let document = parse(
        "app Demo\nrecipe surface for box\n  @w-full bg-surface\nrecipe panel for box extends surface\n  @px-16px py-11px border border-border rounded-9px\nrecipe label for text\n  @text-12.5px\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n  surface #111111\n  border #222222\nview\n  box @panel\n    text \"Panel\" @label\n",
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
fn parses_component_prop_defaults() {
    let document = parse(
        "app Demo\ncomponent Badge(label:str=\"Untitled\", selected:bool=false, count:i64)\n  text label\nview\n  Badge count=1\n",
    )
    .unwrap();

    let params = &document.components[0].params;
    assert_eq!(params[0].name, "label");
    assert_eq!(params[0].ty, Type::Str);
    assert!(matches!(params[0].default, Some(Expr::Str(ref value)) if value == "Untitled"));
    assert!(matches!(params[1].default, Some(Expr::Bool(false))));
    assert!(params[2].default.is_none());
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
        let source = format!("app Demo\ntheme\n  bg #000000\nview\n  {view}\n");
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
theme
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

    let error = parse(&source.replace("component native_row", "sync native_row")).unwrap_err();
    assert_eq!(error.code, "E021");
    assert!(error.message.contains("only extern component parameters"));
}

#[test]
fn parses_all_native_time_operations() {
    let source = example!("timer.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.states[1].ty, Type::Option(Box::new(Type::Instant)));
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
        Some(Expr::Path(ref path)) if path == &["generation"]
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
            "  run load() -> loaded _ | failed _",
            "  parallel\n    run load() -> loaded _ | failed _\n    sequential\n      task clipboard read -> clipboard_read _\n      task system theme -> theme_read _",
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

    let error =
        parse(&SOURCE.replace("  run load() -> loaded _ | failed _", "  parallel")).unwrap_err();
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
            "  run load() -> loaded _ | failed _",
            "  abortable request abort-on-drop\n    run load() -> loaded _ | failed _",
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
        "  run load() -> loaded _ | failed _",
        "  abortable request later\n    run load() -> loaded _ | failed _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("abort-on-drop"));

    let error = parse(&SOURCE.replace(
            "  run load() -> loaded _ | failed _",
            "  abortable request\n    run load() -> loaded _ | failed _\n    run load() -> loaded _ | failed _",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("exactly one"));
}

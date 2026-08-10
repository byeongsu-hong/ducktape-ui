use super::*;

#[test]
fn parses_derived_values_and_handler_locals() {
    let source = r#"app Demo
state
  draft = ""
derived
  normalized = trim(draft)
  ready = !empty(normalized)
on submit
  let title = normalized
  draft = title
view
  text normalized
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.derived.len(), 2);
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Let { name, .. } if name == "title"
    ));
}

#[test]
fn parses_typed_task_streams() {
    let source = r#"app Streams
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  stream range(start:i64, limit:i64) -> i64
  stream fallible() -> str ! AppError
  recipe snapshot(id:i64) -> str
  event-filter raw_event() -> str
theme contract AppTheme
  bg
palette app for AppTheme
  bg #000000
on start
  parallel
    stream every numbers(3) -> number _
    stream every fallible() -> text _ | failed _
on number(value)
on text(value)
on failed(error)
on observed(result)
subscribe
  run fallible() -> observed _
  run numbers(3) -> number _
  run range(1, 3) -> number _
  recipe snapshot(3) -> text _
  events 3 using=raw_event -> text _
view
  text "Streams"
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Stream);
    assert_eq!(
        document.functions[2].error,
        Some(Type::Named("AppError".into()))
    );
    let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
        panic!("expected task group");
    };
    assert!(statements.iter().all(|statement| matches!(
        statement,
        Statement::Run {
            kind: EffectKind::Stream,
            ..
        }
    )));
    assert!(matches!(
        &document.subscriptions[0].source,
        SubscriptionSource::Run { function, args }
            if function == "fallible" && args.is_empty()
    ));
    assert!(matches!(
        &document.subscriptions[2].source,
        SubscriptionSource::Run { function, args }
            if function == "range" && args.len() == 2
    ));
    assert!(matches!(
        &document.subscriptions[3].source,
        SubscriptionSource::Recipe { function, args }
            if function == "snapshot" && args.len() == 1
    ));
    assert!(matches!(
        &document.subscriptions[4].source,
        SubscriptionSource::Events { id: Expr::I64(3), filter }
            if filter == "raw_event"
    ));

    let error = parse(&source.replace(
        "recipe snapshot(id:i64) -> str",
        "recipe snapshot(id:i64) -> str ! AppError",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E023");

    let error = parse(&source.replace(
        "event-filter raw_event() -> str",
        "event-filter raw_event(value:i64) -> str",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E022");

    let error = parse(&source.replace(
        "stream every numbers(3) -> number _",
        "stream every numbers(3)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("stream requires"));
}

#[test]
fn parses_generic_event_subscriptions() {
    let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
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
on received(value)
on identified(id, value)
subscribe
  event -> received _
  event status=any -> received _
  event with-id status=ignored -> identified _ _
  event raw status=captured -> received _
  event raw with-id -> identified _ _
view
  text "Events"
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Event);
    assert!(matches!(
        document.subscriptions[0].source,
        SubscriptionSource::Event { raw: false }
    ));
    assert!(!document.subscriptions[0].window_id);
    assert_eq!(document.subscriptions[0].status, None);
    assert_eq!(document.subscriptions[2].status, Some(EventStatus::Ignored));
    assert!(document.subscriptions[2].window_id);
    assert!(matches!(
        document.subscriptions[3].source,
        SubscriptionSource::Event { raw: true }
    ));
    assert_eq!(document.subscriptions[4].status, None);
    assert!(document.subscriptions[4].window_id);

    let error =
        parse(&source.replace("event -> received _", "event redraw -> received _")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("event [raw] [with-id]"));
}

#[test]
fn parses_typed_task_sips() {
    let source = r#"app Sips
extern crate::backend
  AppError(message:str)
  sip download(size:i64) progress=f64 -> bytes
  sip fallible() progress=i64 -> str ! AppError
theme contract AppTheme
  bg
palette app for AppTheme
  bg #000000
on start
  parallel
    sip download(3)
      progress -> advanced _
      done -> downloaded _
    sip fallible()
      progress -> counted _
      done -> finished _
      error -> failed _
on advanced(value)
on downloaded(value)
on counted(value)
on finished(value)
on failed(error)
view
  text "Sips"
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Sip);
    assert_eq!(document.functions[0].progress, Some(Type::F64));
    assert_eq!(
        document.functions[1].error,
        Some(Type::Named("AppError".into()))
    );
    let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
        panic!("expected task group");
    };
    assert!(
        statements
            .iter()
            .all(|statement| matches!(statement, Statement::Sip { .. }))
    );

    let error = parse(&source.replace("      progress -> advanced _\n", "")).unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("progress"));
}

#[test]
fn parses_structured_task_flows() {
    let source = r#"app Flows
extern crate::backend
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
theme contract AppTheme
  bg
palette app for AppTheme
  bg #000000
on start
  flow
    from stream numbers(3)
    map value -> value + 1
    then value -> task double(value)
    collect
    done -> collected _
    units -> planned _
on collected(values)
on planned(units)
view
  text "Flows"
"#;
    let document = parse(source).unwrap();
    let Statement::TaskFlow {
        source: task_source,
        transforms,
        success,
        units,
        ..
    } = &document.handlers[0].statements[0]
    else {
        panic!("expected task flow");
    };
    assert!(matches!(
        task_source,
        TaskSource::Effect {
            kind: EffectKind::Stream,
            ..
        }
    ));
    assert_eq!(transforms.len(), 3);
    assert!(matches!(transforms[0], TaskTransform::Map { .. }));
    assert!(matches!(transforms[1], TaskTransform::Then { .. }));
    assert!(matches!(transforms[2], TaskTransform::Collect { .. }));
    assert!(success.is_some());
    assert!(units.is_some());

    let error = parse(&source.replace("    from stream numbers(3)", "    collect")).unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("first flow line"));

    let error = parse(&source.replace("    map value -> value + 1", "    map value value + 1"))
        .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("map value -> expr"));
}

#[test]
fn parses_task_error_mapping_and_native_sources() {
    let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize(error:NetworkError) -> AppError
  task request() -> i64 ! NetworkError
theme contract AppTheme
  bg
palette app for AppTheme
  bg #000000
state
  results:[result[i64,AppError]] = []
on start
  parallel
    flow
      from task request()
      map-err reason -> normalize(reason)
      collect
      done -> collected _
    flow
      from done 1
      then value -> done value + 1
      done -> finished _
    flow
      from none i64
      done -> finished _
on collected(values)
on finished(value)
view
  text "Errors"
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Sync);
    assert_eq!(
        document.states[0].ty,
        Type::List(Box::new(Type::Result(
            Box::new(Type::I64),
            Box::new(Type::Named("AppError".into()))
        )))
    );
    let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
        panic!("expected task group");
    };
    assert!(matches!(
        &statements[0],
        Statement::TaskFlow { transforms, .. }
            if matches!(transforms[0], TaskTransform::MapError { .. })
    ));
    assert!(matches!(
        &statements[1],
        Statement::TaskFlow {
            source: TaskSource::Done { .. },
            ..
        }
    ));
    assert!(matches!(
        &statements[2],
        Statement::TaskFlow {
            source: TaskSource::None {
                output: Type::I64,
                ..
            },
            ..
        }
    ));
}

#[test]
fn parses_pure_extern_functions() {
    let document = parse(
        "app Pure\nextern crate::backend\n  pure normalize(value:str) -> str\nview\n  text normalize(\"ok\")\n",
    )
    .unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Pure);
}

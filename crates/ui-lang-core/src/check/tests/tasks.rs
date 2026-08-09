use super::*;

#[test]
fn infers_action_result_handler() {
    let source = r#"app Demo
extern crate::backend
  Item(id:i64)
  load() -> [Item] ! Item
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
  items:[Item] = []
on mount
  run load() -> loaded _ | failed _
on loaded(next)
  items = next
on failed(error)
  items = []
view
  text len(items) size=14.0
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "[Item]"
    );
}

#[test]
fn checks_structured_task_groups() {
    let source = r#"app Grouped
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
  mode = ""
on start
  parallel
    task system theme -> theme_read _
    sequential
      task clipboard read -> clipboard_read _
      task system info -> info_read _
on theme_read(next)
  mode = next
on clipboard_read(next)
on info_read(info)
view
  text mode
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "str"
    );
    assert_eq!(
        document.source_document().handlers[2].params[0]
            .ty
            .display(),
        "str?"
    );
    assert_eq!(
        document.source_document().handlers[3].params[0]
            .ty
            .display(),
        "system-info"
    );

    let error = analyze(&source.replace(
        "      task clipboard read -> clipboard_read _",
        "      mode = \"invalid\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E143");
    assert!(error.message.contains("task-producing"));

    let error = analyze(&source.replace(
        "on theme_read(next)",
        "  mode = \"too late\"\non theme_read(next)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E141");
    assert!(error.message.contains("final statement"));
}

#[test]
fn checks_native_task_cancellation() {
    let source = r#"app Cancel
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
  request:task-handle? = none
  canceled = false
on start
  abortable request abort-on-drop
    task system theme -> loaded _
on loaded(next)
on cancel
  abort request
  canceled = aborted(request)
view
  col
    if aborted(request)
      text "Canceled"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().states[0].ty.display(),
        "task-handle?"
    );

    let error = analyze(&source.replace("request:task-handle?", "request:str?")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("task-handle?"));

    let error = analyze(&source.replace("abort request", "abort missing")).unwrap_err();
    assert_eq!(error.code, "E143");
    assert!(error.message.contains("unknown task handle"));

    let error =
        analyze(&source.replace("    task system theme -> loaded _", "    canceled = false"))
            .unwrap_err();
    assert_eq!(error.code, "E143");
    assert!(error.message.contains("task-producing"));

    let error = analyze(&source.replace(
            "  abortable request abort-on-drop\n    task system theme -> loaded _",
            "  parallel\n    abortable request\n      canceled = false\n    task system theme -> loaded _",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E143");
    assert_eq!(error.line, 18);

    let error = analyze(&source.replace("on loaded(next)", "  canceled = false\non loaded(next)"))
        .unwrap_err();
    assert_eq!(error.code, "E141");
    assert!(error.message.contains("final statement"));

    let error = analyze(&source.replace("aborted(request)", "aborted(true)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("aborted(request)", "request == none")).unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("opaque"));
}

#[test]
fn checks_typed_task_streams() {
    let source = r#"app Streams
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  stream coordinates(value:f64) -> i64
  stream fallible() -> str ! AppError
  recipe snapshot(value:i64) -> str
  event-filter raw_event() -> str
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
  count = 0
on start
  parallel
    stream numbers(3) -> number _
    stream fallible() -> text _ | failed _
on number(value)
  count = value
on text(value)
on failed(error)
on observed(result)
subscribe
  run fallible() -> observed _
  run numbers(count) -> number _
  recipe snapshot(count) -> text _
  events count using=raw_event -> text _
view
  text count
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "i64"
    );
    assert_eq!(
        document.source_document().handlers[2].params[0]
            .ty
            .display(),
        "str"
    );
    assert_eq!(
        document.source_document().handlers[3].params[0]
            .ty
            .display(),
        "AppError"
    );
    assert_eq!(
        document.source_document().handlers[4].params[0]
            .ty
            .display(),
        "result[str,AppError]"
    );

    let error = analyze(&source.replace("numbers(3)", "numbers(true)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace(
        "stream fallible() -> text _ | failed _",
        "stream fallible() -> text _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E131");

    let error = analyze(&source.replace(
        "stream numbers(3) -> number _",
        "stream numbers(3) -> number count",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");
    assert!(error.message.contains("at most one `_`"));

    let error = analyze(&source.replace(
        "stream numbers(3) -> number _",
        "stream numbers(3) -> number _ _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error = analyze(&source.replace("stream numbers(3)", "stream missing(3)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("extern stream"));

    let error = analyze(&source.replace(
        "run numbers(count) -> number _",
        "run coordinates(1.5) -> number _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("run data must be hashable"));

    let error =
        analyze(&source.replace("recipe snapshot(count)", "recipe missing(count)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("extern recipe"));

    let error =
        analyze(&source.replace("events count using=raw_event", "events 1.5 using=raw_event"))
            .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("event identity must be hashable"));

    let error = analyze(&source.replace("using=raw_event", "using=missing")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("event filter"));
}

#[test]
fn checks_typed_task_sips() {
    let source = r#"app Sips
extern crate::backend
  AppError(message:str)
  sip transfer(size:i64) progress=f64 -> bytes
  sip fallible() progress=i64 -> str ! AppError
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
on start
  parallel
    sip transfer(3)
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
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0].ty,
        Type::F64
    );
    assert_eq!(
        document.source_document().handlers[2].params[0].ty,
        Type::Bytes
    );
    assert_eq!(
        document.source_document().handlers[3].params[0].ty,
        Type::I64
    );
    assert_eq!(
        document.source_document().handlers[4].params[0].ty,
        Type::Str
    );
    assert_eq!(
        document.source_document().handlers[5].params[0].ty,
        Type::Named("AppError".into())
    );

    let error = analyze(&source.replace("transfer(3)", "transfer(true)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("      error -> failed _\n", "")).unwrap_err();
    assert_eq!(error.code, "E131");

    let error = analyze(&source.replace(
        "      progress -> advanced _",
        "      progress -> advanced 1.0",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error = analyze(&source.replace("sip transfer(3)", "sip missing(3)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("extern sip"));
}

#[test]
fn checks_structured_task_flows() {
    let source = r#"app Flows
extern crate::backend
  AppError(message:str)
  OtherError(message:str)
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
  task optional(value:i64) -> i64?
  task fallible(value:i64) -> i64 ! AppError
  task fallible_double(value:i64) -> i64 ! AppError
  task wrong_error(value:i64) -> i64 ! OtherError
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
  limit = 3
on start
  parallel
    flow
      from stream numbers(limit)
      map value -> value + 1
      then value -> task double(value)
      collect
      done -> collected _
      units -> planned _
    flow
      from task optional(2)
      try value -> task double(value)
      done -> finished _
    flow
      from task fallible(2)
      map value -> value + 1
      try value -> task fallible_double(value)
      done -> finished _
      error -> failed _
on collected(values)
on planned(units)
on finished(value)
on failed(error)
view
  text "Flows"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0].ty,
        Type::List(Box::new(Type::I64))
    );
    assert_eq!(
        document.source_document().handlers[2].params[0].ty,
        Type::I64
    );
    assert_eq!(
        document.source_document().handlers[3].params[0].ty,
        Type::I64
    );
    assert_eq!(
        document.source_document().handlers[4].params[0].ty,
        Type::Named("AppError".into())
    );

    let error = analyze(&source.replace(
        "try value -> task fallible_double(value)",
        "then value -> task fallible_double(value)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E144");
    assert!(error.message.contains("use try"));

    let error = analyze(&source.replace(
        "try value -> task fallible_double(value)",
        "try value -> task wrong_error(value)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace(
        "then value -> task double(value)",
        "then value -> task double(limit)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.hint.unwrap().contains("only read its `value`"));

    let error = analyze(&source.replacen("map value -> value + 1", "map value -> limit + 1", 1))
        .unwrap_err();
    assert_eq!(error.code, "E150");
    assert_eq!(
        error.hint.as_deref(),
        Some("map may only read its `value` binding")
    );
}

#[test]
fn checks_task_error_mapping_and_native_sources() {
    let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  pure normalize(error:NetworkError) -> AppError
  task request() -> i64 ! NetworkError
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
  results = values
on finished(value)
view
  text len(results)
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0].ty,
        Type::List(Box::new(Type::Result(
            Box::new(Type::I64),
            Box::new(Type::Named("AppError".into()))
        )))
    );
    assert_eq!(
        document.source_document().handlers[2].params[0].ty,
        Type::I64
    );

    let error = analyze(&source.replace(
        "map-err reason -> normalize(reason)",
        "map-err reason -> normalize(1)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace(
        "from task request()\n      map-err reason -> normalize(reason)",
        "from done 1\n      map-err reason -> normalize(reason)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E144");
    assert!(error.message.contains("fallible"));

    let error = analyze(&source.replace("from none i64", "from none Missing")).unwrap_err();
    assert_eq!(error.code, "E103");
}

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
  run every load() -> loaded _ | failed _
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
fn rejects_handler_streams_anywhere_inside_abortable() {
    for task in [
        "abortable request\n    stream every ticks() -> ticked _",
        "parallel\n    abortable request\n      sequential\n        stream replace lane=ticks ticks() -> ticked _",
    ] {
        let source = warning_app(&format!(
            r#"extern crate::backend
  stream ticks() -> i64
state
  request:task-handle? = none
on start
  {task}
on ticked(value)
view
  text "Ticks"
"#
        ));
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E143");
        assert_eq!(
            error.message,
            "`stream` cannot be nested inside `abortable`"
        );
        assert_eq!(
            error.hint.as_deref(),
            Some(
                "use `stream replace lane=name ...` for owned replacement, or `stream every ...` without an outer `abortable`"
            )
        );
    }
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
    stream every numbers(3) -> number _
    stream every fallible() -> text _ | failed _
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
        "stream every fallible() -> text _ | failed _",
        "stream every fallible() -> text _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E131");

    let error = analyze(&source.replace(
        "stream every numbers(3) -> number _",
        "stream every numbers(3) -> number count",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");
    assert!(error.message.contains("at most one `_`"));

    let error = analyze(&source.replace(
        "stream every numbers(3) -> number _",
        "stream every numbers(3) -> number _ _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error =
        analyze(&source.replace("stream every numbers(3)", "stream every missing(3)")).unwrap_err();
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

#[test]
fn allows_future_and_task_completion_route_snapshots() {
    let source = r#"app Snapshots
extern crate::backend
  AppError(message:str)
  pure decorate(value:str) -> str
  request(id:i64) -> str ! AppError
  task cached(id:i64) -> str ! AppError
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
  token = "initial"
derived
  decorated = decorate(token)
preset seeded
  state
    token = "preset"
  boot
    run every request(1) -> future_loaded(_, token, decorated, 1, "preset") | future_failed(_, token, 1, "preset")
on start(id)
  let local = decorate(token)
  parallel
    run every request(id) -> future_loaded(_, token, decorated, id, local) | future_failed(_, token, id, local)
    task cached(id) -> task_loaded(_, token, decorated, id, local) | task_failed(_, token, id, local)
on future_loaded(value, state_value, derived_value, param_value, local_value)
on future_failed(error, state_value, param_value, local_value)
on task_loaded(value, state_value, derived_value, param_value, local_value)
on task_failed(error, state_value, param_value, local_value)
view
  button "Start" -> start(1)
"#;

    let error = analyze(source).err();
    assert!(
        error.is_none(),
        "Future and Task completion routes should snapshot launch-time values: {error:?}"
    );
}

#[test]
fn infers_completion_snapshot_parameters_independent_of_handler_order() {
    let reversed = r#"app SnapshotOrder
extern crate::backend
  request() -> str
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
on middle(context)
  run every request() -> finished(context, _)
on start(context)
  run every request() -> middle(context)
on finished(context, value)
view
  button "Start" -> start("launch")
"#;
    let forward = reversed.replace(
        "on middle(context)\n  run every request() -> finished(context, _)\non start(context)\n  run every request() -> middle(context)",
        "on start(context)\n  run every request() -> middle(context)\non middle(context)\n  run every request() -> finished(context, _)",
    );

    assert!(
        analyze(&forward).is_ok(),
        "control order should infer the forwarded snapshot parameter"
    );
    let error = analyze(reversed).err();
    assert!(
        error.is_none(),
        "handler declaration order must not affect snapshot parameter inference: {error:?}"
    );
}

#[test]
#[ignore = "explicit reverse handler signature propagation linearity contract"]
fn performance_contract_four_thousand_reverse_handler_routes_use_a_worklist() {
    use std::fmt::Write as _;

    const HANDLERS: usize = 4_000;
    let mut source = String::from(
        r#"app SnapshotOrderPerf
extern crate::backend
  request() -> str
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
"#,
    );
    for index in (0..HANDLERS).rev() {
        let target = if index + 1 == HANDLERS {
            "finished".to_owned()
        } else {
            format!("step_{}", index + 1)
        };
        writeln!(
            source,
            "on step_{index}(context)\n  run every request() -> {target}(context)"
        )
        .unwrap();
    }
    source.push_str("on finished(context)\nview\n  button \"Start\" -> step_0(\"launch\")\n");

    crate::check::reset_handler_signature_worklist_visits();
    analyze(&source).unwrap();
    assert_eq!(
        crate::check::handler_signature_worklist_visits(),
        HANDLERS * 2 + 1,
        "reverse chains must revisit only the handler whose signature gained evidence"
    );
}

#[test]
fn completion_route_snapshots_include_component_state_but_not_props() {
    let source = r#"app ComponentSnapshots
extern crate::backend
  request(id:i64) -> str
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
component Snapshot(label:str)
  state
    token = "initial"
  on start(id)
    let local = token
    run every request(id) -> loaded(_, token, id, local)
  on loaded(value, state_value, param_value, local_value)
  button "Start" -> start(1)
view
  Snapshot label="Snapshot"
"#;

    let error = analyze(source).err();
    assert!(
        error.is_none(),
        "component completion routes should snapshot component state and handler locals: {error:?}"
    );

    let error =
        analyze(&source.replace("loaded(_, token, id, local)", "loaded(_, label, id, local)"))
            .unwrap_err();
    assert_eq!(error.code, "E150");
    assert_eq!(error.message, "unknown value `label`");
}

#[test]
fn checks_completion_route_snapshot_purity_and_cloneability() {
    let source = r#"app SnapshotValues
extern crate::backend
  Token(value:i64)
  sync runtime_token() -> str
  pure opaque_token() -> Token
  request() -> str
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
  handle:task-handle? = none
on start
  let captured = runtime_token()
  let window = window_id.unique()
  parallel
    run every request() -> accepted_sync(captured)
    run every request() -> accepted_opaque(opaque_token())
    run every request() -> accepted_handle(captured)
    run every request() -> accepted_window(window)
on accepted_sync(value)
on accepted_opaque(value)
on accepted_handle(value)
on accepted_window(value)
view
  button "Start" -> start
"#;

    analyze(source).unwrap();

    let error = analyze(&source.replace(
        "accepted_window(window)",
        "accepted_window(window_id.unique())",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E152");
    assert_eq!(
        error.message,
        "completion route expression cannot call recomputation-unsafe builtin `window_id.unique`"
    );
    assert_eq!(
        error.hint.as_deref(),
        Some("evaluate `window_id.unique(...)` in an earlier handler `let` and route that local")
    );

    let error = analyze(&source.replace("accepted_handle(captured)", "accepted_handle(handle)"))
        .unwrap_err();
    assert_eq!(error.code, "E152");
    assert_eq!(
        error.message,
        "completion route expression must produce ordinary cloneable Ice data, got `task-handle?`"
    );
}

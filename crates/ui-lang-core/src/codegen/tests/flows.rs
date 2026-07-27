use super::*;

#[test]
fn lowers_derived_values_to_getters_and_handler_locals_to_rust_lets() {
    let source = r#"app Derived
extern crate::backend
  save(title:str) -> unit
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
  draft = ""
  loading = false
derived
  normalized = trim(draft)
  can_submit = !loading && !empty(normalized)
on submit
  let title = normalized
  return if !can_submit
  run save(title) -> saved
on saved
view
  button "Save" disabled=!can_submit -> submit
"#;
    let generated = compile(source, "derived.ice").unwrap();
    assert!(generated.contains("fn __ice_derived_normalized(&self) -> ::std::string::String"));
    assert!(generated.contains("fn __ice_derived_can_submit(&self) -> bool"));
    assert!(generated.contains("let title = Self::__ice_derived_normalized(self);"));
    assert!(generated.contains("crate::backend::save(title.clone())"));
}

#[test]
fn lowers_structured_task_groups_to_native_combinators() {
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
on start
  parallel
    task system theme -> theme_read _
    sequential
      task clipboard read -> clipboard_read _
      task system info -> info_read _
on theme_read(next)
on clipboard_read(next)
on info_read(info)
view
  text "Tasks"
"#;
    let generated = compile(source, "grouped.ice").unwrap();
    assert!(generated.contains("return ::iced::Task::batch(["));
    assert!(generated.contains("::iced::Task::none().chain({"));
    assert!(generated.contains(".chain({ ::iced::system::information()"));
    assert!(generated.contains("fn __ice_system_info"));
}

#[test]
fn lowers_native_task_cancellation() {
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
on start
  abortable request abort-on-drop
    task system theme -> loaded _
on loaded(next)
on cancel
  abort request
view
  col
    if aborted(request)
      text "Canceled"
"#;
    let generated = compile(source, "cancel.ice").unwrap();
    assert!(generated.contains("pub(crate) request: ::std::option::Option<::iced::task::Handle>"));
    assert!(generated.contains("let (__task, __handle) = ({"));
    assert!(generated.contains("}).abortable()"));
    assert!(generated.contains("Some(__handle.abort_on_drop())"));
    assert!(generated.contains("__handle.abort()"));
    assert!(generated.contains("is_some_and(::iced::task::Handle::is_aborted)"));
}

#[test]
fn lowers_typed_task_streams() {
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
    stream numbers(3) -> number _
    stream fallible() -> text _ | failed _
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
    let generated = compile(source, "streams.ice").unwrap();
    assert!(generated.contains("fn __ui_lang_check_stream_numbers"));
    assert!(generated.contains("Task::run(crate::backend::numbers(arg0), |value| value)"));
    assert!(generated.contains("Task::run(crate::backend::numbers(3), |value|"));
    assert!(generated.contains("Task::run(crate::backend::fallible(), |result| match result"));
    assert!(generated.contains("Result::Err(error) => __StreamsMessage::Failed(error)"));
    assert!(generated.contains(
            "Subscription::run(crate::backend::fallible).map(move |__value| __StreamsMessage::Observed(__value))"
        ));
    assert!(generated.contains(
        "Subscription::run_with(3, |__data: &i64| crate::backend::numbers(__data.clone()))"
    ));
    assert!(generated.contains(
            "Subscription::run_with((1, 3,), |__data: &(i64, i64,)| crate::backend::range(__data.0.clone(), __data.1.clone()))"
        ));
    assert!(generated.contains("fn __ui_lang_check_recipe_snapshot"));
    assert!(generated.contains(
            "advanced::subscription::from_recipe(crate::backend::snapshot(3)).map(move |__value| __StreamsMessage::Text(__value))"
        ));
    assert!(generated.contains("fn __ui_lang_check_event_filter_raw_event"));
    assert!(generated.contains(
            "advanced::subscription::from_recipe(__IceEventFilterRawEvent { id: 3 }).map(move |__value| __StreamsMessage::Text(__value))"
        ));
}

#[test]
fn lowers_typed_task_sips() {
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
    let generated = compile(source, "sips.ice").unwrap();
    assert!(generated.contains("fn __ui_lang_check_sip_transfer"));
    assert!(generated.contains("let _: f64 = value"));
    assert!(generated.contains("Task::sip(crate::backend::transfer(3), |value|"));
    assert!(generated.contains("Task::sip(crate::backend::fallible(), |value|"));
    assert!(generated.contains("Result::Err(error) => __SipsMessage::Failed(error)"));
}

#[test]
fn lowers_structured_task_flows() {
    let source = r#"app Flows
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
  task fallible(value:i64) -> i64 ! AppError
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
    flow
      from stream numbers(3)
      map value -> value + 1
      then value -> task double(value)
      collect
      done -> collected _
      units -> planned _
    flow
      from task fallible(2)
      map value -> value + 1
      try value -> task fallible(value)
      done -> finished _
      error -> failed _
    flow
      from stream numbers(1)
      discard
on collected(values)
on planned(units)
on finished(value)
on failed(error)
view
  text "Flows"
"#;
    let generated = compile(source, "flows.ice").unwrap();
    assert!(generated.contains("Task::run(crate::backend::numbers(3), |value| value)"));
    assert!(generated.contains(".map(move |value| (value + 1))"));
    assert!(generated.contains(".then(move |value| crate::backend::double(value))"));
    assert!(generated.contains(".map(move |result| result.map(|value| (value + 1)))"));
    assert!(generated.contains(".and_then(move |value| crate::backend::fallible(value))"));
    assert!(generated.contains(".collect()"));
    assert!(generated.contains(".discard::<__FlowsMessage>()"));
    assert!(generated.contains("i64::try_from(__task.units())"));
}

#[test]
fn lowers_task_error_mapping_and_native_sources() {
    let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize(error:NetworkError) -> AppError
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
    let generated = compile(source, "errors.ice").unwrap();
    assert!(generated.contains("fn __ui_lang_check_sync_normalize"));
    assert!(
        generated.contains(".map_err(move |reason| crate::backend::normalize(reason.clone()))")
    );
    assert!(generated.contains(".collect()"));
    assert!(generated.contains("Task::done(1)"));
    assert!(generated.contains("Task::done((value + 1))"));
    assert!(generated.contains("Task::<i64>::none()"));
    assert!(generated.contains("Vec<::std::result::Result<i64, crate::backend::AppError>>"));
}

use super::*;

const THEME: &str = r#"theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
"#;

fn generated_line<'a>(generated: &'a str, needle: &str) -> &'a str {
    generated
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("missing generated line containing `{needle}`"))
}

fn first_argument<'a>(line: &'a str, variant: &str) -> &'a str {
    line.split_once(variant)
        .unwrap_or_else(|| panic!("missing generated variant `{variant}`"))
        .1
        .split_once(',')
        .expect("snapshot route must have a payload after its first argument")
        .0
}

#[test]
fn snapshots_future_and_task_route_expressions_before_the_mapper() {
    let source = format!(
        r#"app RouteSnapshots
extern crate::backend
  AppError(message:str)
  fetch(value:str) -> str ! AppError
  task cached(value:str) -> str ! AppError
{THEME}on future(seed)
  let context = "future"
  run every fetch(trim(seed)) -> future_loaded(context, _) | future_failed("future-error", _)
on cached(seed)
  let context = "cached"
  task cached(trim(seed)) -> cached_loaded(context, _) | cached_failed("cached-error", _)
on builtin(seed)
  let context = "builtin"
  task system theme -> themed(context, _)
on future_loaded(context, value)
on future_failed(context, error)
on cached_loaded(context, value)
on cached_failed(context, error)
on themed(context, value)
view
  col
    button "Future" -> future(" future ")
    button "Task" -> cached(" cached ")
    button "Builtin" -> builtin(" builtin ")
"#
    );

    let generated = compile(&source, "route_snapshots.ice").unwrap();

    assert_eq!(generated.matches("let __ice_run_route_").count(), 5);
    assert!(generated.contains("::iced::Task::perform(({ let __ice_call = ::ui_lang_runtime::dev::Span::extern_call(\"fetch\", \"route_snapshots.ice:4\"); crate::backend::fetch("));
    assert!(generated.contains("), move |result| match result"));
    assert!(generated.contains("crate::backend::cached("));
    assert!(generated.contains(").map(move |result| match result"));
    assert!(generated.contains("::iced::system::theme().map(__ice_system_theme).map(move |value|"));

    let future_success = generated_line(&generated, "FutureLoaded(__ice_run_route_");
    let future_error = generated_line(&generated, "FutureFailed(__ice_run_route_");
    assert!(!future_success.contains(".clone()"));
    assert!(!future_error.contains(".clone()"));
    assert!(future_success.contains(", value)"));
    assert!(future_error.contains(", error)"));
    let success_snapshot = first_argument(future_success, "FutureLoaded(");
    let error_snapshot = first_argument(future_error, "FutureFailed(");
    assert_ne!(success_snapshot, error_snapshot);
    assert!(generated.contains(&format!("let {success_snapshot} = context.to_owned();")));
    assert!(generated.contains(&format!(
        "let {error_snapshot} = \"future-error\".to_owned();"
    )));

    let task_success = generated_line(&generated, "CachedLoaded(__ice_run_route_");
    let task_error = generated_line(&generated, "CachedFailed(__ice_run_route_");
    let builtin = generated_line(&generated, "Themed(__ice_run_route_");
    assert!(task_success.contains(".clone(), value)"));
    assert!(task_error.contains(".clone(), error)"));
    assert!(builtin.contains(".clone(), value)"));

    let first_snapshot = generated.find("let __ice_run_route_").unwrap();
    let future_mapper = generated
        .find("::iced::Task::perform(({ let __ice_call")
        .unwrap();
    assert!(first_snapshot < future_mapper);
}

#[test]
fn snapshots_both_routes_of_a_fallible_builtin_task() {
    let source = format!(
        r#"app BuiltinSnapshots
{THEME}state
  handle:image = rgba(1, 1, bytes(ff 00 ff ff))
on allocate
  let ready_context = "ready"
  let failed_context = "failed"
  task image allocate handle -> ready(ready_context, _) | failed(failed_context, _)
on ready(context, value)
on failed(context, error)
view
  button "Allocate" -> allocate
"#
    );

    let generated = compile(&source, "builtin_snapshots.ice").unwrap();

    assert_eq!(generated.matches("let __ice_run_route_").count(), 2);
    assert!(
        generated
            .contains("::iced::widget::image::allocate(self.handle.clone()).map(move |result|")
    );
    assert!(generated_line(&generated, "Ready(__ice_run_route_").contains(".clone(), value)"));
    assert!(generated_line(&generated, "Failed(__ice_run_route_").contains(".clone(), error)"));
    let ready = first_argument(
        generated_line(&generated, "Ready(__ice_run_route_"),
        "Ready(",
    )
    .trim_end_matches(".clone()");
    let failed = first_argument(
        generated_line(&generated, "Failed(__ice_run_route_"),
        "Failed(",
    )
    .trim_end_matches(".clone()");
    assert_ne!(ready, failed);
    assert!(generated.contains(&format!("let {ready} = ready_context.to_owned();")));
    assert!(generated.contains(&format!("let {failed} = failed_context.to_owned();")));
}

#[test]
fn keeps_component_scope_and_lane_wrappers_around_snapshotted_runs() {
    let source = format!(
        r#"app ScopedSnapshots
extern crate::backend
  AppError(message:str)
  fetch(value:str) -> str ! AppError
{THEME}component Search()
  state
    query = ""
  on search(seed)
    let context = "component"
    run latest lane=request fetch(query) -> loaded(query, _) | failed(context, _)
  on loaded(context, value)
    query = value
  on failed(context, error)
    query = context
  button "Search" -> search(" launch ")
view
  Search #search
"#
    );

    let generated = compile(&source, "scoped_snapshots.ice").unwrap();
    assert_eq!(generated.matches("let __ice_run_route_").count(), 2);
    assert!(generated.lines().any(|line| {
        line.contains("let __ice_run_route_") && line.contains("= __local.query.to_owned();")
    }));
    assert!(generated.contains("let __ice_lane_scope_"));
    assert!(generated.contains("let __ice_run_scope_"));
    assert!(generated.contains("crate::backend::fetch(__local.query.to_owned())"));
    assert!(generated.contains("move |result| match result"));
    assert!(generated.contains("__ScopedSnapshotsMessage::__RequestLane0(__ice_lane_scope_"));
    assert!(
        generated_line(&generated, "__SearchHandleLoaded((__ice_run_scope_")
            .contains("__ice_run_route_")
    );
    assert!(
        generated_line(&generated, "__SearchHandleFailed((__ice_run_scope_")
            .contains("__ice_run_route_")
    );
}

#[test]
fn snapshots_each_nested_every_run_leaf_without_adding_request_lane_state() {
    let source = format!(
        r#"app NestedSnapshots
extern crate::backend
  fetch(value:str) -> str
  task cached(value:str) -> str
  stream events() -> str
{THEME}on start(seed)
  let context = trim(seed)
  parallel
    run every fetch(context) -> loaded(context, _)
    sequential
      task cached(context) -> loaded(context, _)
      run every fetch(context) -> loaded(context, _)
    stream every events() -> observed _
on loaded(context, value)
on observed(value)
view
  button "Start" -> start(" launch ")
"#
    );

    let generated = compile(&source, "nested_snapshots.ice").unwrap();

    assert_eq!(generated.matches("let __ice_run_route_").count(), 3);
    assert!(generated.contains("return ::iced::Task::batch(["));
    assert!(generated.contains("::iced::Task::none().chain({"));
    assert_eq!(generated.matches("move |value|").count(), 3);
    assert!(generated.contains("::iced::Task::run(crate::backend::events(), |value|"));
    assert!(!generated.contains("__ice_run_lane_"));
    assert!(!generated.contains("__RequestLane"));
}

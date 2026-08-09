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

fn item_body<'a>(generated: &'a str, header: &str) -> &'a str {
    generated
        .split_once(header)
        .unwrap_or_else(|| panic!("missing generated item `{header}`"))
        .1
        .split_once("\n}")
        .expect("generated item must close")
        .0
}

#[test]
fn shares_one_app_request_lane_across_handlers_and_presets() {
    let source = format!(
        r#"app SharedLane
extern crate::backend
  fetch(value:i64) -> i64
{THEME}state
  value = 0
preset seeded
  boot
    run latest lane=request fetch(0) -> loaded _
preset isolated
  boot
    run latest lane=preset_only fetch(1) -> loaded _
on search
  run latest lane=request fetch(value) -> loaded _
on refresh
  run latest lane=request fetch(value) -> loaded _
on loaded(next)
  value = next
view
  text value
"#
    );

    let generated = compile(&source, "shared_lane.ice").unwrap();
    let state = item_body(&generated, "pub struct SharedLane {");
    let messages = item_body(&generated, "pub(crate) enum __SharedLaneMessage {");

    assert_eq!(
        state
            .matches("pub(crate) __ice_run_lane_0_generation: u64,")
            .count(),
        1
    );
    assert!(!state.contains("__ice_run_lane_0_handle"));
    assert!(state.contains("pub(crate) __ice_run_lane_1_generation: u64,"));
    assert_eq!(messages.matches("__RequestLane0(").count(), 1);
    assert_eq!(messages.matches("__RequestLane1(").count(), 1);
    assert_eq!(
        generated
            .matches("self.__ice_run_lane_0_generation.wrapping_add(1)")
            .count(),
        3
    );
    assert!(generated.contains("fn __preset_task_0(&mut self)"));
    assert!(generated.contains("fn __preset_task_1(&mut self)"));
    assert_eq!(
        generated
            .matches("self.__ice_run_lane_1_generation.wrapping_add(1)")
            .count(),
        1
    );
    assert!(generated.contains(
        "__SharedLaneMessage::__RequestLane0(__generation, ::std::boxed::Box::new(__message))"
    ));
}

#[test]
fn wraps_named_lanes_at_nested_run_leaves() {
    let source = format!(
        r#"app NestedLanes
extern crate::backend
  fetch(value:i64) -> i64
{THEME}state
  value = 0
  cancellation:task-handle? = none
on start
  parallel
    run latest lane=parallel_request fetch(value) -> loaded _
    sequential
      run latest lane=sequential_request fetch(value) -> loaded _
      abortable cancellation
        run replace lane=abortable_request fetch(value) -> loaded _
on loaded(next)
  value = next
view
  text value
"#
    );

    let generated = compile(&source, "nested_lanes.ice").unwrap();
    let state = item_body(&generated, "pub struct NestedLanes {");

    assert!(generated.contains("return ::iced::Task::batch(["));
    assert!(generated.contains("::iced::Task::none().chain({"));
    assert!(generated.matches(".abortable();").count() >= 2);
    assert_eq!(state.matches("__ice_run_lane_").count(), 4);
    for lane in 0..3 {
        assert!(generated.contains(&format!("__RequestLane{lane}")));
        assert!(generated.contains(&format!(
            "self.__ice_run_lane_{lane}_generation.wrapping_add(1)"
        )));
    }
    assert!(generated.contains("self.__ice_run_lane_2_handle.replace(__handle.abort_on_drop())"));
}

#[test]
fn shares_one_component_lane_across_handlers_and_keeps_instance_scope() {
    let source = format!(
        r#"app ComponentLanes
extern crate::backend
  fetch(value:i64) -> i64
{THEME}component Search()
  state
    value = 0
  on search
    run latest lane=request fetch(value) -> loaded _
  on retry
    run latest lane=request fetch(value) -> loaded _
  on loaded(next)
    value = next
  col
    button "Search" -> search
    button "Retry" -> retry
view
  col
    Search #first
    Search #second
"#
    );

    let generated = compile(&source, "component_lanes.ice").unwrap();
    let local = item_body(&generated, "pub(crate) struct __IceSearchState {");
    let messages = item_body(&generated, "pub(crate) enum __ComponentLanesMessage {");

    assert_eq!(
        local.matches("__ice_run_lane_0_generation: u64,").count(),
        1
    );
    assert_eq!(messages.matches("__RequestLane0(").count(), 1);
    assert!(messages.contains("::std::string::String, u64"));
    assert_eq!(
        generated
            .matches("__ComponentLanesMessage::__RequestLane0(__ice_lane_scope_")
            .count(),
        2
    );
    assert!(
        generated.contains("self.__ice_component_search.get_mut(&__scope).is_some_and(|__local|")
    );
}

#[test]
fn every_run_emits_no_request_lane_state() {
    let source = format!(
        r#"app EveryRun
extern crate::backend
  fetch(value:i64) -> i64
{THEME}state
  value = 0
on search
  run every fetch(value) -> loaded _
on loaded(next)
  value = next
view
  text value
"#
    );

    let generated = compile(&source, "every_run.ice").unwrap();

    assert!(!generated.contains("__ice_run_lane_"));
    assert!(!generated.contains("__RequestLane"));
}

#[test]
fn replace_lane_aborts_previous_work_and_clears_the_matching_handle() {
    let source = format!(
        r#"app ReplaceLane
extern crate::backend
  fetch(value:i64) -> i64
{THEME}state
  value = 0
on search
  run replace lane=request fetch(value) -> loaded _
on retry
  run replace lane=request fetch(value) -> loaded _
on loaded(next)
  value = next
view
  text value
"#
    );

    let generated = compile(&source, "replace_lane.ice").unwrap();
    let state = item_body(&generated, "pub struct ReplaceLane {");
    let messages = item_body(&generated, "pub(crate) enum __ReplaceLaneMessage {");

    assert_eq!(
        state
            .matches("pub(crate) __ice_run_lane_0_generation: u64,")
            .count(),
        1
    );
    assert_eq!(
        state.matches("pub(crate) __ice_run_lane_0_handle:").count(),
        1
    );
    assert_eq!(messages.matches("__RequestLane0(").count(), 1);
    assert_eq!(generated.matches("__previous.abort();").count(), 2);
    assert!(generated.contains(
        "self.__ice_run_lane_0_handle = ::std::option::Option::None; return self.__update(*__message)"
    ));
}

#[test]
fn invalidates_shared_app_and_preset_lanes_in_place() {
    let source = format!(
        r#"app InvalidateAppLanes
extern crate::backend
  fetch(value:i64) -> i64
{THEME}state
  value = 0
preset seeded
  boot
    run latest lane=request fetch(0) -> loaded _
on start_latest
  run latest lane=request fetch(value) -> loaded _
on invalidate_latest
  invalidate lane=request
on start_replace
  run replace lane=preview fetch(value) -> loaded _
on invalidate_replace
  invalidate lane=preview
on loaded(next)
  value = next
view
  text value
"#
    );

    let generated = compile(&source, "invalidate_app_lanes.ice").unwrap();
    let state = item_body(&generated, "pub struct InvalidateAppLanes {");
    let messages = item_body(&generated, "pub(crate) enum __InvalidateAppLanesMessage {");

    assert_eq!(
        state
            .matches("pub(crate) __ice_run_lane_0_generation: u64,")
            .count(),
        1
    );
    assert_eq!(
        state
            .matches("pub(crate) __ice_run_lane_1_generation: u64,")
            .count(),
        1
    );
    assert_eq!(state.matches("__ice_run_lane_1_handle:").count(), 1);
    assert_eq!(messages.matches("__RequestLane0(").count(), 1);
    assert_eq!(messages.matches("__RequestLane1(").count(), 1);
    assert_eq!(
        generated
            .matches("self.__ice_run_lane_0_generation = self.__ice_run_lane_0_generation.wrapping_add(1);")
            .count(),
        3
    );
    assert_eq!(
        generated
            .matches("self.__ice_run_lane_1_generation = self.__ice_run_lane_1_generation.wrapping_add(1);")
            .count(),
        2
    );
    assert!(generated.contains(
        "if let ::std::option::Option::Some(__previous) = self.__ice_run_lane_1_handle.take() { __previous.abort(); }"
    ));
}

#[test]
fn invalidates_component_lanes_in_the_current_instance() {
    let source = format!(
        r#"app InvalidateComponentLanes
extern crate::backend
  fetch(value:i64) -> i64
{THEME}component Retained()
  state
    value = 0
  on start
    run replace lane=request fetch(value) -> loaded _
  on invalidate_request
    invalidate lane=request
  on loaded(next)
    value = next
  button "Start retained" -> start
component Mounted()
  lifetime mounted
  state
    value = 0
  on start
    run latest lane=request fetch(value) -> loaded _
  on invalidate_request
    invalidate lane=request
  on loaded(next)
    value = next
  button "Start mounted" -> start
view
  col
    Retained #retained
    Mounted #mounted
"#
    );

    let generated = compile(&source, "invalidate_component_lanes.ice").unwrap();
    let retained = item_body(&generated, "pub(crate) struct __IceRetainedState {");
    let mounted = item_body(&generated, "pub(crate) struct __IceMountedState {");

    assert_eq!(
        retained
            .matches("__ice_run_lane_0_generation: u64,")
            .count(),
        1
    );
    assert_eq!(retained.matches("__ice_run_lane_0_handle:").count(), 1);
    assert_eq!(
        mounted.matches("__ice_run_lane_1_generation: u64,").count(),
        1
    );
    assert_eq!(
        generated
            .matches("__local.__ice_run_lane_0_generation = __local.__ice_run_lane_0_generation.wrapping_add(1);")
            .count(),
        2
    );
    assert!(generated.contains(
        "if let ::std::option::Option::Some(__previous) = __local.__ice_run_lane_0_handle.take() { __previous.abort(); }"
    ));
    assert_eq!(
        generated
            .matches("__local.__ice_run_lane_1_generation = self.__ice_component_mounted.next_generation();")
            .count(),
        1
    );
}

#[test]
fn a_daemon_invalidates_one_shared_root_lane() {
    let source = format!(
        r#"daemon LaneDaemon
  title "Lane daemon"
  window dashboard
    size 320 240
extern crate::backend
  fetch(value:i64) -> i64
{THEME}state
  value = 0
on start
  run latest lane=request fetch(value) -> loaded _
on invalidate_request
  invalidate lane=request
on loaded(next)
  value = next
view
  text value
"#
    );

    let generated = compile(&source, "invalidate_daemon_lane.ice").unwrap();
    let state = item_body(&generated, "pub struct LaneDaemon {");

    assert_eq!(
        state
            .matches("pub(crate) __ice_run_lane_0_generation: u64,")
            .count(),
        1
    );
    assert_eq!(
        generated
            .matches("self.__ice_run_lane_0_generation = self.__ice_run_lane_0_generation.wrapping_add(1);")
            .count(),
        2
    );
}

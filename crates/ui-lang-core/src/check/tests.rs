use crate::test_support::example;
use crate::{PaneConfiguration, Type, ViewNode, analyze};

#[path = "tests/components.rs"]
mod components;
#[path = "tests/events.rs"]
mod events;
#[path = "tests/native.rs"]
mod native;
#[path = "tests/platform.rs"]
mod platform;
#[path = "tests/sum_types.rs"]
mod sum_types;
#[path = "tests/tasks.rs"]
mod tasks;
#[path = "tests/testing.rs"]
mod testing;
#[path = "tests/widgets.rs"]
mod widgets;

const THEMED_APP: &str = r#"app Demo
  palette active_palette
theme contract Ducktape
  bg
  fg
  primary
  danger
  surface
palette light for Ducktape
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
  surface #f4f4f4
palette dark for Ducktape
  bg #111111
  fg #ffffff
  primary #88aaff
  danger #ff6677
  surface #222222
state
  active_palette:palette[Ducktape] = Ducktape.light
view
  box bg=surface
    text "Theme"
"#;

fn warning_app(body: &str) -> String {
    format!(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n{body}"
    )
}

#[test]
fn checks_complete_dynamic_palettes() {
    analyze(THEMED_APP).unwrap();
}

#[test]
fn exhaustively_matches_nominal_palettes() {
    let source = THEMED_APP.replace(
        "  box bg=surface\n    text \"Theme\"",
        "  match active_palette\n    Ducktape.light\n      text \"Light\"\n    Ducktape.dark\n      text \"Dark\"",
    );
    analyze(&source).unwrap();

    let error =
        analyze(&source.replace("    Ducktape.dark\n      text \"Dark\"\n", "")).unwrap_err();
    assert_eq!(error.code, "E195");
    assert!(error.message.contains("Ducktape.dark"));
}

#[test]
fn rejects_invalid_theme_contracts_and_palettes() {
    for (source, message) in [
        (
            THEMED_APP.replace("  danger\n  surface", "  surface"),
            "missing `danger`",
        ),
        (
            THEMED_APP.replace("palette light for Ducktape", "palette light for Other"),
            "not `Ducktape`",
        ),
        (
            THEMED_APP.replace("  surface #f4f4f4", "  accent #f4f4f4"),
            "unknown token `accent`",
        ),
        (
            THEMED_APP.replace("  surface #f4f4f4\npalette dark", "palette dark"),
            "missing token `surface`",
        ),
        (
            THEMED_APP.replace(
                "  active_palette:palette[Ducktape] = Ducktape.light",
                "  active_palette:palette[Ducktape] = true",
            ),
            "expected `palette[Ducktape]`, got `bool`",
        ),
        (
            THEMED_APP.replace("Ducktape.light", "\"light\""),
            "expected `palette[Ducktape]`, got `str`",
        ),
        (
            THEMED_APP.replace("palette active_palette", "palette Ducktape.missing"),
            "has no palette `missing`",
        ),
    ] {
        let error = analyze(&source).unwrap_err();
        assert!(
            error.message.contains(message),
            "{}: {}",
            error.code,
            error.message
        );
    }
}

#[test]
fn rejects_duplicate_or_non_color_palette_entries_during_parsing() {
    for (source, message) in [
        (
            THEMED_APP.replace("  surface #f4f4f4", "  surface #f4f4f4\n  surface #eeeeee"),
            "duplicate palette token `surface`",
        ),
        (
            THEMED_APP.replace("  surface #f4f4f4", "  surface blue"),
            "palette colors use #RRGGBB or #RRGGBBAA",
        ),
        (
            THEMED_APP.replace(
                "palette light for Ducktape",
                "palette light for Ducktape\npalette light for Ducktape",
            ),
            "duplicate palette `light`",
        ),
    ] {
        let error = analyze(&source).unwrap_err();
        assert!(
            error.message.contains(message),
            "{}: {}",
            error.code,
            error.message
        );
    }
}

#[test]
fn rejects_invalid_constant_integer_arithmetic() {
    for (expression, message) in [
        ("1 / 0", "non-zero divisor"),
        ("1 % -0", "non-zero divisor"),
        ("1 / (2 - 2)", "non-zero divisor"),
        ("9223372036854775807 + 1", "overflows"),
        ("-9223372036854775808 / -1", "overflows"),
    ] {
        let source = example!("component_state.ice")
            .replace("count = 0", &format!("count:i64 = {expression}"));
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E153");
        assert!(error.message.contains(message));
    }

    let error = analyze(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  value = 1\nview\n  text (value / (1 - 1))\n",
    )
    .unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("non-zero divisor"));
}

#[test]
fn rejects_duplicate_handler_parameters() {
    for source in [
        r#"app Demo
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
on pressed(value, value)
view
  button "ok" -> pressed(1, 2)
"#,
        r#"app Demo
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
component Card()
  on pressed(value, value)
  button "ok" -> pressed(1, 2)
view
  Card
"#,
    ] {
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E100");
        assert!(
            error
                .message
                .contains("duplicate handler parameter `value`")
        );
    }
}

#[test]
fn checks_derived_values_and_immutable_handler_locals() {
    let source = r#"app Demo
extern crate::backend
  sync normalize(value:str) -> str
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
  draft = ""
view
  col
    input "Draft" <-> draft
    button "Save" disabled=!can_submit -> submit
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.derived[0].ty, Type::Str);
    assert_eq!(document.derived[1].ty, Type::Bool);

    let forward = source.replace(
        "normalized = trim(draft)\n  can_submit = !loading && !empty(normalized)",
        "can_submit = !loading && !empty(normalized)\n  normalized = trim(draft)",
    );
    analyze(&forward).unwrap();

    let cycle = source.replace(
        "normalized = trim(draft)\n  can_submit = !loading && !empty(normalized)",
        "normalized = can_submit\n  can_submit = normalized",
    );
    let error = analyze(&cycle).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("dependency cycle"));

    let impure = source.replace("normalized = trim(draft)", "normalized = normalize(draft)");
    let error = analyze(&impure).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("pure Ice expression"));

    let shadow = source.replace("let title = normalized", "let draft = normalized");
    let error = analyze(&shadow).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("shadows an existing value"));

    let duplicate_local = source.replace(
        "let title = normalized",
        "let title = normalized\n  let title = normalized",
    );
    let error = analyze(&duplicate_local).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("shadows an existing value"));

    let parameter_shadow = source
        .replace("on submit\n", "on submit(value)\n")
        .replace("let title = normalized", "let value = normalized")
        .replace("-> submit\n", "-> submit(draft)\n");
    let error = analyze(&parameter_shadow).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("shadows an existing value"));

    let assignment = source.replace("draft = \"\"\nview", "can_submit = false\nview");
    let error = analyze(&assignment).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("not writable state"));

    let binding = source.replace("<-> draft", "<-> normalized");
    let error = analyze(&binding).unwrap_err();
    assert!(
        error.message.contains("writable")
            || error.message.contains("state binding")
            || error.message.contains("app state"),
        "{}",
        error.message
    );
}

#[test]
fn warns_for_unreachable_component_graphs() {
    let document = analyze(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Used()\n  text \"Used\"\ncomponent Hidden()\n  HiddenLeaf\ncomponent HiddenLeaf()\n  text \"Hidden\"\ncomponent TestOnly()\n  text \"Test only\"\nview\n  Used\ntest mounts_component\n  mount\n    TestOnly\n",
    )
    .unwrap();
    let warnings = document.warnings();
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.code == "W001")
            .map(|warning| warning.line)
            .collect::<Vec<_>>(),
        [14, 16]
    );
    assert!(
        warnings
            .iter()
            .all(|warning| !warning.message.contains("Used` is unreachable")
                && !warning.message.contains("TestOnly` is unreachable"))
    );
}

#[test]
fn warns_for_positional_and_unbounded_component_identity() {
    let document = analyze(&warning_app(
        r#"state
  items = [1, 2]
  selected = 1
component RetainedRow()
  state
    count = 0
  on increment
    count = count + 1
  button "Increment" -> increment
component MountedRow()
  lifetime mounted
  state
    count = 0
  on increment
    count = count + 1
  button "Increment" -> increment
view
  col
    for item in items
      RetainedRow
    keyed item in items by=item
      RetainedRow
    keyed item in items by=item
      MountedRow
    RetainedRow #selected(selected)
"#,
    ))
    .unwrap();
    let warnings = document.warnings();
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.code == "W008")
            .count(),
        1,
        "{warnings:?}"
    );
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.code == "W009")
            .count(),
        3,
        "{warnings:?}"
    );
    assert!(warnings.iter().all(|warning| {
        warning.code != "W008" && warning.code != "W009" || !warning.message.contains("MountedRow")
    }));
}

#[test]
fn warns_for_unused_bindings_noops_dead_statements_and_duplicate_subscriptions() {
    let document = analyze(&warning_app(
        r#"state
  total = 0
derived
  base = total + 1
  shown = base + 1
  abandoned = total + 2
on act(value, ignored)
  let next = total + 1
  let discarded = total + 2
  total = next
  total = total
  return if false
  return if true
  total = 1
on tick(now)
subscribe
  every 1s -> tick _
  every 1s -> tick _
view
  col
    text shown
    button "Act" -> act 1 2
    if false
      text "Dead"
"#,
    ))
    .unwrap();
    let warnings = document.warnings();
    for name in ["abandoned", "value", "ignored", "discarded", "now"] {
        assert!(
            warnings.iter().any(|warning| {
                warning.code == "W011" && warning.message.contains(&format!("`{name}`"))
            }),
            "missing unused warning for {name}: {warnings:?}"
        );
    }
    assert!(warnings.iter().all(|warning| {
        warning.code != "W011"
            || !["base", "shown", "next"]
                .iter()
                .any(|name| warning.message.contains(&format!("`{name}`")))
    }));
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.code == "W012")
            .count(),
        3,
        "{warnings:?}"
    );
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.code == "W013")
            .count(),
        1,
        "{warnings:?}"
    );
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.code == "W014")
            .count(),
        1,
        "{warnings:?}"
    );
}

#[test]
fn warns_for_state_without_readers_or_writers() {
    let document = analyze(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  read_only = 0\n  write_only = 0\n  healthy = 0\n  unused = 0\non mutate\n  write_only = 1\n  healthy = healthy + 1\nview\n  col\n    text read_only\n    text healthy\n    button \"Mutate\" -> mutate\n",
    )
    .unwrap();
    let warnings = document
        .warnings()
        .iter()
        .map(|warning| (warning.code, warning.message.as_str()))
        .collect::<Vec<_>>();
    assert!(
        warnings
            .iter()
            .any(|(code, message)| { *code == "W003" && message.contains("state `read_only`") })
    );
    assert!(warnings.iter().any(|(code, message)| {
        *code == "W002" && message.contains("state `write_only`") && message.contains("never read")
    }));
    assert!(warnings.iter().any(|(code, message)| {
        *code == "W002"
            && message.contains("state `unused`")
            && message.contains("never read or written")
    }));
    assert!(
        warnings
            .iter()
            .all(|(_, message)| !message.contains("state `healthy`"))
    );
}

#[test]
fn warns_for_immediate_handler_routing_cycles() {
    let document = analyze(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non refresh\n  flow\n    from done 1\n    done -> refresh\non first\n  flow\n    from done 1\n    then value -> done value + 1\n    done -> second\non second\n  flow\n    from done 1\n    done -> first\non empty\n  flow\n    from none i64\n    collect\n    done -> empty\nview\n  col\n    button \"Refresh\" -> refresh\n    button \"First\" -> first\n    button \"Empty\" -> empty\n",
    )
    .unwrap();
    let warnings = document
        .warnings()
        .iter()
        .filter(|warning| warning.code == "W004")
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 3);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.message.contains("`refresh`")
                && warning.message.contains("back to itself"))
    );
    assert!(warnings.iter().any(|warning| {
        warning.message.contains("`first`") && warning.message.contains("`second`")
    }));
    assert!(
        warnings
            .iter()
            .any(|warning| warning.message.contains("`empty`"))
    );
}

#[test]
fn distinguishes_guarded_immediate_cycles_from_task_completion_cycles() {
    let document = analyze(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  stopped = false\non guarded\n  return if stopped\n  flow\n    from done 1\n    done -> guarded\non poll\n  flow\n    from task system theme\n    done -> polled _\non polled(theme)\n  flow\n    from done theme\n    done -> poll\nview\n  col\n    button \"Guarded\" -> guarded\n    button \"Poll\" -> poll\n",
    )
    .unwrap();
    assert!(
        document
            .warnings()
            .iter()
            .all(|warning| warning.code != "W004")
    );
    assert!(
        document
            .warnings()
            .iter()
            .any(|warning| warning.code == "W006"
                && warning.message.contains("`poll`")
                && warning.message.contains("`polled`"))
    );
}

#[test]
fn warns_for_handlers_unreachable_from_runtime_and_test_roots() {
    let document = analyze(&warning_app(
        r#"on root
  flow
    from done 1
    done -> chained
on chained
on dead
component Used()
  on live
  on dead_local
  button "Live" -> live
view
  col
    button "Root" -> root
    Used
"#,
    ))
    .unwrap();
    let warnings = document
        .warnings()
        .iter()
        .filter(|warning| warning.code == "W005")
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(
        warnings
            .iter()
            .any(|message| message.contains("handler `dead`"))
    );
    assert!(
        warnings
            .iter()
            .any(|message| message.contains("component handler `Used.dead_local`"))
    );
    assert!(warnings.iter().all(|message| !message.contains("chained")));
}

#[test]
fn treats_mount_presets_and_test_dispatches_as_handler_roots() {
    let document = analyze(&warning_app(
        r#"preset ready
  boot
    flow
      from done 1
      done -> from_preset
on mount
on from_preset
on from_test
test routes
  dispatch from_test
view
  text "Ready"
"#,
    ))
    .unwrap();
    assert!(
        document
            .warnings()
            .iter()
            .all(|warning| warning.code != "W005"),
        "{:?}",
        document.warnings()
    );
}

#[test]
fn ignores_state_accesses_in_unreachable_handlers() {
    let document = analyze(&warning_app(
        r#"state
  observed = 0
  dead_only = 0
on dead_writer
  observed = 1
on dead_reader
  let value = dead_only
view
  text observed
"#,
    ))
    .unwrap();
    assert!(
        document.warnings().iter().any(|warning| {
            warning.code == "W003" && warning.message.contains("state `observed`")
        })
    );
    assert!(document.warnings().iter().any(|warning| {
        warning.code == "W002"
            && warning.message.contains("state `dead_only`")
            && warning.message.contains("never read or written")
    }));
}

#[test]
fn literal_false_does_not_hide_an_immediate_cycle() {
    let document = analyze(&warning_app(
        r#"on refresh
  return if false
  flow
    from done 1
    done -> refresh
view
  button "Refresh" -> refresh
"#,
    ))
    .unwrap();
    assert!(
        document
            .warnings()
            .iter()
            .any(|warning| warning.code == "W004")
    );
}

#[test]
fn warns_for_repeated_stream_feedback_cycles() {
    let document = analyze(&warning_app(
        r#"extern crate::backend
  stream ticks() -> i64
on start
  stream ticks() -> ticked _
on ticked(value)
  flow
    from done value
    done -> start
view
  button "Start" -> start
"#,
    ))
    .unwrap();
    assert!(document.warnings().iter().any(|warning| {
        warning.code == "W006"
            && warning.message.contains("`start`")
            && warning.message.contains("`ticked`")
            && warning.message.contains("multiply")
    }));
}

#[test]
fn warns_for_task_and_widget_query_completion_cycles() {
    let document = analyze(&warning_app(
        r#"extern crate::backend
  task load() -> i64
on load_next
  task load() -> loaded _
on loaded(value)
  flow
    from done value
    done -> load_next
on inspect
  task widget focused #control -> inspected _
on inspected(value)
  flow
    from done value
    done -> inspect
state
  field = ""
view
  col
    button "Load" -> load_next
    input "Field" #control <-> field
    button "Inspect" -> inspect
"#,
    ))
    .unwrap();
    let warnings = document
        .warnings()
        .iter()
        .filter(|warning| warning.code == "W006")
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings.iter().all(|warning| {
        warning
            .message
            .contains("future, task, or query completion")
            && warning.message.contains("refresh forever")
    }));
}

#[test]
fn warns_for_component_local_handler_cycles() {
    let document = analyze(&warning_app(
        r#"extern crate::backend
  fetch() -> i64
component Loader()
  on start
    run fetch() -> loaded _
  on loaded(value)
    run fetch() -> loaded _
  button "Load" -> start
view
  Loader
"#,
    ))
    .unwrap();
    assert!(
        document.warnings().iter().any(|warning| {
            warning.code == "W006" && warning.message.contains("`Loader.loaded`")
        })
    );
}

#[test]
fn warns_for_unfiltered_raw_event_feedback() {
    let source = warning_app(
        r#"on received(value)
subscribe
  event raw -> received _
view
  text "Events"
"#,
    );
    let document = analyze(&source).unwrap();
    assert!(
        document
            .warnings()
            .iter()
            .any(|warning| warning.code == "W007")
    );

    for safe in [
        source.replace("event raw ->", "event raw status=captured ->"),
        source.replace("event raw ->", "event raw when false ->"),
        source.replace("event raw ->", "event ->"),
    ] {
        let document = analyze(&safe).unwrap();
        assert!(
            document
                .warnings()
                .iter()
                .all(|warning| warning.code != "W007"),
            "{:?}",
            document.warnings()
        );
    }
}

#[test]
fn follows_controlled_component_bindings_for_state_usage() {
    let document = analyze(
        "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  draft = \"\"\ncomponent Field(bind value:str)\n  input \"Value\" <-> value\nview\n  Field value<->draft\n",
    )
    .unwrap();
    assert!(document.warnings().is_empty(), "{:?}", document.warnings());
}

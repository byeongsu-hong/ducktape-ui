use super::*;

#[test]
fn lowers_first_class_tests_to_headless_rust_tests() {
    let source = r#"app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #111111
  fg #eeeeee
  primary #3366ff
  danger #cc3333
preset test
state
  draft = ""
  count = 0
component Card(bind value:str)
  col #root
    input "Draft" #draft <-> value
on increment
  count = count + 1
test render_contract
  preset test
  viewport 320 240
  timeout 2s
  mount
    Card value<->draft #card
  target root = #card/root
  target draft_input = root/draft
  expect root.width ~= 240.0
  expect root.background == background.color(color.rgb8(17, 17, 17))
  expect exists draft_input
  expect text "Draft" within root
  click draft_input
  type "local"
  key enter
  resize 480 720
  dispatch increment
  expect count == 1
view
  Card value<->draft #card
"#;

    let generated = compile(source, "contract.ice").unwrap();

    for expected in [
        "#[cfg(test)]\nmod __ice_tests",
        "fn render_contract()",
        "Config::new(\"render_contract\")",
        ".viewport(320.0f32, 240.0f32)",
        "Duration::from_millis(2000)",
        ".preset(\"test\")",
        "#[cfg(test)]\nfn __ice_test_mount_0",
        "#[cfg(test)]\nfn __ice_test_program_0",
        "Driver::new(Demo::__ice_test_program_0(), __config)",
        "__test.check_approx",
        ").background()",
        "__test.check_exists",
        "__test.check_text",
        "Action::Click",
        "Action::Type",
        "Named::Enter",
        "Action::Resize",
        "__test.perform_action",
        "__test.dispatch",
        "Location::new(\"contract.ice\"",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_the_daemon_test_window_from_the_driver() {
    let generated = compile(
        r#"daemon Monitor
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
view
  text window.display #window
test window_context
  target label = #window
  expect window.display != ""
  expect text window.display within label
"#,
        "monitor.ice",
    )
    .unwrap();

    assert!(generated.contains("(__test.window()).to_string()"));
    assert!(generated.contains("__test.check_text"));
}

#[test]
fn lowers_expanded_semantic_test_actions_to_the_runtime_driver() {
    let source = r#"app Semantic
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
view
  col #root
    input "Draft" #field <-> draft
test semantic_actions
  theme dark
  scale 1.5
  locale "ko-KR"
  platform linux
  reduced-motion true
  target root = #root
  target field = root/field
  enter field
  leave
  move field
  move 10 20
  double-click field right
  click-at 10 20 middle
  press field back
  release back
  wheel lines 0 -1
  scroll-to root 0 100
  scroll-by root 0 -20
  snap root 0.0 0.5
  snap-end root
  drag field root
  press field
  drop root
  focus field
  focus-next
  focus-previous
  blur
  window focus
  clear
  replace "value"
  select 0 2
  select-all
  cursor 1
  cursor front
  cursor end
  composition update "ime" 0 3
  key arrow-left
  key TVInputHDMI1
  key-down "a" modified="A" location=left physical=KeyA text="a" repeat=true
  key-up shift location=right physical=ShiftRight
  modifiers shift control
  chord control "p"
  repeat backspace 2
  tap field 2
  touch down 1 10 20
  window move -10 20
  window resize 800 600
  window rescale 2.0
  window close-request
  window opened
  window closed
  window redraw
  system-theme none
  file-hover "/tmp/file.txt"
  file-drop "/tmp/file.txt"
  file-leave
  wait 10ms
  advance 16ms
  idle
  capture semantic_controls
  a11y activate field
  a11y focus field
  expect a11y field role "text_input"
  expect a11y field disabled false
  expect a11y field action click
  expect root.surface_count >= 0
  expect root.text_baseline >= 0.0
  expect root.pixel_aligned
  expect root.accessibility_role != ""
"#;

    let generated = compile(source, "semantic.ice").unwrap();
    for expected in [
        ".theme(::ui_lang_runtime::testing::ThemeMode::Dark)",
        ".scale_factor(1.5f32)",
        ".locale(\"ko-KR\")",
        ".platform(::ui_lang_runtime::testing::Platform::Linux)",
        ".reduced_motion(true)",
        "Action::Enter",
        "Action::Leave",
        "Action::MoveTo",
        "Action::MoveToPoint",
        "Action::Click",
        "Action::ClickAt",
        "MouseButton::Middle",
        "WheelDelta::Lines",
        "Action::ScrollTo",
        "Action::ScrollBy",
        "Action::Snap",
        "Action::SnapEnd",
        "Action::FocusNext",
        "Action::FocusPrevious",
        "Action::SelectAll",
        "Action::CursorFront",
        "Action::CursorEnd",
        "CompositionPhase::Update",
        "Named::ArrowLeft",
        "Named::TVInputHDMI1",
        "Code::KeyA",
        "Action::KeyDown",
        "Action::KeyUp",
        "Action::Chord",
        "Action::Repeat",
        "Action::Touch",
        "Action::WindowMove",
        "Action::CloseRequested",
        "Action::WindowOpened",
        "Action::WindowClosed",
        "ThemeMode::None",
        "Action::FileDrop",
        "Action::Advance",
        "Action::Capture",
        "Action::Accessibility",
        "__test.check_accessibility_str",
        "__test.check_accessibility_bool",
        "__test.check_accessibility_action",
        ").surface_count()",
        ").text_baseline()",
        ").pixel_aligned()",
        ").accessibility_role_name()",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn wraps_sync_test_expressions_in_source_mapped_panic_context() {
    let generated = compile(
        r#"app Demo
extern crate::backend
  sync explode() -> bool
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
test sync_context
  expect explode()
view
  text "ok"
"#,
        "sync_context.ice",
    )
    .unwrap();

    let context = generated
        .find("::ui_lang_runtime::testing::step(\"sync_context\"")
        .expect("generated test step must establish panic context");
    let call = generated[context..]
        .find("crate::backend::explode()")
        .map(|offset| context + offset)
        .expect("sync expression must be generated inside the test step");
    let check = generated[call..]
        .find("__test.check(__actual")
        .map(|offset| call + offset)
        .expect("expectation must be generated after the sync call");
    assert!(context < call && call < check);
    assert!(generated[context..call].contains("Location::new(\"sync_context.ice\""));
    assert!(generated[context..call].contains("\"expect explode()\""));
}

#[test]
fn discovers_support_used_only_by_a_test_mount() {
    let generated = compile(
        r#"app MountFeatures
extern crate::backend
  themer panel(active:bool) -> unit
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
  body:editor = ""
  selected = 1
  tabs:[i64] = [1]
preset mounted
  boot
    pane #test_panes split main tab(selected) horizontal ratio=0.4
on focus_draft
  task widget focus #mount_root/draft
test mounted
  preset mounted
  mount
    col #mount_root
      input "Draft" #draft <-> draft
      editor <-> body
      themer panel(true)
      canvas w=40.0 h=20.0
      panes #test_panes resize=4.0 drag
        pane main
          text "Pane"
        pane tab in tabs by=tab
          text tab
  dispatch focus_draft
view
  text "Production"
"#,
        "mount_features.ice",
    )
    .unwrap();

    for expected in [
        "pub(crate) __pane_test_panes:",
        "let __pane_test_panes =",
        "__PaneTestPanesResize",
        "__PaneTestPanesDrag",
        "__IcePaneTestPanes::Tab(self.selected)",
        "self.__pane_test_panes.iter().find_map",
        "__IceCanvasProgram",
        "__BindDraft",
        "__EditBody",
        "__ExternNoop",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
    assert!(!generated.contains("#[cfg(test)]\npub(crate) __pane_test_panes:"));
    assert!(!generated.contains("#[cfg(test)]\nlet __pane_test_panes ="));
}

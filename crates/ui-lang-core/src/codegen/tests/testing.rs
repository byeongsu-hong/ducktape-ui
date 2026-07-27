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
        "__test.click",
        "__test.typewrite",
        "Named::Enter",
        "__test.resize",
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

use super::*;

const VALID: &str = r#"app Demo

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

extern crate::backend
  pure normalize(value:str) -> str
  pure dimension(value:f64) -> i64

preset test
  state
    draft = "ready"

state
  draft = ""
  count = 0

component Card(bind value:str)
  col #root
    input "Draft" #draft <-> value
    if value == "optional"
      box #optional
        text "Optional"

on increment
  count = count + 1

on selected(value)
  draft = value

test render_contract
  preset test
  viewport 320 240
  timeout 2s
  mount
    Card value<->draft #card

  target root = #card/root
  target draft_input = root/draft

  expect count == 0
  expect normalize(draft) == "ready"
  expect root.kind == "box"
  expect root.value == ""
  expect root.visible
  expect root.width ~= 240.0
  expect root.left == root.x
  expect root.right == root.x + root.width
  expect root.center_x == root.x + root.width / 2.0
  expect root.visible_width >= 0.0
  expect root.content_width >= 0.0
  expect root.scroll_x >= 0.0
  expect root.translation_x == 0.0
  expect root.background == background.color(color.rgb8(17, 17, 17))
  expect root.border.width == 1.0
  expect root.shadow.blur == 0.0
  expect root.text_color == color.white()
  expect root.text_size == 16.0
  expect root.font == font.default()
  expect root.line_height == line_height.default()
  expect exists draft_input
  expect missing #card/root/optional
  expect text "Draft" within root
  expect no text "Failed"
  click draft_input
  move #card/root/draft
  press draft_input
  release
  type "local"
  key enter
  window resize 480 720
  dispatch increment
  dispatch selected("next")

view
  Card value<->draft #card
"#;

#[test]
fn checks_test_mount_targets_expressions_and_dispatch() {
    let document = analyze(VALID).unwrap();
    assert_eq!(document.source_document().tests.len(), 1);
    assert_eq!(
        document.source_document().handlers[1].params[0].ty,
        Type::Str
    );
}

#[test]
fn checks_digits_in_snake_case_test_names() {
    let source = VALID.replace("test render_contract", "test render_contract_2");
    let document = analyze(&source).unwrap();

    assert_eq!(
        document.source_document().tests[0].name,
        "render_contract_2"
    );
}

#[test]
fn checks_expanded_semantic_test_actions_and_inspection_fields() {
    let source = VALID.replace(
        "  expect count == 0",
        r#"  leave
  move draft_input
  move root.center_x root.center_y
  click draft_input right
  double-click draft_input
  click-at root.left root.top middle
  press draft_input back
  release back
  wheel lines 0 -1
  scroll-to root 0 root.scroll_y
  scroll-by root 0 12
  snap root 0.0 1.0
  snap-end root
  drag draft_input root
  press draft_input
  drop root
  focus draft_input
  focus-next
  focus-previous
  blur
  window focus
  type "draft"
  clear
  replace normalize(draft)
  select 0 2
  select-all
  cursor 1
  cursor front
  cursor end
  composition start
  composition update "preedit" 0 3
  composition commit "done"
  composition cancel
  key arrow-left
  key-down "a" modified="A" location=left physical=KeyA text="a" repeat=true
  key-up shift location=right physical=ShiftRight
  modifiers shift control
  chord control "p"
  repeat backspace 2
  tap draft_input 2
  touch down 1 10 20
  touch up 1 10 20
  window move -20 10
  window resize 640 480
  window rescale 1.5
  window close-request
  window opened
  window closed
  window redraw
  system-theme none
  file-hover "/tmp/file.txt"
  file-drop "/tmp/file.txt"
  file-leave
  wait 1ms
  advance 16ms
  idle
  capture semantic_states
  a11y activate draft_input
  a11y focus draft_input
  expect a11y draft_input role "text_input"
  expect a11y draft_input name "Draft"
  expect a11y draft_input checked false
  expect a11y draft_input action click
  expect root.surface_count >= 0
  expect root.text_count >= 0
  expect root.image_count >= 0
  expect root.text_x >= 0.0
  expect root.text_y >= 0.0
  expect root.text_width >= 0.0
  expect root.text_height >= 0.0
  expect root.text_baseline >= 0.0
  expect root.image_x >= 0.0
  expect root.image_y >= 0.0
  expect root.image_width >= 0.0
  expect root.image_height >= 0.0
  expect root.pixel_aligned
  expect root.focused == false
  expect root.accessibility_role != ""
  expect root.accessibility_name == ""
  expect root.accessibility_description == ""
  expect root.accessibility_value == ""
  expect root.accessibility_checked == false
  expect root.accessibility_disabled == false
  expect root.accessibility_supports_activate == false
  expect root.accessibility_supports_focus == false
  expect count == 0"#,
    );

    analyze(&source).unwrap();
}

#[test]
fn rejects_comparisons_of_rendered_targets() {
    for expression in ["root == root", "root < root", "[root] == [root]"] {
        let source = VALID.replace("count == 0", expression);
        let failure = analyze(&source).unwrap_err();

        assert_eq!(failure.code, "E153");
        assert!(failure.message.contains("target values"));
        assert!(failure.message.contains("explicit field"));
    }
}

#[test]
fn explains_that_component_ids_are_scopes() {
    // A scope alias is declarable — `expect component` reads the instance
    // behind it — but each rendered-widget use of the scope, through the
    // alias or the bare path, is rejected where it happens.
    for step in [
        "click card",
        "expect exists card",
        "expect card.width ~= 1.0",
        "click #card",
    ] {
        let source = VALID
            .replace(
                "target root = #card/root",
                "target card = #card\n  target root = #card/root",
            )
            .replace(
                "  expect count == 0\n  expect normalize",
                &format!("  expect count == 0\n  {step}\n  expect normalize"),
            );
        let failure = analyze(&source).unwrap_err();
        assert_eq!(failure.code, "E194", "{step}: {}", failure.message);
        assert!(
            failure.message.contains("component scope"),
            "{step}: {}",
            failure.message
        );
        assert!(failure.message.contains("not a rendered widget"), "{step}");
    }
}

#[test]
fn requires_explicit_component_ids_for_public_test_scopes() {
    let source = VALID
        .replace("Card value=draft #card", "Card value=draft")
        .replace("#card/root/draft", "#Card/root/draft")
        .replace("#card/root", "#Card/root");
    let failure = analyze(&source).unwrap_err();

    assert_eq!(failure.code, "E194");
    assert!(failure.message.contains("unknown rendered widget target"));
}

#[test]
fn rejects_persistent_pane_ids_reused_across_test_mounts() {
    let source = format!(
        "{VALID}\n\ntest duplicate_panes\n  mount\n    panes #work\n      pane main\n        text \"One\"\n\ntest duplicate_panes_again\n  mount\n    panes #work\n      pane main\n        text \"Two\"\n"
    );
    let failure = analyze(&source).unwrap_err();

    assert_eq!(failure.code, "E187");
    assert!(
        failure
            .message
            .contains("duplicate persistent panes `#work`")
    );
}

#[test]
fn app_operations_can_target_widgets_and_panes_from_test_mounts() {
    analyze(
        r#"app MountOperations
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
  selected = 1
  tabs:[i64] = [1]
on focus_draft
  task widget focus #mount_root/draft
on open_tab
  pane #mount_panes split main tab(selected) horizontal ratio=0.4
test mounted
  mount
    col #mount_root
      input "Draft" #draft <-> draft
      panes #mount_panes
        pane main
          text "Main"
        pane tab in tabs by=tab
          text tab
  dispatch focus_draft
  dispatch open_tab
view
  text "Production"
"#,
    )
    .unwrap();
}

#[test]
fn target_alias_keys_only_reference_earlier_aliases() {
    let source = r#"app TargetOrder
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
  col #root
    text "Item" #item("Item")
test ordered
  target first = #root/item(second.value)
  target second = #root/item("Item")
"#;
    for invalid in [
        source.to_owned(),
        source.replace("second.value", "first.value"),
    ] {
        let failure = analyze(&invalid).unwrap_err();
        assert_eq!(failure.code, "E194", "{}", failure.message);
    }
}

#[test]
fn rejects_invalid_test_semantics() {
    for (source, code, message) in [
        (
            VALID.replace("preset test\n  state", "preset setup\n  state"),
            "E194",
            "unknown test preset",
        ),
        (
            VALID.replace("target root = #card/root", "target root = #missing"),
            "E194",
            "unknown rendered widget target",
        ),
        (
            VALID.replace("click draft_input", "click unknown"),
            "E194",
            "unknown test target alias",
        ),
        (
            VALID.replace("expect count == 0", "expect count + 1"),
            "E101",
            "expected `bool`",
        ),
        (
            VALID.replace("expect root.width ~= 240.0", "expect root.kind ~= 240.0"),
            "E194",
            "must be numeric",
        ),
        (
            VALID.replace("dispatch selected(\"next\")", "dispatch selected(1)"),
            "E101",
            "expected `str`",
        ),
    ] {
        let failure = analyze(&source).unwrap_err();
        assert_eq!(failure.code, code, "{}", failure.message);
        assert!(failure.message.contains(message), "{}", failure.message);
    }
}

#[test]
fn rejects_duplicate_tests_aliases_and_state_shadowing() {
    for extra in [
        "\ntest render_contract\n",
        "\ntest duplicate_alias\n  target root = #card/root\n  target root = #card/root\n",
        "\ntest state_shadow\n  target count = #card/root\n",
    ] {
        let source = format!("{VALID}{extra}");
        let failure = analyze(&source).unwrap_err();
        assert_eq!(failure.code, "E100");
    }
}

#[test]
fn limits_custom_renderers_to_layout_and_interaction_assertions() {
    let source = VALID.replacen("app Demo", "app Demo\n  renderer crate::Renderer", 1);
    let failure = analyze(&source).unwrap_err();
    assert_eq!(failure.code, "E194");
    assert!(failure.message.contains("paint assertions"));

    let layout_only = source
        .lines()
        .filter(|line| {
            !line.trim_start().starts_with("expect text ")
                && !line.trim_start().starts_with("expect no text ")
                && ![
                    "root.background",
                    "root.border",
                    "root.shadow",
                    "root.text_color",
                    "root.text_size",
                    "root.font",
                    "root.line_height",
                ]
                .iter()
                .any(|field| line.contains(field))
        })
        .collect::<Vec<_>>()
        .join("\n");
    analyze(&layout_only).unwrap();

    let layout_with_pixel_alignment = layout_only.replacen(
        "  expect root.visible\n",
        "  expect root.visible\n  expect root.pixel_aligned\n",
        1,
    );
    analyze(&layout_with_pixel_alignment).unwrap();

    for assertion in [
        "expect root.background == background.color(color.rgb8(17, 17, 17))",
        "expect root.surface_count >= 0",
        "expect text \"Draft\" within root",
        "expect no text \"Failed\"",
    ] {
        let structured_paint = layout_only.replacen(
            "  expect root.visible\n",
            &format!("  expect root.visible\n  {assertion}\n"),
            1,
        );
        let failure = analyze(&structured_paint).unwrap_err();
        assert_eq!(failure.code, "E194");
        assert!(failure.message.contains("paint assertions"));
    }

    let target_geometry = layout_only
        .replace("#draft <-> value", "#draft(1) <-> value")
        .replace("root/draft", "root/draft(1)")
        .replace(
            "move #card/root/draft(1)",
            "move #card/root/draft(dimension(root.width))",
        );
    analyze(&target_geometry).unwrap();

    let paint_outside_expect =
        layout_only.replace("window resize 480 720", "window resize root.text_size 720");
    let failure = analyze(&paint_outside_expect).unwrap_err();
    assert_eq!(failure.code, "E194");
    assert!(failure.message.contains("paint assertions"));

    let paint_in_target = target_geometry.replace(
        "move #card/root/draft(dimension(root.width))",
        "move #card/root/draft(dimension(root.text_size))",
    );
    let failure = analyze(&paint_in_target).unwrap_err();
    assert_eq!(failure.code, "E194");
    assert!(failure.message.contains("paint assertions"));

    let selector_dependency = target_geometry.replace(
        "target draft_input = root/draft(1)",
        "target draft_input = root/draft(dimension(root.text_size))",
    );
    for (source, statement) in [
        (
            selector_dependency.replace("expect exists draft_input", "expect exists root"),
            "click draft_input",
        ),
        (
            selector_dependency.replace(
                "expect exists draft_input",
                "expect draft_input.value == \"\"",
            ),
            "expect draft_input.value == \"\"",
        ),
        (
            selector_dependency
                .replace("expect exists draft_input", "expect exists root")
                .replace("click draft_input", "click root")
                .replace("press draft_input", "press root"),
            "target draft_input = root/draft(dimension(root.text_size))",
        ),
    ] {
        let line = source
            .lines()
            .position(|line| line.trim() == statement)
            .unwrap()
            + 1;
        let failure = analyze(&source).unwrap_err();
        assert_eq!(failure.code, "E194");
        assert!(failure.message.contains("paint assertions"));
        assert_eq!(failure.line, line);
    }
}

#[test]
fn exposes_direct_leaf_ids_to_first_class_tests() {
    let source = r#"app Identified
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
  enabled = false
  amount = 25.0
  mode = 0
  choices = ["One", "Two"]
  selected:str? = none
  search:combo[str] = ["One", "Two"]
on toggled(next)
  enabled = next
on changed(next)
  amount = next
on mode_changed(next)
  mode = next
on selected_value(next)
  selected = some(next)
view
  col #root
    text "Plain" #plain
    rich-text #rich
      span "Rich"
    toggler "Toggle" #toggle checked=enabled -> toggled _
    slider amount #horizontal min=0.0 max=100.0 -> changed _
    slider amount #vertical min=0.0 max=100.0 vertical w=20.0 h=100.0 -> changed _
    radio "Mode" #radio value=1 selected=(mode == 1) -> mode_changed _
    pick choices selected #pick -> selected_value _
    combo search selected "Search" #combo -> selected_value _
test direct_ids
  target plain = #root/plain
  target rich = #root/rich
  target toggle = #root/toggle
  target horizontal = #root/horizontal
  target vertical = #root/vertical
  target radio = #root/radio
  target pick = #root/pick
  target combo = #root/combo
  expect exists plain
  expect exists rich
  expect exists toggle
  expect exists horizontal
  expect exists vertical
  expect exists radio
  expect exists pick
  expect exists combo
"#;
    analyze(source).unwrap();

    let failure = analyze(&source.replace("#combo", "#pick")).unwrap_err();
    assert_eq!(failure.code, "E161");
    assert!(failure.message.contains("duplicate local id"));
}

#[test]
fn exposes_every_rendered_leaf_id_to_first_class_tests() {
    let source = r##"app Leaves
extern crate::backend
  component native() -> unit
  themer themed() -> unit
  shader shaded() -> unit
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
  amount = 50.0
  docs:markdown = "# Docs"
on open_link(url)
view
  col #root
    progress amount #progress
    rule horizontal #rule
    qr "https://example.com" #qr
    space #space w=10.0 h=10.0
    markdown docs #markdown -> open_link _
    extern native() #extern
    themer themed() #themer
    shader shaded() #shader w=20.0 h=20.0
    image "image.png" #image
    svg "image.svg" #svg
    viewer "image.png" #viewer
    canvas #canvas w=20.0 h=20.0
test leaf_ids
  target progress = #root/progress
  target rule = #root/rule
  target qr = #root/qr
  target space = #root/space
  target markdown = #root/markdown
  target native = #root/extern
  target themed = #root/themer
  target shaded = #root/shader
  target image = #root/image
  target svg = #root/svg
  target viewer = #root/viewer
  target canvas = #root/canvas
  expect exists progress
  expect exists rule
  expect exists qr
  expect exists space
  expect exists markdown
  expect exists native
  expect exists themed
  expect exists shaded
  expect exists image
  expect exists svg
  expect exists viewer
  expect exists canvas
"##;
    analyze(source).unwrap();
}

#[test]
fn exposes_the_daemon_window_in_tests() {
    analyze(
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
    )
    .unwrap();
}

/// `tray choose` names a row an author wrote. Without a menu there is no row
/// to name and never will be, so it is a mistake at check time rather than a
/// panic when the test runs.
#[test]
fn rejects_tray_choose_without_a_menu() {
    let source = r#"app Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
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
  text "ready"
test picks
  tray choose "Quit"
"#;
    let error = analyze(source).unwrap_err();
    assert!(
        error.message.contains("`tray choose` needs a `tray` block"),
        "{}",
        error.message
    );

    analyze(&source.replace(
        "    icon-rgba \"assets/tray.rgba\" 2 2\n",
        "    icon-rgba \"assets/tray.rgba\" 2 2\n    menu\n      \"Quit\" -> quit\non quit\n  exit\n",
    ))
    .unwrap();
}

/// A tray expectation is about text, on every field.
#[test]
fn rejects_a_non_string_tray_expectation() {
    let source = r#"app Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
    menu
      "Quit" -> quit
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
on quit
  exit
view
  text "ready"
test reads
  expect tray command 3
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(
        error.message.contains("expected `str`"),
        "{}",
        error.message
    );
}

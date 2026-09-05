use super::*;

const TEST_SOURCE: &str = r#"app Demo
preset test

state
  draft = ""

on increment

test render_contract
  preset test
  viewport 320 240
  timeout 2s
  mount
    col #render
      input "Draft" #draft <-> draft

  target root = #render
  target draft = root/draft

  expect root.width ~= 240.0
  expect root.visible
  expect root.background == background.color(color.rgb8(17, 17, 17))
  expect exists draft
  expect missing #render/optional
  expect text "Draft" within root
  expect no text "Failed"
  click draft
  move draft
  press draft
  release
  type "local"
  key enter
  key escape
  key tab
  key backspace
  window resize 480 720
  dispatch increment

view
  text "App"
"#;

#[test]
fn parses_complete_first_class_test_declarations() {
    let document = parse(TEST_SOURCE).unwrap();
    let test = &document.tests[0];
    assert_eq!(test.name, "render_contract");
    assert_eq!(test.preset.as_deref(), Some("test"));
    assert_eq!(test.viewport, Some((320.0, 240.0)));
    assert_eq!(test.timeout_ms, Some(2_000));
    assert!(test.mount.is_some());
    assert_eq!(test.targets.len(), 2);
    assert_eq!(
        test.targets[1]
            .target
            .segments
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>(),
        ["render", "draft"]
    );
    assert_eq!(test.steps.len(), 18);
    assert!(matches!(
        test.steps[0].kind,
        TestStepKind::Expect(TestExpectation::Approx { .. })
    ));
    assert!(matches!(
        test.steps[9].kind,
        TestStepKind::Press {
            target: TestTargetRef::Alias(ref name),
            ..
        } if name == "draft"
    ));
    assert!(matches!(
        test.steps[17].kind,
        TestStepKind::Dispatch { ref handler, ref args }
            if handler == "increment" && args.is_empty()
    ));
}

#[test]
fn accepts_digits_in_snake_case_test_names() {
    let document = parse("app Demo\ntest render_contract_2\nview\n  text \"ok\"\n").unwrap();

    assert_eq!(document.tests[0].name, "render_contract_2");
}

#[test]
fn rejects_invalid_test_declaration_shapes() {
    for (body, message) in [
        ("test RenderContract\n", "snake_case"),
        ("test foo__bar\n", "snake_case"),
        ("test trailing_\n", "snake_case"),
        ("test demo\n  viewport 0 240\n", "positive"),
        ("test demo\n  timeout forever\n", "duration"),
        ("test demo\n  mount\n", "exactly one"),
        ("test demo\n  key Arrow-Left\n", "named keys use"),
        ("test demo\n  key Self\n", "named keys use"),
        ("test demo\n  key self\n", "named keys use"),
        ("test demo\n  key -\n", "named keys use"),
        ("test demo\n  key 1\n", "named keys use"),
        ("test demo\n  key arrow--left\n", "named keys use"),
        (
            "test demo\n  key-down enter physical=_\n",
            "physical keys use",
        ),
        ("test demo\n  key \"\"\n", "character key must not be empty"),
        (
            "test demo\n  expect true\n  target root = #root\n",
            "must precede",
        ),
        (
            "test demo\n  target child = missing/child\n",
            "earlier target alias",
        ),
    ] {
        let source = format!("app Demo\n{body}view\n  text \"ok\"\n");
        let failure = parse(&source).unwrap_err();
        assert_eq!(failure.code, "E194", "{body}");
        assert!(failure.message.contains(message), "{}", failure.message);
    }
}

#[test]
fn accepts_non_empty_string_character_key_values() {
    let document = parse(
        r#"app Demo
test character_key
  key "Dead"
view
  text "ok"
"#,
    )
    .unwrap();

    assert!(matches!(
        &document.tests[0].steps[0].kind,
        TestStepKind::Key(TestKey::Character(value)) if value == "Dead"
    ));
}

#[test]
fn records_relative_and_dynamic_test_target_alias_references() {
    let source = r#"app Demo
view
  col #root
    text "Key" #key
    text "Item" #item("Key")
test dynamic_targets
  target root = #root
  target key = root/key
  target item = root/item(key.value)
  expect exists #root/item(key.value)
  expect text "Item" within #root/item(key.value)
  click #root/item(key.value)
"#;
    let (_, symbols) = parse_with_symbols(source).unwrap();
    let key = symbols
        .iter()
        .filter(|symbol| {
            symbol.kind == SymbolKind::TestTarget
                && symbol.scope.as_deref() == Some("dynamic_targets")
                && symbol.name == "key"
        })
        .collect::<Vec<_>>();

    assert_eq!(key.iter().filter(|symbol| symbol.definition).count(), 1);
    assert_eq!(key.iter().filter(|symbol| !symbol.definition).count(), 4);
    assert_eq!(
        key.iter()
            .filter_map(|symbol| (!symbol.definition).then_some(symbol.range.as_ref()?.line))
            .collect::<Vec<_>>(),
        [9, 10, 11, 12]
    );

    let root = symbols
        .iter()
        .filter(|symbol| {
            symbol.kind == SymbolKind::TestTarget
                && symbol.scope.as_deref() == Some("dynamic_targets")
                && symbol.name == "root"
        })
        .collect::<Vec<_>>();
    assert_eq!(root.iter().filter(|symbol| symbol.definition).count(), 1);
    assert_eq!(root.iter().filter(|symbol| !symbol.definition).count(), 2);
}

#[test]
fn parses_semantic_conformance_configuration_and_steps() {
    let source = r#"app Conformance
state
  draft = ""
view
  col #root
    input "Draft" #field <-> draft
test interactions
  theme dark
  scale 1.5
  locale "ko-KR"
  platform linux
  reduced-motion true
  target root = #root
  target field = root/field
  leave
  move field
  move 12 24
  click field right
  double-click field
  click-at 10 20 middle
  press field back
  release back
  wheel pixels 0 -16
  wheel lines 0 -3
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
  window blur
  type "draft"
  clear
  replace "value"
  select 0 3
  select-all
  cursor 2
  cursor front
  cursor end
  composition start
  composition update "preedit" 0 7
  composition commit "done"
  composition cancel
  key arrow-left
  key TVInputHDMI1
  key "x"
  key-down "a" modified="A" location=left physical=KeyA text="a" repeat=true
  key-up shift modified=shift location=right physical=ShiftRight
  modifiers shift control
  modifiers
  chord control shift "p"
  repeat backspace 3
  tap field 2
  touch down 1 10 20
  touch move 1 12 22
  touch up 1 12 22
  touch down 2 0 0
  touch cancel 2 0 0
  window move -20 40
  window resize 800 600
  window rescale 2.0
  window close-request
  window opened
  window closed
  window redraw
  system-theme none
  file-hover "/tmp/a.txt"
  file-drop "/tmp/a.txt"
  file-leave
  wait 10ms
  advance 16ms
  idle
  capture dark_controls
  a11y activate field
  a11y focus field
  a11y increment field
  a11y decrement field
  expect a11y field role "text_input"
  expect a11y field name "Draft"
  expect a11y field value ""
  expect a11y field checked false
  expect a11y field disabled false
  expect a11y field focused true
  expect a11y field action click
  expect a11y field action focus false
  expect a11y field action decrement false
"#;

    let document = parse(source).unwrap();
    let test = &document.tests[0];
    assert_eq!(test.theme, Some(TestTheme::Dark));
    assert_eq!(test.scale_factor, Some(1.5));
    assert_eq!(test.locale.as_deref(), Some("ko-KR"));
    assert_eq!(test.platform, Some(TestPlatform::Linux));
    assert_eq!(test.reduced_motion, Some(true));
    assert!(test.steps.iter().any(|step| matches!(
        step.kind,
        TestStepKind::Wheel {
            unit: TestWheelUnit::Lines,
            ..
        }
    )));
    assert!(test.steps.iter().any(|step| matches!(
        step.kind,
        TestStepKind::KeyDown(TestKeyEvent {
            repeat: true,
            text: Some(_),
            ..
        })
    )));
    assert!(test.steps.iter().any(|step| matches!(
        step.kind,
        TestStepKind::Composition(TestComposition::Update {
            selection: Some(_),
            ..
        })
    )));
    assert!(test.steps.iter().any(|step| matches!(
        step.kind,
        TestStepKind::Expect(TestExpectation::Accessibility { .. })
    )));
}

#[test]
fn rejects_pointer_and_window_step_spellings_that_duplicate_move_and_window_resize() {
    for step in ["hover field", "enter field", "resize 480 720"] {
        let source = format!(
            "app Demo\nview\n  text \"ok\" #field\ntest invalid\n  target field = #field\n  {step}\n"
        );
        let failure = parse(&source).unwrap_err();
        assert_eq!(failure.code, "E194");
        assert_eq!(failure.message, format!("unknown test step `{step}`"));
    }
}

#[test]
fn rejects_native_event_shapes_the_semantic_driver_cannot_represent() {
    for (step, message) in [
        ("key-up enter repeat=true", "does not carry repeat"),
        ("key-up enter text=\"x\"", "does not carry text"),
        (
            "key-down \"x\" text=\"\"",
            "keyboard text must not be empty",
        ),
        (
            "expect a11y field action expand",
            "support click, focus, increment, and decrement",
        ),
        (
            "a11y expand field",
            "activate, focus, increment, decrement, or scroll-into-view",
        ),
    ] {
        let source = format!(
            "app Demo\nview\n  text \"ok\" #field\ntest invalid\n  target field = #field\n  {step}\n"
        );
        let failure = parse(&source).unwrap_err();
        assert_eq!(failure.code, "E194");
        assert!(failure.message.contains(message), "{}", failure.message);
    }
}

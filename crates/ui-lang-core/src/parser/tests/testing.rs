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
  target draft = #render/draft

  expect root.width ~= 240.0
  expect root.visible
  expect root.background == background.color(color.rgb8(17, 17, 17))
  expect exists draft
  expect missing #render/optional
  expect text "Draft" within root
  expect no text "Failed"
  click draft
  hover draft
  press draft
  release
  type "local"
  key enter
  key escape
  key tab
  key backspace
  resize 480 720
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
    assert_eq!(test.steps.len(), 18);
    assert!(matches!(
        test.steps[0].kind,
        TestStepKind::Expect(TestExpectation::Approx { .. })
    ));
    assert!(matches!(
        test.steps[9].kind,
        TestStepKind::Press(TestTargetRef::Alias(ref name)) if name == "draft"
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
        ("test demo\n  key space\n", "enter, escape"),
        (
            "test demo\n  expect true\n  target root = #root\n",
            "must precede",
        ),
    ] {
        let source = format!("app Demo\n{body}view\n  text \"ok\"\n");
        let failure = parse(&source).unwrap_err();
        assert_eq!(failure.code, "E194", "{body}");
        assert!(failure.message.contains(message), "{}", failure.message);
    }
}

#[test]
fn records_aliases_only_inside_dynamic_test_target_keys() {
    let source = r#"app Demo
view
  col #root
    text "Key" #key
    text "Item" #item("Key")
test dynamic_targets
  target key = #root/key
  target item = #root/item(key.value)
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
        [8, 9, 10, 11]
    );
}

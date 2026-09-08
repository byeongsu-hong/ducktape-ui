use ui_lang_core::{analyze as analyze_source, compile as compile_source, format_source};

fn with_theme(source: &str) -> String {
    source.replacen("\n", "\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n", 1)
}
fn analyze(source: &str) -> Result<ui_lang_core::CheckedDocument, ui_lang_core::Error> {
    analyze_source(&with_theme(source))
}
fn compile(source: &str, path: &str) -> Result<String, ui_lang_core::Error> {
    ui_lang_core::compile_for(&with_theme(source), path, ui_lang_core::Target::Tree)?;
    compile_source(&with_theme(source), path)
}

#[test]
fn many_slots_accept_ordered_direct_named_and_omitted_children() {
    let source = r#"app Many
component Actions()
  row
    slot children*
component Named()
  col
    slot actions*
view
  col
    Actions
      text "first"
      text "second"
    Actions
    Actions
      text "only"
    Named
      actions:
        text "third"
        text "fourth"
    Named
      actions:
"#;
    analyze(source).unwrap();
    compile(source, "many.ice").unwrap();
    fn text_roots(node: &ui_lang_core::ViewNode, output: &mut Vec<String>) {
        if let ui_lang_core::ViewNode::Text {
            value: ui_lang_core::Expr::Str(value),
            ..
        } = node
        {
            output.push(value.clone());
        }
        ui_lang_core::for_each_child(node, &mut |child| text_roots(child, output));
    }
    let formatted = format_source(source).unwrap();
    let mut texts = Vec::new();
    text_roots(&ui_lang_core::parse(&formatted).unwrap().view, &mut texts);
    assert_eq!(texts, ["first", "second", "only", "third", "fourth"]);
    assert_eq!(
        format_source(&format_source(source).unwrap()).unwrap(),
        format_source(source).unwrap()
    );
}

#[test]
fn many_slot_forwarding_accepts_siblings() {
    let source = "app Many\ncomponent Inner()\n  row\n    slot children*\ncomponent Outer()\n  Inner\n    slot children*\nview\n  Outer\n    text \"first\"\n    text \"second\"\n";
    analyze(source).unwrap();
    compile(source, "forward.ice").unwrap();
}

#[test]
fn many_slots_reject_scalar_placement_and_single_root_forwarding() {
    for body in [
        "slot children*",
        "box\n    slot children*",
        "Single\n    slot children*",
    ] {
        let source = format!(
            "app Many\ncomponent Single()\n  slot\ncomponent Bad()\n  {body}\nview\n  text \"ok\"\n"
        );
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("multi-child"), "{error:?}");
    }
}

#[test]
fn single_root_slots_still_reject_siblings() {
    let error = analyze("app Single\ncomponent One()\n  slot\nview\n  One\n    text \"first\"\n    text \"second\"\n").unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("exactly one root"));
    // The helper inserts ten theme lines after the app declaration.
    assert_eq!(error.line, 15);
    assert!(
        error
            .hint
            .as_deref()
            .is_some_and(|hint| hint.contains("wrap siblings"))
    );
}

#[test]
fn many_slots_expose_api_cardinality() {
    let checked = analyze("app Many\ncomponent Actions()\n  row\n    slot children*\nview\n  Actions\n    text \"first\"\n    text \"second\"\n").unwrap();
    let api = ui_lang_core::ApiSurface::from_checked(&checked);
    assert!(api.components[0].slots[0].multiple);
    assert!(!api.components[0].slots[0].required);
}

#[test]
fn many_slots_expand_flow_in_caller_and_callee_child_lists() {
    let source = r#"app Many
state
  visible = true
  items = [1, 2]
  choice:str? = some("selected")
component Actions(show:bool)
  col
    if show
      slot children*
view
  Actions show=visible
    if visible
      text "conditional"
    for item in items
      text item
    match choice
      some(label)
        text label
      none
        text "empty"
"#;
    compile(source, "flow.ice").unwrap();
}

#[test]
fn many_slots_reject_combined_cardinality_and_keep_compound_names_unambiguous() {
    for cardinality in ["children?*", "children*?"] {
        let source = format!("component Actions()\n  row\n    slot {cardinality}\n");
        let error = ui_lang_core::parse(&source).unwrap_err();
        assert_eq!(error.code, "E040");
        assert!(error.message.contains("cannot combine"));
    }
    let source = "app Compound\ncomponent Card()\n  col\n    slot Body\ncomponent Card.Body()\n  col\n    slot children*\nview\n  Card\n    Card.Body\n      text \"one\"\n      text \"two\"\n";
    compile(source, "compound.ice").unwrap();
}

#[test]
fn many_slot_grid_minimum_binds_in_receiving_component() {
    compile("app GridSlots\ncomponent Cells(width:f64)\n  grid min-cell=width\n    slot children*\nview\n  Cells width=120.0\n    text \"first\"\n    text \"second\"\n", "grid-scope.ice").unwrap();
}

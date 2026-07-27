use super::*;

#[test]
fn parses_dynamic_widget_operation_ids() {
    let source = r#"app Operations
theme contract AppTheme
  bg
palette app for AppTheme
  bg #000000
state
  selected = 1
  value = ""
on focus
  task widget focus #outer(selected)/inner/field
view
  input "Value" #field(selected) <-> value
"#;
    let document = parse(source).unwrap();
    let Statement::WidgetOperation {
        operation: WidgetOperation::Focus { target },
        ..
    } = &document.handlers[0].statements[0]
    else {
        panic!("expected dynamic focus operation");
    };
    assert_eq!(target.segments.len(), 3);
    let id = &target.segments[0];
    assert_eq!(id.name, "outer");
    assert!(matches!(
        id.key.as_ref(),
        Some(Expr::Path(path)) if path == &["selected"]
    ));
    assert_eq!(target.segments[1].name, "inner");
    assert!(target.segments[1].key.is_none());
    assert_eq!(target.segments[2].name, "field");
    assert!(target.segments[2].key.is_none());

    let error = parse(&source.replace(
        "focus #outer(selected)/inner/field",
        "focus outer(selected)/inner/field",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E052");
    assert!(error.message.contains("#id(key)"));
}

#[test]
fn parses_widget_selectors() {
    let source = example!("widget_selectors.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Selector);
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::WidgetOperation {
            operation: WidgetOperation::Find {
                selector: WidgetSelector::Id(_),
                all: false,
            },
            ..
        }
    ));
    assert!(matches!(
        &document.handlers[4].statements[0],
        Statement::WidgetOperation {
            operation: WidgetOperation::Find {
                selector: WidgetSelector::Text(_),
                all: true,
            },
            ..
        }
    ));
    assert!(matches!(
        &document.handlers[5].statements[0],
        Statement::WidgetOperation {
            operation: WidgetOperation::Find {
                selector: WidgetSelector::Extern { function, args },
                all: true,
            },
            ..
        } if function == "by_kind" && args.len() == 1
    ));

    let error = parse(&source.replace("point 12.0 24.0", "point 12.0")).unwrap_err();
    assert_eq!(error.code, "E052");
    assert!(error.message.contains("requires x and y"));
}

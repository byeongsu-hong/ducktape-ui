use super::*;
use crate::test_support::example;

#[test]
fn syntax_boundaries_ignore_escaped_quotes() {
    let quoted = r#""a\" b,=->)""#;

    assert_eq!(
        split_words(&format!("{quoted} tail")),
        [quoted.to_owned(), "tail".to_owned()]
    );

    let comma = format!("{quoted}, tail");
    assert_eq!(split_top(&comma, ','), [quoted, "tail"]);

    let assignment = format!("{quoted}=tail");
    assert_eq!(split_top_once(&assignment, '='), Some((quoted, "tail")));

    let route = format!("{quoted} -> tail");
    let (left, right) = split_top_marker(&route, "->").unwrap();
    assert_eq!((left.trim(), right.trim()), (quoted, "tail"));

    let call = format!("call({quoted}, tail)");
    let line = Line {
        number: 1,
        indent: 0,
        text: call.clone(),
        children: Vec::new(),
        symbols: std::rc::Rc::default(),
        track_symbols: false,
    };
    assert_eq!(matching_paren(&call, &line).unwrap(), call.len() - 1);

    assert_eq!(strip_wrapping_parens("(left)+(right)"), "(left)+(right)");
    assert_eq!(strip_wrapping_parens("((left)+(right))"), "(left)+(right)");
}

#[test]
fn rejects_rust_and_compiler_reserved_identifiers() {
    let error = parse("app Demo\nstate\n  type = 0\nview\n  text \"ok\"\n").unwrap_err();
    assert_eq!(error.code, "E072");

    let error = parse("app Demo\nstate\n  none = 0\nview\n  text \"ok\"\n").unwrap_err();
    assert_eq!(error.code, "E072");

    let error =
        parse("app Demo\nstate\n  __ice_accessibility = 0\nview\n  text \"ok\"\n").unwrap_err();
    assert_eq!(error.code, "E072");

    let error =
        parse("app Demo\nextern crate::backend\n  sync bytes() -> i64\nview\n  text \"ok\"\n")
            .unwrap_err();
    assert_eq!(error.code, "E021");
    assert!(error.message.contains("byte literal"));

    assert!(!SymbolKind::Handler.accepts("match"));
    assert!(!SymbolKind::Handler.accepts("_"));
    assert!(!SymbolKind::Component.accepts("Self"));
    assert!(SymbolKind::Recipe.accepts("primary_action"));

    let error = parse("app Demo\nextern backend::crate\nview\n  text \"ok\"\n").unwrap_err();
    assert_eq!(error.code, "E073");

    parse("app Demo\nextern crate::none::__backend\nview\n  text \"ok\"\n").unwrap();
}

#[test]
fn shadowed_image_constructors_require_explicit_state_types() {
    for (declaration, call) in [
        ("sync encoded(value:bytes) -> str", "encoded(bytes(00))"),
        (
            "sync rgba(width:i64, height:i64, value:bytes) -> str",
            "rgba(1, 1, bytes(00))",
        ),
    ] {
        let inferred = format!(
            "app Demo\nextern crate::backend\n  {declaration}\nstate\n  value = {call}\nview\n  text \"ok\"\n"
        );
        let error = parse(&inferred).unwrap_err();
        assert_eq!(error.code, "E031");
        assert!(error.message.contains("explicit type"));

        let explicit = inferred.replace("value =", "value:str =");
        parse(&explicit).unwrap();
    }
}

#[test]
fn rejects_lowercase_component_declarations() {
    let error =
        parse("app Demo\ncomponent card()\n  text \"card\"\nview\n  text \"ok\"\n").unwrap_err();
    assert_eq!(error.code, "E072");
    assert!(error.message.contains("invalid component name"));
}

#[test]
fn parses_the_full_i64_literal_range() {
    let document = parse(
        "app Demo\nstate\n  lowest = -9223372036854775808\n  highest = 9223372036854775807\nview\n  text \"ok\"\n",
    )
    .unwrap();
    assert!(matches!(document.states[0].initial, Expr::I64(i64::MIN)));
    assert!(matches!(document.states[1].initial, Expr::I64(i64::MAX)));

    for value in ["9223372036854775808", "-9223372036854775809"] {
        let source = format!("app Demo\nstate\n  value = {value}\nview\n  text \"ok\"\n");
        assert_eq!(parse(&source).unwrap_err().code, "E070");
    }
    for value in ["--9223372036854775808", "-(-9223372036854775808)"] {
        let source = format!("app Demo\nstate\n  value = {value}\nview\n  text \"ok\"\n");
        assert_eq!(parse(&source).unwrap_err().code, "E070");
    }
}

#[test]
fn rejects_non_finite_float_literals() {
    let value = format!("{}.0", "9".repeat(400));
    let source = format!("app Demo\nstate\n  value = {value}\nview\n  text \"ok\"\n");
    assert_eq!(parse(&source).unwrap_err().code, "E070");
}

#[test]
fn parses_scientific_float_literals() {
    let document =
        parse("app Demo\nstate\n  small = 1e-3\n  large = 2E+3\nview\n  text \"ok\"\n").unwrap();
    assert!(matches!(document.states[0].initial, Expr::F64(value) if value == 0.001));
    assert!(matches!(document.states[1].initial, Expr::F64(value) if value == 2000.0));

    let source = "app Demo\nstate\n  value = 1e+\nview\n  text \"ok\"\n";
    assert_eq!(parse(source).unwrap_err().code, "E070");
}

#[test]
fn rejects_static_settings_outside_the_runtime_number_range() {
    let source = "app Demo\n  text-size 3.5e38\nview\n  text \"ok\"\n";
    let error = parse(source).unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("f32 range"));
}

#[test]
fn rejects_text_after_a_parenthesized_route() {
    let source = "app Demo\non pressed\nview\n  button \"ok\" -> pressed() trailing\n";
    let error = parse(source).unwrap_err();
    assert_eq!(error.code, "E052");
    assert!(error.message.contains("after route"));
}

#[test]
fn rejects_empty_route_arguments() {
    for route in ["pressed(,)", "pressed(_,)", "pressed(,,_)"] {
        let source = format!("app Demo\non pressed(value)\nview\n  button \"ok\" -> {route}\n");
        assert_eq!(parse(&source).unwrap_err().code, "E070", "{route}");
    }
}

#[test]
fn rejects_empty_handler_and_canvas_bindings() {
    for source in [
        "app Demo\non pressed(value,)\nview\n  text \"ok\"\n",
        "app Demo\nview\n  canvas\n    event mouse moved as x,, y\n      emit moved(x, y)\n",
    ] {
        assert_eq!(parse(source).unwrap_err().code, "E072");
    }
}

#[test]
fn rejects_multiple_ids_on_one_widget() {
    for (node, code) in [
        ("input \"Draft\" #first #second <-> draft", "E065"),
        ("button \"Save\" #first #second -> pressed", "E066"),
        (
            "checkbox \"Ready\" #first #second checked=true -> changed _",
            "E067",
        ),
        ("editor #first #second <-> draft -> edited _", "E099"),
    ] {
        let source = format!("app Demo\nview\n  {node}\n");
        let error = parse(&source).unwrap_err();
        assert_eq!(error.code, code, "{node}");
        assert!(error.message.contains("more than one ID"), "{node}");
    }
}

#[test]
fn rejects_multiple_control_bindings() {
    for (node, code) in [
        ("input \"Draft\" <-> first <-> second", "E065"),
        ("editor <-> first <-> second -> edited _", "E099"),
    ] {
        let source = format!("app Demo\nview\n  {node}\n");
        let error = parse(&source).unwrap_err();
        assert_eq!(error.code, code, "{node}");
        assert!(error.message.contains("more than one binding"), "{node}");
    }
}

const SOURCE: &str = r#"app Demo

extern crate::backend
  Item(id:i64, name:str)
  load() -> [Item] ! Item

theme
  bg #000000

qr docs "https://example.com/ice docs" correction=high version=normal(4)

state
  items:[Item] = []
  query = ""

on mount
  run load() -> loaded _ | failed _

on loaded(next)
  items = next

on failed(error)
  query = error.name

view
  input "Query" #query <-> query @w-full
"#;

#[path = "tests/application.rs"]
mod application;
#[path = "tests/basics.rs"]
mod basics;
#[path = "tests/flows.rs"]
mod flows;
#[path = "tests/operations.rs"]
mod operations;
#[path = "tests/values.rs"]
mod values;

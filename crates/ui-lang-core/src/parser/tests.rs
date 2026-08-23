use super::*;
use crate::test_support::example;

fn namespaced_line() -> Line {
    line_tree("Card", &[Some("catalog".into())], Rc::default())
        .unwrap()
        .pop()
        .unwrap()
}

#[test]
fn qualification_adds_only_a_missing_namespace() {
    let line = namespaced_line();

    assert_eq!(line.qualify("Card"), "catalog::Card");
    assert_eq!(line.qualify("catalog::Card"), "catalog::Card");
    assert_eq!(line.qualify("catalog"), "catalog");
    assert_eq!(line.qualify("catalogue::Card"), "catalog::catalogue::Card");
}

#[test]
#[ignore = "allocation contract; run alone with --test-threads=1"]
fn qualification_scans_the_namespace_prefix_in_place() {
    use stats_alloc::{INSTRUMENTED_SYSTEM, Region};

    const CALLS: usize = 4_000;
    let line = namespaced_line();
    let region = Region::new(&INSTRUMENTED_SYSTEM);

    for _ in 0..CALLS {
        std::hint::black_box(line.qualify(std::hint::black_box("Card")));
    }
    let stats = region.change();

    eprintln!(
        "{CALLS} qualified names: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, CALLS, "{stats:?}");
    assert_eq!(stats.reallocations, CALLS, "{stats:?}");
}

#[test]
fn fallback_symbol_ranges_require_one_unambiguous_occurrence() {
    const NAME: &str = "target";
    for (case, source, use_first, expected) in [
        ("unique candidate", "target", false, Some(1)),
        ("ambiguous candidates", "target target", false, None),
        ("uniquely assigned", "value=target target", false, Some(7)),
        ("multiply assigned", "a=target b=target", false, None),
        ("already used", "target target", true, Some(8)),
    ] {
        let symbols = Rc::default();
        let line = line_tree(source, &[None], Rc::clone(&symbols))
            .unwrap()
            .pop()
            .unwrap();
        if use_first {
            line.record_symbol(SymbolKind::Handler, NAME, false, &line.text[..NAME.len()]);
        }

        line.record_symbol(SymbolKind::Handler, NAME, false, NAME);
        let actual = symbols
            .borrow()
            .last()
            .unwrap()
            .range
            .as_ref()
            .map(|range| range.start_column);
        assert_eq!(actual, expected, "{case}");
    }
}

#[test]
#[ignore = "allocation contract; run alone with --test-threads=1"]
fn performance_contract_symbol_fallback_scans_offsets_in_place() {
    use stats_alloc::{INSTRUMENTED_SYSTEM, Region};

    const CALLS: usize = 4_000;
    const NAME: &str = "target";
    let symbols = Rc::default();
    let line = line_tree("value=target target", &[None], Rc::clone(&symbols))
        .unwrap()
        .pop()
        .unwrap();

    line.record_symbol(SymbolKind::Handler, NAME, false, NAME);
    assert_eq!(
        symbols.borrow()[0]
            .range
            .as_ref()
            .map(|range| range.start_column),
        Some(7)
    );
    symbols.borrow_mut().clear();

    let region = Region::new(&INSTRUMENTED_SYSTEM);
    for _ in 0..CALLS {
        std::hint::black_box(&line).record_symbol(
            SymbolKind::Handler,
            std::hint::black_box(NAME),
            false,
            std::hint::black_box(NAME),
        );
        symbols.borrow_mut().clear();
    }
    let stats = region.change();

    eprintln!(
        "{CALLS} fallback symbol locations: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, CALLS, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, CALLS * NAME.len(), "{stats:?}");
}

#[test]
#[ignore = "allocation contract; run alone with --test-threads=1"]
fn expression_parser_consumes_owned_token_payloads() {
    use stats_alloc::{INSTRUMENTED_SYSTEM, Region};

    const CALLS: usize = 4_000;
    const SOURCE: &str =
        r#"alpha + beta * gamma + service::lookup(item.field, "value", bytes(00 ff), 42)"#;
    let line = namespaced_line();
    drop(parse_expr(SOURCE, &line).unwrap());
    let region = Region::new(&INSTRUMENTED_SYSTEM);

    for _ in 0..CALLS {
        drop(std::hint::black_box(parse_expr(std::hint::black_box(SOURCE), &line)).unwrap());
    }
    let stats = region.change();

    eprintln!(
        "{CALLS} parsed expressions: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, 108_000, "{stats:?}");
    assert_eq!(stats.reallocations, 36_000, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 8_636_000, "{stats:?}");
}

#[test]
fn syntax_boundaries_ignore_escaped_quotes() {
    let quoted = r#""a\" b,=->)""#;

    assert_eq!(
        split_words(&format!("{quoted}\u{2003}tail")),
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
        original_text: None,
        metadata: Vec::new(),
        children: Vec::new(),
        namespace: None,
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

    for kind in ["pure", "sync"] {
        let error = parse(&format!(
            "app Demo\nextern crate::backend\n  {kind} bytes() -> i64\nview\n  text \"ok\"\n"
        ))
        .unwrap_err();
        assert_eq!(error.code, "E021");
        assert!(error.message.contains("byte literal"));
    }

    assert!(!SymbolKind::Handler.accepts("match"));
    assert!(!SymbolKind::Handler.accepts("_"));
    assert!(!SymbolKind::Component.accepts("Self"));
    assert!(SymbolKind::Recipe.accepts("primary_action"));

    let error = parse("app Demo\nextern backend::crate\nview\n  text \"ok\"\n").unwrap_err();
    assert_eq!(error.code, "E073");

    parse("app Demo\nextern crate::none::__backend\nview\n  text \"ok\"\n").unwrap();
}

#[test]
fn component_names_enforce_namespace_and_case_boundaries() {
    for name in [
        "Card",
        "Card.Header",
        "catalog::Card",
        "Catalog::Card",
        "catalog::controls::Card.Header",
    ] {
        assert!(SymbolKind::Component.accepts(name), "{name}");
    }
    for name in [
        "::Card",
        "catalog::",
        "catalog::::Card",
        "catalog::card",
        "Catalog::card",
        "catalog::Card.header",
    ] {
        assert!(!SymbolKind::Component.accepts(name), "{name}");
    }
}

#[test]
fn shadowed_image_constructors_require_explicit_state_types() {
    for (kind, signature, call) in [
        ("pure", "encoded(value:bytes) -> str", "encoded(bytes(00))"),
        ("sync", "encoded(value:bytes) -> str", "encoded(bytes(00))"),
        (
            "pure",
            "rgba(width:i64, height:i64, value:bytes) -> str",
            "rgba(1, 1, bytes(00))",
        ),
        (
            "sync",
            "rgba(width:i64, height:i64, value:bytes) -> str",
            "rgba(1, 1, bytes(00))",
        ),
    ] {
        let declaration = format!("{kind} {signature}");
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
fn parses_explicit_component_bind_props() {
    let document = parse(
        "app Demo\nstate\n  draft = \"\"\ncomponent Field(bind value:str, label:str)\n  text label\nview\n  Field value<->draft label=\"Name\"\n",
    )
    .unwrap();

    assert!(document.components[0].params[0].bind);
    assert!(!document.components[0].params[1].bind);
    let ViewNode::Component { args, .. } = &document.view else {
        panic!("expected component call");
    };
    assert!(args[0].bind);
    assert!(!args[1].bind);
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

theme contract AppTheme
  bg
palette app for AppTheme
  bg #000000

state
  items:[Item] = []
  query = ""

on mount
  run every load() -> loaded _ | failed _

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
#[path = "tests/testing.rs"]
mod testing;
#[path = "tests/values.rs"]
mod values;

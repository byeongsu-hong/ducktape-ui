//! Executes the generated surface fixture as a real wasm component.
//! CI bundles it separately from the user-facing catalog, then runs this test.

use ui_lang_wire::{self as wire, Event, Node, SurfaceValue as V};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

wasmtime::component::bindgen!({
    path: "../../../crates/ui-lang-guest/wit/view.wit",
    world: "view",
});

struct Host;
impl ViewImports for Host {
    fn panicked(&mut self, message: String) {
        panic!("guest panicked: {message}");
    }
}

fn surface<'a>(node: &'a Node, name: &str) -> (&'a [V], u32) {
    fn find<'a>(node: &'a Node, name: &str) -> Option<(&'a [V], u32)> {
        if let Node::Surface {
            name: found,
            args,
            on_event: Some(handler),
            ..
        } = node
            && found == name
        {
            return Some((args, *handler));
        }
        node.children().iter().find_map(|child| find(child, name))
    }
    find(node, name).unwrap_or_else(|| panic!("missing surface {name}"))
}

fn has_text(node: &Node, value: &str) -> bool {
    matches!(node, Node::Text { content, .. } if content == value)
        || node.children().iter().any(|child| has_text(child, value))
}

#[test]
#[ignore = "requires cargo ice bundle -p app-store-surface-fixture --out target/surface-fixture"]
fn a_bundled_guest_receives_typed_surface_events_and_patches_its_view() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/surface-fixture/app_store_surface_fixture.wasm");
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let component = Component::from_file(&engine, path).unwrap();
    let mut linker = Linker::new(&engine);
    View::add_to_linker::<Host, wasmtime::component::HasSelf<Host>>(&mut linker, |host| host)
        .unwrap();
    linker.define_unknown_imports_as_traps(&component).unwrap();
    let mut store = Store::new(&engine, Host);
    store.set_fuel(100_000_000).unwrap();
    let view = View::instantiate(&mut store, &component, &linker).unwrap();
    view.call_init(&mut store).unwrap();
    let mut root = Node::empty();
    let mut tick = |events: Vec<Event>| {
        store.set_fuel(100_000_000).unwrap();
        let bytes = view.call_tick(&mut store, &wire::encode(&events)).unwrap();
        let frame: wire::Frame = wire::decode(&bytes).unwrap();
        if let Some(node) = frame.root {
            root = node;
        } else if !frame.unchanged {
            wire::apply(&mut root, frame.patches).unwrap();
        }
        root.clone()
    };
    let boot = tick(vec![]);
    assert_eq!(
        surface(&boot, "preview").0,
        [
            V::Str("first".into()),
            V::Bool(false),
            V::I64(7),
            V::F64(1.5)
        ]
    );
    fn shader_bounds(node: &Node, name: &str) -> Option<(wire::Length, wire::Length)> {
        if let Node::Container {
            width: Some(width),
            height: Some(height),
            content,
            ..
        } = node
            && matches!(content.as_ref(), Node::Surface { name: found, .. } if found == name)
        {
            return Some((*width, *height));
        }
        node.children()
            .iter()
            .find_map(|child| shader_bounds(child, name))
    }
    assert_eq!(
        shader_bounds(&boot, "pulse"),
        Some((wire::Length::Fill, wire::Length::Fixed(24.0)))
    );
    assert_eq!(
        shader_bounds(&boot, "passive"),
        Some((wire::Length::Fixed(100.0), wire::Length::Fixed(100.0)))
    );
    assert_eq!(
        shader_bounds(&boot, "collapsed"),
        Some((wire::Length::Fixed(0.0), wire::Length::Fixed(0.0))),
        "shader shrink must preserve its zero intrinsic size"
    );
    assert_eq!(
        surface(&boot, "pulse").0,
        [
            V::F64(1.5),
            V::List(vec![V::Str("host".into()), V::Str("shader".into())])
        ]
    );
    let shader_wrong = tick(vec![Event::Surface {
        handler: surface(&boot, "pulse").1,
        value: V::Str("bad".into()),
    }]);
    assert_eq!(surface(&shader_wrong, "toggle").0, [V::Bool(false)]);
    let shader_changed = tick(vec![Event::Surface {
        handler: surface(&shader_wrong, "pulse").1,
        value: V::Bool(true),
    }]);
    assert_eq!(surface(&shader_changed, "toggle").0, [V::Bool(true)]);
    let boot = tick(vec![Event::Surface {
        handler: surface(&shader_changed, "pulse").1,
        value: V::Bool(false),
    }]);
    let wrong = tick(vec![Event::Surface {
        handler: surface(&boot, "preview").1,
        value: V::Bool(true),
    }]);
    assert!(has_text(&wrong, "initial"));
    let events = [
        ("preview", V::Str("duck://pages/example".into())),
        ("toggle", V::Bool(true)),
        ("integer", V::I64(i64::MAX)),
        ("number", V::F64(2.5)),
        ("action", V::Unit),
    ]
    .into_iter()
    .map(|(name, value)| Event::Surface {
        handler: surface(&wrong, name).1,
        value,
    })
    .collect();
    let changed = tick(events);
    assert_eq!(
        surface(&changed, "preview").0,
        [
            V::Str("first".into()),
            V::Bool(true),
            V::I64(i64::MAX),
            V::F64(2.5)
        ]
    );
    assert!(has_text(&changed, "duck://pages/example"));
    assert!(has_text(&changed, "first"));
    assert!(has_text(&changed, "1"));
    let mut rows = surface(&changed, "rows").0[0].clone();
    let V::List(values) = &mut rows else {
        panic!("expected row list")
    };
    assert_eq!(values.len(), 1);
    let V::Record { name, fields } = &mut values[0] else {
        panic!("expected record")
    };
    assert_eq!(name, "Row");
    assert_eq!(
        fields[2],
        (
            "note".into(),
            V::Option(Some(Box::new(V::Str("note".into()))))
        )
    );
    fields[1].1 = V::Str("edited in wasm".into());
    let changed = tick(vec![Event::Surface {
        handler: surface(&changed, "rows").1,
        value: rows.clone(),
    }]);
    assert_eq!(surface(&changed, "rows").0[0], rows);
    let wrong = tick(vec![Event::Surface {
        handler: surface(&changed, "rows").1,
        value: V::List(vec![V::Record {
            name: "Row".into(),
            fields: vec![],
        }]),
    }]);
    assert_eq!(surface(&wrong, "rows").0[0], rows);
    let V::List(values) = rows else {
        unreachable!()
    };
    let selected = V::Option(Some(Box::new(values[0].clone())));
    let changed = tick(vec![Event::Surface {
        handler: surface(&wrong, "optional").1,
        value: selected.clone(),
    }]);
    assert_eq!(surface(&changed, "rows").0[1], selected);
    fn document(node: &Node, name: &str) -> wire::MarkdownDocument {
        wire::MarkdownDocument::from_surface_value(&surface(node, name).0[0])
            .expect("markdown document argument")
    }
    fn button(node: &Node, label: &str) -> Option<u32> {
        if let Node::Button {
            content: wire::ButtonContent::Label(found),
            on_press,
            ..
        } = node
            && found == label
        {
            return *on_press;
        }
        node.children()
            .iter()
            .find_map(|child| button(child, label))
    }
    let doc = document(&changed, "ice.markdown");
    assert_eq!(doc.source, "[Open](duck://docs/start)");
    assert_eq!(doc.metrics[0], 18.0);
    assert_eq!(doc.metrics[1], 30.0);
    assert_eq!(doc.metrics[8], 7.0);
    assert_eq!(doc.padding, [2.0; 4]);
    assert_eq!(doc.radii, [3.0; 4]);
    assert_eq!(doc.palette[1], [1.0; 4]);
    assert_eq!(
        surface(&changed, "docs_viewer").0[1],
        V::Str("custom".into())
    );
    let appended = tick(vec![Event::Message(
        button(&changed, "Append docs").unwrap(),
    )]);
    assert!(
        document(&appended, "ice.markdown")
            .source
            .ends_with("![More](asset:more)"),
        "appended markdown source must cross the wire"
    );
    assert!(has_text(&appended, "asset:more"));
    let linked = tick(vec![Event::Surface {
        handler: surface(&appended, "ice.markdown").1,
        value: V::Str("duck://docs/start".into()),
    }]);
    assert!(has_text(&linked, "duck://docs/start"));
    assert!(has_text(&linked, "default"));
    let wrong = tick(vec![Event::Surface {
        handler: surface(&linked, "docs_viewer").1,
        value: V::Bool(true),
    }]);
    assert!(has_text(&wrong, "default"));
    let linked = tick(vec![Event::Surface {
        handler: surface(&wrong, "docs_viewer").1,
        value: V::Str("duck://custom".into()),
    }]);
    assert!(has_text(&linked, "duck://custom"));
    assert!(has_text(&linked, "custom"));
    let reset = tick(vec![Event::Message(button(&linked, "Reset docs").unwrap())]);
    assert_eq!(document(&reset, "ice.markdown").source, "Replacement");
    assert!(!has_text(&reset, "asset:more"));
}

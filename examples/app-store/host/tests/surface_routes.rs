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
}

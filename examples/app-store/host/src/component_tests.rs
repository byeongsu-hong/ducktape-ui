//! Mounted guest state and tasks, through an actual componentized module.
use super::layers_tests::{build, click, redraw, renderer};
use super::*;
use iced_test::runtime::user_interface;

fn guest() -> Guest {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/component-fixture/app_store_component_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle component fixture first");
    Guest::load(&CatalogEntry {
        preferred_size: None,
        id: "component-fixture".into(),
        name: "Component fixture".into(),
        description: String::new(),
        capabilities: vec![],
        hash: Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        path: path.to_string_lossy().into_owned(),
        mark: "C".into(),
    })
    .unwrap()
}
fn value(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Text { key, content, .. } = node
        && key.ends_with(suffix)
    {
        return Some(content.clone());
    }
    node.children().iter().find_map(|node| value(node, suffix))
}
fn read(guest: &Guest, suffix: &str) -> Option<String> {
    value(guest.frame.root.as_ref().unwrap(), suffix)
}
fn press(guest: &mut Guest, label: &str) {
    fn route(node: &wire::Node, label: &str) -> Option<u32> {
        if let wire::Node::Button {
            content: wire::ButtonContent::Label(text),
            on_press,
            ..
        } = node
            && text == label
        {
            return *on_press;
        }
        node.children().iter().find_map(|node| route(node, label))
    }
    let handler = route(guest.frame.root.as_ref().unwrap(), label).unwrap();
    guest.pending.push(wire::Event::Message(handler));
    guest.tick();
    assert!(guest.fault.is_none(), "{:?}", guest.fault);
}

#[test]
#[ignore = "requires bundled component-fixture wasm"]
fn bundled_component_mount_reboots_while_retained_state_survives() {
    let guest = Arc::new(Mutex::new(guest()));
    let mut renderer = renderer();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    let mut now = Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(
        read(&guest.lock().unwrap(), "/mounted-value").as_deref(),
        Some("7")
    );
    for label in ["Mounted increment", "Retained increment"] {
        click(&mut ui, &mut renderer, label);
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
    }
    assert_eq!(
        read(&guest.lock().unwrap(), "/mounted-value").as_deref(),
        Some("8")
    );
    assert_eq!(
        read(&guest.lock().unwrap(), "/retained-value").as_deref(),
        Some("1")
    );
    click(&mut ui, &mut renderer, "Toggle counters");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(read(&guest.lock().unwrap(), "/mounted-value"), None);
    click(&mut ui, &mut renderer, "Toggle counters");
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(
        read(&guest.lock().unwrap(), "/mounted-value").as_deref(),
        Some("27")
    );
    assert_eq!(
        read(&guest.lock().unwrap(), "/retained-value").as_deref(),
        Some("1")
    );
    drop(ui);
    let mut other = self::guest();
    for _ in 0..3 {
        other.tick();
    }
    assert_eq!(read(&other, "/mounted-value").as_deref(), Some("7"));
    assert_eq!(read(&other, "/retained-value").as_deref(), Some("0"));
}

#[test]
#[ignore = "requires bundled component-fixture wasm"]
fn bundled_component_unmount_cancels_work_and_ignores_old_replies() {
    let mut guest = guest();
    guest.tick();
    press(&mut guest, "Toggle fetch");
    assert!(guest.frame.busy, "first sighting must schedule its boot");
    guest.tick();
    let request = guest
        .frame
        .requests
        .iter()
        .find(|r| r.kind == "lifecycle.fetch")
        .unwrap()
        .clone();
    assert_eq!(request.payload, 17i64.to_le_bytes());
    press(&mut guest, "Toggle fetch");
    assert!(
        guest.frame.busy,
        "pruning wakes cancellation after rendering"
    );
    guest.tick();
    assert!(
        guest.frame.cancels.contains(&request.id),
        "unmount cancels the pending host request"
    );
    assert_eq!(read(&guest, "/fetched"), None);
    press(&mut guest, "Toggle fetch");
    guest.tick();
    let replacement = guest
        .frame
        .requests
        .iter()
        .find(|r| r.kind == "lifecycle.fetch")
        .unwrap()
        .clone();
    assert_ne!(request.id, replacement.id);
    guest.pending.push(wire::Event::Response {
        id: request.id,
        result: Ok(999i64.to_le_bytes().to_vec()),
        done: true,
    });
    guest.tick();
    assert_eq!(read(&guest, "/fetched").as_deref(), Some("0"));
    guest.pending.push(wire::Event::Response {
        id: replacement.id,
        result: Ok(37i64.to_le_bytes().to_vec()),
        done: true,
    });
    guest.tick();
    assert!(guest.fault.is_none(), "{:?}", guest.fault);
    assert_eq!(read(&guest, "/fetched").as_deref(), Some("37"));
}

// Instantiate the actual component without init: restoring must be sufficient
// to establish its driver. The old host/window is deliberately not replaced by
// these export tests; catalog-driven host replacement has its own follow-up.
struct Restored {
    store: Store<HostState>,
    view: View,
    root: wire::Node,
}
impl Restored {
    fn new(entry: &CatalogEntry, bytes: &[u8]) -> Self {
        let engine = engine();
        let (component, _) = component(entry).unwrap();
        let mut linker = Linker::new(engine);
        View::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |s| s,
        )
        .unwrap();
        linker.define_unknown_imports_as_traps(&component).unwrap();
        let mut store = Store::new(
            engine,
            HostState {
                limits: StoreLimitsBuilder::new().build(),
                panic: None,
            },
        );
        arm(&mut store);
        let view = View::instantiate(&mut store, &component, &linker).unwrap();
        arm(&mut store);
        view.call_restore(&mut store, bytes, cfg!(target_os = "macos"))
            .unwrap()
            .unwrap();
        Self {
            store,
            view,
            root: wire::Node::empty(),
        }
    }
    fn tick(&mut self, events: Vec<wire::Event>) -> wire::Frame {
        arm(&mut self.store);
        let bytes = self
            .view
            .call_tick(&mut self.store, &wire::encode(&events))
            .unwrap();
        let frame: wire::Frame = wire::decode(&bytes).unwrap();
        if let Some(root) = &frame.root {
            self.root = root.clone();
        } else if !frame.unchanged {
            wire::apply(&mut self.root, frame.patches.clone()).unwrap();
        }
        frame
    }
    fn press(&mut self, label: &str) -> wire::Frame {
        fn route(node: &wire::Node, label: &str) -> Option<u32> {
            if let wire::Node::Button {
                content: wire::ButtonContent::Label(text),
                on_press,
                ..
            } = node
                && text == label
            {
                return *on_press;
            }
            node.children().iter().find_map(|child| route(child, label))
        }
        self.tick(vec![wire::Event::Message(
            route(&self.root, label).unwrap(),
        )])
    }
}
fn snapshot(guest: &mut Guest) -> Result<Vec<u8>, String> {
    guest.backend.snapshot()
}

#[test]
#[ignore = "requires bundled component-fixture wasm"]
fn bundled_snapshot_restores_native_draft_component_state_and_fresh_routes() {
    use super::layers_tests::{focus, type_text};
    let guest = Arc::new(Mutex::new(guest()));
    let mut renderer = renderer();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    let mut now = Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert!(focus(&mut ui, &renderer, "Repro/root/draft"));
    for character in "saved draft".chars() {
        type_text(&mut ui, &mut renderer, &character.to_string());
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
    }
    for label in ["Mounted increment", "Retained increment"] {
        click(&mut ui, &mut renderer, label);
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
    }
    drop(ui);
    let mut original = guest.lock().unwrap();
    assert_eq!(
        read(&original, "/draft-value").as_deref(),
        Some("saved draft")
    );
    let bytes = snapshot(&mut original).unwrap();
    let mut restored = Restored::new(&original.entry, &bytes);
    assert!(restored.tick(vec![]).root.is_some());
    for _ in 0..3 {
        assert!(restored.tick(vec![]).requests.is_empty());
    }
    assert_eq!(
        value(&restored.root, "/draft-value").as_deref(),
        Some("saved draft")
    );
    assert_eq!(
        value(&restored.root, "/mounted-value").as_deref(),
        Some("8")
    );
    assert_eq!(
        value(&restored.root, "/retained-value").as_deref(),
        Some("1")
    );
    restored.press("Mounted increment");
    assert_eq!(
        value(&restored.root, "/mounted-value").as_deref(),
        Some("9")
    );
    assert_eq!(read(&original, "/mounted-value").as_deref(), Some("8"));
    for changed_schema in [true, false] {
        let mut invalid = wire::Snapshot::decode(&bytes).unwrap();
        if changed_schema {
            invalid.schema = "0".repeat(64);
        } else if let wire::SnapshotValue::Record { fields, .. } = &mut invalid.state {
            fields.pop();
        }
        arm(&mut restored.store);
        assert!(
            restored
                .view
                .call_restore(&mut restored.store, &invalid.encode().unwrap(), false)
                .unwrap()
                .is_err()
        );
    }
    restored.press("Mounted increment");
    assert_eq!(
        value(&restored.root, "/mounted-value").as_deref(),
        Some("10"),
        "failed restores preserve state and routes"
    );
}

#[test]
#[ignore = "requires bundled component-fixture wasm"]
fn bundled_snapshot_waits_for_writes_and_restores_boot_markers() {
    let mut original = guest();
    for _ in 0..3 {
        original.tick();
    }
    press(&mut original, "Toggle fetch");
    original.tick();
    let request = original
        .frame
        .requests
        .iter()
        .find(|r| r.kind == "lifecycle.fetch")
        .unwrap()
        .clone();
    assert!(
        snapshot(&mut original).is_err(),
        "a pending user task must survive until completion"
    );
    original.pending.push(wire::Event::Response {
        id: request.id,
        result: Ok(43i64.to_le_bytes().to_vec()),
        done: true,
    });
    original.tick();
    let bytes = snapshot(&mut original).unwrap();
    let mut restored = Restored::new(&original.entry, &bytes);
    for _ in 0..4 {
        assert!(
            restored.tick(vec![]).requests.is_empty(),
            "restored mounted boot must not request again"
        );
    }
    assert_eq!(value(&restored.root, "/fetched").as_deref(), Some("43"));
    restored.press("Toggle fetch");
    restored.tick(vec![]);
    restored.press("Toggle fetch");
    let frame = restored.tick(vec![]);
    assert_eq!(
        frame
            .requests
            .iter()
            .filter(|r| r.kind == "lifecycle.fetch")
            .count(),
        1,
        "a genuinely remounted component boots normally"
    );
}

#[test]
#[ignore = "requires bundled component-fixture wasm"]
fn bundled_snapshot_uses_saved_initials_for_untouched_components() {
    let mut original = guest();
    for _ in 0..3 {
        original.tick();
    }
    assert_eq!(read(&original, "/retained-value").as_deref(), Some("0"));
    // A compatible old build can have a different initializer. This is its
    // saved initial instance; the retained scope has never received an event.
    let mut saved = wire::Snapshot::decode(&snapshot(&mut original).unwrap()).unwrap();
    let wire::SnapshotValue::Record { fields, .. } = &mut saved.state else {
        panic!("app record")
    };
    let initial = fields
        .iter_mut()
        .find_map(|(_, value)| match value {
            wire::SnapshotValue::Record { name, fields } if name == "RetainedCounter" => {
                Some(fields)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(initial[0].0, "count");
    initial[0].1 = wire::SnapshotValue::I64(55);
    let mut restored = Restored::new(&original.entry, &saved.encode().unwrap());
    restored.tick(vec![]);
    assert_eq!(
        value(&restored.root, "/retained-value").as_deref(),
        Some("55"),
        "untouched state must use the saved initializer"
    );
    restored.press("Retained increment");
    assert_eq!(
        value(&restored.root, "/retained-value").as_deref(),
        Some("56"),
        "first event must materialize the saved initial state"
    );
}

#[test]
#[ignore = "requires bundled component-fixture wasm"]
fn bundled_snapshot_round_trips_all_owned_data_shapes_and_rejects_nested_mismatch() {
    use wire::SnapshotValue as V;
    let mut original = guest();
    for _ in 0..3 {
        original.tick();
    }
    let bytes = snapshot(&mut original).unwrap();
    let saved = wire::Snapshot::decode(&bytes).unwrap();
    let V::Record { fields, .. } = &saved.state else {
        panic!("app record")
    };
    let field = |name: &str| &fields.iter().find(|(key, _)| key == name).unwrap().1;
    assert_eq!(field("payload"), &V::Bytes(vec![0, 255, 164]));
    assert_eq!(field("content"), &V::Str("editor draft".into()));
    assert_eq!(field("document"), &V::Str("# Heading".into()));
    assert_eq!(
        field("nested"),
        &V::List(vec![
            V::Option(Some(Box::new(V::Str("one".into())))),
            V::Option(None)
        ])
    );
    assert_eq!(
        field("success"),
        &V::Record {
            name: "result".into(),
            fields: vec![("ok".into(), V::Str("ready".into()))]
        }
    );
    assert_eq!(
        field("failure"),
        &V::Record {
            name: "result".into(),
            fields: vec![("error".into(), V::Str("offline".into()))]
        }
    );
    assert_eq!(
        field("choice"),
        &V::Record {
            name: "SnapshotChoice".into(),
            fields: vec![("page".into(), V::Str("details".into()))]
        }
    );
    assert_eq!(
        field("fallback_choice"),
        &V::Record {
            name: "SnapshotChoice".into(),
            fields: vec![("idle".into(), V::Unit)]
        }
    );
    assert_eq!(
        field("sample"),
        &V::Record {
            name: "Sample".into(),
            fields: vec![
                ("name".into(), V::Str("nested record".into())),
                ("values".into(), V::List(vec![V::F64(1.5), V::F64(-2.0)])),
                (
                    "optional".into(),
                    V::Option(Some(Box::new(V::Str("note".into()))))
                )
            ]
        }
    );
    let mut restored = Restored::new(&original.entry, &bytes);
    arm(&mut restored.store);
    let again = restored
        .view
        .call_snapshot(&mut restored.store)
        .unwrap()
        .unwrap();
    assert_eq!(
        again, bytes,
        "complete snapshot is canonical across instances"
    );
    let mut invalid = saved;
    let V::Record { fields, .. } = &mut invalid.state else {
        unreachable!()
    };
    let V::Record { fields, .. } = &mut fields
        .iter_mut()
        .find(|(name, _)| name == "sample")
        .unwrap()
        .1
    else {
        unreachable!()
    };
    fields
        .iter_mut()
        .find(|(name, _)| name == "values")
        .unwrap()
        .1 = V::List(vec![V::Str("not a number".into())]);
    arm(&mut restored.store);
    assert!(
        restored
            .view
            .call_restore(&mut restored.store, &invalid.encode().unwrap(), false)
            .unwrap()
            .is_err()
    );
    arm(&mut restored.store);
    assert_eq!(
        restored
            .view
            .call_snapshot(&mut restored.store)
            .unwrap()
            .unwrap(),
        bytes,
        "nested rejection preserves the complete previous state"
    );
}

#[test]
#[ignore = "requires bundled component-fixture wasm"]
fn bundled_snapshot_restore_does_not_replay_source_initializers() {
    let mut original = guest();
    for _ in 0..3 {
        original.tick();
    }
    press(&mut original, "Report initializations");
    assert_eq!(read(&original, "/init-report").as_deref(), Some("1"));
    let bytes = snapshot(&mut original).unwrap();
    let mut restored = Restored::new(&original.entry, &bytes);
    restored.tick(vec![]);
    restored.press("Report initializations");
    assert_eq!(
        value(&restored.root, "/init-report").as_deref(),
        Some("0"),
        "a fresh instance restores without executing any source initializer"
    );
}

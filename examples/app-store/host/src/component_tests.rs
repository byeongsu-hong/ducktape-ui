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

//! Actual wasm lazy cache hits, native pointer routes, and instance isolation.
use super::*;
use iced::Rectangle;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Operation;
use iced::{Event, Font, Pixels, Size, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};

type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;
fn renderer() -> iced::Renderer {
    iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .unwrap()
}
fn guest() -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/lazy-fixture/app_store_lazy_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle lazy fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut guest = Guest::load(&CatalogEntry {
        preferred_size: None,
        id: "lazy-fixture".into(),
        name: "Lazy fixture".into(),
        description: String::new(),
        capabilities: vec![],
        path: path.to_string_lossy().into_owned(),
        mark: "L".into(),
        hash,
    })
    .unwrap();
    guest.surfaces.insert(
        "echo".into(),
        Arc::new(|_, args| {
            let [wire::SurfaceValue::I64(value)] = args else {
                panic!("typed echo args");
            };
            iced::widget::button("Echo")
                .on_press(wire::SurfaceValue::I64(*value))
                .into()
        }),
    );
    Arc::new(Mutex::new(guest))
}
fn build(
    guest: &Arc<Mutex<Guest>>,
    cache: user_interface::Cache,
    renderer: &mut iced::Renderer,
    width: f32,
) -> Ui {
    UserInterface::build(
        wasm_view(Surface(guest.clone()), false),
        Size::new(width, 600.0),
        cache,
        renderer,
    )
}
fn redraw(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut std::time::Instant,
    width: f32,
) -> Ui {
    *now += std::time::Duration::from_secs(1);
    let mut messages = vec![];
    ui.update(
        &[Event::Window(window::Event::RedrawRequested(*now))],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut messages,
    );
    assert!(
        guest.lock().unwrap().fault.is_none(),
        "guest must remain live"
    );
    if messages.iter().any(|m| m == "wake") {
        build(guest, ui.into_cache(), renderer, width)
    } else {
        ui
    }
}
struct TextBounds<'a> {
    label: &'a str,
    bounds: Option<Rectangle>,
}
impl Operation for TextBounds<'_> {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn text(&mut self, _id: Option<&iced::widget::Id>, bounds: Rectangle, text: &str) {
        if text == self.label {
            self.bounds = Some(bounds);
        }
    }
}
fn bounds(ui: &mut Ui, renderer: &mut iced::Renderer, label: &str) -> Option<Rectangle> {
    let mut op = TextBounds {
        label,
        bounds: None,
    };
    ui.operate(renderer, &mut op);
    op.bounds
}
fn click(ui: &mut Ui, renderer: &mut iced::Renderer, label: &str) {
    let point = bounds(ui, renderer, label)
        .expect("visible button label")
        .center();
    ui.update(
        &[
            Event::Mouse(mouse::Event::CursorMoved { position: point }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
        mouse::Cursor::Available(point),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}

fn snapshot(guest: &Arc<Mutex<Guest>>) -> (Vec<u64>, String, Option<u32>) {
    fn walk(node: &wire::Node, result: &mut (Vec<u64>, String, Option<u32>)) {
        match node {
            wire::Node::Lazy { generation, .. } => result.0.push(*generation),
            wire::Node::Text { key, content, .. } if key.contains("chosen") => {
                result.1 = content.clone()
            }
            wire::Node::Button { key, on_press, .. } if key.contains("choose") => {
                result.2 = *on_press
            }
            _ => {}
        }
        for child in node.children() {
            walk(child, result);
        }
    }
    let mut result = (vec![], String::new(), None);
    walk(
        guest.lock().unwrap().frame.root.as_ref().unwrap(),
        &mut result,
    );
    result
}

#[test]
#[ignore = "requires bundled lazy-fixture wasm"]
fn bundled_nested_lazy_preserves_pointer_routes_on_hits_and_expires_old_generations() {
    let guest = guest();
    let mut renderer = renderer();
    let mut now = std::time::Instant::now();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        400.0,
    );
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    }
    fn identified_lazy(node: &wire::Node, suffix: &str) -> bool {
        matches!(node, wire::Node::Lazy { key, .. } if key.ends_with(suffix))
            || node
                .children()
                .iter()
                .any(|child| identified_lazy(child, suffix))
    }
    for suffix in ["/cached_outer", "/cached_outer/cached_inner"] {
        assert!(
            identified_lazy(guest.lock().unwrap().frame.root.as_ref().unwrap(), suffix),
            "authored nested lazy IDs must cross unchanged: {suffix}"
        );
    }
    let initial = snapshot(&guest);
    assert_eq!(
        initial.0.len(),
        2,
        "both lazy boundaries must cross the wire"
    );
    assert_eq!(initial.1, "-1");
    let old_route = initial.2.expect("cached button route");
    assert!(
        old_route >= 1 << 31,
        "cached route must use its durable range"
    );

    click(&mut ui, &mut renderer, "Pulse");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert_eq!(
        snapshot(&guest).0,
        initial.0,
        "unrelated state must preserve nested cache generations"
    );
    click(&mut ui, &mut renderer, "1");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert_eq!(
        snapshot(&guest).1,
        "1",
        "native pointer click must execute the cached route after a hit"
    );

    click(&mut ui, &mut renderer, "Toggle");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert!(
        snapshot(&guest).0.is_empty(),
        "hidden lazy boundaries must leave the wire tree"
    );
    click(&mut ui, &mut renderer, "Toggle");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert_eq!(
        snapshot(&guest).0,
        initial.0,
        "same-content remount must reclaim the guest cache"
    );

    click(&mut ui, &mut renderer, "Next");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert_ne!(
        snapshot(&guest).0,
        initial.0,
        "changed dependencies must rebuild both boundaries"
    );
    click(&mut ui, &mut renderer, "Pulse");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert_eq!(snapshot(&guest).1, "1");
    click(&mut ui, &mut renderer, "Echo");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert_eq!(
        snapshot(&guest).1,
        "2",
        "typed surface route must survive a nested cache hit"
    );
    guest.lock().unwrap().deliver(Output::Activate(old_route));
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert_eq!(
        snapshot(&guest).1,
        "2",
        "an old generation's route must not fire or alias the new route"
    );

    let other = self::guest();
    ui = build(&other, ui.into_cache(), &mut renderer, 400.0);
    for _ in 0..4 {
        ui = redraw(ui, &other, &mut renderer, &mut now, 400.0);
    }
    assert_eq!(snapshot(&other).1, "-1");
    click(&mut ui, &mut renderer, "1");
    let _ui = redraw(ui, &other, &mut renderer, &mut now, 400.0);
    assert_eq!(
        snapshot(&other).1,
        "1",
        "replacement instance must use its own native and guest caches"
    );
    assert_eq!(snapshot(&guest).1, "2");
}

fn row_snapshot(guest: &Arc<Mutex<Guest>>) -> std::collections::BTreeMap<String, (u64, String)> {
    fn walk(node: &wire::Node, result: &mut std::collections::BTreeMap<String, (u64, String)>) {
        if let wire::Node::Lazy {
            key,
            generation,
            content,
        } = node
        {
            fn count(node: &wire::Node) -> Option<String> {
                if let wire::Node::Text { key, content, .. } = node
                    && key.contains("row_count")
                {
                    return Some(content.clone());
                }
                node.children().iter().find_map(count)
            }
            if let Some(count) = count(content) {
                result.insert(key.clone(), (*generation, count));
            }
        }
        for child in node.children() {
            walk(child, result);
        }
    }
    let mut result = std::collections::BTreeMap::new();
    walk(
        guest.lock().unwrap().frame.root.as_ref().unwrap(),
        &mut result,
    );
    result
}

#[test]
#[ignore = "requires bundled lazy-fixture wasm"]
fn bundled_lazy_component_routes_follow_keyed_rows_after_reordering() {
    let guest = guest();
    let mut renderer = renderer();
    let mut now = std::time::Instant::now();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    click(&mut ui, &mut renderer, "Rows");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    let initial = row_snapshot(&guest);
    assert_eq!(
        initial.len(),
        3,
        "all virtual component rows must contain lazy boundaries"
    );
    assert!(initial.values().all(|(_, count)| count == "0"));
    let second_key = initial
        .keys()
        .find(|key| key.contains("/key(2)/"))
        .expect("stable row key")
        .clone();
    click(&mut ui, &mut renderer, "2");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    let changed = row_snapshot(&guest);
    assert_eq!(
        changed[&second_key].1, "1",
        "component-local handler must update row 2"
    );
    for (key, value) in &initial {
        if key != &second_key {
            assert_eq!(
                &changed[key], value,
                "other keyed component caches must remain unchanged"
            );
        }
    }
    click(&mut ui, &mut renderer, "Reorder");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    assert_eq!(
        row_snapshot(&guest),
        changed,
        "reordering must preserve local state and cache generations"
    );
    click(&mut ui, &mut renderer, "2");
    let _ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    let reordered = row_snapshot(&guest);
    assert_eq!(
        reordered[&second_key].1, "2",
        "cached local route must still address row 2 after moving"
    );
    for (key, value) in &changed {
        if key != &second_key {
            assert_eq!(&reordered[key], value);
        }
    }
}

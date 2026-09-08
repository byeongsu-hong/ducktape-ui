//! Actual wasm layers, exercised through native pointer events.
use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Operation;
use iced::{Event, Font, Pixels, Rectangle, Size, keyboard, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};
use std::sync::atomic::{AtomicUsize, Ordering};
use ui_lang_runtime::view_tree;

pub(super) type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;
pub(super) fn renderer() -> iced::Renderer {
    iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .unwrap()
}
fn guest(live: Arc<AtomicUsize>) -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/layers-fixture/app_store_layers_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle layers fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut guest = Guest::load(&CatalogEntry {
        id: "layers-fixture".into(),
        name: "Layers fixture".into(),
        description: String::new(),
        capabilities: vec![],
        path: path.to_string_lossy().into_owned(),
        mark: "L".into(),
        hash,
    })
    .unwrap();
    struct Lease(Arc<AtomicUsize>);
    impl Drop for Lease {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    impl Lease {
        fn keep(&self) {}
    }
    guest.surfaces.insert(
        "marker".into(),
        Arc::new(move |_, _| {
            live.fetch_add(1, Ordering::SeqCst);
            let lease = Lease(live.clone());
            let element: iced::Element<'static, (), iced::Theme, iced::Renderer> =
                iced::widget::text("Mounted marker").into();
            element.map(move |()| {
                lease.keep();
                wire::SurfaceValue::Unit
            })
        }),
    );
    Arc::new(Mutex::new(guest))
}
pub(super) fn build(
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
pub(super) fn redraw(
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
pub(super) fn click(ui: &mut Ui, renderer: &mut iced::Renderer, label: &str) {
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

pub(super) fn container_bounds(ui: &mut Ui, renderer: &iced::Renderer, key: &str) -> Rectangle {
    struct Find<'a>(&'a str, Option<Rectangle>);
    impl Operation for Find<'_> {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn container(&mut self, id: Option<&iced::widget::Id>, bounds: Rectangle) {
            if id == Some(&iced::widget::Id::from(self.0.to_owned())) {
                self.1 = Some(bounds);
            }
        }
    }
    let mut find = Find(key, None);
    ui.operate(renderer, &mut find);
    find.1.expect("named container bounds")
}
fn green_area(ui: &mut Ui, renderer: &mut iced::Renderer, cursor: mouse::Cursor) -> usize {
    ui.draw(
        renderer,
        &iced::Theme::Dark,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::WHITE,
        },
        cursor,
    );
    renderer
        .screenshot(Size::new(640, 600), 1.0, iced::Color::BLACK)
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] == 0 && pixel[2] == 0)
        .map(|pixel| usize::from(pixel[1]))
        .sum::<usize>()
        / 255
}

pub(super) fn focus(ui: &mut Ui, renderer: &iced::Renderer, key: &str) -> bool {
    view_tree::execute_widget_command(wire::WidgetCommand::Focus { target: key.into() }, |op| {
        ui.operate(renderer, op)
    })
    .expect("host focus command");
    let result = view_tree::execute_widget_command(
        wire::WidgetCommand::Focused { target: key.into() },
        |op| ui.operate(renderer, op),
    )
    .unwrap();
    wire::decode(&result).unwrap()
}
pub(super) fn type_text(ui: &mut Ui, renderer: &mut iced::Renderer, text: &str) {
    ui.update(
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(text.into()),
            modified_key: keyboard::Key::Character(text.into()),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: Some(text.into()),
            repeat: false,
        })],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn base_value(guest: &Arc<Mutex<Guest>>) -> String {
    fn find(node: &wire::Node) -> Option<String> {
        if let wire::Node::Input { key, value, .. } = node
            && key == "LayersFixture/modal/base-input"
        {
            return Some(value.clone());
        }
        node.children().iter().find_map(find)
    }
    find(guest.lock().unwrap().frame.root.as_ref().unwrap()).expect("base input")
}

#[test]
#[ignore = "requires bundled layers-fixture wasm"]
fn bundled_layers_route_top_and_reveal_buttons_and_modal_dismissal() {
    let mut renderer = renderer();
    let live = Arc::new(AtomicUsize::new(0));
    let guest = guest(live.clone());
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        640.0,
    );
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(bounds(&mut ui, &mut renderer, "Wide layer").is_some());
    assert!(
        bounds(&mut ui, &mut renderer, "Narrow layer").is_none(),
        "stack splices only selected responsive children"
    );
    assert_eq!(
        container_bounds(&mut ui, &renderer, "LayersFixture/modal/under-parent").size(),
        Size::new(50.0, 20.0),
        "under layers preserve native base sizing"
    );
    let union = container_bounds(&mut ui, &renderer, "LayersFixture/modal/union-parent");
    assert_eq!(
        union.size(),
        Size::new(80.0, 80.0),
        "stack measures the union of its layers"
    );
    let fill = container_bounds(&mut ui, &renderer, "LayersFixture/modal/fill-parent");
    assert!(
        fill.height > 10.0 && fill.width == 200.0,
        "unconstrained stack must retain its children's Fill hint: {fill:?}"
    );
    let hidden = green_area(&mut ui, &mut renderer, mouse::Cursor::Unavailable);
    let reveal_point = bounds(&mut ui, &mut renderer, "Reveal").unwrap().center();
    let ticks = guest.lock().unwrap().ticks;
    let revealed = green_area(
        &mut ui,
        &mut renderer,
        mouse::Cursor::Available(reveal_point),
    );
    assert!(revealed > hidden + 200, "native hover paints the reveal");
    assert_eq!(
        guest.lock().unwrap().ticks,
        ticks,
        "hover painting needs no guest tick"
    );
    click(&mut ui, &mut renderer, "Top");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(
        bounds(&mut ui, &mut renderer, "top").is_some(),
        "top layer wins overlapping buttons"
    );
    click(&mut ui, &mut renderer, "Reveal");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(
        bounds(&mut ui, &mut renderer, "reveal").is_some(),
        "native hover reveals and routes without guest hover state"
    );
    click(&mut ui, &mut renderer, "Hold reveal");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(
        green_area(&mut ui, &mut renderer, mouse::Cursor::Unavailable) > hidden + 200,
        "guest open holds the reveal visible after the cursor leaves"
    );
    assert_eq!(
        live.load(Ordering::SeqCst),
        0,
        "closed modal does not mount its native surface"
    );
    assert!(focus(&mut ui, &renderer, "LayersFixture/modal/base-input"));
    type_text(&mut ui, &mut renderer, "A");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert_eq!(
        base_value(&guest),
        "A",
        "visible base input receives native keyboard events"
    );
    click(&mut ui, &mut renderer, "Open modal");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(bounds(&mut ui, &mut renderer, "Modal panel").is_some());
    if let Ok(path) = std::env::var("ICE_LAYERS_CAPTURE") {
        ui.draw(
            &mut renderer,
            &iced::Theme::Dark,
            &iced::advanced::renderer::Style {
                text_color: iced::Color::WHITE,
            },
            mouse::Cursor::Unavailable,
        );
        std::fs::write(
            path,
            renderer.screenshot(Size::new(640, 600), 1.0, iced::Color::BLACK),
        )
        .unwrap();
    }
    assert_eq!(
        live.load(Ordering::SeqCst),
        1,
        "open modal owns one native surface"
    );
    assert!(
        !focus(&mut ui, &renderer, "LayersFixture/modal/base-input"),
        "modal excludes the base from focus operations"
    );
    type_text(&mut ui, &mut renderer, "X");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert_eq!(base_value(&guest), "A", "modal blocks base keyboard input");

    click(&mut ui, &mut renderer, "Panel action");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(
        bounds(&mut ui, &mut renderer, "Modal panel").is_some(),
        "panel click must not dismiss"
    );
    let point = iced::Point::new(630.0, 590.0);
    ui.update(
        &[
            Event::Mouse(mouse::Event::CursorMoved { position: point }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
        mouse::Cursor::Available(point),
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(
        bounds(&mut ui, &mut renderer, "Modal panel").is_none(),
        "backdrop route closes modal"
    );
    assert!(
        bounds(&mut ui, &mut renderer, "panel").is_some(),
        "panel interaction reached guest"
    );
    assert_eq!(
        live.load(Ordering::SeqCst),
        0,
        "closing releases the mounted modal surface"
    );
    assert!(focus(&mut ui, &renderer, "LayersFixture/modal/base-input"));
    type_text(&mut ui, &mut renderer, "B");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert_eq!(
        base_value(&guest),
        "AB",
        "base input resumes after modal closes"
    );
}

#[test]
#[ignore = "requires bundled layers-fixture wasm"]
fn bundled_pin_uses_local_coordinates_and_routes_after_moving() {
    let mut renderer = renderer();
    let guest = guest(Arc::new(AtomicUsize::new(0)));
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        640.0,
    );
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    let parent = container_bounds(&mut ui, &renderer, "LayersFixture/modal/pin-parent");
    let child = container_bounds(
        &mut ui,
        &renderer,
        "LayersFixture/modal/pin-parent/pin-child",
    );
    assert_eq!(parent.size(), Size::new(200.0, 60.0));
    assert_eq!(child.size(), Size::new(70.0, 28.0));
    assert_eq!(
        (child.x - parent.x, child.y - parent.y),
        (32.0, 8.0),
        "nested offsets are widget-local, including negative offsets"
    );
    click(&mut ui, &mut renderer, "Pinned");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(
        bounds(&mut ui, &mut renderer, "pinned").is_some(),
        "positioned button must reach the guest"
    );
    let moved = container_bounds(
        &mut ui,
        &renderer,
        "LayersFixture/modal/pin-parent/pin-child",
    );
    assert_eq!(
        moved.x - child.x,
        20.0,
        "guest updates reposition the same child"
    );
    click(&mut ui, &mut renderer, "Pinned");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    let moved_again = container_bounds(
        &mut ui,
        &renderer,
        "LayersFixture/modal/pin-parent/pin-child",
    );
    assert_eq!(
        moved_again.x - moved.x,
        20.0,
        "hit testing follows the updated position"
    );
}

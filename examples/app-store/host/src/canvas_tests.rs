//! The actual wasm guest mounted in GuestView, using native mouse/key events.
use super::*;
use iced::advanced::renderer::Headless;
use iced::{Event, Size, mouse, window};
use iced::{Font, Pixels};
use iced_test::runtime::{UserInterface, user_interface};

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
        .join("../target/canvas-fixture/app_store_canvas_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle canvas fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Arc::new(Mutex::new(
        Guest::load(&CatalogEntry {
            id: "canvas-fixture".into(),
            name: "Canvas fixture".into(),
            description: String::new(),
            capabilities: vec![],
            path: path.to_string_lossy().into_owned(),
            mark: "W".into(),
            hash,
        })
        .unwrap(),
    ))
}
type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;
fn build(
    guest: &Arc<Mutex<Guest>>,
    cache: user_interface::Cache,
    renderer: &mut iced::Renderer,
) -> Ui {
    UserInterface::build(
        wasm_view(Surface(guest.clone()), false),
        Size::new(240.0, 240.0),
        cache,
        renderer,
    )
}
fn redraw(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut std::time::Instant,
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
    if messages.iter().any(|message| message == "wake") {
        build(guest, ui.into_cache(), renderer)
    } else {
        ui
    }
}
fn value(guest: &Arc<Mutex<Guest>>, key: &str) -> String {
    fn find(node: &wire::Node, key: &str) -> Option<String> {
        if let wire::Node::Text {
            key: found,
            content,
            ..
        } = node
            && found == key
        {
            return Some(content.clone());
        }
        node.children().iter().find_map(|child| find(child, key))
    }
    find(
        guest.lock().unwrap().frame.root.as_ref().unwrap(),
        &format!("CanvasFixture/{key}"),
    )
    .unwrap()
}
fn draw(ui: &mut Ui, renderer: &mut iced::Renderer) -> Vec<u8> {
    ui.draw(
        renderer,
        &iced::Theme::Dark,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::WHITE,
        },
        mouse::Cursor::Unavailable,
    );
    renderer.screenshot(Size::new(240, 240), 1.0, iced::Color::BLACK)
}
fn pixel(pixels: &[u8], x: usize, y: usize) -> [u8; 3] {
    pixels[(y * 240 + x) * 4..(y * 240 + x) * 4 + 3]
        .try_into()
        .unwrap()
}
#[test]
#[ignore = "requires bundled canvas-fixture wasm"]
fn bundled_canvas_geometry_pixels_and_native_pointer_route() {
    let guest = guest();
    let mut renderer = renderer();
    let mut now = Instant::now();
    let mut ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    let pixels = draw(&mut ui, &mut renderer);
    assert_eq!(
        pixel(&pixels, 20, 20),
        [255, 0, 0],
        "guest rectangle must be painted"
    );
    assert_eq!(
        pixel(&pixels, 80, 25),
        [0, 255, 0],
        "guest circle must be painted"
    );
    assert_eq!(
        pixel(&pixels, 135, 20),
        [0, 0, 255],
        "closed guest path must be filled"
    );
    assert_eq!(
        pixel(&pixels, 25, 85),
        [255, 255, 255],
        "group scale and translation must apply"
    );
    assert_eq!(
        pixel(&pixels, 165, 75),
        [255, 255, 255],
        "clipped group must paint its interior"
    );
    assert_eq!(
        pixel(&pixels, 178, 88),
        [0, 0, 0],
        "clipped group must not paint outside its bounds"
    );
    assert_eq!(
        pixel(&pixels, 10, 130),
        [255, 0, 0],
        "guest loop must emit first shape"
    );
    assert_eq!(
        pixel(&pixels, 40, 130),
        [255, 0, 0],
        "guest loop must emit second shape"
    );
    assert_eq!(
        pixel(&pixels, 200, 20),
        [0, 255, 0],
        "initial guest conditional must be visible"
    );
    if let Ok(path) = std::env::var("ICE_CANVAS_CAPTURE") {
        std::fs::write(path, &pixels).unwrap();
    }
    let position = iced::Point::new(15.0, 15.0);
    ui.update(
        &[
            Event::Mouse(mouse::Event::CursorMoved { position }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
        mouse::Cursor::Available(position),
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    assert_eq!(
        value(&guest, "hits"),
        "1",
        "native mouse wrapper must route once into wasm"
    );
    assert_eq!(value(&guest, "last_x"), "15");
    assert_eq!(value(&guest, "last_y"), "15");
    let pixels = draw(&mut ui, &mut renderer);
    assert_eq!(
        pixel(&pixels, 200, 20),
        [0, 0, 0],
        "guest state must update the native geometry"
    );
    assert_eq!(
        pixel(&pixels, 20, 20),
        [255, 0, 0],
        "unchanged geometry must survive the guest update"
    );
}

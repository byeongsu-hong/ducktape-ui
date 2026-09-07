//! Actual wasm text layout and button interaction.
use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Operation;
use iced::{Event, Font, Pixels, Rectangle, Size, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};

type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;
fn renderer() -> iced::Renderer {
    for (name, bytes) in [
        (
            "Geist",
            include_bytes!("../../../../assets/fonts/Geist-Regular.ttf").as_slice(),
        ),
        (
            "Geist Mono",
            include_bytes!("../../../../assets/fonts/GeistMono-Regular.ttf").as_slice(),
        ),
    ] {
        ui_lang_runtime::view_tree::register_font_family(name);
        iced_test::renderer::graphics::text::font_system()
            .write()
            .unwrap()
            .load_font(bytes.into());
    }

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
        .join("../target/text-fixture/app_store_text_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle layers fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let guest = Guest::load(&CatalogEntry {
        id: "text-fixture".into(),
        name: "Text fixture".into(),
        description: String::new(),
        capabilities: vec![],
        path: path.to_string_lossy().into_owned(),
        mark: "L".into(),
        hash,
    })
    .unwrap();
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

#[test]
#[ignore = "requires bundled text-fixture wasm"]
fn text_wasm_preserves_layout_and_padding_routes() {
    let guest = guest();
    let mut renderer = renderer();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        900.0,
    );
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 900.0);
    }
    let key = guest
        .lock()
        .unwrap()
        .frame
        .root
        .as_ref()
        .unwrap()
        .key()
        .unwrap()
        .to_owned();
    assert_eq!(
        container_bounds(&mut ui, &renderer, &key).width,
        620.0,
        "box maximum width bounds a wide host"
    );
    let heading = bounds(&mut ui, &mut renderer, "Node overview").unwrap();
    assert_eq!(heading.height, 44.0, "component text keeps explicit height");
    assert_eq!(heading.x, 16.0, "box px utility supplies native padding");
    let paragraph = bounds(
        &mut ui,
        &mut renderer,
        "A paragraph that wraps between words and also breaks long_unbroken_identifiers.",
    )
    .unwrap();
    assert_eq!(paragraph.width, 180.0);
    assert!(paragraph.height > 48.0, "paragraph wraps to multiple lines");
    fn clipped_key(node: &wire::Node) -> Option<String> {
        if let wire::Node::Container {
            clip: true,
            width: Some(wire::Length::Fixed(80.0)),
            key,
            ..
        } = node
        {
            return Some(key.clone());
        }
        node.children().iter().find_map(clipped_key)
    }
    let clipped_key = clipped_key(guest.lock().unwrap().frame.root.as_ref().unwrap()).unwrap();
    let clipped = container_bounds(&mut ui, &renderer, &clipped_key);
    ui.draw(
        &mut renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::BLACK,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(900, 600), 1.0, iced::Color::WHITE);
    for y in clipped.y.ceil() as usize..(clipped.y + clipped.height).floor() as usize {
        for x in (clipped.x + clipped.width).ceil() as usize..500 {
            let pixel = &pixels[(y * 900 + x) * 4..][..3];
            assert_eq!(
                pixel,
                &[255, 255, 255],
                "clipped text cannot paint outside its box at {x},{y}"
            );
        }
    }
    assert!(bounds(&mut ui, &mut renderer, "Applied").is_none());
    click(&mut ui, &mut renderer, "Apply");
    ui = redraw(
        ui,
        &guest,
        &mut renderer,
        &mut std::time::Instant::now(),
        900.0,
    );
    assert!(
        bounds(&mut ui, &mut renderer, "Applied").is_some(),
        "padded button routes a native click through wasm"
    );
}

fn container_bounds(ui: &mut Ui, renderer: &iced::Renderer, key: &str) -> Rectangle {
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

#[test]
#[ignore = "requires bundled text-fixture wasm"]
fn text_wasm_wrapping_reflows_and_routes_after_resize() {
    let guest = guest();
    let mut renderer = renderer();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        900.0,
    );
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 900.0);
    }
    let first = bounds(&mut ui, &mut renderer, "First").unwrap();
    let second = bounds(&mut ui, &mut renderer, "Second").unwrap();
    assert_eq!(first.y, second.y, "wide host keeps actions on one row");
    ui = build(&guest, ui.into_cache(), &mut renderer, 180.0);
    let first = bounds(&mut ui, &mut renderer, "First").unwrap();
    let second = bounds(&mut ui, &mut renderer, "Second").unwrap();
    let third = bounds(&mut ui, &mut renderer, "Third").unwrap();
    assert_eq!(
        second.y - first.y,
        36.0,
        "narrow host wraps with 6px inter-line gap"
    );
    assert_eq!(third.y - second.y, 36.0);
    click(&mut ui, &mut renderer, "Third");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 180.0);
    assert!(
        bounds(&mut ui, &mut renderer, "Applied").is_some(),
        "wrapped button routes through wasm"
    );
}

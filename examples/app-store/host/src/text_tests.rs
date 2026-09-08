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
        preferred_size: None,
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
    assert_eq!(
        bounds(&mut ui, &mut renderer, "Settings content")
            .unwrap()
            .width,
        560.0,
        "column maximum width bounds the settings content inside the box"
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
    fn clipped_linear_keys(node: &wire::Node, keys: &mut Vec<String>) {
        if let wire::Node::Linear {
            clip: true,
            width: Some(wire::Length::Fixed(80.0)),
            key,
            ..
        } = node
        {
            keys.push(key.clone());
        }
        for child in node.children() {
            clipped_linear_keys(child, keys);
        }
    }
    let mut keys = vec![];
    clipped_linear_keys(
        guest.lock().unwrap().frame.root.as_ref().unwrap(),
        &mut keys,
    );
    assert_eq!(
        keys.len(),
        2,
        "bundled row and column clipping options survive"
    );
    let mut clipped_bounds = vec![clipped];
    for label in [
        "Column content extending past its parent",
        "Row content extending past its parent",
    ] {
        let text = bounds(&mut ui, &mut renderer, label).expect("clipped layout text");
        assert_eq!(text.width, 80.0);
        clipped_bounds.push(Rectangle {
            height: 20.0,
            ..text
        });
    }

    ui.draw(
        &mut renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::BLACK,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(900, 600), 1.0, iced::Color::WHITE);
    for clipped in clipped_bounds {
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

#[test]
#[ignore = "requires bundled text-fixture wasm"]
fn text_wasm_tooltip_delays_hides_and_describes_its_button() {
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
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    let mut stream = task::into_stream(ui_lang_runtime::snapshot::<String>("Tooltip")).unwrap();
    let Some(Action::Widget(mut operation)) = iced::futures::executor::block_on(stream.next())
    else {
        panic!("snapshot operation")
    };
    ui.operate(&renderer, operation.as_mut());
    let _ = operation.finish();
    let Some(Action::Output(snapshot)) = iced::futures::executor::block_on(stream.next()) else {
        panic!("snapshot output")
    };
    assert!(
        snapshot
            .update
            .nodes
            .iter()
            .any(|(_, node)| node.description() == Some("Apply changes")),
        "the tooltip supplies the accessible button description before hover"
    );
    fn red_pixels(ui: &mut Ui, renderer: &mut iced::Renderer, cursor: mouse::Cursor) -> usize {
        ui.draw(
            renderer,
            &iced::Theme::Light,
            &iced::advanced::renderer::Style {
                text_color: iced::Color::BLACK,
            },
            cursor,
        );
        renderer
            .screenshot(Size::new(900, 600), 1.0, iced::Color::WHITE)
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel[0] > 240 && pixel[1] < 20 && pixel[2] < 20)
            .count()
    }
    let point = bounds(&mut ui, &mut renderer, "Apply").unwrap().center();
    let cursor = mouse::Cursor::Available(point);
    ui.update(
        &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
        cursor,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    assert_eq!(
        red_pixels(&mut ui, &mut renderer, cursor),
        0,
        "tip stays hidden before its delay"
    );
    std::thread::sleep(std::time::Duration::from_millis(110));
    ui.update(
        &[Event::Window(window::Event::RedrawRequested(
            std::time::Instant::now(),
        ))],
        cursor,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    assert!(
        red_pixels(&mut ui, &mut renderer, cursor) > 1500,
        "native delayed tooltip crosses the guest overlay boundary"
    );
    ui.update(
        &[Event::Mouse(mouse::Event::CursorMoved {
            position: iced::Point::new(850.0, 550.0),
        })],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    assert_eq!(
        red_pixels(&mut ui, &mut renderer, mouse::Cursor::Unavailable),
        0,
        "tip hides after leaving its content"
    );
}

#[test]
#[ignore = "requires bundled text-fixture wasm"]
fn text_wasm_rich_spans_wrap_and_links_survive_lazy_cache_hits() {
    fn paragraph_key(node: &wire::Node) -> Option<String> {
        if let wire::Node::Container { key, .. } = node
            && key.ends_with("/paragraph")
        {
            return Some(key.clone());
        }
        node.children().iter().find_map(paragraph_key)
    }
    fn value(node: &wire::Node, suffix: &str) -> Option<String> {
        if let wire::Node::Text { key, content, .. } = node
            && key.ends_with(suffix)
        {
            return Some(content.clone());
        }
        node.children().iter().find_map(|node| value(node, suffix))
    }
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
    let key = paragraph_key(guest.lock().unwrap().frame.root.as_ref().unwrap()).unwrap();
    let wide = container_bounds(&mut ui, &renderer, &key);
    assert!(
        wide.width > 500.0 && wide.height >= 48.0,
        "styled spans form a wrapping paragraph: {wide:?}"
    );
    ui.draw(
        &mut renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::BLACK,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(900, 600), 1.0, iced::Color::WHITE);
    let mut green = 0;
    for y in wide.y.ceil() as usize..(wide.y + wide.height).floor().min(600.0) as usize {
        for x in wide.x.ceil() as usize..(wide.x + wide.width).floor().min(900.0) as usize {
            if pixels[(y * 900 + x) * 4..][..3] == [0, 170, 68] {
                green += 1;
            }
        }
    }
    assert!(
        green > 100,
        "mention span backgrounds must paint inside the paragraph: {green}"
    );
    if let Ok(path) = std::env::var("ICE_RICH_CAPTURE") {
        std::fs::write(path, &pixels).unwrap();
    }
    assert_eq!(
        value(
            guest.lock().unwrap().frame.root.as_ref().unwrap(),
            "/link-status"
        )
        .as_deref(),
        Some("none")
    );
    for (width, expected_count) in [(900.0, "1"), (360.0, "2")] {
        ui = build(&guest, ui.into_cache(), &mut renderer, width);
        let paragraph = container_bounds(&mut ui, &renderer, &key);
        if width < 900.0 {
            assert!(
                paragraph.height > wide.height,
                "one paragraph reflows at host limits"
            );
        }
        let point = iced::Point::new(paragraph.x + 12.0, paragraph.y + 12.0);
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
            ui = redraw(ui, &guest, &mut renderer, &mut now, width);
        }
        let guard = guest.lock().unwrap();
        let root = guard.frame.root.as_ref().unwrap();
        assert_eq!(
            value(root, "/link-status").as_deref(),
            Some("Open"),
            "native span hit testing forwards the link String through its component"
        );
        assert_eq!(
            value(root, "/link-count").as_deref(),
            Some(expected_count),
            "cached link route fires again after unrelated guest state changes"
        );
    }
}

#[test]
#[ignore = "requires bundled text-fixture wasm"]
fn text_wasm_box_shadow_paints_outside_its_bounds() {
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
    fn shadow_key(node: &wire::Node) -> Option<String> {
        if let wire::Node::Container { shadow, key, .. } = node
            && shadow.color.is_some()
        {
            return Some(key.clone());
        }
        node.children().iter().find_map(shadow_key)
    }
    let key =
        shadow_key(guest.lock().unwrap().frame.root.as_ref().unwrap()).expect("copied box shadow");
    let bounds = container_bounds(&mut ui, &renderer, &key);
    assert_eq!(bounds.size(), Size::new(20.0, 20.0));
    ui.draw(
        &mut renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::BLACK,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(900, 600), 1.0, iced::Color::WHITE);
    let x = (bounds.x + 34.0) as usize;
    let y = (bounds.y + 10.0) as usize;
    let pixel = &pixels[(y * 900 + x) * 4..][..3];
    assert!(
        pixel[0] > pixel[1] && pixel[1] > 0 && pixel[1] < 255,
        "blurred translucent red shadow outside the box, got {pixel:?}"
    );
}

//! Actual bundled native pick options, menus and routes.
use super::layers_tests::{Ui, build, container_bounds, redraw, renderer};
use super::*;
use iced::advanced::renderer::Headless;
use iced::{Color, Size, mouse};
use iced_test::runtime::{UserInterface, user_interface};

fn guest() -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/pick-fixture/app_store_pick_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle Pick fixture first");
    let guest = Guest::load(&CatalogEntry {
        preferred_size: None,
        id: "pick-fixture".into(),
        name: "Pick fixture".into(),
        description: String::new(),
        capabilities: vec![],
        path: path.to_string_lossy().into_owned(),
        mark: "P".into(),
        hash: Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    })
    .unwrap();
    Arc::new(Mutex::new(guest))
}
fn send(ui: &mut Ui, renderer: &mut iced::Renderer, point: iced::Point, click: bool) {
    let mut events = vec![iced::Event::Mouse(mouse::Event::CursorMoved {
        position: point,
    })];
    if click {
        events.extend([
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]);
    }
    ui.update(
        &events,
        mouse::Cursor::Available(point),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn draw(ui: &mut Ui, renderer: &mut iced::Renderer, point: iced::Point) -> Vec<u8> {
    // Native pick caches its paint status on RedrawRequested, not cursor motion.
    ui.update(
        &[iced::Event::Window(iced::window::Event::RedrawRequested(
            Instant::now(),
        ))],
        mouse::Cursor::Available(point),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui.draw(
        renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: Color::BLACK,
        },
        mouse::Cursor::Available(point),
    );
    renderer.screenshot(Size::new(600, 600), 1.0, Color::WHITE)
}
fn value(guest: &Arc<Mutex<Guest>>, suffix: &str) -> String {
    fn read(node: &wire::Node, suffix: &str) -> Option<String> {
        if let wire::Node::Text { key, content, .. } = node
            && key.ends_with(suffix)
        {
            return Some(content.clone());
        }
        node.children().iter().find_map(|node| read(node, suffix))
    }
    read(guest.lock().unwrap().frame.root.as_ref().unwrap(), suffix).unwrap()
}
fn reference(renderer: &mut iced::Renderer) -> Ui {
    use iced::{font::Family, widget::pick_list};
    let font = iced::Font {
        family: Family::Name("Geist"),
        ..iced::Font::DEFAULT
    };
    let pick = pick_list(vec!["First", "Second"], None::<&str>, |value| {
        value.to_owned()
    })
    .placeholder("Choose")
    .width(180.0)
    .menu_height(96.0)
    .padding(9.0)
    .text_size(20.0)
    .text_line_height(iced::widget::text::LineHeight::Relative(1.5))
    .text_shaping(iced::widget::text::Shaping::Advanced)
    .font(font)
    .handle(pick_list::Handle::Dynamic {
        closed: pick_list::Icon {
            font,
            code_point: '▼',
            size: Some(11.0.into()),
            line_height: iced::widget::text::LineHeight::Relative(1.0),
            shaping: iced::widget::text::Shaping::Basic,
        },
        open: pick_list::Icon {
            font,
            code_point: '▲',
            size: Some(13.0.into()),
            line_height: iced::widget::text::LineHeight::Relative(1.1),
            shaping: iced::widget::text::Shaping::Advanced,
        },
    })
    .style(|theme, status| {
        let mut style = pick_list::default(theme, status);
        style.text_color = Color::BLACK;
        style.placeholder_color = Color::BLACK;
        style.handle_color = Color::BLACK;
        style.background = Color::WHITE.into();
        style.border = iced::Border {
            color: Color::BLACK,
            width: 1.0,
            radius: 8.0.into(),
        };
        match status {
            pick_list::Status::Hovered => style.background = Color::from_rgb8(0, 170, 68).into(),
            pick_list::Status::Opened { is_hovered } => {
                style.background = Color::from_rgb8(255, 0, 0).into();
                if is_hovered {
                    style.border.color = Color::from_rgb8(0, 170, 68);
                }
            }
            _ => {}
        }
        style
    })
    .menu_style(|theme| {
        let mut style = iced::overlay::menu::default(theme);
        style.text_color = Color::BLACK;
        style.selected_text_color = Color::BLACK;
        style.selected_background = Color::from_rgb8(0, 170, 68).into();
        style.background = Color::WHITE.into();
        style.border = iced::Border {
            color: Color::BLACK,
            width: 1.0,
            radius: 10.0.into(),
        };
        style.shadow.color = Color::BLACK;
        style.shadow.offset.y = 6.0;
        style.shadow.blur_radius = 18.0;
        style
    });
    UserInterface::build(
        iced::widget::column![iced::widget::container(pick)],
        Size::new(600.0, 600.0),
        user_interface::Cache::default(),
        renderer,
    )
}
#[test]
#[ignore = "requires bundled pick-fixture wasm"]
fn bundled_pick_matches_native_faces_menu_and_routes() {
    ui_lang_runtime::view_tree::register_font_family("Geist");
    iced_test::renderer::graphics::text::font_system()
        .write()
        .unwrap()
        .load_font(
            include_bytes!("../../../../assets/fonts/Geist-Regular.ttf")
                .as_slice()
                .into(),
        );
    let guest = guest();
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
    let mut native = reference(&mut renderer);
    fn pick_key(node: &wire::Node) -> Option<String> {
        if let wire::Node::Container { key, .. } = node
            && key.ends_with("/frame")
        {
            return Some(key.clone());
        }
        node.children().iter().find_map(pick_key)
    }
    let key = pick_key(guest.lock().unwrap().frame.root.as_ref().unwrap()).unwrap();
    let bounds = container_bounds(&mut ui, &renderer, &key);
    assert_eq!(
        bounds.size(),
        Size::new(180.0, 48.0),
        "copied padding and text layout"
    );
    for (point, click, height, face) in [
        (iced::Point::new(250.0, 250.0), false, 48, [255, 255, 255]),
        (iced::Point::new(10.0, 10.0), false, 48, [0, 170, 68]),
        (iced::Point::new(10.0, 10.0), true, 48, [255, 0, 0]),
        (iced::Point::new(15.0, 70.0), false, 168, [255, 0, 0]),
    ] {
        send(&mut ui, &mut renderer, point, click);
        send(&mut native, &mut renderer, point, click);
        let actual = draw(&mut ui, &mut renderer, point);
        let expected = draw(&mut native, &mut renderer, point);
        let interior = (20 * 600 + 100) * 4;
        assert_eq!(
            &actual[interior..interior + 3],
            &face,
            "native paint status must visibly change"
        );
        if height == 168
            && let Ok(path) = std::env::var("ICE_PICK_CAPTURE")
        {
            std::fs::write(&path, &actual).unwrap();
            std::fs::write(format!("{path}.native"), &expected).unwrap();
        }
        for y in 0..height {
            for x in 0..if height == 168 { 220 } else { 180 } {
                let i = (y * 600 + x) * 4;
                assert_eq!(
                    &actual[i..i + 4],
                    &expected[i..i + 4],
                    "native pick parity at {x},{y}; click={click}, cursor={point:?}"
                );
            }
        }
    }
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(value(&guest, "/opened"), "1");
    send(&mut ui, &mut renderer, iced::Point::new(20.0, 70.0), true);
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(
        value(&guest, "/selected"),
        "First",
        "menu selection returns typed guest value"
    );
    send(&mut ui, &mut renderer, iced::Point::new(10.0, 10.0), true);
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    send(&mut ui, &mut renderer, iced::Point::new(250.0, 250.0), true);
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(value(&guest, "/opened"), "2");
    assert_eq!(
        value(&guest, "/closed"),
        "1",
        "outside dismissal returns close route"
    );
}

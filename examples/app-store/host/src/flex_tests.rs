//! Actual wasm flex wrapping and native pointer/input routes.
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
        .join("../target/flex-fixture/app_store_flex_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle flex fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let guest = Guest::load(&CatalogEntry {
        preferred_size: None,
        id: "flex-fixture".into(),
        name: "Flex fixture".into(),
        description: String::new(),
        capabilities: vec![],
        path: path.to_string_lossy().into_owned(),
        mark: "F".into(),
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

fn text_value(guest: &Arc<Mutex<Guest>>, name: &str) -> String {
    fn find(node: &wire::Node, name: &str) -> Option<String> {
        if let wire::Node::Text { key, content, .. } = node
            && key.contains(name)
        {
            return Some(content.clone());
        }
        node.children().iter().find_map(|node| find(node, name))
    }
    find(guest.lock().unwrap().frame.root.as_ref().unwrap(), name).expect("guest text node")
}

#[test]
#[ignore = "requires bundled flex-fixture wasm"]
fn bundled_flex_reflows_reaction_routes_and_preserves_composer_input() {
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
    let first = bounds(&mut ui, &mut renderer, "1").unwrap();
    let fourth = bounds(&mut ui, &mut renderer, "4").unwrap();
    assert_eq!(first.y, fourth.y, "wide reaction row must fit on one line");
    click(&mut ui, &mut renderer, "4");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 400.0);
    assert_eq!(text_value(&guest, "chosen"), "4");

    ui = build(&guest, ui.into_cache(), &mut renderer, 150.0);
    let first = bounds(&mut ui, &mut renderer, "1").unwrap();
    let third = bounds(&mut ui, &mut renderer, "3").unwrap();
    assert!(third.y > first.y + 25.0, "narrow reaction row must wrap");
    click(&mut ui, &mut renderer, "3");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 150.0);
    assert_eq!(
        text_value(&guest, "chosen"),
        "3",
        "wrapped button must retain its loop route"
    );

    // Find the native input itself; its placeholder is not a text operation.
    struct InputBounds(Option<Rectangle>);
    impl Operation for InputBounds {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn text_input(
            &mut self,
            _id: Option<&iced::widget::Id>,
            bounds: Rectangle,
            _state: &mut dyn iced::advanced::widget::operation::TextInput,
        ) {
            self.0 = Some(bounds);
        }
    }
    let mut op = InputBounds(None);
    ui.operate(&renderer, &mut op);
    let input = op.0.expect("native composer input");
    let send = bounds(&mut ui, &mut renderer, "Send").unwrap();
    assert!(
        input.width > 0.0 && input.x + input.width <= send.x,
        "fill input must leave Send visible"
    );
    let point = input.center();
    ui.update(
        &[
            Event::Mouse(mouse::Event::CursorMoved { position: point }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Character("x".into()),
                modified_key: iced::keyboard::Key::Character("x".into()),
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::empty(),
                text: Some("x".into()),
                repeat: false,
            }),
        ],
        mouse::Cursor::Available(point),
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = redraw(ui, &guest, &mut renderer, &mut now, 150.0);
    click(&mut ui, &mut renderer, "Send");
    let _ui = redraw(ui, &guest, &mut renderer, &mut now, 150.0);
    assert_eq!(
        text_value(&guest, "sent"),
        "x",
        "native input and Send must update guest state"
    );
}

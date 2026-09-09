//! Real window events through both the native child and bundled Wasm guest.
use super::layers_tests::{Ui, click, renderer};
use super::*;
use iced::{Event, Point, Size, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};

fn entry(native: bool) -> CatalogEntry {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/resize-native"
    } else {
        "../target/resize-wasm"
    });
    crate::catalog::scan_dir(&directory)
        .into_iter()
        .find(|entry| entry.name == "Resize fixture")
        .expect("build both current resize fixtures first")
}
fn app(native: bool) -> Arc<Mutex<Guest>> {
    Arc::new(Mutex::new(Guest::load(&entry(native)).unwrap()))
}
fn build(
    guest: &Arc<Mutex<Guest>>,
    cache: user_interface::Cache,
    renderer: &mut iced::Renderer,
) -> Ui {
    UserInterface::build(
        iced::widget::container(wasm_view(Surface(guest.clone()), false)).padding(20),
        Size::new(600.0, 600.0),
        cache,
        renderer,
    )
}
fn frame(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut Instant,
) -> Ui {
    *now += Duration::from_secs(1);
    ui.update(
        &[Event::Window(window::Event::RedrawRequested(*now))],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    assert!(guest.lock().unwrap().fault.is_none());
    build(guest, ui.into_cache(), renderer)
}
fn text(guest: &Arc<Mutex<Guest>>, suffix: &str) -> String {
    fn find(node: &wire::Node, suffix: &str) -> Option<String> {
        if let wire::Node::Text { key, content, .. } = node
            && key.ends_with(suffix)
        {
            return Some(content.clone());
        }
        node.children().iter().find_map(|node| find(node, suffix))
    }
    find(
        guest.lock().unwrap().frame.root.as_ref().unwrap(),
        &format!("/{suffix}"),
    )
    .unwrap()
}

fn send(
    ui: &mut Ui,
    renderer: &mut iced::Renderer,
    point: Point,
    event: mouse::Event,
) -> user_interface::State {
    ui.update(
        &[Event::Mouse(event)],
        mouse::Cursor::Available(point),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    )
    .0
}
fn divider(ui: &mut Ui, renderer: &iced::Renderer) -> Point {
    use iced::advanced::widget::Operation;
    struct Find(Option<Point>);
    impl Operation for Find {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn text(&mut self, _: Option<&iced::widget::Id>, bounds: iced::Rectangle, value: &str) {
            if value == "|" {
                self.0 = Some(bounds.center());
            }
        }
    }
    let mut find = Find(None);
    ui.operate(renderer, &mut find);
    find.0.expect("mounted divider")
}
#[test]
#[ignore = "requires current native and Wasm resize fixtures"]
fn bundled_resize_native_and_wasm_grab_outside_release_and_retire() {
    for native in [false, true] {
        let guest = app(native);
        let mut renderer = renderer();
        let mut ui = build(&guest, user_interface::Cache::default(), &mut renderer);
        let mut now = Instant::now();
        ui = frame(ui, &guest, &mut renderer, &mut now);
        let origin = divider(&mut ui, &renderer);
        let hovered = send(
            &mut ui,
            &mut renderer,
            origin,
            mouse::Event::CursorMoved { position: origin },
        );
        assert!(
            matches!(
                hovered,
                user_interface::State::Updated {
                    mouse_interaction: mouse::Interaction::ResizingHorizontally,
                    ..
                }
            ),
            "native resize cursor: {hovered:?}"
        );
        send(
            &mut ui,
            &mut renderer,
            origin,
            mouse::Event::ButtonPressed(mouse::Button::Left),
        );
        for dx in [25.0, 70.0] {
            let position = origin + iced::Vector::new(dx, 15.0);
            send(
                &mut ui,
                &mut renderer,
                position,
                mouse::Event::CursorMoved { position },
            );
        }
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            text(&guest, "width"),
            "230",
            "all pre-frame movement survives outside divider"
        );
        assert_eq!(text(&guest, "presses"), "1");
        let right = origin + iced::Vector::new(600.0, 0.0);
        send(
            &mut ui,
            &mut renderer,
            right,
            mouse::Event::CursorMoved { position: right },
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "width"), "300");
        let left = Point::new(-100.0, origin.y);
        send(
            &mut ui,
            &mut renderer,
            left,
            mouse::Event::CursorMoved { position: left },
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "width"), "80");
        send(
            &mut ui,
            &mut renderer,
            left,
            mouse::Event::ButtonReleased(mouse::Button::Left),
        );
        send(
            &mut ui,
            &mut renderer,
            right,
            mouse::Event::CursorMoved { position: right },
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "width"), "80", "outside release ends drag");
        assert_eq!(text(&guest, "releases"), "1");
        let origin = divider(&mut ui, &renderer);
        send(
            &mut ui,
            &mut renderer,
            origin,
            mouse::Event::ButtonPressed(mouse::Button::Left),
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        click(&mut ui, &mut renderer, "Toggle divider");
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            text(&guest, "releases"),
            "1",
            "hide must retire an unreleased grab"
        );
        click(&mut ui, &mut renderer, "Toggle divider");
        ui = frame(ui, &guest, &mut renderer, &mut now);
        send(
            &mut ui,
            &mut renderer,
            right,
            mouse::Event::CursorMoved { position: right },
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            text(&guest, "width"),
            "80",
            "readded divider has no old grab"
        );
        let origin = divider(&mut ui, &renderer);
        send(
            &mut ui,
            &mut renderer,
            origin,
            mouse::Event::ButtonPressed(mouse::Button::Left),
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        let metadata = entry(native);
        let running = Running {
            id: metadata.id.clone(),
            name: metadata.name.clone(),
            surface: Surface(guest.clone()),
            window: window::Id::unique(),
        };
        let prepared =
            iced::futures::executor::block_on(prepare_reload(metadata, vec![running.clone()], 1));
        super::reload::finish_reload(std::slice::from_ref(&running), 1, prepared).unwrap();
        send(
            &mut ui,
            &mut renderer,
            right,
            mouse::Event::CursorMoved { position: right },
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        send(
            &mut ui,
            &mut renderer,
            left,
            mouse::Event::CursorMoved { position: left },
        );
        let mut ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            text(&guest, "width"),
            "80",
            "replacement drops old grab and stale events"
        );
        use iced::advanced::renderer::Headless as _;
        ui.draw(
            &mut renderer,
            &iced::Theme::Light,
            &iced::advanced::renderer::Style {
                text_color: iced::Color::BLACK,
            },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(600, 600), 1.0, iced::Color::WHITE);
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/resize-evidence");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(if native { "native.rgba" } else { "wasm.rgba" }),
            pixels,
        )
        .unwrap();
    }
}

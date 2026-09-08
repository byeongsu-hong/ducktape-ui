//! Real window events through both the native child and bundled Wasm guest.
use super::layers_tests::{Ui, click, renderer};
use super::*;
use iced::{Event, Point, Size, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};

fn app(native: bool) -> Arc<Mutex<Guest>> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/mouse-native"
    } else {
        "../target/mouse-wasm"
    });
    let entry = crate::catalog::scan_dir(&directory)
        .into_iter()
        .find(|entry| entry.name == "Mouse fixture")
        .expect("build both current mouse fixtures first");
    Arc::new(Mutex::new(Guest::load(&entry).unwrap()))
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
fn send(ui: &mut Ui, renderer: &mut iced::Renderer, events: &[mouse::Event]) {
    ui.update(
        &events.iter().copied().map(Event::Mouse).collect::<Vec<_>>(),
        mouse::Cursor::Available(Point::new(12.0, 35.0)),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}

#[test]
#[ignore = "requires current native and Wasm mouse fixtures"]
fn bundled_mouse_native_and_wasm_deliver_local_coalesced_events_and_unsubscribe() {
    for native in [false, true] {
        let guest = app(native);
        let sibling = app(native);
        let mut renderer = renderer();
        let mut ui = build(&guest, user_interface::Cache::default(), &mut renderer);
        let mut now = Instant::now();
        for _ in 0..3 {
            ui = frame(ui, &guest, &mut renderer, &mut now);
        }
        assert!(guest.lock().unwrap().frame.mouse_interest);
        send(
            &mut ui,
            &mut renderer,
            &[
                mouse::Event::CursorEntered,
                mouse::Event::CursorMoved {
                    position: Point::new(21.0, 22.0),
                },
                mouse::Event::ButtonPressed(mouse::Button::Other(65535)),
                mouse::Event::CursorMoved {
                    position: Point::new(12.0, 35.0),
                },
                mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: -2.5, y: 8.0 },
                },
                mouse::Event::ButtonReleased(mouse::Button::Other(65535)),
                mouse::Event::CursorLeft,
            ],
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "moves"), "1");
        assert_eq!(text(&guest, "position"), "-8,15", "guest origin is (20,20)");
        assert_eq!(
            text(&guest, "order"),
            "EPMWRL",
            "the latest move keeps its order among discrete events"
        );
        assert_eq!(
            text(&guest, "events"),
            "6",
            "generic event listeners receive exactly the delivered subset"
        );
        assert_eq!(text(&guest, "delta"), "-2.5,8,true");
        assert_eq!(text(&guest, "captured"), "0");
        assert!(
            sibling.lock().unwrap().pending.is_empty(),
            "another instance receives nothing"
        );
        send(
            &mut ui,
            &mut renderer,
            &[mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 1.5, y: -4.0 },
            }],
        );
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "delta"), "1.5,-4,false");
        click(&mut ui, &mut renderer, "Open overlay");
        ui = frame(ui, &guest, &mut renderer, &mut now);
        let before = text(&guest, "captured").parse::<usize>().unwrap();
        click(&mut ui, &mut renderer, "Capture overlay");
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "clicked"), "1", "real overlay route fires");
        assert_eq!(
            text(&guest, "captured").parse::<usize>().unwrap(),
            before + 2,
            "captured overlay press and release each arrive exactly once"
        );
        click(&mut ui, &mut renderer, "Close overlay");
        ui = frame(ui, &guest, &mut renderer, &mut now);
        click(&mut ui, &mut renderer, "Disable mouse");
        ui = frame(ui, &guest, &mut renderer, &mut now);
        assert!(
            !guest.lock().unwrap().frame.mouse_interest,
            "when branch removes host interest"
        );
        let previous = text(&guest, "events");
        send(
            &mut ui,
            &mut renderer,
            &[mouse::Event::CursorMoved {
                position: Point::new(30.0, 40.0),
            }],
        );
        assert!(
            !guest
                .lock()
                .unwrap()
                .pending
                .iter()
                .any(|event| matches!(event, wire::Event::Mouse { .. })),
            "disabled guest receives no mouse wire event"
        );
        let _ui = frame(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "events"), previous);
    }
}

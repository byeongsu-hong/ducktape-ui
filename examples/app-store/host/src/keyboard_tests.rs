//! Keyboard subscriptions exercised through an actual bundled wasm view.
use super::layers_tests::{Ui, build, click, container_bounds, redraw, renderer};
use super::*;
use iced::{Event, keyboard, mouse};
use iced_test::runtime::user_interface;

fn guest(macos: bool) -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/keyboard-fixture/app_store_keyboard_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle keyboard fixture first");
    let guest = Arc::new(Mutex::new(
        Guest::load(&CatalogEntry {
            preferred_size: None,
            id: "keyboard-fixture".into(),
            name: "Keyboard fixture".into(),
            description: String::new(),
            capabilities: vec![],
            path: path.to_string_lossy().into_owned(),
            mark: "K".into(),
            hash: Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        })
        .unwrap(),
    ));
    {
        let mut guest = guest.lock().unwrap();
        guest.backend.init(macos).unwrap();
    }
    guest
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
fn send(ui: &mut Ui, renderer: &mut iced::Renderer, event: keyboard::Event) {
    ui.update(
        &[Event::Keyboard(event)],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn escape() -> keyboard::Event {
    keyboard::Event::KeyPressed {
        key: keyboard::key::Named::Escape.into(),
        modified_key: keyboard::key::Named::Escape.into(),
        physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    }
}
#[test]
#[ignore = "requires bundled keyboard-fixture wasm"]
fn bundled_keyboard_reaches_the_guest_subscription() {
    let guest = guest(false);
    let mut renderer = renderer();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    let mut now = Instant::now();
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    send(&mut ui, &mut renderer, escape());
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(
        text(&guest, "presses"),
        "1",
        "native Escape reaches the guest subscription"
    );
    assert_eq!(text(&guest, "captured"), "0");
    click(&mut ui, &mut renderer, "Open modal");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(text(&guest, "open"), "true");
    fn input_box(node: &wire::Node) -> Option<String> {
        if let wire::Node::Container { key, .. } = node
            && key.ends_with("/input-box")
        {
            return Some(key.clone());
        }
        node.children().iter().find_map(input_box)
    }
    let key = input_box(guest.lock().unwrap().frame.root.as_ref().unwrap()).unwrap();
    let point = container_bounds(&mut ui, &renderer, &key).center();
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
    send(
        &mut ui,
        &mut renderer,
        keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("z".into()),
            modified_key: keyboard::Key::Character("z".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyZ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: Some("z".into()),
            repeat: false,
        },
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(
        text(&guest, "draft"),
        "z",
        "native overlay input consumes the character"
    );
    assert_eq!(
        text(&guest, "captured"),
        "1",
        "captured overlay key delivered exactly once"
    );
    assert_eq!(
        text(&guest, "presses"),
        "1",
        "captured key never reaches ignored route"
    );
    click(&mut ui, &mut renderer, "Close modal");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(text(&guest, "open"), "false");
    click(&mut ui, &mut renderer, "Disable keys");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    send(&mut ui, &mut renderer, escape());
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(
        text(&guest, "presses"),
        "1",
        "removed subscription receives no keys"
    );
}

#[test]
#[ignore = "requires bundled keyboard-fixture wasm"]
fn bundled_keyboard_preserves_metadata_and_host_platform_between_instances() {
    let other = guest(false);
    let mut renderer = renderer();
    let mut other_ui = build(
        &other,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    let mut now = Instant::now();
    for _ in 0..3 {
        other_ui = redraw(other_ui, &other, &mut renderer, &mut now, 600.0);
    }
    for macos in [false, true] {
        let guest = guest(macos);
        let mut ui = build(
            &guest,
            user_interface::Cache::default(),
            &mut renderer,
            600.0,
        );
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        assert_eq!(
            text(&guest, "boot-logo"),
            macos.to_string(),
            "host platform applies before wasm boot"
        );
        send(
            &mut ui,
            &mut renderer,
            keyboard::Event::ModifiersChanged(keyboard::Modifiers::LOGO | keyboard::Modifiers::ALT),
        );
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        for field in ["command", "jump", "mac-command"] {
            assert_eq!(
                text(&guest, field),
                macos.to_string(),
                "host platform controls {field}"
            );
        }
        send(
            &mut ui,
            &mut renderer,
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Character("å".into()),
                modified_key: keyboard::key::Named::Enter.into(),
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyQ),
                location: keyboard::Location::Right,
                modifiers: keyboard::Modifiers::ALT,
                text: Some("Å".into()),
                repeat: true,
            },
        );
        send(
            &mut ui,
            &mut renderer,
            keyboard::Event::KeyReleased {
                key: keyboard::Key::Character("å".into()),
                modified_key: keyboard::key::Named::Enter.into(),
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyQ),
                location: keyboard::Location::Right,
                modifiers: keyboard::Modifiers::ALT,
            },
        );
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        assert_eq!(
            text(&guest, "metadata"),
            "true",
            "logical/modified/physical key, location, text and repeat survive wasm"
        );
        assert_eq!(text(&guest, "releases"), "1");
        assert_eq!(text(&guest, "presses"), "1");
        other_ui = redraw(other_ui, &other, &mut renderer, &mut now, 600.0);
        assert_eq!(
            text(&other, "presses"),
            "0",
            "another instance receives none of these keys"
        );
    }
}

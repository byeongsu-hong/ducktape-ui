//! Actual native child and Wasm combo search, routes, and retained host state.
use super::layers_tests::{Ui, build, redraw, renderer};
use super::reload::finish_reload;
use super::*;
use iced::{Event, keyboard, mouse};
use iced_test::runtime::user_interface;

fn entry(native: bool) -> CatalogEntry {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/combo-native"
    } else {
        "../target/combo-wasm"
    });
    crate::catalog::scan_dir(&dir)
        .into_iter()
        .find(|entry| entry.name == "Combo fixture")
        .expect("bundle native and wasm combo fixtures first")
}
fn app(native: bool) -> Running {
    let entry = entry(native);
    Running {
        id: entry.id.clone(),
        name: entry.name.clone(),
        surface: Surface(Arc::new(Mutex::new(Guest::load(&entry).unwrap()))),
        window: iced::window::Id::unique(),
    }
}
fn read(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Text { key, content, .. } = node
        && key.ends_with(suffix)
    {
        return Some(content.clone());
    }
    node.children().iter().find_map(|node| read(node, suffix))
}
fn value(app: &Running, suffix: &str) -> String {
    read(
        app.surface.0.lock().unwrap().frame.root.as_ref().unwrap(),
        suffix,
    )
    .expect("fixture text")
}
fn send(ui: &mut Ui, renderer: &mut iced::Renderer, event: Event, point: iced::Point) {
    ui.update(
        &[event],
        mouse::Cursor::Available(point),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn click(ui: &mut Ui, renderer: &mut iced::Renderer, y: f32) {
    let point = iced::Point::new(20.0, y);
    send(
        ui,
        renderer,
        Event::Mouse(mouse::Event::CursorMoved { position: point }),
        point,
    );
    send(
        ui,
        renderer,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        point,
    );
    send(
        ui,
        renderer,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        point,
    );
}
fn key(key: keyboard::Key, text: Option<&str>) -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: text.map(Into::into),
        repeat: false,
    })
}
fn tick(mut ui: Ui, app: &Running, renderer: &mut iced::Renderer, now: &mut Instant) -> Ui {
    for _ in 0..4 {
        ui = redraw(ui, &app.surface.0, renderer, now, 480.0);
    }
    ui
}
// Controlled guest state mutation keeps focus in the combo. Clicking a second
// widget would blur the native combo before testing its set/push semantics.
fn change_options(app: &Running, label: &str) {
    fn route(node: &wire::Node, label: &str) -> Option<u32> {
        if let wire::Node::Button {
            content: wire::ButtonContent::Label(text),
            on_press,
            ..
        } = node
            && text == label
        {
            return *on_press;
        }
        node.children().iter().find_map(|node| route(node, label))
    }
    let mut guest = app.surface.0.lock().unwrap();
    let handler = route(guest.frame.root.as_ref().unwrap(), label).expect("fixture option handler");
    guest.pending.push(wire::Event::Message(handler));
}

#[test]
#[ignore = "requires actual native and wasm combo fixtures"]
fn bundled_combo_filters_and_routes_keyboard_and_pointer_selection() {
    for native in [false, true] {
        for pointer in [false, true] {
            let app = app(native);
            let mut renderer = renderer();
            let mut now = Instant::now();
            let mut ui = tick(
                build(
                    &app.surface.0,
                    user_interface::Cache::default(),
                    &mut renderer,
                    480.0,
                ),
                &app,
                &mut renderer,
                &mut now,
            );
            click(&mut ui, &mut renderer, 15.0);
            ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(
                value(&app, "/opens"),
                "0",
                "native hover publication suppresses open for this event"
            );
            assert_eq!(value(&app, "/hovered"), "Apple");
            send(
                &mut ui,
                &mut renderer,
                key(keyboard::Key::Character("b".into()), Some("b")),
                iced::Point::new(20.0, 15.0),
            );
            ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(value(&app, "/query"), "b", "native={native}");
            assert_eq!(value(&app, "/result"), "waiting");
            if pointer {
                click(&mut ui, &mut renderer, 55.0);
            } else {
                send(
                    &mut ui,
                    &mut renderer,
                    key(keyboard::Key::Named(keyboard::key::Named::Enter), None),
                    iced::Point::new(20.0, 15.0),
                );
            }
            let mut ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(
                value(&app, "/result"),
                "Berry",
                "native={native}, pointer={pointer}: filtered first row must not select Apple"
            );
            let bounds =
                super::layers_tests::container_bounds(&mut ui, &renderer, "ComboFixture/lifecycle");
            click(&mut ui, &mut renderer, bounds.y + 15.0);
            ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(
                value(&app, "/opens"),
                "1",
                "combo without hover emits native on_open"
            );
            click(&mut ui, &mut renderer, 550.0);
            let _ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(
                value(&app, "/closes"),
                "1",
                "outside click emits native on_close"
            );
        }
    }
}

#[test]
#[ignore = "requires actual native and wasm combo fixtures"]
fn bundled_combo_set_push_and_reload_preserve_the_search_contract() {
    for native in [false, true] {
        for mode in ["frame", "push", "set", "reload", "hide", "hidden reload"] {
            let app = app(native);
            let mut renderer = renderer();
            let mut now = Instant::now();
            let mut ui = tick(
                build(
                    &app.surface.0,
                    user_interface::Cache::default(),
                    &mut renderer,
                    480.0,
                ),
                &app,
                &mut renderer,
                &mut now,
            );
            click(&mut ui, &mut renderer, 15.0);
            send(
                &mut ui,
                &mut renderer,
                key(keyboard::Key::Character("b".into()), Some("b")),
                iced::Point::new(20.0, 15.0),
            );
            ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(value(&app, "/query"), "b");
            match mode {
                "push" => change_options(&app, "Append banana"),
                "set" => change_options(&app, "Reset options"),
                "hide" | "hidden reload" => {
                    change_options(&app, "Hide combos");
                    ui = tick(ui, &app, &mut renderer, &mut now);
                    if mode == "hidden reload" {
                        let prepared = iced::futures::executor::block_on(prepare_reload(
                            entry(native),
                            vec![app.clone()],
                            1,
                        ));
                        finish_reload(std::slice::from_ref(&app), 1, prepared).unwrap();
                        ui = build(&app.surface.0, ui.into_cache(), &mut renderer, 480.0);
                    }
                    change_options(&app, "Show combos");
                    ui = tick(ui, &app, &mut renderer, &mut now);
                    click(&mut ui, &mut renderer, 15.0);
                }
                "reload" => {
                    let alive = app.surface.0.lock().unwrap().alive.clone();
                    let prepared = iced::futures::executor::block_on(prepare_reload(
                        entry(native),
                        vec![app.clone()],
                        1,
                    ));
                    finish_reload(std::slice::from_ref(&app), 1, prepared).unwrap();
                    assert!(!alive.load(Ordering::Relaxed));
                    // This UI still holds the old native overlay and generation.
                    click(&mut ui, &mut renderer, 55.0);
                    assert!(
                        app.surface.0.lock().unwrap().pending.is_empty(),
                        "old overlay must not enqueue a replacement selection"
                    );
                    assert_eq!(value(&app, "/result"), "waiting");
                    ui = build(&app.surface.0, ui.into_cache(), &mut renderer, 480.0);
                }
                _ => {}
            }
            ui = tick(ui, &app, &mut renderer, &mut now);
            if mode == "push" {
                for named in [keyboard::key::Named::ArrowDown, keyboard::key::Named::Enter] {
                    send(
                        &mut ui,
                        &mut renderer,
                        key(keyboard::Key::Named(named), None),
                        iced::Point::new(20.0, 15.0),
                    );
                }
            } else {
                click(&mut ui, &mut renderer, 55.0);
            }
            let _ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(
                value(&app, "/result"),
                match mode {
                    "set" => "Apple",
                    "push" => "Banana",
                    _ => "Berry",
                },
                "native={native}, {mode}"
            );
        }
    }
}

#[test]
#[ignore = "requires actual native and wasm combo fixtures"]
fn bundled_combo_shares_owned_state_but_separates_widget_focus_and_other_state() {
    for native in [false, true] {
        for (target, expected) in [("shared", "Berry"), ("lifecycle", "Apple")] {
            let app = app(native);
            let mut renderer = renderer();
            let mut now = Instant::now();
            let mut ui = tick(
                build(
                    &app.surface.0,
                    user_interface::Cache::default(),
                    &mut renderer,
                    480.0,
                ),
                &app,
                &mut renderer,
                &mut now,
            );
            click(&mut ui, &mut renderer, 15.0);
            send(
                &mut ui,
                &mut renderer,
                key(keyboard::Key::Character("b".into()), Some("b")),
                iced::Point::new(20.0, 15.0),
            );
            ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(value(&app, "/query"), "b");
            let bounds = super::layers_tests::container_bounds(
                &mut ui,
                &renderer,
                &format!("ComboFixture/{target}"),
            );
            click(&mut ui, &mut renderer, bounds.y + 15.0);
            ui = tick(ui, &app, &mut renderer, &mut now);
            send(
                &mut ui,
                &mut renderer,
                key(keyboard::Key::Named(keyboard::key::Named::Enter), None),
                iced::Point::new(20.0, bounds.y + 15.0),
            );
            let _ui = tick(ui, &app, &mut renderer, &mut now);
            assert_eq!(
                value(&app, "/result"),
                expected,
                "native={native}, target={target}: search state belongs to the state binding, focus to its widget"
            );
        }
    }
}

//! A bundled editor driven through native pointer/key events and the guest binding.
use super::*;
use iced::advanced::{renderer::Headless, widget::Operation};
use iced::{Event, Font, Pixels, Point, Rectangle, Size, keyboard, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};
type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;

fn guest() -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/editor-fixture/app_store_editor_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle editor fixture first");
    Arc::new(Mutex::new(
        Guest::load(&CatalogEntry {
            preferred_size: None,
            id: "editor-fixture".into(),
            name: "Editor fixture".into(),
            description: String::new(),
            capabilities: vec![],
            path: path.to_string_lossy().into_owned(),
            mark: "E".into(),
            hash: Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        })
        .unwrap(),
    ))
}
fn build(
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    cache: user_interface::Cache,
) -> Ui {
    UserInterface::build(
        wasm_view(Surface(guest.clone()), false),
        Size::new(240.0, 400.0),
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
    assert!(guest.lock().unwrap().fault.is_none());
    if messages.iter().any(|m| m == "wake") {
        build(guest, renderer, ui.into_cache())
    } else {
        ui
    }
}
fn key_event(value: &str, modifiers: keyboard::Modifiers) -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Character(value.into()),
        modified_key: keyboard::Key::Character(value.into()),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        text: Some(value.into()),
        repeat: false,
    })
}
fn send(ui: &mut Ui, renderer: &mut iced::Renderer, event: Event, point: Point) {
    ui.update(
        &[event],
        mouse::Cursor::Available(point),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn click(ui: &mut Ui, renderer: &mut iced::Renderer, point: Point) {
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
fn editor(node: &wire::Node) -> Option<&wire::Node> {
    if matches!(node, wire::Node::Editor { .. }) {
        Some(node)
    } else {
        node.children().iter().find_map(editor)
    }
}
fn text(guest: &Arc<Mutex<Guest>>) -> String {
    let guest = guest.lock().unwrap();
    let wire::Node::Editor { text, .. } = editor(guest.frame.root.as_ref().unwrap()).unwrap()
    else {
        unreachable!()
    };
    text.clone()
}
struct Bounds<'a>(&'a str, Option<Rectangle>);
impl Operation for Bounds<'_> {
    fn text(&mut self, _: Option<&iced::widget::Id>, bounds: Rectangle, text: &str) {
        if text == self.0 {
            self.1 = Some(bounds);
        }
    }
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn focusable(
        &mut self,
        id: Option<&iced::widget::Id>,
        bounds: Rectangle,
        _: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        if id == Some(&iced::widget::Id::from(self.0.to_owned())) {
            self.1 = Some(bounds);
        }
    }
}

#[test]
#[ignore = "requires bundled editor fixture"]
fn editor_wasm_preserves_presentation_selection_editing_and_disabled_state() {
    let guest = guest();
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .unwrap();
    let mut ui = build(&guest, &mut renderer, user_interface::Cache::default());
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    let key = {
        let g = guest.lock().unwrap();
        editor(g.frame.root.as_ref().unwrap())
            .unwrap()
            .key()
            .unwrap()
            .to_owned()
    };
    let mut find = Bounds(&key, None);
    ui.operate(&renderer, &mut find);
    let bounds = find.1.expect("native editor bounds");
    assert_eq!(bounds.size(), Size::new(200.0, 93.2));
    let point = Point::new(bounds.x + 15.0, bounds.y + 15.0);
    click(&mut ui, &mut renderer, point);
    let modifiers = if cfg!(target_os = "macos") {
        keyboard::Modifiers::LOGO
    } else {
        keyboard::Modifiers::CTRL
    };
    send(&mut ui, &mut renderer, key_event("a", modifiers), point);
    ui = redraw(ui, &guest, &mut renderer, &mut now);
    ui.draw(
        &mut renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::BLACK,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(240, 200), 1.0, iced::Color::WHITE);
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[..3] == [0, 255, 255]),
        "native selection paints the guest's cyan color"
    );
    send(
        &mut ui,
        &mut renderer,
        key_event("Z", keyboard::Modifiers::default()),
        point,
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    assert_eq!(
        text(&guest),
        "Z",
        "native edit crosses the guest editor binding"
    );
    click(&mut ui, &mut renderer, Point::new(100.0, 15.0));
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    {
        let g = guest.lock().unwrap();
        assert!(matches!(
            editor(g.frame.root.as_ref().unwrap()),
            Some(wire::Node::Editor { on_edit: None, .. })
        ));
    }
    click(&mut ui, &mut renderer, point);
    send(
        &mut ui,
        &mut renderer,
        key_event("X", keyboard::Modifiers::default()),
        point,
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    assert_eq!(text(&guest), "Z", "disabled editor must not edit");
    ui.draw(
        &mut renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::BLACK,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(240, 200), 1.0, iced::Color::WHITE);
    let offset = (((bounds.y + 5.0) as usize) * 240 + 100) * 4;
    assert_eq!(
        &pixels[offset..][..3],
        &[128, 128, 128],
        "disabled editor paints its copied face"
    );
}

#[test]
#[ignore = "requires bundled editor fixture"]
fn editor_wasm_overlay_refreshes_layout_between_batched_native_edits() {
    let guest = guest();
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .unwrap();
    let mut ui = build(&guest, &mut renderer, user_interface::Cache::default());
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    let mut open = Bounds("Open editor overlay", None);
    ui.operate(&renderer, &mut open);
    click(
        &mut ui,
        &mut renderer,
        open.1.expect("open overlay button").center(),
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    let key = {
        let g = guest.lock().unwrap();
        let wire::Node::Overlay { children, .. } = g.frame.root.as_ref().unwrap() else {
            panic!("fixture overlay")
        };
        editor(
            children
                .get(1)
                .expect("overlay must open after native button click"),
        )
        .expect("open overlay editor")
        .key()
        .unwrap()
        .to_owned()
    };
    let mut find = Bounds(&key, None);
    ui.operate(&renderer, &mut find);
    let bounds = find.1.expect("mounted native overlay editor");
    let point = Point::new(bounds.x + 15.0, bounds.y + 10.0);
    click(&mut ui, &mut renderer, point);
    let modifiers = if cfg!(target_os = "macos") {
        keyboard::Modifiers::LOGO
    } else {
        keyboard::Modifiers::CTRL
    };
    // Iced processes queued events before the outer application handles wake
    // messages. The edited Content must have valid geometry within this batch.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ui.update(
            &[
                key_event("a", modifiers),
                key_event("Z", keyboard::Modifiers::default()),
                Event::Window(window::Event::RedrawRequested(now)),
            ],
            mouse::Cursor::Available(point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut vec![],
        );
    }));
    assert!(
        outcome.is_ok(),
        "overlay edits must refresh native caret/IME layout before the next queued event"
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    assert_eq!(
        text(&guest),
        "Z",
        "overlay selection replacement crosses the guest binding"
    );
}

#[test]
#[ignore = "requires native and Wasm editor fixtures"]
fn editor_wasm_and_native_report_caret_only_changes_and_utf8_selection() {
    for native in [false, true] {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
            "../target/editor-native"
        } else {
            "../target/editor-fixture"
        });
        let entries = crate::catalog::scan_dir(&directory);
        assert_eq!(entries.len(), 1, "build current editor fixtures first");
        let guest = Arc::new(Mutex::new(Guest::load(&entries[0]).unwrap()));
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut ui = build(&guest, &mut renderer, user_interface::Cache::default());
        let mut now = std::time::Instant::now();
        for _ in 0..4 {
            ui = redraw(ui, &guest, &mut renderer, &mut now);
        }
        let key = editor(guest.lock().unwrap().frame.root.as_ref().unwrap())
            .unwrap()
            .key()
            .unwrap()
            .to_owned();
        let mut find = Bounds(&key, None);
        ui.operate(&renderer, &mut find);
        let point = find.1.unwrap().center();
        click(&mut ui, &mut renderer, point);
        let command = if cfg!(target_os = "macos") {
            keyboard::Modifiers::LOGO
        } else {
            keyboard::Modifiers::CTRL
        };
        send(&mut ui, &mut renderer, key_event("a", command), point);
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        let state = || {
            let locked = guest.lock().unwrap();
            let wire::Node::Editor {
                text,
                cursor,
                reset,
                revision,
                ..
            } = editor(locked.frame.root.as_ref().unwrap()).unwrap()
            else {
                unreachable!()
            };
            wire::EditorState {
                text: text.clone(),
                cursor: *cursor,
                reset: *reset,
                revision: *revision,
            }
        };
        let selected = state();
        assert_eq!(selected.text, "ab\ncd");
        assert!(
            selected.cursor.selection.is_some(),
            "selection-only events reach {native}"
        );
        send(
            &mut ui,
            &mut renderer,
            key_event("한", keyboard::Modifiers::empty()),
            point,
        );
        send(
            &mut ui,
            &mut renderer,
            key_event("글", keyboard::Modifiers::empty()),
            point,
        );
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        let typed = state();
        assert_eq!(typed.text, "한글");
        assert_eq!(
            typed.cursor.position,
            wire::EditorPosition { line: 0, column: 6 }
        );
        assert_eq!(typed.cursor.selection, None);
        let named = |key, modifiers| {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key),
                modified_key: keyboard::Key::Named(key),
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers,
                text: None,
                repeat: false,
            })
        };
        send(
            &mut ui,
            &mut renderer,
            named(keyboard::key::Named::ArrowLeft, keyboard::Modifiers::SHIFT),
            point,
        );
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        let shifted = state();
        let snapshot = guest
            .lock()
            .unwrap()
            .backend
            .snapshot()
            .expect("editor snapshot");
        let mut restored = Instance::new(&entries[0]).unwrap();
        restored
            .backend
            .restore(&snapshot, cfg!(target_os = "macos"))
            .unwrap();
        let restored = Arc::new(Mutex::new(Guest::from_instance(
            &entries[0],
            restored,
            None,
            Arc::new(crate::surfaces::log::Session::default()),
        )));
        let mut restored_ui = build(&restored, &mut renderer, user_interface::Cache::default());
        for _ in 0..4 {
            restored_ui = redraw(restored_ui, &restored, &mut renderer, &mut now);
        }
        {
            let locked = restored.lock().unwrap();
            let wire::Node::Editor {
                text,
                cursor,
                revision,
                ..
            } = editor(locked.frame.root.as_ref().unwrap()).unwrap()
            else {
                unreachable!()
            };
            assert_eq!(text, &shifted.text);
            assert_eq!(
                cursor, &shifted.cursor,
                "fresh restore preserves active end and anchor"
            );
            assert_eq!(*revision, shifted.revision);
        }
        click(&mut restored_ui, &mut renderer, point);
        send(
            &mut restored_ui,
            &mut renderer,
            named(keyboard::key::Named::End, keyboard::Modifiers::empty()),
            point,
        );
        send(
            &mut restored_ui,
            &mut renderer,
            key_event("!", keyboard::Modifiers::empty()),
            point,
        );
        restored_ui = redraw(restored_ui, &restored, &mut renderer, &mut now);
        assert_eq!(
            text(&restored),
            "한글!",
            "fresh host's first input is not rejected as an old observation"
        );
        drop(restored_ui);
        drop(restored);
        assert_eq!(shifted.text, "한글");
        assert_eq!(shifted.cursor.position.column, 3);
        assert_eq!(
            shifted.cursor.selection.unwrap().column,
            6,
            "active end and anchor retain backward direction"
        );
        send(
            &mut ui,
            &mut renderer,
            named(
                keyboard::key::Named::ArrowRight,
                keyboard::Modifiers::empty(),
            ),
            point,
        );
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            state().cursor.selection,
            None,
            "collapse propagates without changing text"
        );
        assert_eq!(state().text, "한글");
        assert!(state().revision > shifted.revision);
        let mut replace = Bounds("Replace editor", None);
        ui.operate(&renderer, &mut replace);
        click(&mut ui, &mut renderer, replace.1.unwrap().center());
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        assert_eq!(state().text, "한글\n👍🏽");
        assert_eq!(state().cursor.position, wire::EditorPosition::default());
        assert!(state().reset > typed.reset);
        let mut place = Bounds("Place caret", None);
        ui.operate(&renderer, &mut place);
        click(&mut ui, &mut renderer, place.1.unwrap().center());
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            state().cursor.position,
            wire::EditorPosition { line: 1, column: 0 },
            "guest requested byte 6 inside emoji cluster snaps backward"
        );
        assert_eq!(state().cursor.selection, None);
        let mut open = Bounds("Open editor overlay", None);
        ui.operate(&renderer, &mut open);
        click(&mut ui, &mut renderer, open.1.unwrap().center());
        for _ in 0..3 { ui = redraw(ui, &guest, &mut renderer, &mut now); }
        let overlay_key = {
            let locked = guest.lock().unwrap();
            let wire::Node::Overlay { children, .. } = locked.frame.root.as_ref().unwrap() else { unreachable!() };
            editor(&children[1]).unwrap().key().unwrap().to_owned()
        };
        let mut overlay_bounds = Bounds(&overlay_key, None);
        ui.operate(&renderer, &mut overlay_bounds);
        let overlay_point = overlay_bounds.1.unwrap().center();
        click(&mut ui, &mut renderer, overlay_point);
        send(&mut ui, &mut renderer, key_event("a", command), overlay_point);
        send(&mut ui, &mut renderer, named(keyboard::key::Named::ArrowRight, keyboard::Modifiers::empty()), overlay_point);
        send(&mut ui, &mut renderer, key_event("o", keyboard::Modifiers::empty()), overlay_point);
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        assert_eq!(state().text, "한글\n👍🏽o", "base document initializes the sibling overlay");
        let mut close = Bounds("Close editor overlay", None);
        ui.operate(&renderer, &mut close);
        click(&mut ui, &mut renderer, close.1.unwrap().center());
        for _ in 0..3 { ui = redraw(ui, &guest, &mut renderer, &mut now); }
        click(&mut ui, &mut renderer, point);
        send(&mut ui, &mut renderer, key_event("a", command), point);
        send(&mut ui, &mut renderer, named(keyboard::key::Named::ArrowRight, keyboard::Modifiers::empty()), point);
        send(&mut ui, &mut renderer, key_event("b", keyboard::Modifiers::empty()), point);
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        assert_eq!(state().text, "한글\n👍🏽ob", "overlay edits synchronize the retained base Content");
        drop(ui);
    }
}

//! The actual wasm guest mounted in GuestView, using native mouse/key events.
use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Operation;
use iced::{Event, Rectangle, Size, mouse, window};
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
fn guest(program: Option<std::path::PathBuf>, allowed: bool) -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/terminal-fixture/app_store_terminal_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle widget fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Arc::new(Mutex::new(
        Guest::load_with_terminal(
            &CatalogEntry {
                id: "terminal-fixture".into(),
                name: "Terminal fixture".into(),
                description: String::new(),
                capabilities: if allowed {
                    vec![crate::catalog::Capability {
                        name: "terminal".into(),
                    }]
                } else {
                    vec![]
                },
                path: path.to_string_lossy().into_owned(),
                mark: "W".into(),
                hash,
            },
            program,
        )
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
        Size::new(600.0, 1000.0),
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
#[derive(Default)]
struct TextBounds<'a> {
    label: &'a str,
    bounds: Option<Rectangle>,
    translation: iced::Vector,
    pending: Option<iced::Vector>,
}
impl Operation for TextBounds<'_> {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        let offset = self.pending.take().unwrap_or(iced::Vector::ZERO);
        self.translation += offset;
        visit(self);
        self.translation -= offset;
    }
    fn scrollable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: Rectangle,
        _content: Rectangle,
        translation: iced::Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.pending = Some(translation);
    }
    fn text(&mut self, _id: Option<&iced::widget::Id>, bounds: Rectangle, text: &str) {
        if text == self.label {
            self.bounds = Some(Rectangle {
                x: bounds.x - self.translation.x,
                y: bounds.y - self.translation.y,
                ..bounds
            });
        }
    }
}
fn click(ui: &mut Ui, renderer: &mut iced::Renderer, label: &str) {
    let mut find = TextBounds {
        label,
        bounds: None,
        ..Default::default()
    };
    ui.operate(renderer, &mut find);
    let point = find.bounds.expect("rendered button label").center();
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
fn value(guest: &Arc<Mutex<Guest>>, key: &str) -> String {
    fn find(node: &ui_lang_wire::Node, key: &str) -> Option<String> {
        match node {
            ui_lang_wire::Node::Input {
                key: found, value, ..
            } if found == key => return Some(value.clone()),
            ui_lang_wire::Node::Text {
                key: found,
                content,
                ..
            } if found == key => return Some(content.clone()),
            _ => {}
        }
        node.children().iter().find_map(|child| find(child, key))
    }
    find(
        guest.lock().unwrap().frame.root.as_ref().unwrap(),
        &format!("TerminalFixture/{key}"),
    )
    .unwrap()
}

fn key(
    ui: &mut Ui,
    renderer: &mut iced::Renderer,
    key: iced::keyboard::Key,
    code: iced::keyboard::key::Code,
    modifiers: iced::keyboard::Modifiers,
    text: Option<&str>,
) {
    ui.update(
        &[Event::Keyboard(iced::keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: iced::keyboard::key::Physical::Code(code),
            location: iced::keyboard::Location::Standard,
            modifiers,
            text: text.map(Into::into),
            repeat: false,
        })],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn type_text(ui: &mut Ui, renderer: &mut iced::Renderer, text: &str) {
    for c in text.chars() {
        let text = c.to_string();
        key(
            ui,
            renderer,
            iced::keyboard::Key::Character(text.clone().into()),
            iced::keyboard::key::Code::KeyA,
            iced::keyboard::Modifiers::empty(),
            Some(&text),
        );
    }
}

fn until(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut Instant,
    expected: &str,
) -> Ui {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        ui = redraw(ui, guest, renderer, now);
        if value(guest, "title") == expected {
            return ui;
        }
        assert!(
            Instant::now() < deadline,
            "missing title {expected}: {}",
            value(guest, "title")
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
#[ignore = "requires bundled terminal-fixture wasm and Unix PTY"]
fn bundled_terminal_native_keyboard_output_and_isolation() {
    let mut renderer = renderer();
    let guest = guest(Some("/bin/sh".into()), true);
    let mut now = Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = until(ui, &guest, &mut renderer, &mut now, "Terminal");
    let position = iced::Point::new(40.0, 150.0);
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
    type_text(
        &mut ui,
        &mut renderer,
        "printf '\\033]2;KEYBOARD_OK\\007\\033[48;2;255;0;0m        \\033[0m'",
    );
    key(
        &mut ui,
        &mut renderer,
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
        iced::keyboard::key::Code::Enter,
        iced::keyboard::Modifiers::empty(),
        None,
    );
    ui = until(ui, &guest, &mut renderer, &mut now, "KEYBOARD_OK");
    ui.draw(
        &mut renderer,
        &iced::Theme::Dark,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::WHITE,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(600, 1000), 1.0, iced::Color::BLACK);
    if let Ok(path) = std::env::var("ICE_TERMINAL_CAPTURE") {
        std::fs::write(path, &pixels).unwrap();
    }
    assert!(
        pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 100
                && pixel[0] > pixel[1].saturating_mul(2)
                && pixel[0] > pixel[2].saturating_mul(2))
            .count()
            > 100,
        "ANSI red background must be painted by the native terminal"
    );
    let terminal = Arc::downgrade(guest.lock().unwrap().terminal.as_ref().unwrap());
    let other = self::guest(None, true);
    assert!(!Arc::ptr_eq(
        guest.lock().unwrap().terminal.as_ref().unwrap(),
        other.lock().unwrap().terminal.as_ref().unwrap()
    ));
    drop(ui);
    drop(guest);
    assert!(
        terminal.upgrade().is_none(),
        "dropping guest and view must release terminal owner"
    );
}
#[test]
#[ignore = "requires bundled terminal-fixture wasm and Unix PTY"]
fn bundled_terminal_capability_refusal_and_stream_cancellation() {
    let denied = guest(Some("/does/not/exist".into()), false);
    assert!(
        denied.lock().unwrap().terminal.is_none(),
        "undeclared capability must not spawn a process"
    );
    let allowed = guest(None, true);
    let mut renderer = renderer();
    let mut now = Instant::now();
    let ui = build(&allowed, user_interface::Cache::default(), &mut renderer);
    let _ui = until(
        ui,
        &allowed,
        &mut renderer,
        &mut now,
        "Terminal is not configured by this host",
    );
    let mut guest = allowed.lock().unwrap();
    assert_eq!(guest.terminal_subscriptions.len(), 1);
    let id = guest.terminal_subscriptions[0];
    guest.cancel(id);
    assert!(guest.terminal_subscriptions.is_empty());
    assert!(
        guest.terminal.is_some(),
        "stream cancellation must not end session ownership"
    );
}

#[test]
#[ignore = "requires bundled terminal-fixture wasm and Unix PTY"]
fn bundled_terminal_drains_background_output_while_unmounted() {
    use std::os::unix::fs::PermissionsExt;
    struct Directory(std::path::PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let dir = Directory(
        std::env::temp_dir().join(format!("ice-terminal-background-{}", std::process::id())),
    );
    std::fs::create_dir_all(&dir.0).unwrap();
    let program = dir.0.join("program");
    let gate = dir.0.join("gate");
    std::fs::write(&program, format!("#!/bin/sh\nwhile [ ! -e '{}' ]; do sleep 0.01; done\nprintf '\\033]2;BACKGROUND_DONE\\007\\033[48;2;255;0;0m        \\033[0m'\n", gate.display())).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let guest = guest(Some(program), true);
    let mut renderer = renderer();
    let mut now = Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = until(ui, &guest, &mut renderer, &mut now, "Terminal");
    click(&mut ui, &mut renderer, "Toggle");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    fn terminal_count(node: &wire::Node) -> usize {
        usize::from(matches!(node, wire::Node::Surface { .. }))
            + node.children().iter().map(terminal_count).sum::<usize>()
    }
    assert_eq!(
        terminal_count(guest.lock().unwrap().frame.root.as_ref().unwrap()),
        0
    );
    {
        let mut guest = guest.lock().unwrap();
        guest.due = (0..MAX_DUE)
            .map(|index| {
                (
                    now + Duration::from_secs(300),
                    wire::Event::Response {
                        id: 1_000_000 + index as u64,
                        result: Ok(vec![]),
                        done: true,
                    },
                )
            })
            .collect();
        guest.resting_until = Some(now + Duration::from_secs(120));
    }
    std::fs::write(&gate, "go").unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while guest.lock().unwrap().terminal_notice.is_none() {
        assert!(
            Instant::now() < deadline,
            "native output must be polled while guest rests"
        );
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        std::thread::sleep(Duration::from_millis(2));
    }
    {
        let mut guest = guest.lock().unwrap();
        assert_eq!(
            guest.due.len(),
            MAX_DUE,
            "native metadata must not overflow the due queue"
        );
        guest.due.clear();
        guest.resting_until = None;
    }
    ui = until(ui, &guest, &mut renderer, &mut now, "BACKGROUND_DONE");
    let deadline = Instant::now() + Duration::from_secs(30);
    while value(&guest, "running") != "false" {
        assert!(Instant::now() < deadline, "exit must reach hidden guest");
        ui = redraw(ui, &guest, &mut renderer, &mut now);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        guest
            .lock()
            .unwrap()
            .terminal
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .next_poll()
            .is_none()
    );
    click(&mut ui, &mut renderer, "Toggle");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now);
    }
    assert_eq!(
        terminal_count(guest.lock().unwrap().frame.root.as_ref().unwrap()),
        1
    );
    ui.draw(
        &mut renderer,
        &iced::Theme::Dark,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::WHITE,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(600, 1000), 1.0, iced::Color::BLACK);
    assert!(
        pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 100
                && pixel[0] > pixel[1].saturating_mul(2)
                && pixel[0] > pixel[2].saturating_mul(2))
            .count()
            > 100,
        "remount must paint final native ANSI output"
    );
}

#[test]
#[ignore = "requires bundled terminal-fixture wasm"]
fn bundled_terminal_initial_notice_precedes_coalesced_exit() {
    let guest = guest(None, true);
    let mut guest = guest.lock().unwrap();
    let now = Instant::now();
    guest.answer(
        now,
        wire::Request {
            id: 9876,
            kind: "terminal.events".into(),
            payload: vec![],
        },
    );
    let mut latest = guest
        .terminal
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .notice(false);
    if let wire::SurfaceValue::Record { fields, .. } = &mut latest {
        fields[1].1 = wire::SurfaceValue::Str("LATEST_EXIT".into());
    }
    guest.terminal_notice = Some(latest.clone());
    guest.deliver_due(now);
    let received = guest
        .pending
        .iter()
        .filter_map(|event| match event {
            wire::Event::Response {
                id: 9876,
                result: Ok(bytes),
                ..
            } => Some(wire::decode::<wire::SurfaceValue>(bytes).unwrap()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(received.len(), 2);
    assert_eq!(
        received.last(),
        Some(&latest),
        "latest terminal state must follow its initial snapshot"
    );
}

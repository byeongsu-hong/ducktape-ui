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
fn guest() -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/composer-fixture/app_store_composer_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle widget fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Arc::new(Mutex::new(
        Guest::load(&CatalogEntry {
            preferred_size: None,
            id: "composer-fixture".into(),
            name: "Composer fixture".into(),
            description: String::new(),
            capabilities: vec![],
            path: path.to_string_lossy().into_owned(),
            mark: "W".into(),
            hash,
        })
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
            ui_lang_wire::Node::Surface {
                key: found, args, ..
            } if key == "ComposerFixture/draft" && found == "ComposerFixture/first" => {
                if let Some(ui_lang_wire::SurfaceValue::Str(text)) = args.first() {
                    return Some(text.clone());
                }
            }
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
    let guest = guest.lock().unwrap();
    let root = guest.frame.root.as_ref().unwrap();
    find(root, &format!("ComposerFixture/{key}"))
        .unwrap_or_else(|| panic!("missing {key}: {root:#?}"))
}

fn frames(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut std::time::Instant,
) -> Ui {
    for _ in 0..4 {
        ui = redraw(ui, guest, renderer, now);
    }
    ui
}

fn editor_click(ui: &mut Ui, renderer: &mut iced::Renderer, index: usize) {
    #[derive(Default)]
    struct Editors(Vec<Rectangle>);
    impl Operation for Editors {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn focusable(
            &mut self,
            _id: Option<&iced::widget::Id>,
            bounds: Rectangle,
            _state: &mut dyn iced::advanced::widget::operation::Focusable,
        ) {
            if bounds.height >= 80.0 {
                self.0.push(bounds);
            }
        }
    }
    let mut editors = Editors::default();
    ui.operate(renderer, &mut editors);
    assert_eq!(
        editors.0.len(),
        2,
        "both native rich editors must be mounted"
    );
    let point = editors.0[index].position() + iced::Vector::new(8.0, 8.0);
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

#[test]
#[ignore = "requires bundled composer-fixture wasm"]
fn bundled_composer_typing_enter_selection_history_and_instances() {
    use iced::keyboard::{
        Key, Modifiers,
        key::{Code, Named},
    };
    let mut renderer = renderer();
    let guest = guest();
    let mut now = std::time::Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &guest, &mut renderer, &mut now);
    editor_click(&mut ui, &mut renderer, 0);
    for character in "hello".chars() {
        type_text(&mut ui, &mut renderer, &character.to_string());
        ui = frames(ui, &guest, &mut renderer, &mut now);
    }
    assert_eq!(value(&guest, "draft"), "hello");
    assert_eq!(value(&guest, "other"), "");
    assert_eq!(value(&guest, "column"), "5");
    key(
        &mut ui,
        &mut renderer,
        Key::Named(Named::Enter),
        Code::Enter,
        Modifiers::empty(),
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "hello");
    assert_eq!(value(&guest, "submitted"), "1");
    key(
        &mut ui,
        &mut renderer,
        Key::Named(Named::Enter),
        Code::Enter,
        Modifiers::SHIFT,
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "hello\n");
    assert_eq!(value(&guest, "submitted"), "1");
    let command = if cfg!(target_os = "macos") {
        Modifiers::LOGO
    } else {
        Modifiers::CTRL
    };
    key(
        &mut ui,
        &mut renderer,
        Key::Character("z".into()),
        Code::KeyZ,
        command,
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "hello");
    key(
        &mut ui,
        &mut renderer,
        Key::Character("z".into()),
        Code::KeyZ,
        command | Modifiers::SHIFT,
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "hello\n");
    key(
        &mut ui,
        &mut renderer,
        Key::Character("a".into()),
        Code::KeyA,
        command,
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "selected"), "hello\n");
    key(
        &mut ui,
        &mut renderer,
        Key::Character("b".into()),
        Code::KeyB,
        command,
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "**hello\n**");
    editor_click(&mut ui, &mut renderer, 1);
    type_text(&mut ui, &mut renderer, "second");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "other"), "second");
    assert_eq!(value(&guest, "draft"), "**hello\n**");
    click(&mut ui, &mut renderer, "Advance");
    let _ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "**hello\n**");
    let mut ui = _ui;
    if let Ok(path) = std::env::var("ICE_COMPOSER_CAPTURE") {
        ui.draw(
            &mut renderer,
            &iced::Theme::Dark,
            &iced::advanced::renderer::Style {
                text_color: iced::Color::WHITE,
            },
            mouse::Cursor::Unavailable,
        );
        std::fs::write(
            path,
            renderer.screenshot(Size::new(600, 1000), 1.0, iced::Color::BLACK),
        )
        .unwrap();
    }
}

#[test]
#[ignore = "requires bundled composer-fixture wasm"]
fn bundled_composer_ime_reset_disabled_and_remount() {
    use iced::advanced::input_method;
    use iced::keyboard::{Key, Modifiers, key::Code};
    let mut renderer = renderer();
    let guest = guest();
    let mut now = std::time::Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &guest, &mut renderer, &mut now);
    editor_click(&mut ui, &mut renderer, 0);
    ui.update(
        &[
            Event::InputMethod(input_method::Event::Opened),
            Event::InputMethod(input_method::Event::Preedit("한글".into(), Some(6..6))),
        ],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "draft"),
        "",
        "composition must not publish an uncommitted draft"
    );
    ui.update(
        &[Event::InputMethod(input_method::Event::Commit(
            "한글".into(),
        ))],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "한글");
    assert_eq!(value(&guest, "other"), "");
    click(&mut ui, &mut renderer, "Reset");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "replacement");
    editor_click(&mut ui, &mut renderer, 0);
    let command = if cfg!(target_os = "macos") {
        Modifiers::LOGO
    } else {
        Modifiers::CTRL
    };
    key(
        &mut ui,
        &mut renderer,
        Key::Character("z".into()),
        Code::KeyZ,
        command,
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "draft"),
        "replacement",
        "reset must discard the old document history"
    );
    click(&mut ui, &mut renderer, "Disable");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    editor_click(&mut ui, &mut renderer, 0);
    type_text(&mut ui, &mut renderer, "ignored");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "replacement");
    click(&mut ui, &mut renderer, "Disable");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    click(&mut ui, &mut renderer, "Toggle");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    click(&mut ui, &mut renderer, "Toggle");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    editor_click(&mut ui, &mut renderer, 0);
    key(
        &mut ui,
        &mut renderer,
        Key::Character("a".into()),
        Code::KeyA,
        command,
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "selected"),
        "replacement",
        "remount must seed the retained guest draft"
    );
    type_text(&mut ui, &mut renderer, "fresh");
    let _ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "fresh");
}

#[test]
#[ignore = "requires bundled composer-fixture wasm"]
fn bundled_composer_large_selection_stays_deliverable_and_oversize_edit_is_atomic() {
    use iced::advanced::input_method;
    use iced::keyboard::{Key, Modifiers, key::Code};
    let mut renderer = renderer();
    let guest = guest();
    let mut now = std::time::Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &guest, &mut renderer, &mut now);
    editor_click(&mut ui, &mut renderer, 0);
    let text = "x".repeat(32_000);
    ui.update(
        &[Event::InputMethod(input_method::Event::Commit(
            text.clone(),
        ))],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), text);
    let command = if cfg!(target_os = "macos") {
        Modifiers::LOGO
    } else {
        Modifiers::CTRL
    };
    key(
        &mut ui,
        &mut renderer,
        Key::Character("a".into()),
        Code::KeyA,
        command,
        None,
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "selected_bytes"),
        text.len().to_string(),
        "full selection must fit the complete surface notice"
    );
    ui.update(
        &[Event::InputMethod(input_method::Event::Commit(
            "y".repeat(40_000),
        ))],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "draft"),
        text,
        "reject an oversized replacement without losing the draft"
    );
    assert_eq!(
        value(&guest, "selected_bytes"),
        text.len().to_string(),
        "rejection must restore the original selected range"
    );
    type_text(&mut ui, &mut renderer, "ok");
    let _ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "ok");
}

// Deliver a checked fixture route without a mouse press that would blur the editor.
fn dispatch(guest: &Arc<Mutex<Guest>>, key: &str) {
    fn find(node: &ui_lang_wire::Node, key: &str) -> Option<u32> {
        if let ui_lang_wire::Node::Button {
            key: found,
            on_press,
            ..
        } = node
            && found == key
        {
            return *on_press;
        }
        node.children().iter().find_map(|child| find(child, key))
    }
    let mut guest = guest.lock().unwrap();
    let handler = find(
        guest.frame.root.as_ref().unwrap(),
        &format!("ComposerFixture/{key}"),
    )
    .unwrap();
    guest.pending.push(ui_lang_wire::Event::Message(handler));
    guest.tick();
    assert!(guest.fault.is_none());
}
fn ime(ui: &mut Ui, renderer: &mut iced::Renderer, event: iced::advanced::input_method::Event) {
    ui.update(
        &[Event::InputMethod(event)],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
#[test]
#[ignore = "requires bundled composer-fixture wasm"]
fn bundled_composer_programmatic_reset_cancels_old_composition() {
    use iced::advanced::input_method::Event as Ime;
    let mut renderer = renderer();
    let guest = guest();
    let mut now = std::time::Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &guest, &mut renderer, &mut now);
    editor_click(&mut ui, &mut renderer, 0);
    ime(&mut ui, &mut renderer, Ime::Opened);
    ime(
        &mut ui,
        &mut renderer,
        Ime::Preedit("old".into(), Some(3..3)),
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    dispatch(&guest, "reset");
    ui = build(&guest, ui.into_cache(), &mut renderer);
    ui = frames(ui, &guest, &mut renderer, &mut now);
    ime(&mut ui, &mut renderer, Ime::Commit("old".into()));
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "draft"),
        "replacement",
        "old composition must not enter a replacement document"
    );
    editor_click(&mut ui, &mut renderer, 0);
    type_text(&mut ui, &mut renderer, "new");
    let _ui = frames(ui, &guest, &mut renderer, &mut now);
    assert!(
        value(&guest, "draft").contains("new"),
        "the replacement must accept a fresh interaction"
    );
}
#[test]
#[ignore = "requires bundled composer-fixture wasm"]
fn bundled_composer_disabled_composition_does_not_resume_on_enable() {
    use iced::advanced::input_method::Event as Ime;
    let mut renderer = renderer();
    let guest = guest();
    let mut now = std::time::Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &guest, &mut renderer, &mut now);
    editor_click(&mut ui, &mut renderer, 0);
    ime(&mut ui, &mut renderer, Ime::Opened);
    ime(
        &mut ui,
        &mut renderer,
        Ime::Preedit("old".into(), Some(3..3)),
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    for _ in 0..2 {
        dispatch(&guest, "disable");
        ui = build(&guest, ui.into_cache(), &mut renderer);
        ui = frames(ui, &guest, &mut renderer, &mut now);
    }
    ime(&mut ui, &mut renderer, Ime::Commit("old".into()));
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "draft"),
        "",
        "re-enabling must not restore the old composing focus"
    );
    editor_click(&mut ui, &mut renderer, 0);
    type_text(&mut ui, &mut renderer, "fresh");
    let _ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "draft"), "fresh");
}
#[test]
#[ignore = "requires bundled composer-fixture wasm"]
fn bundled_composer_replacement_guest_cannot_inherit_focus_composition_or_history() {
    use iced::advanced::input_method::Event as Ime;
    let mut renderer = renderer();
    let first = guest();
    let mut now = std::time::Instant::now();
    let ui = build(&first, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &first, &mut renderer, &mut now);
    editor_click(&mut ui, &mut renderer, 0);
    type_text(&mut ui, &mut renderer, "private");
    ui = frames(ui, &first, &mut renderer, &mut now);
    ime(&mut ui, &mut renderer, Ime::Opened);
    ime(
        &mut ui,
        &mut renderer,
        Ime::Preedit("old".into(), Some(3..3)),
    );
    ui = frames(ui, &first, &mut renderer, &mut now);
    let second = guest();
    let ui = build(&second, ui.into_cache(), &mut renderer);
    let mut ui = frames(ui, &second, &mut renderer, &mut now);
    ime(&mut ui, &mut renderer, Ime::Commit("old".into()));
    type_text(&mut ui, &mut renderer, "unfocused");
    ui = frames(ui, &second, &mut renderer, &mut now);
    assert_eq!(
        value(&second, "draft"),
        "",
        "replacement must not inherit composing focus"
    );
    editor_click(&mut ui, &mut renderer, 0);
    let command = if cfg!(target_os = "macos") {
        iced::keyboard::Modifiers::LOGO
    } else {
        iced::keyboard::Modifiers::CTRL
    };
    key(
        &mut ui,
        &mut renderer,
        iced::keyboard::Key::Character("z".into()),
        iced::keyboard::key::Code::KeyZ,
        command,
        None,
    );
    ui = frames(ui, &second, &mut renderer, &mut now);
    assert_eq!(
        value(&second, "draft"),
        "",
        "replacement must not inherit another guest's undo stack"
    );
    type_text(&mut ui, &mut renderer, "fresh");
    let _ui = frames(ui, &second, &mut renderer, &mut now);
    assert_eq!(value(&second, "draft"), "fresh");
    assert_eq!(value(&first, "draft"), "private");
}

#[test]
#[ignore = "requires bundled composer-fixture wasm"]
fn bundled_composer_concurrent_guests_keep_private_drafts() {
    let mut renderer = renderer();
    let first = guest();
    let second = guest();
    let mut now = std::time::Instant::now();
    let ui = build(&first, user_interface::Cache::default(), &mut renderer);
    let mut first_ui = frames(ui, &first, &mut renderer, &mut now);
    editor_click(&mut first_ui, &mut renderer, 0);
    type_text(&mut first_ui, &mut renderer, "private");
    first_ui = frames(first_ui, &first, &mut renderer, &mut now);
    let ui = build(&second, user_interface::Cache::default(), &mut renderer);
    let mut second_ui = frames(ui, &second, &mut renderer, &mut now);
    editor_click(&mut second_ui, &mut renderer, 0);
    type_text(&mut second_ui, &mut renderer, "public");
    let _second_ui = frames(second_ui, &second, &mut renderer, &mut now);
    type_text(&mut first_ui, &mut renderer, "!");
    let _first_ui = frames(first_ui, &first, &mut renderer, &mut now);
    assert_eq!(
        value(&first, "draft"),
        "private!",
        "another guest must not replace the live native draft"
    );
    assert_eq!(value(&second, "draft"), "public");
}

use super::*;

#[test]
fn a_key_binding_override_parts_plain_enter_from_shift_enter() {
    use iced::advanced::clipboard;
    use iced::advanced::renderer::Headless;
    use iced::keyboard::key::{Code, Physical};
    use iced::keyboard::{Key, Location, Modifiers};

    let content = Content::new();
    let mut editor = RichTextEditor::new(&content, ContentVersion::new(1, 0))
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(80.0))
        .on_action(|action| action)
        .key_binding(|press| {
            let plain_enter =
                matches!(press.key, Key::Named(key::Named::Enter)) && !press.modifiers.shift();
            if plain_enter {
                // The application owns plain Enter (e.g. submit) — the key
                // bubbles instead of editing the document.
                None
            } else {
                default_key_binding(press)
            }
        });
    let renderer = iced_test::futures::futures::executor::block_on(
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
    )
    .expect("headless renderer");
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
    tree.state
        .downcast_mut::<State<text::highlighter::PlainText>>()
        .focus = Some(Focus::now());
    let limits = layout::Limits::new(Size::ZERO, Size::new(120.0, 80.0));
    let node = editor.layout(&mut tree, &renderer, &limits);
    let viewport = Rectangle::with_size(Size::new(120.0, 80.0));
    let mut clipboard = clipboard::Null;
    let mut messages: Vec<Action> = Vec::new();

    let enter_key = Key::Named(key::Named::Enter);
    let enter = |modifiers: Modifiers| {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: enter_key.clone(),
            modified_key: enter_key.clone(),
            physical_key: Physical::Code(Code::Enter),
            location: Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    };

    let mut shell = Shell::new(&mut messages);
    editor.update(
        &mut tree,
        &enter(Modifiers::empty()),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
        &mut clipboard,
        &mut shell,
        &viewport,
    );
    assert!(
        messages.is_empty(),
        "plain Enter must bubble to the application, got {messages:?}"
    );

    let mut shell = Shell::new(&mut messages);
    editor.update(
        &mut tree,
        &enter(Modifiers::SHIFT),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
        &mut clipboard,
        &mut shell,
        &viewport,
    );
    assert_eq!(
        messages,
        [Action::Edit(text_editor::Action::Edit(Edit::Enter))],
        "shift+enter delegates to the stock newline binding"
    );
}

//! The `on_chord` seam: a key press that resolves to NO binding — the ones
//! `default_key_binding` deliberately lets bubble — is offered to the chord
//! route, and a message consumes the press instead of bubbling it.

use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, Widget, clipboard, layout, mouse, widget};
use iced::keyboard;
use iced::keyboard::key::{Code, Physical};
use iced::keyboard::{Key, Location, Modifiers};
use iced::widget::text_editor::Content;
use iced::{Event, Font, Pixels, Point, Rectangle, Size, Theme};
use ui_lang_runtime::rich_text_editor::default_key_binding;
use ui_lang_runtime::{ContentVersion, RichTextEditor, rich_text_editor::Action};

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Edited,
    Mark(&'static str),
}

fn headless_renderer() -> iced::Renderer {
    iced_test::futures::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        None,
    ))
    .expect("headless renderer")
}

fn key_press(character: char, modifiers: Modifiers) -> Event {
    let key = Key::Character(character.to_string().into());
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: Physical::Code(match character {
            'b' => Code::KeyB,
            'k' => Code::KeyK,
            _ => panic!("unmapped test key"),
        }),
        location: Location::Standard,
        modifiers,
        text: (!modifiers.command()).then(|| character.to_string().into()),
        repeat: false,
    })
}

#[test]
fn unbound_command_chords_reach_the_chord_route_and_are_consumed() {
    let content = Content::with_text("draft");
    let renderer = headless_renderer();
    let limits = layout::Limits::new(Size::ZERO, Size::new(200.0, 100.0));
    let viewport = Rectangle::with_size(Size::new(200.0, 100.0));
    let mut editor = RichTextEditor::<_, Message>::new(&content, ContentVersion::new(1, 0))
        .on_action(|_: Action| Message::Edited)
        .key_binding(default_key_binding)
        .on_chord(|press| {
            let bold = press.modifiers.command()
                && matches!(press.physical_key, Physical::Code(Code::KeyB));
            bold.then_some(Message::Mark("bold"))
        });
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
    let node = editor.layout(&mut tree, &renderer, &limits);
    let mut clipboard = clipboard::Null;
    let mut messages: Vec<Message> = Vec::new();

    // Focus the editor with a click, like a reader would.
    let mut shell = Shell::new(&mut messages);
    editor.update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(5.0, 5.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &viewport,
    );
    messages.clear();

    let mut drive = |event: Event, messages: &mut Vec<Message>| {
        let mut shell = Shell::new(messages);
        editor.update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        shell.is_event_captured()
    };

    // Cmd+B resolves to no binding; the chord route claims it.
    let captured = drive(key_press('b', Modifiers::COMMAND), &mut messages);
    assert!(captured, "a claimed chord must not bubble");
    assert_eq!(messages, [Message::Mark("bold")]);
    messages.clear();

    // Cmd+K is unclaimed: nothing publishes and the press bubbles on.
    let captured = drive(key_press('k', Modifiers::COMMAND), &mut messages);
    assert!(!captured, "an unclaimed chord keeps the bubble contract");
    assert!(messages.is_empty());

    // A plain letter is a bound edit — the chord route is never consulted.
    let captured = drive(key_press('b', Modifiers::empty()), &mut messages);
    assert!(captured);
    assert_eq!(messages, [Message::Edited]);
}

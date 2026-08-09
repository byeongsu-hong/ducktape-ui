//! What the keys do in the message box.
//!
//! The box is a multi-line editor, so Enter has to mean one of two things. It
//! sends, because that is what Enter does in a chat window; every other way of
//! pressing it — with shift, with command, with both — falls through to the
//! editor's own binding and inserts a line break.

/// Ice needs a type for the command a binding produces; sending is the only
/// one this box has.
#[derive(Clone, Debug, PartialEq)]
pub struct Send;

pub fn composer_keys(
    event: iced::widget::text_editor::KeyPress,
) -> Option<iced::widget::text_editor::Binding<Send>> {
    let plain = !event.modifiers.shift() && !event.modifiers.command() && !event.modifiers.alt();
    if event.key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) && plain {
        return Some(iced::widget::text_editor::Binding::Custom(Send));
    }
    iced::widget::text_editor::Binding::from_key_press(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::{Modifiers, key};
    use iced::widget::text_editor::{Binding, KeyPress, Status};

    fn press(modifiers: Modifiers) -> KeyPress {
        KeyPress {
            key: iced::keyboard::Key::Named(key::Named::Enter),
            modified_key: iced::keyboard::Key::Named(key::Named::Enter),
            modifiers,
            text: None,
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            status: Status::Focused { is_hovered: false },
        }
    }

    /// The whole point of the binding: bare Enter sends, and every other way of
    /// pressing it writes a line instead. Getting this backwards makes the box
    /// impossible to write a paragraph in.
    #[test]
    fn only_a_bare_enter_sends() {
        assert!(
            matches!(
                composer_keys(press(Modifiers::default())),
                Some(Binding::Custom(Send))
            ),
            "Enter on its own sends"
        );
        for held in [
            Modifiers::SHIFT,
            Modifiers::COMMAND,
            Modifiers::SHIFT | Modifiers::COMMAND,
        ] {
            assert!(
                !matches!(composer_keys(press(held)), Some(Binding::Custom(Send))),
                "Enter held with {held:?} must write a line, not send"
            );
        }
    }
}

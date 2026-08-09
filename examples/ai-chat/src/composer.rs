//! What the keys do in the message box.
//!
//! The box is a multi-line editor, so Enter has to mean one of two things. It
//! sends, because that is what Enter does in a chat window; every other way of
//! pressing it — with shift, with command, with both — falls through to the
//! editor's own binding and inserts a line break.
//!
//! Backspace is here for a different reason. iced maps every Backspace to
//! deleting one character whatever is held with it, so the two erasures every
//! other text box on the platform has — a word at a time, and back to the
//! start of the line — simply did nothing. Both are spelled the same way: take
//! the selection to where the erasure should reach, then erase it.

/// Ice needs a type for the command a binding produces; sending is the only
/// one this box has.
#[derive(Clone, Debug, PartialEq)]
pub struct Send;

pub fn composer_keys(
    event: iced::widget::text_editor::KeyPress,
) -> Option<iced::widget::text_editor::Binding<Send>> {
    use iced::keyboard::{Key, key::Named};
    use iced::widget::text_editor::Binding;
    use iced::widget::text_editor::Motion;

    let held = event.modifiers;
    let plain = !held.shift() && !held.command() && !held.alt();

    match event.key.as_ref() {
        Key::Named(Named::Enter) if plain => Some(Binding::Custom(Send)),
        // Erase a word, or back to the start of the line. `command()` is the
        // platform's own modifier — ⌘ on macOS, ctrl elsewhere — so this reads
        // the same on either.
        Key::Named(Named::Backspace) if held.command() => Some(erase(Motion::Home)),
        Key::Named(Named::Backspace) if held.alt() => Some(erase(Motion::WordLeft)),
        // And forwards, for the platforms that have those.
        Key::Named(Named::Delete) if held.command() => Some(erase(Motion::End)),
        Key::Named(Named::Delete) if held.alt() => Some(erase(Motion::WordRight)),
        _ => Binding::from_key_press(event),
    }
}

/// Select as far as the erasure should reach, then erase the selection.
fn erase(reach: iced::widget::text_editor::Motion) -> iced::widget::text_editor::Binding<Send> {
    use iced::widget::text_editor::Binding;
    Binding::Sequence(vec![Binding::Select(reach), Binding::Backspace])
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::{Modifiers, key};
    use iced::widget::text_editor::{Binding, KeyPress, Motion, Status};

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

    fn press_key(key: key::Named, modifiers: Modifiers) -> KeyPress {
        KeyPress {
            key: iced::keyboard::Key::Named(key),
            modified_key: iced::keyboard::Key::Named(key),
            modifiers,
            text: None,
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            status: Status::Focused { is_hovered: false },
        }
    }

    /// iced maps every Backspace to one character whatever is held with it, so
    /// erasing a word or a line — which every other text box on the platform
    /// does — needs saying here or it does not happen at all.
    #[test]
    fn a_held_backspace_erases_more_than_one_character() {
        for (held, reach) in [
            (Modifiers::ALT, Motion::WordLeft),
            (Modifiers::COMMAND, Motion::Home),
        ] {
            let binding = composer_keys(press_key(key::Named::Backspace, held));
            assert!(
                matches!(
                    binding,
                    Some(Binding::Sequence(ref steps))
                        if steps.len() == 2
                            && matches!(steps[0], Binding::Select(m) if m == reach)
                            && matches!(steps[1], Binding::Backspace)
                ),
                "Backspace with {held:?} must reach {reach:?}, got {binding:?}"
            );
        }

        assert!(
            matches!(
                composer_keys(press_key(key::Named::Backspace, Modifiers::default())),
                Some(Binding::Backspace)
            ),
            "and on its own it is still one character"
        );
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

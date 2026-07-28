use iced::widget::text_editor::{Binding, KeyPress};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorCommand;

pub fn editor_keys(event: KeyPress) -> Option<Binding<EditorCommand>> {
    if event.key.to_latin(event.physical_key) == Some('p')
        && event.modifiers.command()
        && event.modifiers.shift()
    {
        Some(Binding::Custom(EditorCommand))
    } else {
        Binding::from_key_press(event)
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorCommand, editor_keys};
    use iced::keyboard::{Key, Modifiers, key};
    use iced::widget::text_editor::{Binding, KeyPress, Status};

    fn key_press(character: char, modifiers: Modifiers) -> KeyPress {
        let key = Key::Character(character.to_string().into());
        KeyPress {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(match character {
                'p' => key::Code::KeyP,
                _ => key::Code::KeyX,
            }),
            modifiers,
            text: Some(character.to_string().into()),
            status: Status::Focused { is_hovered: false },
        }
    }

    #[test]
    fn preview_shortcut_preserves_native_edit_bindings() {
        assert_eq!(
            editor_keys(key_press('p', Modifiers::COMMAND | Modifiers::SHIFT)),
            Some(Binding::Custom(EditorCommand))
        );
        assert_eq!(
            editor_keys(key_press('x', Modifiers::empty())),
            Some(Binding::Insert('x'))
        );
    }
}

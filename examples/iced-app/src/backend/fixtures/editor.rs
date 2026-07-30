use super::*;

#[cfg(test)]
pub fn slider_number(value: f64) -> SliderNumber {
    value as SliderNumber
}

#[cfg(test)]
pub fn editor_keys(
    event: iced::widget::text_editor::KeyPress,
    readonly: bool,
) -> Option<iced::widget::text_editor::Binding<EditorCommand>> {
    if event.key.to_latin(event.physical_key) == Some('s') && event.modifiers.command() {
        Some(iced::widget::text_editor::Binding::Custom(EditorCommand {
            save: true,
        }))
    } else if readonly {
        None
    } else {
        iced::widget::text_editor::Binding::from_key_press(event)
    }
}

#[cfg(test)]
pub fn editor_highlight<'a, Message: 'a>(
    editor: iced::widget::text_editor::TextEditor<
        'a,
        iced::advanced::text::highlighter::PlainText,
        Message,
    >,
    token: String,
) -> iced::Element<'a, Message> {
    editor
        .highlight_with::<DemoHighlighter>(token, |_, theme| {
            iced::advanced::text::highlighter::Format {
                color: Some(theme.palette().primary),
                font: Some(iced::Font::MONOSPACE),
            }
        })
        .into()
}

#[cfg(test)]
pub fn editor_surface(
    theme: &iced::Theme,
    status: iced::widget::text_editor::Status,
    readonly: bool,
) -> iced::widget::text_editor::Style {
    let mut style = iced::widget::text_editor::default(theme, status);
    if readonly {
        style.value = theme.palette().text;
    }
    style
}

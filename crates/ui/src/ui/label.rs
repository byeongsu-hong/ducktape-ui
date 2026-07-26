use super::theme::Theme;
use iced::font::Weight;
use iced::widget::text::IntoFragment;
use iced::widget::{Text, text};
use iced::{Color, Font};

/// Visible label text for a nearby native control.
pub fn label<'a>(content: impl IntoFragment<'a>, theme: &Theme) -> Text<'a> {
    let style = label_style(theme);
    text(content)
        .size(style.size)
        .font(style.font)
        .color(style.color)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LabelStyle {
    size: f32,
    color: Color,
    font: Font,
}

fn label_style(theme: &Theme) -> LabelStyle {
    LabelStyle {
        size: theme.typography.field_label,
        color: theme.palette.muted_foreground,
        font: Font {
            weight: Weight::Semibold,
            ..Font::MONOSPACE
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn label_uses_semantic_text_tokens() {
        let style = label_style(&LIGHT);
        assert_eq!(style.size, LIGHT.typography.field_label);
        assert_eq!(style.color, LIGHT.palette.muted_foreground);
        assert_eq!(style.font.weight, Weight::Semibold);
    }
}

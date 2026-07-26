use super::input::{InputVariant, style as input_style};
use super::theme::Theme;
use iced::widget::overlay::menu;
use iced::widget::{ComboBox, combo_box};
use iced::{Background, Border, Shadow, Vector};
use std::fmt::Display;

/// A themed native iced combo box with searchable, keyboard-capable state.
///
/// Keep [`iced::widget::combo_box::State`] in the application state. Returning
/// iced's widget preserves its builder methods, including `on_input`,
/// `on_option_hovered`, `on_open`, `on_close`, and sizing controls.
pub fn combobox<'a, T, Message>(
    state: &'a iced::widget::combo_box::State<T>,
    placeholder: &str,
    selection: Option<&T>,
    on_selected: impl Fn(T) -> Message + 'static,
    theme: &Theme,
) -> ComboBox<'a, T, Message>
where
    T: Display + Clone,
{
    let theme = *theme;

    combo_box(state, placeholder, selection, on_selected)
        .padding([8, 12])
        .size(theme.typography.sm)
        .input_style(move |_iced_theme, status| input_style(&theme, InputVariant::Default, status))
        .menu_style(move |_iced_theme| menu_style(&theme))
}

pub fn menu_style(theme: &Theme) -> menu::Style {
    menu::Style {
        background: Background::Color(theme.palette.popover),
        border: Border {
            color: theme.palette.border,
            width: 1.0,
            radius: theme.radius.md.into(),
        },
        text_color: theme.palette.popover_foreground,
        selected_text_color: theme.palette.accent_foreground,
        selected_background: Background::Color(theme.palette.accent),
        shadow: Shadow {
            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.14),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn menu_uses_popover_and_accent_roles() {
        let style = menu_style(&LIGHT);

        assert_eq!(style.background, Background::Color(LIGHT.palette.popover));
        assert_eq!(style.text_color, LIGHT.palette.popover_foreground);
        assert_eq!(
            style.selected_background,
            Background::Color(LIGHT.palette.accent)
        );
        assert_eq!(style.selected_text_color, LIGHT.palette.accent_foreground);
    }
}

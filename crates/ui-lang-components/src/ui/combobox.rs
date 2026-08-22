use super::input::{InputVariant, style as input_style};
use super::theme::{Theme, menu_style};
use iced::widget::{ComboBox, combo_box};
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
        .size(theme.typography.list)
        .input_style(move |_iced_theme, status| input_style(&theme, InputVariant::Default, status))
        .menu_style(move |_iced_theme| menu_style(&theme))
}

use super::theme::Theme;
use iced::widget::overlay::menu;
use iced::widget::{PickList, pick_list};
use iced::{Background, Border, Shadow, Vector};
use std::borrow::Borrow;

/// A themed native iced pick list with controlled options and selection.
///
/// iced 0.14's `PickList` opens and selects through pointer or touch events;
/// it does not expose keyboard focus, opening, or menu navigation. Use the
/// searchable `combobox` component when keyboard selection is required.
/// Returning iced's widget preserves builders such as `placeholder`, `width`,
/// `on_open`, `on_close`, and `menu_height`.
pub fn native_select<'a, T, L, V, Message>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
    theme: &Theme,
) -> PickList<'a, T, L, V, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Message: Clone + 'a,
{
    let theme = *theme;

    pick_list(options, selected, on_selected)
        .padding([8, 12])
        .text_size(theme.typography.sm)
        .style(move |_iced_theme, status| style(&theme, status))
        .menu_style(move |_iced_theme| menu_style(&theme))
}

pub fn style(
    theme: &Theme,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    use iced::widget::pick_list::Status;

    let (border_color, border_width, handle_color) = match status {
        Status::Active => (theme.palette.input, 1.0, theme.palette.muted_foreground),
        Status::Hovered => (theme.palette.foreground, 1.0, theme.palette.foreground),
        Status::Opened { .. } => (theme.palette.ring, 2.0, theme.palette.foreground),
    };

    iced::widget::pick_list::Style {
        text_color: theme.palette.foreground,
        placeholder_color: theme.palette.muted_foreground,
        handle_color,
        background: Background::Color(theme.palette.background),
        border: Border {
            color: border_color,
            width: border_width,
            radius: theme.radius.md.into(),
        },
    }
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
    use super::*;
    use crate::ui::theme::LIGHT;

    #[test]
    fn opened_select_uses_ring_and_accent_roles() {
        let field = style(
            &LIGHT,
            iced::widget::pick_list::Status::Opened { is_hovered: false },
        );
        let menu = menu_style(&LIGHT);

        assert_eq!(field.border.color, LIGHT.palette.ring);
        assert_eq!(field.border.width, 2.0);
        assert_eq!(
            menu.selected_background,
            Background::Color(LIGHT.palette.accent)
        );
        assert_eq!(menu.selected_text_color, LIGHT.palette.accent_foreground);
    }
}

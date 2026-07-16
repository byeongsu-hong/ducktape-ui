use super::theme::Theme;
use iced::widget::scrollable::{self as iced_scrollable, Rail, Scroller, Status};
use iced::widget::{Scrollable, scrollable};
use iced::{Background, Border, Element, Length};

/// Creates a styled native vertical scrollable.
///
/// The returned [`Scrollable`] keeps native builder methods, including
/// [`Scrollable::height`], [`Scrollable::direction`], and
/// [`Scrollable::on_scroll`].
pub fn scroll_area<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Scrollable<'a, Message>
where
    Message: 'a,
{
    let theme = *theme;
    scrollable(content)
        .width(Length::Fill)
        .style(move |_iced_theme, status| style(&theme, status))
}

pub fn style(theme: &Theme, status: Status) -> iced_scrollable::Style {
    let (horizontal_active, vertical_active) = match status {
        Status::Active { .. } => (false, false),
        Status::Hovered {
            is_horizontal_scrollbar_hovered,
            is_vertical_scrollbar_hovered,
            ..
        } => (
            is_horizontal_scrollbar_hovered,
            is_vertical_scrollbar_hovered,
        ),
        Status::Dragged {
            is_horizontal_scrollbar_dragged,
            is_vertical_scrollbar_dragged,
            ..
        } => (
            is_horizontal_scrollbar_dragged,
            is_vertical_scrollbar_dragged,
        ),
    };

    let rail = |active| Rail {
        background: None,
        border: Border::default(),
        scroller: Scroller {
            background: Background::Color(if active {
                theme.palette.muted_foreground
            } else {
                theme.palette.input
            }),
            border: Border {
                radius: 999.0.into(),
                ..Default::default()
            },
        },
    };
    let mut base = iced_scrollable::default(&theme.iced(), status);
    base.container = iced::widget::container::Style::default();
    base.horizontal_rail = rail(horizontal_active);
    base.vertical_rail = rail(vertical_active);
    base.gap = None;
    base
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn only_the_hovered_axis_strengthens() {
        let active = style(
            &LIGHT,
            Status::Active {
                is_horizontal_scrollbar_disabled: false,
                is_vertical_scrollbar_disabled: false,
            },
        );
        let hovered = style(
            &LIGHT,
            Status::Hovered {
                is_horizontal_scrollbar_hovered: false,
                is_vertical_scrollbar_hovered: true,
                is_horizontal_scrollbar_disabled: false,
                is_vertical_scrollbar_disabled: false,
            },
        );
        assert_eq!(
            active.horizontal_rail.scroller.background,
            hovered.horizontal_rail.scroller.background
        );
        assert_ne!(
            active.vertical_rail.scroller.background,
            hovered.vertical_rail.scroller.background
        );
    }

    #[test]
    fn scrollbar_thumbs_clear_non_text_contrast() {
        for theme in [LIGHT, DARK] {
            let idle = style(
                &theme,
                Status::Active {
                    is_horizontal_scrollbar_disabled: false,
                    is_vertical_scrollbar_disabled: false,
                },
            );
            let hovered = style(
                &theme,
                Status::Hovered {
                    is_horizontal_scrollbar_hovered: false,
                    is_vertical_scrollbar_hovered: true,
                    is_horizontal_scrollbar_disabled: false,
                    is_vertical_scrollbar_disabled: false,
                },
            );

            for thumb in [
                idle.vertical_rail.scroller.background,
                hovered.vertical_rail.scroller.background,
            ] {
                let Background::Color(thumb) = thumb else {
                    panic!("scrollbar thumb must be a solid color");
                };
                assert_eq!(thumb.a, 1.0);
                assert!(thumb.relative_contrast(theme.palette.background) >= 3.0);
            }
        }
    }
}

use super::theme::Theme;
use iced::widget::scrollable::{self, AutoScroll, Rail, Scrollable, Scroller};
use iced::widget::{Id, container, scrollable as iced_scrollable};
use iced::{Background, Border, Element, Length, Shadow};

/// A bottom-anchored native transcript viewport.
///
/// The returned `Scrollable` keeps its native `.on_scroll(...)` builder so the
/// caller owns follow-output and unread behavior instead of hidden widget state.
pub fn message_scroller<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    id: impl Into<Id>,
    theme: &Theme,
) -> Scrollable<'a, Message>
where
    Message: 'a,
{
    let theme = *theme;
    iced_scrollable(
        container(content)
            .padding([theme.spacing.lg, theme.spacing.md])
            .width(Length::Fill),
    )
    .id(id)
    .width(Length::Fill)
    .height(Length::Fill)
    .anchor_bottom()
    .style(move |_iced_theme, status| style(&theme, status))
}

pub fn style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let active = matches!(
        status,
        scrollable::Status::Hovered {
            is_horizontal_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged {
            is_horizontal_scrollbar_dragged: true,
            ..
        } | scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        }
    );
    let scroller = Scroller {
        background: Background::Color(if active {
            theme.palette.ring
        } else {
            theme.palette.muted_foreground
        }),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
    };
    let rail = Rail {
        background: Some(Background::Color(theme.palette.muted)),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        scroller,
    };

    scrollable::Style {
        container: iced::widget::container::Style {
            background: Some(Background::Color(theme.palette.background)),
            text_color: Some(theme.palette.foreground),
            ..Default::default()
        },
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: Some(Background::Color(theme.palette.muted)),
        auto_scroll: AutoScroll {
            background: Background::Color(theme.palette.background),
            border: Border {
                color: theme.palette.ring,
                width: 1.0,
                radius: 999.0.into(),
            },
            shadow: Shadow::default(),
            icon: theme.palette.foreground,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn interaction_highlights_the_native_scroll_handle() {
        for theme in [LIGHT, DARK] {
            let normal = style(
                &theme,
                scrollable::Status::Active {
                    is_horizontal_scrollbar_disabled: true,
                    is_vertical_scrollbar_disabled: false,
                },
            );
            let hovered = style(
                &theme,
                scrollable::Status::Hovered {
                    is_horizontal_scrollbar_hovered: false,
                    is_vertical_scrollbar_hovered: true,
                    is_horizontal_scrollbar_disabled: true,
                    is_vertical_scrollbar_disabled: false,
                },
            );

            assert_ne!(
                normal.vertical_rail.scroller.background,
                hovered.vertical_rail.scroller.background
            );
            assert_eq!(
                hovered.vertical_rail.scroller.background,
                Background::Color(theme.palette.ring)
            );
            assert_eq!(
                hovered.container.background,
                Some(Background::Color(theme.palette.background))
            );
            for thumb in [
                normal.vertical_rail.scroller.background,
                hovered.vertical_rail.scroller.background,
            ] {
                let Background::Color(thumb) = thumb else {
                    panic!("message scrollbar thumb must be a solid color");
                };
                assert!(thumb.relative_contrast(theme.palette.muted) >= 3.0);
            }
        }
    }
}

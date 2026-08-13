use super::theme::Theme;
use iced::widget::{Column, Container, Row, container};
use iced::{Background, Border, Element};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonGroupOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Groups caller-owned controls without changing their messages or state.
///
/// Use borderless button variants when the group should provide the only
/// outline. Children remain unclipped so their native focus rings stay visible.
pub fn button_group<'a, Message>(
    children: impl IntoIterator<Item = Element<'a, Message>>,
    orientation: ButtonGroupOrientation,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let content: Element<'a, Message> = match orientation {
        ButtonGroupOrientation::Horizontal => Row::with_children(children).into(),
        ButtonGroupOrientation::Vertical => Column::with_children(children).into(),
    };
    let theme = *theme;

    container(content).style(move |_| style(&theme))
}

pub fn style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.background)),
        border: Border {
            color: theme.palette.input,
            width: 1.0,
            radius: theme.radius.button.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;
    use iced::widget::{button, text};

    #[test]
    fn both_orientations_keep_caller_child_order() {
        for orientation in [
            ButtonGroupOrientation::Horizontal,
            ButtonGroupOrientation::Vertical,
        ] {
            let children: [Element<'_, ()>; 2] = [
                text::<iced::Theme, iced::Renderer>("One").into(),
                button(text("Two")).into(),
            ];
            let expected = children.each_ref().map(|child| child.as_widget().tag());
            let group: Element<'_, ()> = button_group(children, orientation, &LIGHT).into();

            assert!(
                group
                    .as_widget()
                    .children()
                    .into_iter()
                    .map(|child| child.tag)
                    .eq(expected),
                "{orientation:?} reordered children"
            );
        }
    }

    #[test]
    fn group_owns_one_semantic_outline() {
        for theme in [LIGHT, DARK] {
            let style = style(&theme);

            assert_eq!(style.border.color, theme.palette.input);
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius, theme.radius.button.into());
            assert!(
                style
                    .border
                    .color
                    .relative_contrast(theme.palette.background)
                    >= 3.0
            );
        }
    }
}

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
/// outline. Iced clips to rectangular bounds; children keep their own styles.
pub fn button_group<'a, Message>(
    children: impl IntoIterator<Item = Element<'a, Message>>,
    orientation: ButtonGroupOrientation,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let children = children.into_iter().collect::<Vec<_>>();
    let content: Element<'a, Message> = match orientation {
        ButtonGroupOrientation::Horizontal => Row::with_children(children).into(),
        ButtonGroupOrientation::Vertical => Column::with_children(children).into(),
    };
    let theme = *theme;

    container(content).clip(true).style(move |_| style(&theme))
}

pub fn style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.background)),
        border: Border {
            color: theme.palette.border,
            width: 1.0,
            radius: theme.radius.md.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::LIGHT;
    use iced::widget::text;

    #[test]
    fn both_orientations_keep_every_caller_owned_child() {
        for orientation in [
            ButtonGroupOrientation::Horizontal,
            ButtonGroupOrientation::Vertical,
        ] {
            let children = vec![
                text::<iced::Theme, iced::Renderer>("One").into(),
                text("Two").into(),
            ];
            let group: Element<'_, ()> = button_group(children, orientation, &LIGHT).into();

            assert_eq!(group.as_widget().children().len(), 2);
        }
    }

    #[test]
    fn group_owns_one_semantic_outline() {
        let style = style(&LIGHT);

        assert_eq!(style.border.color, LIGHT.palette.border);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius, LIGHT.radius.md.into());
    }
}

use super::theme::Theme;
use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Row, container};
use iced::{Alignment, Element, Length};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MessageSide {
    #[default]
    Incoming,
    Outgoing,
}

/// Composes a message row from caller-owned avatar, header, body, and actions.
pub fn message<'a, Message>(
    side: MessageSide,
    avatar: Option<Element<'a, Message>>,
    header: Option<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    actions: Option<Element<'a, Message>>,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let mut content = Column::new()
        .spacing(theme.spacing.sm)
        .align_x(alignment(side));
    if let Some(header) = header {
        content = content.push(header);
    }
    content = content.push(body);
    if let Some(actions) = actions {
        content = content.push(actions);
    }

    let row = match (avatar_first(side), avatar) {
        (true, Some(avatar)) => Row::new().push(avatar).push(content),
        (false, Some(avatar)) => Row::new().push(content).push(avatar),
        (_, None) => Row::new().push(content),
    }
    .spacing(theme.spacing.sm)
    .align_y(Alignment::End);

    container(row).width(Length::Fill).align_x(alignment(side))
}

fn alignment(side: MessageSide) -> Horizontal {
    match side {
        MessageSide::Incoming => Horizontal::Left,
        MessageSide::Outgoing => Horizontal::Right,
    }
}

fn avatar_first(side: MessageSide) -> bool {
    side == MessageSide::Incoming
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_controls_alignment_and_avatar_order() {
        assert_eq!(alignment(MessageSide::Incoming), Horizontal::Left);
        assert!(avatar_first(MessageSide::Incoming));
        assert_eq!(alignment(MessageSide::Outgoing), Horizontal::Right);
        assert!(!avatar_first(MessageSide::Outgoing));
    }
}

use super::theme::Theme;
use iced::Element;
use iced::widget::Column;

/// A change requested by a caller-owned collapsible trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollapsibleChange {
    Toggle,
    Open,
    Close,
}

/// Computes the next controlled `open` value without storing widget state.
pub const fn next_open(open: bool, change: CollapsibleChange) -> bool {
    match change {
        CollapsibleChange::Toggle => !open,
        CollapsibleChange::Open => true,
        CollapsibleChange::Close => false,
    }
}

/// Composes a caller-owned trigger and content into a controlled disclosure.
///
/// The trigger owns its message and keyboard behavior. Pass the resulting
/// `open` value back on the next view call; this widget stores no hidden state.
pub fn collapsible<'a, Message>(
    open: bool,
    trigger: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Column<'a, Message>
where
    Message: 'a,
{
    let disclosure = Column::new().push(trigger).spacing(theme.spacing.sm);

    if open {
        disclosure.push(content)
    } else {
        disclosure
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;
    use iced::widget::text;

    #[test]
    fn changes_produce_explicit_controlled_state() {
        assert!(next_open(false, CollapsibleChange::Toggle));
        assert!(!next_open(true, CollapsibleChange::Toggle));
        assert!(next_open(false, CollapsibleChange::Open));
        assert!(!next_open(true, CollapsibleChange::Close));
    }

    #[test]
    fn closed_content_is_absent_from_the_widget_tree() {
        let closed: Element<'_, ()> =
            collapsible(false, text("Trigger"), text("Content"), &LIGHT).into();
        let open: Element<'_, ()> =
            collapsible(true, text("Trigger"), text("Content"), &LIGHT).into();

        assert_eq!(closed.as_widget().children().len(), 1);
        assert_eq!(open.as_widget().children().len(), 2);
    }
}

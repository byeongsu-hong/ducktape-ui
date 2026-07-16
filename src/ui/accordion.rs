use super::theme::Theme;
use iced::widget::rule::{FillMode, Style};
use iced::widget::{Column, container, rule};
use iced::{Element, Length, Padding};

/// Controlled accordion state matching shadcn's single and multiple modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccordionState<Id> {
    Single(Option<Id>),
    Multiple(Vec<Id>),
}

impl<Id: Eq> AccordionState<Id> {
    pub fn is_open(&self, id: &Id) -> bool {
        match self {
            Self::Single(open) => open.as_ref() == Some(id),
            Self::Multiple(open) => open.contains(id),
        }
    }
}

impl<Id: Clone + Eq> AccordionState<Id> {
    /// Returns a new controlled state after activating one header.
    #[must_use]
    pub fn toggled(&self, id: Id) -> Self {
        match self {
            Self::Single(open) if open.as_ref() == Some(&id) => Self::Single(None),
            Self::Single(_) => Self::Single(Some(id)),
            Self::Multiple(open) => {
                let mut next = open.clone();
                if self.is_open(&id) {
                    next.retain(|open_id| open_id != &id);
                } else {
                    next.push(id);
                }
                Self::Multiple(next)
            }
        }
    }
}

/// A caller-owned accordion header and its controlled content.
pub struct AccordionItem<'a, Message, Id> {
    id: Id,
    header: Element<'a, Message>,
    content: Element<'a, Message>,
}

pub fn accordion_item<'a, Message, Id>(
    id: Id,
    header: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
) -> AccordionItem<'a, Message, Id>
where
    Message: 'a,
{
    AccordionItem {
        id,
        header: header.into(),
        content: content.into(),
    }
}

/// Composes caller-owned headers and content using explicit open state.
///
/// Header messages should replace the caller's state with
/// `state.toggled(id)`. The trigger must provide its own keyboard behavior;
/// wire it through `focus_control` once that component is installed.
pub fn accordion<'a, Message, Id>(
    items: impl IntoIterator<Item = AccordionItem<'a, Message, Id>>,
    state: &AccordionState<Id>,
    theme: &Theme,
) -> Column<'a, Message>
where
    Message: 'a,
    Id: Eq,
{
    let mut accordion: Column<'a, Message> = Column::new().width(Length::Fill);

    for item in items {
        let mut section = Column::new().width(Length::Fill).push(item.header);
        if state.is_open(&item.id) {
            section = section.push(
                container(item.content)
                    .padding(
                        Padding::default()
                            .horizontal(theme.spacing.lg)
                            .bottom(theme.spacing.lg),
                    )
                    .width(Length::Fill),
            );
        }
        let divider: Element<'a, Message> = divider(theme);
        accordion = accordion.push(section).push(divider);
    }

    accordion
}

fn divider<'a, Message>(theme: &Theme) -> Element<'a, Message>
where
    Message: 'a,
{
    let color = theme.palette.border;
    rule::horizontal::<'a, iced::Theme>(1)
        .style(move |_| Style {
            color,
            radius: 0.0.into(),
            fill_mode: FillMode::Full,
            snap: true,
        })
        .into()
}

/// Keys supported by the accordion header focus-navigation contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccordionKey {
    ArrowUp,
    ArrowDown,
    Home,
    End,
}

/// Returns the header index that should receive focus for a navigation key.
///
/// Arrow navigation wraps at either end. The caller performs focus movement;
/// this stays pure until iced exposes per-button focus IDs.
pub fn header_target(current: usize, count: usize, key: AccordionKey) -> Option<usize> {
    if count == 0 || current >= count {
        return None;
    }

    Some(match key {
        AccordionKey::ArrowUp => current.checked_sub(1).unwrap_or(count - 1),
        AccordionKey::ArrowDown => (current + 1) % count,
        AccordionKey::Home => 0,
        AccordionKey::End => count - 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_and_multiple_modes_toggle_independently() {
        let single = AccordionState::Single(Some("one"));
        assert_eq!(single.toggled("one"), AccordionState::Single(None));
        assert_eq!(single.toggled("two"), AccordionState::Single(Some("two")));

        let multiple = AccordionState::Multiple(vec!["one", "two"]);
        assert_eq!(
            multiple.toggled("one"),
            AccordionState::Multiple(vec!["two"])
        );
        assert_eq!(
            multiple.toggled("three"),
            AccordionState::Multiple(vec!["one", "two", "three"])
        );
    }

    #[test]
    fn header_navigation_wraps_and_supports_home_and_end() {
        assert_eq!(header_target(0, 3, AccordionKey::ArrowUp), Some(2));
        assert_eq!(header_target(2, 3, AccordionKey::ArrowDown), Some(0));
        assert_eq!(header_target(2, 3, AccordionKey::Home), Some(0));
        assert_eq!(header_target(0, 3, AccordionKey::End), Some(2));
        assert_eq!(header_target(0, 0, AccordionKey::Home), None);
        assert_eq!(header_target(3, 3, AccordionKey::ArrowDown), None);
    }
}

use iced::Element;
use iced::Length;
use iced::alignment::Horizontal;
use ui_lang_runtime::{
    AlignContent, AlignItems, Flex, FlexDirection, JustifyContent, flex, flex_item,
};

/// Explicit layout direction for components that cannot inherit DOM-style context.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    LeftToRight,
    RightToLeft,
}

impl Direction {
    pub const fn start(self) -> Horizontal {
        match self {
            Self::LeftToRight => Horizontal::Left,
            Self::RightToLeft => Horizontal::Right,
        }
    }

    pub const fn end(self) -> Horizontal {
        match self {
            Self::LeftToRight => Horizontal::Right,
            Self::RightToLeft => Horizontal::Left,
        }
    }
}

/// Places items in reading order while preserving logical keyboard and semantic order.
pub fn directed_row<'a, Message>(
    items: impl IntoIterator<Item = Element<'a, Message>>,
    direction: Direction,
) -> Flex<'a, Message>
where
    Message: 'a,
{
    let mut width = Length::Shrink;
    let mut height = Length::Shrink;
    let items = items
        .into_iter()
        .filter_map(|item| {
            let size = item.as_widget().size_hint();
            if size.is_void() {
                return None;
            }
            width = width.enclose(size.width);
            height = height.enclose(size.height);
            Some(flex_item(item).shrink(0.0))
        })
        .collect();
    flex(items)
        .width(width)
        .height(height)
        .direction(match direction {
            Direction::LeftToRight => FlexDirection::Row,
            Direction::RightToLeft => FlexDirection::RowReverse,
        })
        .justify_content(JustifyContent::Start)
        .align_items(AlignItems::Start)
        .align_content(AlignContent::Start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_and_end_follow_reading_direction() {
        assert_eq!(Direction::LeftToRight.start(), Horizontal::Left);
        assert_eq!(Direction::LeftToRight.end(), Horizontal::Right);
        assert_eq!(Direction::RightToLeft.start(), Horizontal::Right);
        assert_eq!(Direction::RightToLeft.end(), Horizontal::Left);
    }
}

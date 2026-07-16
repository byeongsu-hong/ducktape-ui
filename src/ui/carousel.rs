use iced::keyboard::{self, key::Named};
use iced::widget::{Column, Row, Space, container};
use iced::{Alignment, Element, Length};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CarouselBoundary {
    #[default]
    Bounded,
    Wrap,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CarouselOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarouselCommand {
    Previous,
    Next,
    First,
    Last,
}

/// Caller-owned carousel position with normalized boundary behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CarouselState {
    index: usize,
    slide_count: usize,
    boundary: CarouselBoundary,
}

impl CarouselState {
    pub const fn new(index: usize, slide_count: usize, boundary: CarouselBoundary) -> Self {
        let index = if slide_count == 0 {
            0
        } else {
            match boundary {
                CarouselBoundary::Bounded => {
                    if index < slide_count {
                        index
                    } else {
                        slide_count - 1
                    }
                }
                CarouselBoundary::Wrap => index % slide_count,
            }
        };

        Self {
            index,
            slide_count,
            boundary,
        }
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn slide_count(self) -> usize {
        self.slide_count
    }

    pub const fn boundary(self) -> CarouselBoundary {
        self.boundary
    }

    pub const fn is_empty(self) -> bool {
        self.slide_count == 0
    }

    pub const fn can_previous(self) -> bool {
        match self.boundary {
            CarouselBoundary::Bounded => self.index > 0,
            CarouselBoundary::Wrap => self.slide_count > 1,
        }
    }

    pub const fn can_next(self) -> bool {
        match self.boundary {
            CarouselBoundary::Bounded => self.slide_count > 0 && self.index < self.slide_count - 1,
            CarouselBoundary::Wrap => self.slide_count > 1,
        }
    }

    #[must_use]
    pub const fn reduce(self, command: CarouselCommand) -> Self {
        let index = match command {
            CarouselCommand::Previous => match self.boundary {
                CarouselBoundary::Bounded => self.index.saturating_sub(1),
                CarouselBoundary::Wrap if self.slide_count == 0 => 0,
                CarouselBoundary::Wrap if self.index == 0 => self.slide_count - 1,
                CarouselBoundary::Wrap => self.index - 1,
            },
            CarouselCommand::Next => match self.boundary {
                CarouselBoundary::Bounded if self.can_next() => self.index + 1,
                CarouselBoundary::Bounded => self.index,
                CarouselBoundary::Wrap if self.slide_count == 0 => 0,
                CarouselBoundary::Wrap if self.index == self.slide_count - 1 => 0,
                CarouselBoundary::Wrap => self.index + 1,
            },
            CarouselCommand::First => 0,
            CarouselCommand::Last => self.slide_count.saturating_sub(1),
        };

        Self {
            index,
            slide_count: self.slide_count,
            boundary: self.boundary,
        }
    }
}

/// Maps a focused carousel's keyboard input to a state command.
///
/// Route keyboard events here only while the carousel owns focus. Horizontal
/// carousels use Left/Right; vertical carousels use Up/Down. Home and End work
/// in both orientations.
pub fn keyboard_command(
    key: &keyboard::Key,
    orientation: CarouselOrientation,
) -> Option<CarouselCommand> {
    match key {
        keyboard::Key::Named(Named::Home) => Some(CarouselCommand::First),
        keyboard::Key::Named(Named::End) => Some(CarouselCommand::Last),
        keyboard::Key::Named(Named::ArrowLeft)
            if orientation == CarouselOrientation::Horizontal =>
        {
            Some(CarouselCommand::Previous)
        }
        keyboard::Key::Named(Named::ArrowRight)
            if orientation == CarouselOrientation::Horizontal =>
        {
            Some(CarouselCommand::Next)
        }
        keyboard::Key::Named(Named::ArrowUp) if orientation == CarouselOrientation::Vertical => {
            Some(CarouselCommand::Previous)
        }
        keyboard::Key::Named(Named::ArrowDown) if orientation == CarouselOrientation::Vertical => {
            Some(CarouselCommand::Next)
        }
        _ => None,
    }
}

/// Composes a clipped active slide with caller-owned previous/next controls.
///
/// Pass native iced buttons as `previous` and `next`, disabling them with
/// [`CarouselState::can_previous`] and [`CarouselState::can_next`]. Only the
/// active slide enters the widget tree, so slide changes have no forced motion.
pub fn carousel<'a, Message>(
    state: CarouselState,
    slides: impl IntoIterator<Item = Element<'a, Message>>,
    previous: impl Into<Element<'a, Message>>,
    next: impl Into<Element<'a, Message>>,
    orientation: CarouselOrientation,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let slide = slides
        .into_iter()
        .nth(state.index())
        .unwrap_or_else(|| Space::new().into());
    let viewport = container(slide).clip(true);

    match orientation {
        CarouselOrientation::Horizontal => Row::new()
            .push(previous)
            .push(viewport.width(Length::Fill))
            .push(next)
            .align_y(Alignment::Center)
            .into(),
        CarouselOrientation::Vertical => Column::new()
            .push(previous)
            .push(viewport.height(Length::Fill))
            .push(next)
            .align_x(Alignment::Center)
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_and_wrapped_navigation_hold_their_edges() {
        let bounded = CarouselState::new(0, 3, CarouselBoundary::Bounded);
        assert_eq!(bounded.reduce(CarouselCommand::Previous).index(), 0);
        assert_eq!(
            bounded
                .reduce(CarouselCommand::Last)
                .reduce(CarouselCommand::Next)
                .index(),
            2
        );
        assert!(!bounded.can_previous());

        let wrapped = CarouselState::new(0, 3, CarouselBoundary::Wrap);
        assert_eq!(wrapped.reduce(CarouselCommand::Previous).index(), 2);
        assert_eq!(
            wrapped
                .reduce(CarouselCommand::Previous)
                .reduce(CarouselCommand::Next)
                .index(),
            0
        );
        assert!(wrapped.can_previous());
    }

    #[test]
    fn count_changes_and_empty_carousels_remain_valid() {
        assert_eq!(
            CarouselState::new(8, 3, CarouselBoundary::Bounded).index(),
            2
        );
        assert_eq!(CarouselState::new(8, 3, CarouselBoundary::Wrap).index(), 2);

        let empty = CarouselState::new(8, 0, CarouselBoundary::Wrap);
        assert_eq!(empty.reduce(CarouselCommand::Next).index(), 0);
        assert_eq!(empty.reduce(CarouselCommand::Last).index(), 0);
        assert!(!empty.can_next());
    }

    #[test]
    fn keyboard_commands_follow_orientation() {
        let left = keyboard::Key::Named(Named::ArrowLeft);
        let down = keyboard::Key::Named(Named::ArrowDown);
        let home = keyboard::Key::Named(Named::Home);

        assert_eq!(
            keyboard_command(&left, CarouselOrientation::Horizontal),
            Some(CarouselCommand::Previous)
        );
        assert_eq!(keyboard_command(&left, CarouselOrientation::Vertical), None);
        assert_eq!(
            keyboard_command(&down, CarouselOrientation::Vertical),
            Some(CarouselCommand::Next)
        );
        assert_eq!(
            keyboard_command(&home, CarouselOrientation::Vertical),
            Some(CarouselCommand::First)
        );
    }

    #[test]
    fn only_the_active_slide_enters_the_viewport() {
        use iced::widget::{Column, button, text};

        let slides = vec![
            Column::new().push(text("one")).into(),
            Column::new()
                .push(text("two"))
                .push(text("selected"))
                .into(),
        ];
        let carousel: Element<'_, ()> = carousel(
            CarouselState::new(1, slides.len(), CarouselBoundary::Bounded),
            slides,
            button("Previous"),
            button("Next"),
            CarouselOrientation::Horizontal,
        );
        let children = carousel.as_widget().children();

        assert_eq!(children.len(), 3);
        assert_eq!(children[1].children.len(), 2);
    }
}

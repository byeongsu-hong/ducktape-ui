use iced::advanced::widget;
use iced::{Rectangle, Task, Vector};

/// Reveal a retained row without moving text-input focus unless requested.
pub(crate) fn reveal_item<Message>(
    results: widget::Id,
    item: widget::Id,
    focus_item: bool,
) -> Task<Message> {
    iced_runtime::task::effect(iced_runtime::Action::widget(RevealItem {
        results,
        item,
        focus_item,
        viewport: None,
        item_bounds: None,
    }))
}

struct RevealItem {
    results: widget::Id,
    item: widget::Id,
    focus_item: bool,
    viewport: Option<(Rectangle, Vector)>,
    item_bounds: Option<Rectangle>,
}

impl<T: 'static> widget::Operation<T> for RevealItem {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn widget::operation::Scrollable,
    ) {
        if id == Some(&self.results) {
            self.viewport = Some((bounds, translation));
        }
    }

    fn focusable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        state: &mut dyn widget::operation::Focusable,
    ) {
        if id == Some(&self.item) {
            self.item_bounds = Some(bounds);
        }

        if self.focus_item {
            if id == Some(&self.item) {
                state.focus();
            } else {
                state.unfocus();
            }
        }
    }

    fn finish(&self) -> widget::operation::Outcome<T> {
        let Some((viewport, translation)) = self.viewport else {
            return widget::operation::Outcome::None;
        };
        let Some(item) = self.item_bounds else {
            return widget::operation::Outcome::None;
        };
        let Some(y) = visibility_delta(viewport, item, translation.y) else {
            return widget::operation::Outcome::None;
        };

        widget::operation::Outcome::Chain(Box::new(widget::operation::scrollable::scroll_by(
            self.results.clone(),
            widget::operation::scrollable::AbsoluteOffset { x: 0.0, y },
        )))
    }
}

pub(crate) fn visibility_delta(viewport: Rectangle, item: Rectangle, scroll_y: f32) -> Option<f32> {
    let top = item.y - scroll_y;
    let bottom = item.y + item.height - scroll_y;

    if top < viewport.y {
        Some(top - viewport.y)
    } else if bottom > viewport.y + viewport.height {
        Some(bottom - viewport.y - viewport.height)
    } else {
        None
    }
}

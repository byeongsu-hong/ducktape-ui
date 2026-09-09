//! Ordered native editor actions, independent of the graphics backend.
use iced::widget::text_editor;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Apply a regular Iced text editor action.
    Edit(text_editor::Action),
    /// Move the cursor while preserving the supplied selection direction.
    MoveTo(text_editor::Cursor),
}

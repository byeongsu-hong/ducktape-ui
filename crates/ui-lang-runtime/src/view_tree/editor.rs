//! The host's widget for a [`wire::Node::Editor`]: iced's `text_editor`
//! over a `Content` the host keeps across frames.
//!
//! `text_editor` borrows its `Content` for the widget's lifetime, and the
//! rendered tree is `'static` — it outlives the lock on the guest it came
//! from. So the widget here holds the shared `Content` and builds the real
//! `TextEditor` afresh inside every `Widget` method, over a lock held for
//! that call alone. The `Tree` state is the `TextEditor`'s own, so focus,
//! drag and click history carry over between calls as they would for a
//! widget built once.

use std::sync::{Arc, Mutex};

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::text_editor::{Content, TextEditor};
use iced::{Element, Event, Length, Rectangle, Size, Vector, widget};
use ui_lang_wire as wire;

use super::Output;

/// A `Content` the host keeps for one editor key, shared with the widget
/// that shows it.
pub(super) type Shared = Arc<Mutex<Content>>;

pub(super) struct HostEditor {
    key: String,
    content: Shared,
    placeholder: String,
    on_edit: Option<u32>,
    width: Option<f32>,
    height: Length,
    min_height: Option<f32>,
    max_height: Option<f32>,
}

impl HostEditor {
    pub(super) fn new(node: &wire::Node, content: Shared) -> Self {
        let wire::Node::Editor {
            key,
            placeholder,
            on_edit,
            width,
            height,
            min_height,
            max_height,
            ..
        } = node
        else {
            unreachable!("built for an editor node")
        };
        Self {
            key: key.clone(),
            content,
            placeholder: placeholder.clone(),
            on_edit: *on_edit,
            width: *width,
            height: height.map_or(Length::Shrink, super::length),
            min_height: *min_height,
            max_height: *max_height,
        }
    }

    /// The `TextEditor` for this call, over the locked content.
    fn build<'a>(&'a self, content: &'a Content) -> TextEditor<'a, PlainText, Output> {
        let mut editor = widget::text_editor(content)
            .id(widget::Id::from(self.key.clone()))
            .placeholder(self.placeholder.as_str())
            .height(self.height);
        if let Some(width) = self.width {
            editor = editor.width(width);
        }
        if let Some(min_height) = self.min_height {
            editor = editor.min_height(min_height);
        }
        if let Some(max_height) = self.max_height {
            editor = editor.max_height(max_height);
        }
        if let Some(handler) = self.on_edit {
            let key = &self.key;
            editor = editor.on_action(move |action| Output::EditorAction {
                key: key.clone(),
                handler,
                action,
            });
        }
        editor
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Content> {
        // A panic while the lock was held leaves the content usable: the
        // widget only ever reads it here and the host writes whole actions.
        self.content
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

type PlainText = iced::advanced::text::highlighter::PlainText;

impl Widget<Output, iced::Theme, iced::Renderer> for HostEditor {
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width.map_or(Length::Fill, Length::Fixed),
            height: self.height,
        }
    }

    fn tag(&self) -> tree::Tag {
        let content = self.lock();
        self.build(&content).tag()
    }

    fn state(&self) -> tree::State {
        let content = self.lock();
        self.build(&content).state()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let content = self.lock();
        self.build(&content).layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let content = self.lock();
        self.build(&content)
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let content = self.lock();
        self.build(&content)
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Output>,
        viewport: &Rectangle,
    ) {
        let content = self.lock();
        self.build(&content).update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let content = self.lock();
        self.build(&content)
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'a>(
        &'a mut self,
        _tree: &'a mut Tree,
        _layout: Layout<'a>,
        _renderer: &iced::Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'a, Output, iced::Theme, iced::Renderer>> {
        // A text editor has no overlay, and one could not outlive the lock.
        None
    }
}

impl From<HostEditor> for Element<'static, Output, iced::Theme, iced::Renderer> {
    fn from(editor: HostEditor) -> Self {
        Element::new(editor)
    }
}

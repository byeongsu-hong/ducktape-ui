//! Native rich composer state; only semantic notices cross the guest boundary.
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::keyboard::{
    Key,
    key::{Code, Named, Physical},
};
use iced::widget::text_editor::{self, Content};
use iced::{Element, Event, Length, Rectangle, Size, Vector, widget};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use ui_lang_runtime::rich_text_editor::{Action, default_key_binding};
use ui_lang_runtime::{ContentVersion, RichTextEditor};
use ui_lang_wire::SurfaceValue as Value;

// A notice carries both the full document and a selection, plus record/field names.
const MAX_DOCUMENT_BYTES: usize = (ui_lang_wire::MAX_STRING_BYTES - 256) / 2;

struct NativeDocument {
    // Keep the document alive while GuestView replaces its rendered Element.
    _lease: Arc<Mutex<Document>>,
    identity: u64,
    generation: u64,
}

struct Document {
    identity: u64,
    content: Content,
    reported: String,
    reset: i64,
    revision: u64,
    generation: u64,
    placeholder: String,
    disabled: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}
struct Composer {
    key: String,
    document: Arc<Mutex<Document>>,
}

pub(super) fn provider() -> ui_lang_runtime::view_tree::Surface {
    static NEXT_DOCUMENT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let documents = Mutex::new(HashMap::<String, Weak<Mutex<Document>>>::new());
    Box::new(move |key, args| {
        let [
            Value::Str(text),
            Value::I64(reset),
            Value::Str(placeholder),
            Value::Bool(disabled),
        ] = args
        else {
            return widget::text("invalid rich_composer arguments").into();
        };
        if text.len() > MAX_DOCUMENT_BYTES {
            return widget::text("rich_composer document exceeds the notice budget").into();
        }
        let mut documents = documents.lock().expect("composer registry");
        documents.retain(|_, document| document.strong_count() > 0);
        let document = documents
            .get(key)
            .and_then(Weak::upgrade)
            .unwrap_or_else(|| {
                let document = Arc::new(Mutex::new(Document {
                    identity: NEXT_DOCUMENT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    content: Content::with_text(text),
                    reported: text.clone(),
                    reset: *reset,
                    revision: 0,
                    generation: 0,
                    placeholder: placeholder.clone(),
                    disabled: *disabled,
                    undo: Vec::new(),
                    redo: Vec::new(),
                }));
                documents.insert(key.into(), Arc::downgrade(&document));
                document
            });
        {
            let mut state = document.lock().expect("composer document");
            if state.reset != *reset || (state.reported != *text && state.content.text() != *text) {
                state.content = Content::with_text(text);
                state.revision += 1;
                state.generation += 1;
                state.undo.clear();
                state.redo.clear();
            }
            state.reported = text.clone();
            state.reset = *reset;
            state.placeholder = placeholder.clone();
            state.disabled = *disabled;
        }
        let identity = document.lock().expect("composer document").identity;
        widget::container(Element::new(Composer {
            key: format!("{identity}/{key}"),
            document,
        }))
        .width(Length::Fill)
        .into()
    })
}

impl Composer {
    fn lock(&self) -> std::sync::MutexGuard<'_, Document> {
        self.document.lock().expect("composer document")
    }
    fn build<'a>(
        &'a self,
        document: &'a Document,
    ) -> RichTextEditor<'a, iced::advanced::text::highlighter::PlainText, Interaction> {
        let editor = RichTextEditor::new(
            &document.content,
            ContentVersion::new(document.identity, document.revision),
        )
        .id(widget::Id::from(self.key.clone()))
        .placeholder(&document.placeholder)
        .width(Length::Fill)
        .focus_enabled(!document.disabled)
        .min_height(80.0)
        .max_height(240.0)
        .key_binding(|press| {
            if matches!(press.key, Key::Named(Named::Enter)) && press.modifiers.shift() {
                Some(text_editor::Binding::Custom(text_editor::Edit::Paste(
                    Arc::new("\n".into()),
                )))
            } else {
                default_key_binding(press)
            }
        });
        if document.disabled {
            editor
        } else {
            editor
                .on_action(|action| {
                    if matches!(
                        action,
                        Action::Edit(text_editor::Action::Edit(text_editor::Edit::Enter))
                    ) {
                        Interaction::Submit
                    } else {
                        Interaction::Apply(action)
                    }
                })
                .on_chord(|press| {
                    if !press.modifiers.command() {
                        return None;
                    }
                    match press.physical_key {
                        Physical::Code(Code::KeyZ) => Some(if press.modifiers.shift() {
                            Interaction::Redo
                        } else {
                            Interaction::Undo
                        }),
                        Physical::Code(Code::KeyB) if !press.modifiers.shift() => {
                            Some(Interaction::Mark("**", "**"))
                        }
                        Physical::Code(Code::KeyI) if !press.modifiers.shift() => {
                            Some(Interaction::Mark("_", "_"))
                        }
                        Physical::Code(Code::KeyC) if press.modifiers.shift() => {
                            Some(Interaction::Mark("```\n", "\n```"))
                        }
                        Physical::Code(Code::Digit9) if press.modifiers.shift() => {
                            Some(Interaction::Mark("> ", ""))
                        }
                        _ => None,
                    }
                })
        }
    }
}

impl Widget<Value, iced::Theme, iced::Renderer> for Composer {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Shrink,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<NativeDocument>()
    }
    fn state(&self) -> tree::State {
        let document = self.lock();
        tree::State::new(NativeDocument {
            _lease: self.document.clone(),
            identity: document.identity,
            generation: document.generation,
        })
    }
    fn children(&self) -> Vec<Tree> {
        let document = self.lock();
        vec![Tree::new(
            &self.build(&document) as &dyn Widget<Interaction, iced::Theme, iced::Renderer>
        )]
    }
    fn diff(&self, tree: &mut Tree) {
        let document = self.lock();
        let state = tree.state.downcast_mut::<NativeDocument>();
        let editor = self.build(&document);
        let editor = &editor as &dyn Widget<Interaction, iced::Theme, iced::Renderer>;
        if state.identity != document.identity || state.generation != document.generation {
            *state = NativeDocument {
                _lease: self.document.clone(),
                identity: document.identity,
                generation: document.generation,
            };
            tree.children = vec![Tree::new(editor)];
        } else {
            tree.diff_children(&[editor]);
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let content = self.lock();
        self.build(&content)
            .layout(&mut tree.children[0], renderer, limits)
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
        self.build(&content).draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
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
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Value>,
        viewport: &Rectangle,
    ) {
        let mut content = self.lock();
        let mut interactions = Vec::new();
        let mut local = Shell::new(&mut interactions);
        self.build(&content).update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut local,
            viewport,
        );
        if local.is_event_captured() {
            shell.capture_event();
        }
        if local.is_layout_invalid() {
            shell.invalidate_layout();
        }
        if local.are_widgets_invalid() {
            shell.invalidate_widgets();
        }
        match local.redraw_request() {
            iced::window::RedrawRequest::NextFrame => shell.request_redraw(),
            iced::window::RedrawRequest::At(at) => shell.request_redraw_at(at),
            iced::window::RedrawRequest::Wait => {}
        }
        shell.input_method_mut().merge(local.input_method());
        for interaction in interactions {
            if content.disabled {
                break;
            }
            let submitted = matches!(interaction, Interaction::Submit);
            content.apply(interaction);
            shell.publish(content.notice(submitted));
            shell.invalidate_layout();
            shell.request_redraw();
        }
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
        Widget::mouse_interaction(
            &self.build(&content),
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'a>(
        &'a mut self,
        _tree: &'a mut Tree,
        _layout: Layout<'a>,
        _renderer: &iced::Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'a, Value, iced::Theme, iced::Renderer>> {
        // A text editor has no overlay, and one could not outlive the lock.
        None
    }
}

#[derive(Clone)]
struct Snapshot {
    text: String,
    cursor: text_editor::Cursor,
}
#[derive(Clone)]
enum Interaction {
    Apply(Action),
    Submit,
    Undo,
    Redo,
    Mark(&'static str, &'static str),
}

impl Document {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.content.text(),
            cursor: self.content.cursor(),
        }
    }
    fn restore(&mut self, snapshot: Snapshot) {
        self.content = Content::with_text(&snapshot.text);
        self.content.move_to(snapshot.cursor);
    }
    fn apply(&mut self, interaction: Interaction) {
        let before = self.snapshot();
        let historical = matches!(interaction, Interaction::Undo | Interaction::Redo);
        match interaction {
            Interaction::Submit => return,
            Interaction::Undo => {
                if let Some(snapshot) = self.undo.pop() {
                    self.redo.push(before.clone());
                    self.restore(snapshot);
                }
            }
            Interaction::Redo => {
                if let Some(snapshot) = self.redo.pop() {
                    self.undo.push(before.clone());
                    self.restore(snapshot);
                }
            }
            Interaction::Apply(Action::Edit(action)) => self.content.perform(action),
            Interaction::Apply(Action::MoveTo(cursor)) => {
                if cursor.selection.is_none() && self.content.cursor().selection.is_some() {
                    self.content
                        .perform(text_editor::Action::Move(text_editor::Motion::Left));
                }
                self.content.move_to(cursor);
            }
            Interaction::Mark(open, close) => {
                let selected = self.content.selection().unwrap_or_default();
                self.content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        Arc::new(format!("{open}{selected}{close}")),
                    )));
                if selected.is_empty() {
                    for _ in close.chars() {
                        self.content
                            .perform(text_editor::Action::Move(text_editor::Motion::Left));
                    }
                }
            }
        }
        let text = self.content.text();
        if text.len() > MAX_DOCUMENT_BYTES {
            // Reject the whole edit; clipping can corrupt a selected range or IME commit.
            self.restore(before);
            return;
        }
        if text != before.text {
            self.revision += 1;
            if !historical {
                self.undo.push(before);
                self.redo.clear();
            }
            while self.undo.len() + self.redo.len() > 128
                || self
                    .undo
                    .iter()
                    .chain(&self.redo)
                    .map(|s| s.text.len())
                    .sum::<usize>()
                    > 1024 * 1024
            {
                if !self.undo.is_empty() {
                    self.undo.remove(0);
                } else {
                    self.redo.remove(0);
                }
            }
        }
    }
    fn notice(&self, submitted: bool) -> Value {
        let cursor = self.content.cursor();
        let anchor = cursor.selection.unwrap_or(cursor.position);
        Value::Record {
            name: "ComposerNotice".into(),
            fields: vec![
                ("text".into(), Value::Str(self.content.text())),
                (
                    "selected".into(),
                    Value::Str(self.content.selection().unwrap_or_default()),
                ),
                ("submitted".into(), Value::Bool(submitted)),
                ("line".into(), Value::I64(cursor.position.line as i64)),
                ("column".into(), Value::I64(cursor.position.column as i64)),
                ("anchor_line".into(), Value::I64(anchor.line as i64)),
                ("anchor_column".into(), Value::I64(anchor.column as i64)),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lease(tree: &Tree) -> Option<Weak<Mutex<Document>>> {
        if tree.tag == tree::Tag::of::<NativeDocument>() {
            return Some(Arc::downgrade(
                &tree.state.downcast_ref::<NativeDocument>()._lease,
            ));
        }
        tree.children.iter().find_map(lease)
    }
    #[test]
    fn composer_tree_owns_its_document_until_unmounted() {
        let provider = provider();
        let args = [
            Value::Str("draft".into()),
            Value::I64(0),
            Value::Str("hint".into()),
            Value::Bool(false),
        ];
        let element = provider("editor", &args);
        let tree = Tree::new(element.as_widget());
        let first = lease(&tree).unwrap();
        drop(element);
        assert!(
            first.upgrade().is_some(),
            "the native Tree must retain the document during Element replacement"
        );
        let replacement = provider("editor", &args);
        let next_tree = Tree::new(replacement.as_widget());
        assert!(
            first.ptr_eq(&lease(&next_tree).unwrap()),
            "a frame rebuild must reuse the mounted document"
        );
        drop(tree);
        drop(replacement);
        drop(next_tree);
        assert!(
            first.upgrade().is_none(),
            "an unmounted view must release its document despite the live registry"
        );
        let remounted = provider("editor", &args);
        let tree = Tree::new(remounted.as_widget());
        assert!(
            !first.ptr_eq(&lease(&tree).unwrap()),
            "a later mount owns a fresh native document"
        );
    }
}

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

#[cfg(any(feature = "tiny-skia", feature = "wgpu"))]
use std::sync::OnceLock;
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
    instance: u64,
    options: Box<wire::EditorOptions>,
    transactions: Option<super::editor_transactions::Shared>,
    status: widget::text_editor::Status,
    key: String,
    content: Shared,
    placeholder: String,
    editable: bool,
    document: String,
    reset: u64,
    width: Option<f32>,
    height: Length,
    min_height: Option<f32>,
    max_height: Option<f32>,
    reference: wire::editor_document::EditorDocumentRef,
    #[cfg(any(feature = "tiny-skia", feature = "wgpu"))]
    presented: OnceLock<Arc<[crate::rich_text_editor::PresentedLine]>>,
    #[cfg(any(feature = "tiny-skia", feature = "wgpu"))]
    valid_presentation: OnceLock<bool>,
}

impl HostEditor {
    pub(super) fn new(
        node: &wire::Node,
        content: Shared,
        transactions: Option<super::editor_transactions::Shared>,
        instance: u64,
    ) -> Self {
        let wire::Node::Editor {
            options,
            key,
            placeholder,
            editable,
            document,
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
            instance,
            transactions,
            options: options.clone(),
            status: if *editable {
                widget::text_editor::Status::Active
            } else {
                widget::text_editor::Status::Disabled
            },
            key: key.clone(),
            content,
            placeholder: placeholder.clone(),
            editable: *editable,
            document: document.document.clone(),
            reset: document.reset,
            width: *width,
            height: height.map_or(Length::Shrink, super::length),
            min_height: *min_height,
            max_height: *max_height,
            reference: document.clone(),
            #[cfg(any(feature = "tiny-skia", feature = "wgpu"))]
            presented: OnceLock::new(),
            #[cfg(any(feature = "tiny-skia", feature = "wgpu"))]
            valid_presentation: OnceLock::new(),
        }
    }

    #[cfg(any(feature = "tiny-skia", feature = "wgpu"))]
    fn build<'a>(&'a self, content: &'a Content) -> Element<'a, Output> {
        let Some(presentation) = &self.options.presentation else {
            return self.build_plain(content).into();
        };
        use crate::rich_text_editor::{ContentVersion, PresentationHighlighter, RichTextEditor};
        use std::hash::{Hash, Hasher};
        let (reset, revision, settings, current) = if let Some(shared) = &self.transactions {
            let control = super::lock(shared);
            let current = control.available
                && control.loaded
                && control.reset == self.reference.reset
                && control.text_revision == self.reference.text_revision
                && control.revision == self.reference.revision
                && control.cursor == self.reference.cursor
                && control.last_text.len() == self.reference.byte_len as usize;
            let current = current
                && *self
                    .valid_presentation
                    .get_or_init(|| presentation.validate(&control.last_text).is_ok());
            let settings = if current {
                self.presented
                    .get_or_init(|| {
                        super::editor_presentation::lines(presentation, &control.last_text)
                            .unwrap_or_else(|| Arc::from([]))
                    })
                    .clone()
            } else {
                Arc::from([])
            };
            (control.reset, control.text_revision, settings, current)
        } else {
            (
                self.reference.reset,
                self.reference.text_revision,
                self.presented
                    .get_or_init(|| {
                        super::editor_presentation::lines(presentation, &content.text())
                            .unwrap_or_else(|| Arc::from([]))
                    })
                    .clone(),
                *self
                    .valid_presentation
                    .get_or_init(|| presentation.validate(&content.text()).is_ok()),
            )
        };
        // Layout identity only: acceptance uses the exact document reference above.
        let mut identity = std::collections::hash_map::DefaultHasher::new();
        self.document.hash(&mut identity);
        reset.hash(&mut identity);
        let mut editor =
            RichTextEditor::new(content, ContentVersion::new(identity.finish(), revision))
                .id(widget::Id::from(self.key.clone()))
                .placeholder(self.placeholder.as_str())
                .height(self.height)
                .focus_enabled(self.editable)
                .style(move |theme, _| native_style(self.options.style, theme, self.status))
                .highlight_with::<PresentationHighlighter>(settings, 0, |format| *format);
        if let Some(size) = self.options.size {
            editor = editor.size(size);
        }
        if let Some(padding) = presentation.padding {
            editor = editor.padding(super::padding(padding));
        } else if let Some(padding) = self.options.padding {
            editor = editor.padding(padding);
        }
        if let Some(line_height) = self.options.line_height {
            editor = editor.line_height(match line_height {
                wire::LineHeight::Relative(value) => widget::text::LineHeight::Relative(value),
                wire::LineHeight::Absolute(value) => {
                    widget::text::LineHeight::Absolute(value.into())
                }
            });
        }
        if let Some(wrapping) = self.options.wrapping {
            editor = editor.wrapping(match wrapping {
                wire::Wrapping::None => widget::text::Wrapping::None,
                wire::Wrapping::Glyph => widget::text::Wrapping::Glyph,
                wire::Wrapping::Word => widget::text::Wrapping::Word,
                wire::Wrapping::WordOrGlyph => widget::text::Wrapping::WordOrGlyph,
            });
        }
        if let Some(font) = &self.options.font {
            editor = editor.font(super::text::named_font(font));
        }
        if let Some(width) = self.width {
            editor = editor.width(width);
        }
        if let Some(height) = self.min_height {
            editor = editor.min_height(height);
        }
        if let Some(height) = self.max_height {
            editor = editor.max_height(height);
        }
        if current && self.transactions.is_some() {
            use crate::rich_text_editor as native;
            use wire::editor_presentation::{
                EditorGutterButton, EditorInteraction, EditorMenuAnchor,
            };
            let affordances = &presentation.affordances;
            let output = move |action| Output::EditorInteraction {
                reference: self.reference.clone(),
                action,
            };
            editor = editor
                .on_line_press(move |_, position| {
                    presentation
                        .affordances
                        .hit(wire::EditorPosition {
                            line: position.line as u32,
                            column: position.column as u32,
                        })
                        .map(output)
                })
                .mouse_interaction(move |_, position| {
                    if presentation
                        .affordances
                        .hit(wire::EditorPosition {
                            line: position.line as u32,
                            column: position.column as u32,
                        })
                        .is_some()
                    {
                        mouse::Interaction::Pointer
                    } else {
                        mouse::Interaction::Text
                    }
                })
                .margin_marks(
                    affordances
                        .margins
                        .iter()
                        .map(|mark| (mark.line as usize, mark.count as usize))
                        .collect(),
                    move |line| output(EditorInteraction::Margin { line: line as u32 }),
                )
                .margin_label(affordances.margin_label.clone());
            if self.editable {
                editor = editor
                    .on_gutter(move |line, button| {
                        let gutter = affordances
                            .gutters
                            .iter()
                            .find(|gutter| gutter.line as usize == line)?;
                        let button = match button {
                            native::GutterButton::Plus if gutter.plus => EditorGutterButton::Plus,
                            native::GutterButton::Handle if gutter.handle => {
                                EditorGutterButton::Handle
                            }
                            _ => return None,
                        };
                        Some(output(EditorInteraction::Gutter {
                            line: line as u32,
                            button,
                        }))
                    })
                    .on_gutter_drop(
                        affordances
                            .drop_boundaries
                            .iter()
                            .map(|line| *line as usize)
                            .collect(),
                        move |from, boundary| {
                            Some(output(EditorInteraction::GutterDrop {
                                from: from as u32,
                                boundary: boundary as u32,
                            }))
                        },
                    )
                    .menu(affordances.menu.as_ref().map(|menu| {
                        native::EditorMenu {
                            anchor: match menu.anchor {
                                EditorMenuAnchor::Caret => native::MenuAnchor::Caret,
                                EditorMenuAnchor::Line(line) => {
                                    native::MenuAnchor::Line(line as usize)
                                }
                            },
                            items: menu
                                .items
                                .iter()
                                .map(|item| native::MenuItem {
                                    tag: item.tag.clone(),
                                    label: item.label.clone(),
                                })
                                .collect(),
                            selected: menu.selected as usize,
                        }
                    }))
                    .on_menu(move |event| {
                        output(match event {
                            native::MenuEvent::Select(index) => EditorInteraction::MenuSelect {
                                index: index as u32,
                            },
                            native::MenuEvent::Pick(tag) => EditorInteraction::MenuPick { tag },
                            native::MenuEvent::Dismiss => EditorInteraction::MenuDismiss,
                        })
                    });
            }
        }
        if self.editable {
            editor = editor.on_action(move |action| Output::RichEditorAction {
                reset: self.reset,
                key: self.key.clone(),
                action,
            });
            if let (Some(binding), Some(shared)) = (&self.options.binding, &self.transactions) {
                let claimed = move |press: &widget::text_editor::KeyPress| {
                    let control = super::lock(shared);
                    !control.bypass_claim
                        && !control.composing
                        && matches!(press.status, widget::text_editor::Status::Focused { .. })
                        && binding.claims.iter().any(|claim| {
                            claim.matches(&key_state(press), cfg!(target_os = "macos"))
                        })
                };
                editor = editor
                    .key_binding(move |press| {
                        if claimed(press) {
                            None
                        } else {
                            crate::rich_text_editor::default_key_binding(press)
                        }
                    })
                    .on_chord(move |press| {
                        claimed(press).then(|| Output::EditorClaim {
                            key: self.key.clone(),
                            state: key_state(press),
                        })
                    });
            }
        }
        editor.into()
    }

    #[cfg(not(any(feature = "tiny-skia", feature = "wgpu")))]
    fn build<'a>(&'a self, content: &'a Content) -> Element<'a, Output> {
        assert!(
            self.options.presentation.is_none(),
            "editor presentation requires a graphics renderer"
        );
        self.build_plain(content).into()
    }

    /// The `TextEditor` for this call, over the locked content.
    fn build_plain<'a>(&'a self, content: &'a Content) -> TextEditor<'a, PlainText, Output> {
        let mut editor = widget::text_editor(content)
            .id(widget::Id::from(self.key.clone()))
            .placeholder(self.placeholder.as_str())
            .height(self.height)
            .style(move |theme, _| native_style(self.options.style, theme, self.status));
        if let Some(size) = self.options.size {
            editor = editor.size(size);
        }
        if let Some(padding) = self.options.padding {
            editor = editor.padding(padding);
        }
        if let Some(line_height) = self.options.line_height {
            editor = editor.line_height(match line_height {
                wire::LineHeight::Relative(value) => widget::text::LineHeight::Relative(value),
                wire::LineHeight::Absolute(value) => {
                    widget::text::LineHeight::Absolute(value.into())
                }
            });
        }
        if let Some(wrapping) = self.options.wrapping {
            editor = editor.wrapping(match wrapping {
                wire::Wrapping::None => widget::text::Wrapping::None,
                wire::Wrapping::Glyph => widget::text::Wrapping::Glyph,
                wire::Wrapping::Word => widget::text::Wrapping::Word,
                wire::Wrapping::WordOrGlyph => widget::text::Wrapping::WordOrGlyph,
            });
        }
        if let Some(font) = &self.options.font {
            editor = editor.font(super::text::named_font(font));
        }
        if let Some(width) = self.width {
            editor = editor.width(width);
        }
        if let Some(min_height) = self.min_height {
            editor = editor.min_height(min_height);
        }
        if let Some(max_height) = self.max_height {
            editor = editor.max_height(max_height);
        }
        if let (Some(binding), Some(shared)) = (&self.options.binding, &self.transactions) {
            let key = &self.key;
            editor = editor.key_binding(move |press| {
                let control = super::lock(shared);
                let native_event = iced::keyboard::Event::KeyPressed {
                    key: press.key.clone(),
                    modified_key: press.modified_key.clone(),
                    physical_key: press.physical_key,
                    modifiers: press.modifiers,
                    text: press.text.clone(),
                    location: iced::keyboard::Location::Standard,
                    repeat: false,
                };
                let wire::keyboard::Event::Press { state, .. } = native_event.into() else {
                    unreachable!()
                };
                if !control.bypass_claim
                    && !control.composing
                    && matches!(press.status, widget::text_editor::Status::Focused { .. })
                    && binding
                        .claims
                        .iter()
                        .any(|claim| claim.matches(&state, cfg!(target_os = "macos")))
                {
                    Some(widget::text_editor::Binding::Custom(Output::EditorClaim {
                        key: key.clone(),
                        state,
                    }))
                } else {
                    widget::text_editor::Binding::from_key_press(press)
                }
            });
        }
        if self.editable {
            let key = &self.key;
            editor = editor.on_action(move |action| Output::EditorAction {
                reset: self.reset,
                key: key.clone(),
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

#[cfg(any(feature = "tiny-skia", feature = "wgpu"))]
fn key_state(press: &widget::text_editor::KeyPress) -> wire::keyboard::KeyState {
    let event = iced::keyboard::Event::KeyPressed {
        key: press.key.clone(),
        modified_key: press.modified_key.clone(),
        physical_key: press.physical_key,
        modifiers: press.modifiers,
        text: press.text.clone(),
        location: iced::keyboard::Location::Standard,
        repeat: false,
    };
    let wire::keyboard::Event::Press { state, .. } = event.into() else {
        unreachable!()
    };
    state
}

fn native_style(
    style: wire::InputStyle,
    theme: &iced::Theme,
    status: widget::text_editor::Status,
) -> widget::text_editor::Style {
    use widget::text_editor::{self, Status};
    let mut out = text_editor::default(theme, status);
    let apply = |face: wire::InputFace, out: &mut text_editor::Style| {
        if let Some(value) = face.background {
            out.background = super::color(value).into();
        }
        if let Some(value) = face.border {
            super::apply_border(value, &mut out.border);
        }
        if let Some(value) = face.value {
            out.value = super::color(value);
        }
        if let Some(value) = face.placeholder {
            out.placeholder = super::color(value);
        }
        if let Some(value) = face.selection {
            out.selection = super::color(value);
        }
    };
    apply(style.utility, &mut out);
    apply(style.active, &mut out);
    if matches!(status, Status::Focused { .. })
        && let Some(value) = style.focus_border
    {
        out.border.color = super::color(value);
    }
    let face = match status {
        Status::Active => None,
        Status::Hovered => style.hovered,
        Status::Focused { .. } => style.focused,
        Status::Disabled => style.disabled,
    };
    if let Some(face) = face {
        apply(face, &mut out);
    }
    if matches!(status, Status::Focused { is_hovered: true })
        && let Some(face) = style.focused_hovered
    {
        apply(face, &mut out);
    }
    out
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
        self.build(&content).as_widget().tag()
    }

    fn state(&self) -> tree::State {
        let content = self.lock();
        self.build(&content).as_widget().state()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let content = self.lock();
        self.build(&content)
            .as_widget_mut()
            .layout(tree, renderer, limits)
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
            .as_widget()
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
            .as_widget_mut()
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
        // TextEditor stores its last paint status on the widget, not in Tree.
        // Rebuilding it per call would otherwise paint Active forever.
        struct Focus(bool);
        impl Operation for Focus {
            fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
                visit(self);
            }
            fn focusable(
                &mut self,
                _: Option<&widget::Id>,
                _: Rectangle,
                state: &mut dyn iced::advanced::widget::operation::Focusable,
            ) {
                self.0 = state.is_focused();
            }
        }
        let focused = {
            let content = self.lock();
            let mut native = self.build(&content);
            let mut focus = Focus(false);
            native
                .as_widget_mut()
                .operate(tree, layout, renderer, &mut focus);
            focus.0
        };
        let mut replay = None;
        let navigation =
            self.options.presentation.as_ref().is_some_and(|p| {
                !p.affordances.hits.is_empty() || !p.affordances.margins.is_empty()
            });
        if (self.editable || navigation)
            && let Some(shared) = &self.transactions
        {
            let mut control = super::lock(shared);
            if !control.available || control.reset != self.reset {
                return;
            }
            let now = control.now_ms();
            control.lane.check_deadline(now);
            if let super::editor_transactions::Phase::Decision { since_ms, .. } =
                control.lane.phase()
            {
                shell.request_redraw_at(
                    control.started
                        + std::time::Duration::from_millis(since_ms.saturating_add(5_000)),
                );
            }
            if matches!(
                control.lane.phase(),
                super::editor_transactions::Phase::Faulted(_)
            ) && !control.fault_reported
            {
                shell.publish(Output::EditorLaneFault {
                    document: self.document.clone(),
                });
            }
            let has_menu = self.editable
                && self
                    .options
                    .presentation
                    .as_ref()
                    .is_some_and(|presentation| presentation.affordances.menu.is_some());
            let relevant = match event {
                Event::Keyboard(
                    iced::keyboard::Event::KeyPressed { .. }
                    | iced::keyboard::Event::KeyReleased { .. }
                    | iced::keyboard::Event::ModifiersChanged(_),
                ) => focused,
                Event::InputMethod(_) => focused,
                Event::Mouse(
                    iced::mouse::Event::ButtonPressed(_) | iced::mouse::Event::ButtonReleased(_),
                ) => cursor.is_over(layout.bounds()) || focused || has_menu,
                Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                    control.dragging
                        || (focused && control.lane.front().is_some())
                        || (has_menu && cursor.is_over(layout.bounds()))
                }
                Event::Window(iced::window::Event::Unfocused) => has_menu,
                Event::Mouse(iced::mouse::Event::WheelScrolled { .. }) => {
                    cursor.is_over(layout.bounds()) && control.lane.front().is_some()
                }
                _ => false,
            };
            if relevant {
                let captured_clipboard = if matches!(event, Event::Keyboard(iced::keyboard::Event::KeyPressed { key: iced::keyboard::Key::Character(key), modifiers, .. }) if key.eq_ignore_ascii_case("v") && modifiers.command())
                {
                    clipboard.read(iced::advanced::clipboard::Kind::Standard)
                } else {
                    None
                };
                let bytes = captured_clipboard.as_ref().map_or(0, String::len)
                    + match event {
                        Event::Keyboard(iced::keyboard::Event::KeyPressed { text, .. }) => {
                            text.as_ref().map_or(0, |s| s.len())
                        }
                        Event::InputMethod(
                            iced::advanced::input_method::Event::Commit(text)
                            | iced::advanced::input_method::Event::Preedit(text, _),
                        ) => text.len(),
                        _ => 0,
                    };
                if let Ok(previous) = control.sequences.fetch_update(
                    std::sync::atomic::Ordering::Relaxed,
                    std::sync::atomic::Ordering::Relaxed,
                    |next| next.checked_add(1),
                ) {
                    let sequence = previous + 1;
                    let input = super::editor_transactions::NativeInput {
                        key: self.key.clone(),
                        work: super::editor_transactions::NativeWork::Event(event.clone()),
                        cursor,
                        clipboard: captured_clipboard,
                        time_ms: control.now_ms(),
                    };
                    control.next_sequence = sequence;
                    if control.lane.admit(sequence, bytes, input).is_err() {
                        control.fault_key = Some(self.key.clone());
                        shell.publish(Output::EditorLaneFault {
                            document: self.document.clone(),
                        });
                    }
                } else {
                    control.lane.fail(super::editor_transactions::Fault::Limit);
                    control.fault_key = Some(self.key.clone());
                    shell.publish(Output::EditorLaneFault {
                        document: self.document.clone(),
                    });
                }
                // Outside clicks still order the editor's blur, but belong to
                // the clicked sibling (for example a Replace document button).
                if !matches!(event, Event::Mouse(_))
                    || cursor.is_over(layout.bounds())
                    || control.dragging
                {
                    shell.capture_event();
                }
            }
            if control.lane.phase() == super::editor_transactions::Phase::Ready
                && let Some(front) = control.lane.front()
                && front.input.key == self.key
            {
                replay = Some((front.sequence, front.input.clone()));
            } else if relevant {
                shell.request_redraw();
                return;
            }
        }
        if let Some((sequence, input)) = &replay {
            let actions = match &input.work {
                super::editor_transactions::NativeWork::Event(_) => None,
                super::editor_transactions::NativeWork::Actions(actions) => Some(
                    actions
                        .iter()
                        .cloned()
                        .map(crate::editor_action::Action::Edit)
                        .collect(),
                ),
                super::editor_transactions::NativeWork::RichActions(actions) => {
                    Some(actions.clone())
                }
                super::editor_transactions::NativeWork::Caret(_) => Some(Vec::new()),
            };
            if let Some(actions) = actions {
                shell.publish(Output::EditorBatch(super::editor_transactions::Batch {
                    instance: self.instance,
                    document: self.document.clone(),
                    key: self.key.clone(),
                    sequence: *sequence,
                    reset: self.reset,
                    actions,
                    request: None,
                    presentation_reference: None,
                }));
                shell.request_redraw();
                return;
            }
        }
        let effective_event = replay
            .as_ref()
            .map_or(event, |(_, input)| match &input.work {
                super::editor_transactions::NativeWork::Event(event) => event,
                _ => unreachable!("direct editor work is replayed above"),
            });
        let effective_cursor = replay.as_ref().map_or(cursor, |(_, input)| input.cursor);
        if let Some(shared) = &self.transactions {
            let mut control = super::lock(shared);
            match effective_event {
                Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                    control.dragging = effective_cursor.is_over(layout.bounds())
                }
                Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                    control.dragging = false
                }
                _ => {}
            }
            if let Event::InputMethod(ime) = effective_event {
                match ime {
                    iced::advanced::input_method::Event::Preedit(text, _) => {
                        control.composing = !text.is_empty()
                    }
                    iced::advanced::input_method::Event::Commit(_)
                    | iced::advanced::input_method::Event::Closed => control.composing = false,
                    _ => {}
                }
            }
        }

        let status = {
            let content = self.lock();
            let mut editor = self.build(&content);
            if self.transactions.is_some() {
                // A guest patch may invalidate native shaping between frames.
                // Reuse native layout before any caret/IME query or replay.
                let _ = editor.as_widget_mut().layout(
                    tree,
                    renderer,
                    &layout::Limits::new(Size::ZERO, layout.bounds().size()),
                );
            }
            let mut actions = Vec::new();
            let mut local = Shell::new(&mut actions);
            struct FrozenClipboard<'a> {
                native: &'a mut dyn Clipboard,
                frozen: bool,
                text: Option<String>,
            }
            impl Clipboard for FrozenClipboard<'_> {
                fn read(&self, kind: iced::advanced::clipboard::Kind) -> Option<String> {
                    if self.frozen && kind == iced::advanced::clipboard::Kind::Standard {
                        self.text.clone()
                    } else {
                        self.native.read(kind)
                    }
                }
                fn write(&mut self, kind: iced::advanced::clipboard::Kind, contents: String) {
                    self.native.write(kind, contents);
                }
            }
            let mut captured = FrozenClipboard {
                native: clipboard,
                frozen: replay.is_some()
                    && matches!(effective_event, Event::Keyboard(iced::keyboard::Event::KeyPressed { key: iced::keyboard::Key::Character(key), modifiers, .. }) if key.eq_ignore_ascii_case("v") && modifiers.command()),
                text: replay
                    .as_ref()
                    .and_then(|(_, input)| input.clipboard.clone()),
            };
            editor.as_widget_mut().update(
                tree,
                effective_event,
                layout,
                effective_cursor,
                renderer,
                &mut captured,
                &mut local,
                viewport,
            );
            // Hosts apply these actions immediately, including inside overlays.
            // Reflow the changed Content before the next batched caret/IME query.
            if !local.is_empty() {
                local.invalidate_layout();
            }
            if let Some((sequence, input)) = &replay {
                // Capture was decided when this input was admitted. Replaying
                // it must not consume the unrelated event driving this update.
                if local.is_layout_invalid() {
                    shell.invalidate_layout();
                }
                if local.are_widgets_invalid() {
                    shell.invalidate_widgets();
                }
                shell.input_method_mut().merge(local.input_method());
                match local.redraw_request() {
                    iced::window::RedrawRequest::NextFrame => shell.request_redraw(),
                    iced::window::RedrawRequest::At(at) => shell.request_redraw_at(at),
                    iced::window::RedrawRequest::Wait => {}
                }
                drop(local);
                let mut native_actions = Vec::new();
                let mut request = None;
                for output in actions {
                    match output {
                        Output::EditorAction { action, .. } => {
                            native_actions.push(crate::editor_action::Action::Edit(action))
                        }
                        Output::RichEditorAction { action, .. } => native_actions.push(action),
                        Output::EditorInteraction { reference, action } => {
                            if let Some(shared) = &self.transactions {
                                let control = super::lock(shared);
                                if reference
                                    == super::editor_documents::current_reference(
                                        &self.document,
                                        &control,
                                    )
                                {
                                    request =
                                        Some(wire::EditorRequestInput::Interaction { action });
                                }
                            }
                        }
                        Output::EditorClaim { state, .. } => {
                            request = Some(wire::EditorRequestInput::Key {
                                key: state,
                                repeat: matches!(
                                    &input.work,
                                    super::editor_transactions::NativeWork::Event(Event::Keyboard(
                                        iced::keyboard::Event::KeyPressed { repeat: true, .. }
                                    ))
                                ),
                            })
                        }
                        _ => shell.publish(output),
                    }
                }
                if native_actions.is_empty()
                    && request.is_none()
                    && self
                        .transactions
                        .as_ref()
                        .is_none_or(|shared| super::lock(shared).pending.is_none())
                {
                    if let Some(shared) = &self.transactions {
                        let mut control = super::lock(shared);
                        let _ = control.lane.commit(*sequence);
                        let _ = control.lane.acknowledge(*sequence);
                    }
                } else {
                    shell.publish(Output::EditorBatch(super::editor_transactions::Batch {
                        instance: self.instance,
                        document: self.document.clone(),
                        key: self.key.clone(),
                        sequence: *sequence,
                        reset: self.reset,
                        actions: native_actions,
                        presentation_reference: matches!(
                            request,
                            Some(wire::EditorRequestInput::Interaction { .. })
                        )
                        .then(|| self.reference.clone()),
                        request,
                    }));
                }
                shell.request_redraw();
            } else {
                shell.merge(local, std::convert::identity);
            }
            let mut focused = Focus(false);
            editor
                .as_widget_mut()
                .operate(tree, layout, renderer, &mut focused);
            use widget::text_editor::Status;
            if !self.editable {
                Status::Disabled
            } else if focused.0 {
                Status::Focused {
                    is_hovered: cursor.is_over(layout.bounds()),
                }
            } else if cursor.is_over(layout.bounds()) {
                Status::Hovered
            } else {
                Status::Active
            }
        };
        if self.status != status {
            self.status = status;
            if !matches!(
                event,
                Event::Window(iced::window::Event::RedrawRequested(_))
            ) {
                shell.request_redraw();
            }
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
        self.build(&content)
            .as_widget()
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::renderer::Headless;
    use iced::{Color, Font, Pixels, Point, keyboard, window};
    use iced_test::runtime::{UserInterface, user_interface};
    type Ui = UserInterface<'static, Output, iced::Theme, iced::Renderer>;

    fn key(value: &str, modifiers: keyboard::Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(value.into()),
            modified_key: keyboard::Key::Character(value.into()),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: Some(value.into()),
            repeat: false,
        })
    }
    fn send(
        ui: &mut Ui,
        content: &Shared,
        renderer: &mut iced::Renderer,
        event: Event,
        cursor: mouse::Cursor,
    ) -> usize {
        let mut messages = vec![];
        ui.update(
            &[event],
            cursor,
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        let count = messages.len();
        for message in messages {
            let Output::EditorAction { key, action, .. } = message else {
                panic!("native editor route");
            };
            assert_eq!(key, "editor");
            content.lock().unwrap().perform(action);
        }
        count
    }
    /// These tests own a `Content` directly: they check native layout, paint
    /// and routing, not the document session that would otherwise fill it.
    fn document(name: &str, text: &str) -> wire::editor_document::EditorDocumentRef {
        wire::editor_document::EditorDocumentRef {
            document: name.into(),
            reset: 0,
            text_revision: 0,
            revision: 0,
            cursor: wire::EditorCursor::default(),
            byte_len: text.len() as u32,
        }
    }
    fn build(
        node: &wire::Node,
        content: &Shared,
        renderer: &mut iced::Renderer,
        cache: user_interface::Cache,
    ) -> Ui {
        UserInterface::build(
            HostEditor::new(node, content.clone(), None, 0),
            Size::new(200.0, 120.0),
            cache,
            renderer,
        )
    }
    fn paint(ui: &mut Ui, renderer: &mut iced::Renderer, cursor: mouse::Cursor) -> Vec<u8> {
        ui.update(
            &[Event::Window(window::Event::RedrawRequested(
                std::time::Instant::now(),
            ))],
            cursor,
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut vec![],
        );
        ui.draw(
            renderer,
            &iced::Theme::Light,
            &renderer::Style {
                text_color: Color::BLACK,
            },
            cursor,
        );
        renderer.screenshot(Size::new(200, 120), 1.0, Color::WHITE)
    }

    #[test]
    fn guest_line_highlight_is_painted_by_the_actual_editor() {
        use wire::editor_presentation::{EditorFormat, EditorPresentation, EditorSpan};
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let source = "# Heading";
        let node = wire::Node::Editor {
            key: "editor".into(),
            placeholder: String::new(),
            document: document("app:notes", source),
            on_document: 7,
            editable: false,
            width: Some(200.0),
            height: Some(wire::Length::Fixed(120.0)),
            min_height: None,
            max_height: None,
            options: Box::new(wire::EditorOptions {
                padding: Some(5.0),
                presentation: Some(Box::new(EditorPresentation {
                    formats: vec![EditorFormat {
                        color: Some(wire::Rgba([0.0; 4])),
                        size: Some(24.0),
                        line_height: Some(wire::LineHeight::Absolute(40.0)),
                        line_background: Some(wire::Rgba([0.0, 1.0, 0.0, 1.0])),
                        ..Default::default()
                    }],
                    spans: vec![EditorSpan {
                        line: 0,
                        start: 0,
                        end: source.len() as u32,
                        format: 0,
                    }],
                    ..Default::default()
                })),
                ..Default::default()
            }),
        };
        let content = Arc::new(Mutex::new(Content::with_text(source)));
        let mut ui = build(
            &node,
            &content,
            &mut renderer,
            user_interface::Cache::default(),
        );
        let pixels = paint(&mut ui, &mut renderer, mouse::Cursor::Unavailable);
        assert_eq!(
            &pixels[(15 * 200 + 100) * 4..][..3],
            &[0, 255, 0],
            "guest line background must reach native paint"
        );
        assert_eq!(content.lock().unwrap().text(), source);
    }

    // Claim: native editor layout, status/selection paint, edit routes and disabled behavior survive copied options.
    // Counterexamples: omitted line height changes the height; omitted styles change pixels; routing disabled edits changes Content.
    #[test]
    fn presentation_preserves_native_layout_selection_editing_and_disabled_state() {
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let face = |rgba| wire::InputFace {
            background: Some(wire::Rgba(rgba)),
            ..Default::default()
        };
        let mut node = wire::Node::Editor {
            key: "editor".into(),
            placeholder: "File contents".into(),
            document: document("app:notes", "ab\ncd"),
            on_document: 7,
            editable: true,
            width: Some(200.0),
            height: None,
            min_height: None,
            max_height: None,
            options: Box::new(wire::EditorOptions {
                binding: None,
                presentation: None,
                size: Some(20.0),
                padding: Some(7.0),
                line_height: Some(wire::LineHeight::Absolute(30.0)),
                wrapping: Some(wire::Wrapping::Word),
                font: None,
                style: wire::InputStyle {
                    active: wire::InputFace {
                        background: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
                        value: Some(wire::Rgba([0.0, 0.0, 0.0, 1.0])),
                        selection: Some(wire::Rgba([0.0, 1.0, 1.0, 1.0])),
                        ..Default::default()
                    },
                    hovered: Some(face([0.0, 1.0, 0.0, 1.0])),
                    focused: Some(face([0.0, 0.0, 1.0, 1.0])),
                    focused_hovered: Some(face([1.0, 1.0, 0.0, 1.0])),
                    disabled: Some(face([0.5, 0.5, 0.5, 1.0])),
                    ..Default::default()
                },
            }),
        };
        let content = Arc::new(Mutex::new(Content::with_text("ab\ncd")));
        let mut element: Element<'_, Output, iced::Theme, iced::Renderer> =
            HostEditor::new(&node, content.clone(), None, 0).into();
        let mut tree = Tree::new(&element);
        let bounds = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(200.0, 120.0)),
        );
        assert_eq!(
            bounds.size(),
            Size::new(200.0, 74.0),
            "two absolute 30px lines plus 7px padding"
        );
        let inside = mouse::Cursor::Available(Point::new(15.0, 15.0));
        let mut ui = build(
            &node,
            &content,
            &mut renderer,
            user_interface::Cache::default(),
        );
        for (cursor, expected) in [
            (mouse::Cursor::Unavailable, [255, 0, 0]),
            (inside, [0, 255, 0]),
        ] {
            let pixels = paint(&mut ui, &mut renderer, cursor);
            assert_eq!(
                &pixels[(5 * 200 + 100) * 4..][..3],
                &expected,
                "active/hovered background"
            );
        }
        send(
            &mut ui,
            &content,
            &mut renderer,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            inside,
        );
        send(
            &mut ui,
            &content,
            &mut renderer,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            inside,
        );
        let modifiers = if cfg!(target_os = "macos") {
            keyboard::Modifiers::LOGO
        } else {
            keyboard::Modifiers::CTRL
        };
        send(
            &mut ui,
            &content,
            &mut renderer,
            key("a", modifiers),
            inside,
        );
        assert_eq!(
            content.lock().unwrap().selection().as_deref(),
            Some("ab\ncd"),
            "native select-all keeps host selection"
        );
        ui = build(&node, &content, &mut renderer, ui.into_cache());
        let pixels = paint(&mut ui, &mut renderer, mouse::Cursor::Unavailable);
        assert_eq!(
            &pixels[(5 * 200 + 100) * 4..][..3],
            &[0, 0, 255],
            "focused background"
        );
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[..3] == [0, 255, 255]),
            "native selection uses copied cyan paint"
        );
        let pixels = paint(&mut ui, &mut renderer, inside);
        assert_eq!(
            &pixels[(5 * 200 + 100) * 4..][..3],
            &[255, 255, 0],
            "focused hovered overrides focused"
        );
        assert!(
            send(
                &mut ui,
                &content,
                &mut renderer,
                key("Z", keyboard::Modifiers::default()),
                inside
            ) > 0
        );
        assert_eq!(
            content.lock().unwrap().text(),
            "Z",
            "typing replaces the native selection"
        );
        struct PasteClipboard;
        impl Clipboard for PasteClipboard {
            fn read(&self, _: iced::advanced::clipboard::Kind) -> Option<String> {
                Some("한".into())
            }
            fn write(&mut self, _: iced::advanced::clipboard::Kind, _: String) {}
        }
        ui = build(&node, &content, &mut renderer, ui.into_cache());
        let mut pasted = vec![];
        ui.update(
            &[key("v", modifiers)],
            inside,
            &mut renderer,
            &mut PasteClipboard,
            &mut pasted,
        );
        for message in pasted {
            let Output::EditorAction { action, .. } = message else {
                panic!("native paste route")
            };
            content.lock().unwrap().perform(action);
        }
        assert_eq!(
            content.lock().unwrap().text(),
            "Z한",
            "unbound editor reads the native clipboard"
        );
        if let wire::Node::Editor { editable, .. } = &mut node {
            *editable = false;
        }
        ui = build(&node, &content, &mut renderer, ui.into_cache());
        assert_eq!(
            send(
                &mut ui,
                &content,
                &mut renderer,
                key("x", keyboard::Modifiers::default()),
                inside
            ),
            0,
            "disabled editor emits no edits"
        );
        assert_eq!(content.lock().unwrap().text(), "Z한");
        let pixels = paint(&mut ui, &mut renderer, inside);
        assert_eq!(
            &pixels[(5 * 200 + 100) * 4..][..3],
            &[128, 128, 128],
            "disabled face"
        );
    }

    #[test]
    fn relative_typography_and_word_wrapping_use_native_line_layout() {
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let height = |wrapping| {
            let text = "one two three four five six";
            let node = wire::Node::Editor {
                key: "wrapped".into(),
                placeholder: String::new(),
                document: document("app:wrapped", text),
                on_document: 7,
                editable: true,
                width: Some(100.0),
                height: None,
                min_height: None,
                max_height: None,
                options: Box::new(wire::EditorOptions {
                    binding: None,
                    size: Some(20.0),
                    padding: Some(5.0),
                    line_height: Some(wire::LineHeight::Relative(1.5)),
                    wrapping: Some(wrapping),
                    font: Some(wire::NamedFont {
                        family: wire::FontFamily::Monospace,
                        weight: wire::Weight::Normal,
                        stretch: wire::FontStretch::Normal,
                        style: wire::FontStyle::Normal,
                    }),
                    ..Default::default()
                }),
            };
            let mut editor: Element<'_, Output, iced::Theme, iced::Renderer> = HostEditor::new(
                &node,
                Arc::new(Mutex::new(Content::with_text(text))),
                None,
                0,
            )
            .into();
            let mut tree = Tree::new(&editor);
            editor
                .as_widget_mut()
                .layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(100.0, 500.0)),
                )
                .size()
                .height
        };
        assert_eq!(
            height(wire::Wrapping::None),
            40.0,
            "20px × 1.5 plus 5px padding"
        );
        assert!(
            height(wire::Wrapping::Word) > 40.0,
            "native words reflow into more lines"
        );
    }
}

//! Draw and edit text.
use crate::core::text::editor::{
    self, Action, Cursor, Decoration, Direction, Edit, Motion, Position,
    Selection,
};
use crate::core::text::highlighter::{self, Highlighter};
use crate::core::text::{Highlight, LineHeight, Wrapping};
use crate::core::{Font, Padding, Pixels, Point, Rectangle, Size};
use crate::text;

use cosmic_text::Edit as _;

use std::borrow::Cow;
use std::fmt;
use std::sync::{self, Arc, RwLock};

/// A multi-line text editor.
#[derive(Debug, PartialEq)]
pub struct Editor(Option<Arc<Internal>>);

struct Internal {
    editor: cosmic_text::Editor<'static>,
    selection: RwLock<Option<Selection>>,
    font: Font,
    bounds: Size,
    topmost_line_changed: Option<usize>,
    decorations: Vec<Vec<DecorationSpec>>,
    version: text::Version,
}

#[derive(Debug, Clone)]
struct DecorationSpec {
    range: std::ops::Range<usize>,
    highlight: Highlight,
    padding: Padding,
    height: f32,
    line: bool,
    strikethrough: bool,
}

impl Editor {
    /// Creates a new empty [`Editor`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the buffer of the [`Editor`].
    pub fn buffer(&self) -> &cosmic_text::Buffer {
        buffer_from_editor(&self.internal().editor)
    }

    /// Creates a [`Weak`] reference to the [`Editor`].
    ///
    /// This is useful to avoid cloning the [`Editor`] when
    /// referential guarantees are unnecessary. For instance,
    /// when creating a rendering tree.
    pub fn downgrade(&self) -> Weak {
        let editor = self.internal();

        Weak {
            raw: Arc::downgrade(editor),
            bounds: editor.bounds,
        }
    }

    fn internal(&self) -> &Arc<Internal> {
        self.0
            .as_ref()
            .expect("Editor should always be initialized")
    }

    fn with_internal_mut<T>(
        &mut self,
        f: impl FnOnce(&mut Internal) -> T,
    ) -> T {
        let editor =
            self.0.take().expect("Editor should always be initialized");

        // TODO: Handle multiple strong references somehow
        let mut internal = Arc::try_unwrap(editor)
            .expect("Editor cannot have multiple strong references");

        // Clear cursor cache
        let _ = internal
            .selection
            .write()
            .expect("Write to cursor cache")
            .take();

        let result = f(&mut internal);

        self.0 = Some(Arc::new(internal));

        result
    }
}

impl editor::Editor for Editor {
    type Font = Font;

    fn with_text(text: &str) -> Self {
        let mut buffer = cosmic_text::Buffer::new_empty(cosmic_text::Metrics {
            font_size: 1.0,
            line_height: 1.0,
        });

        let mut font_system =
            text::font_system().write().expect("Write font system");

        buffer.set_text(
            font_system.raw(),
            text,
            &cosmic_text::Attrs::new(),
            cosmic_text::Shaping::Advanced,
            None,
        );

        Editor(Some(Arc::new(Internal {
            editor: cosmic_text::Editor::new(buffer),
            version: font_system.version(),
            ..Default::default()
        })))
    }

    fn is_empty(&self) -> bool {
        let buffer = self.buffer();

        buffer.lines.is_empty()
            || (buffer.lines.len() == 1 && buffer.lines[0].text().is_empty())
    }

    fn line(&self, index: usize) -> Option<editor::Line<'_>> {
        self.buffer().lines.get(index).map(|line| editor::Line {
            text: Cow::Borrowed(line.text()),
            ending: match line.ending() {
                cosmic_text::LineEnding::Lf => editor::LineEnding::Lf,
                cosmic_text::LineEnding::CrLf => editor::LineEnding::CrLf,
                cosmic_text::LineEnding::Cr => editor::LineEnding::Cr,
                cosmic_text::LineEnding::LfCr => editor::LineEnding::LfCr,
                cosmic_text::LineEnding::None => editor::LineEnding::None,
            },
        })
    }

    fn line_count(&self) -> usize {
        self.buffer().lines.len()
    }

    fn copy(&self) -> Option<String> {
        self.internal().editor.copy_selection()
    }

    fn selection(&self) -> editor::Selection {
        let internal = self.internal();

        if let Ok(Some(cursor)) = internal.selection.read().as_deref() {
            return cursor.clone();
        }

        let buffer = buffer_from_editor(&internal.editor);

        let cursor = match internal.editor.selection_bounds() {
            Some((start, end)) => {
                let regions = buffer
                    .layout_runs()
                    .filter_map(|run| {
                        let (x, width) = run.highlight(start, end)?;
                        (width > 0.0).then_some(Rectangle {
                            x,
                            width,
                            y: run.line_top,
                            height: run.line_height,
                        })
                    })
                    .collect();

                Selection::Range(regions)
            }
            _ => {
                let position = internal
                    .editor
                    .cursor_position()
                    .map(|(x, y)| Point::new(x as f32, y as f32))
                    .unwrap_or(Point::ORIGIN);

                Selection::Caret(position)
            }
        };

        *internal.selection.write().expect("Write to cursor cache") =
            Some(cursor.clone());

        cursor
    }

    fn caret_height(&self) -> Pixels {
        let internal = self.internal();
        let buffer = buffer_from_editor(&internal.editor);
        let cursor_y = internal.editor.cursor_position().map(|(_, y)| y);

        Pixels(
            buffer
                .layout_runs()
                .find(|run| Some(run.line_top as i32) == cursor_y)
                .map_or(buffer.metrics().line_height, |run| run.line_height),
        )
    }

    fn decorations(&self) -> Vec<Decoration> {
        let internal = self.internal();
        let buffer = buffer_from_editor(&internal.editor);
        let mut decorations: Vec<Decoration> = Vec::new();

        for run in buffer.layout_runs() {
            let Some(specs) = internal.decorations.get(run.line_i) else {
                continue;
            };

            for spec in specs {
                let bounds = if spec.line {
                    Rectangle::new(
                        Point::new(-spec.padding.left, run.line_top),
                        Size::new(
                            internal.bounds.width
                                + spec.padding.left
                                + spec.padding.right,
                            run.line_height,
                        ),
                    )
                } else {
                    let start = cosmic_text::Cursor::new(run.line_i, spec.range.start);
                    let end = cosmic_text::Cursor::new(run.line_i, spec.range.end);
                    let Some((x, width)) = run.highlight(start, end) else {
                        continue;
                    };
                    if width <= 0.0 {
                        continue;
                    }
                    if spec.strikethrough {
                        Rectangle {
                            x,
                            y: run.line_top + run.line_height / 2.0,
                            width,
                            height: 1.0,
                        }
                    } else {
                        Rectangle {
                            x: x - spec.padding.left,
                            y: run.line_top
                                + (run.line_height - spec.height) / 2.0
                                - spec.padding.top,
                            width: width + spec.padding.left + spec.padding.right,
                            height: spec.height
                                + spec.padding.top
                                + spec.padding.bottom,
                        }
                    }
                };

                if spec.line
                    && let Some(previous) = decorations.last_mut()
                    && previous.highlight == spec.highlight
                    && previous.bounds.x == bounds.x
                    && previous.bounds.width == bounds.width
                    && (previous.bounds.y + previous.bounds.height - bounds.y).abs()
                        < 0.5
                {
                    previous.bounds.height =
                        bounds.y + bounds.height - previous.bounds.y;
                } else {
                    decorations.push(Decoration {
                        bounds,
                        highlight: spec.highlight,
                    });
                }
            }
        }

        decorations
    }

    fn cursor(&self) -> Cursor {
        let editor = &self.internal().editor;

        let position = {
            let cursor = editor.cursor();

            Position {
                line: cursor.line,
                column: cursor.index,
            }
        };

        let selection = match editor.selection() {
            cosmic_text::Selection::None => None,
            cosmic_text::Selection::Normal(cursor)
            | cosmic_text::Selection::Line(cursor)
            | cosmic_text::Selection::Word(cursor) => Some(Position {
                line: cursor.line,
                column: cursor.index,
            }),
        };

        Cursor {
            position,
            selection,
        }
    }

    fn perform(&mut self, action: Action) {
        let mut font_system =
            text::font_system().write().expect("Write font system");

        self.with_internal_mut(|internal| {
            let editor = &mut internal.editor;

            match action {
                // Motion events
                Action::Move(motion) => {
                    if let Some((start, end)) = editor.selection_bounds() {
                        editor.set_selection(cosmic_text::Selection::None);

                        match motion {
                            // These motions are performed as-is even when a selection
                            // is present
                            Motion::Home
                            | Motion::End
                            | Motion::DocumentStart
                            | Motion::DocumentEnd => {
                                editor.action(
                                    font_system.raw(),
                                    cosmic_text::Action::Motion(to_motion(
                                        motion,
                                    )),
                                );
                            }
                            // Other motions simply move the cursor to one end of the selection
                            _ => editor.set_cursor(match motion.direction() {
                                Direction::Left => start,
                                Direction::Right => end,
                            }),
                        }
                    } else {
                        editor.action(
                            font_system.raw(),
                            cosmic_text::Action::Motion(to_motion(motion)),
                        );
                    }
                }

                // Selection events
                Action::Select(motion) => {
                    let cursor = editor.cursor();

                    if editor.selection_bounds().is_none() {
                        editor.set_selection(cosmic_text::Selection::Normal(
                            cursor,
                        ));
                    }

                    editor.action(
                        font_system.raw(),
                        cosmic_text::Action::Motion(to_motion(motion)),
                    );

                    // Deselect if selection matches cursor position
                    if let Some((start, end)) = editor.selection_bounds()
                        && start.line == end.line
                        && start.index == end.index
                    {
                        editor.set_selection(cosmic_text::Selection::None);
                    }
                }
                Action::SelectWord => {
                    let cursor = editor.cursor();

                    editor.set_selection(cosmic_text::Selection::Word(cursor));
                }
                Action::SelectLine => {
                    let cursor = editor.cursor();

                    editor.set_selection(cosmic_text::Selection::Line(cursor));
                }
                Action::SelectAll => {
                    let buffer = buffer_from_editor(editor);

                    if buffer.lines.len() > 1
                        || buffer
                            .lines
                            .first()
                            .is_some_and(|line| !line.text().is_empty())
                    {
                        let cursor = editor.cursor();

                        editor.set_selection(cosmic_text::Selection::Normal(
                            cosmic_text::Cursor {
                                line: 0,
                                index: 0,
                                ..cursor
                            },
                        ));

                        editor.action(
                            font_system.raw(),
                            cosmic_text::Action::Motion(
                                cosmic_text::Motion::BufferEnd,
                            ),
                        );
                    }
                }

                // Editing events
                Action::Edit(edit) => {
                    let topmost_line_before_edit = editor
                        .selection_bounds()
                        .map(|(start, _)| start)
                        .unwrap_or_else(|| editor.cursor())
                        .line;

                    match edit {
                        Edit::Insert(c) => {
                            editor.action(
                                font_system.raw(),
                                cosmic_text::Action::Insert(c),
                            );
                        }
                        Edit::Paste(text) => {
                            editor.insert_string(&text, None);
                        }
                        Edit::Indent => {
                            editor.action(
                                font_system.raw(),
                                cosmic_text::Action::Indent,
                            );
                        }
                        Edit::Unindent => {
                            editor.action(
                                font_system.raw(),
                                cosmic_text::Action::Unindent,
                            );
                        }
                        Edit::Enter => {
                            editor.action(
                                font_system.raw(),
                                cosmic_text::Action::Enter,
                            );
                        }
                        Edit::Backspace => {
                            editor.action(
                                font_system.raw(),
                                cosmic_text::Action::Backspace,
                            );
                        }
                        Edit::Delete => {
                            editor.action(
                                font_system.raw(),
                                cosmic_text::Action::Delete,
                            );
                        }
                    }

                    let cursor = editor.cursor();
                    let selection_start = editor
                        .selection_bounds()
                        .map(|(start, _)| start)
                        .unwrap_or(cursor);

                    internal.topmost_line_changed = Some(
                        selection_start.line.min(topmost_line_before_edit),
                    );
                }

                // Mouse events
                Action::Click(position) => {
                    editor.action(
                        font_system.raw(),
                        cosmic_text::Action::Click {
                            x: position.x as i32,
                            y: position.y as i32,
                        },
                    );
                }
                Action::Drag(position) => {
                    editor.action(
                        font_system.raw(),
                        cosmic_text::Action::Drag {
                            x: position.x as i32,
                            y: position.y as i32,
                        },
                    );

                    // Deselect if selection matches cursor position
                    if let Some((start, end)) = editor.selection_bounds()
                        && start.line == end.line
                        && start.index == end.index
                    {
                        editor.set_selection(cosmic_text::Selection::None);
                    }
                }
                Action::Scroll { lines } => {
                    editor.action(
                        font_system.raw(),
                        cosmic_text::Action::Scroll {
                            pixels: lines as f32
                                * buffer_from_editor(editor)
                                    .metrics()
                                    .line_height,
                        },
                    );
                }
            }
        });
    }

    fn move_to(&mut self, cursor: Cursor) {
        self.with_internal_mut(|internal| {
            // TODO: Expose `Affinity`
            internal.editor.set_cursor(cosmic_text::Cursor {
                line: cursor.position.line,
                index: cursor.position.column,
                affinity: cosmic_text::Affinity::Before,
            });

            if let Some(selection) = cursor.selection {
                internal
                    .editor
                    .set_selection(cosmic_text::Selection::Normal(
                        cosmic_text::Cursor {
                            line: selection.line,
                            index: selection.column,
                            affinity: cosmic_text::Affinity::Before,
                        },
                    ));
            }
        });
    }

    fn bounds(&self) -> Size {
        self.internal().bounds
    }

    fn min_bounds(&self) -> Size {
        let internal = self.internal();

        let (bounds, _has_rtl) =
            text::measure(buffer_from_editor(&internal.editor));

        bounds
    }

    fn update(
        &mut self,
        new_bounds: Size,
        new_font: Font,
        new_size: Pixels,
        new_line_height: LineHeight,
        new_wrapping: Wrapping,
        new_highlighter: &mut impl Highlighter,
    ) {
        self.with_internal_mut(|internal| {
            let mut font_system =
                text::font_system().write().expect("Write font system");

            let buffer = buffer_mut_from_editor(&mut internal.editor);

            if font_system.version() != internal.version {
                log::trace!("Updating `FontSystem` of `Editor`...");

                for line in buffer.lines.iter_mut() {
                    line.reset();
                }

                internal.version = font_system.version();
                internal.topmost_line_changed = Some(0);
            }

            if new_font != internal.font {
                log::trace!("Updating font of `Editor`...");

                for line in buffer.lines.iter_mut() {
                    let _ = line.set_attrs_list(cosmic_text::AttrsList::new(
                        &text::to_attributes(new_font),
                    ));
                }

                internal.font = new_font;
                internal.topmost_line_changed = Some(0);
            }

            let metrics = buffer.metrics();
            let new_line_height = new_line_height.to_absolute(new_size);

            if new_size.0 != metrics.font_size
                || new_line_height.0 != metrics.line_height
            {
                log::trace!("Updating `Metrics` of `Editor`...");

                buffer.set_metrics(
                    font_system.raw(),
                    cosmic_text::Metrics::new(new_size.0, new_line_height.0),
                );
            }

            let new_wrap = text::to_wrap(new_wrapping);

            if new_wrap != buffer.wrap() {
                log::trace!("Updating `Wrap` strategy of `Editor`...");

                buffer.set_wrap(font_system.raw(), new_wrap);
            }

            if new_bounds != internal.bounds {
                log::trace!("Updating size of `Editor`...");

                buffer.set_size(
                    font_system.raw(),
                    Some(new_bounds.width),
                    Some(new_bounds.height),
                );

                internal.bounds = new_bounds;
            }

            if let Some(topmost_line_changed) =
                internal.topmost_line_changed.take()
            {
                log::trace!(
                    "Notifying highlighter of line \
                    change: {topmost_line_changed}"
                );

                new_highlighter.change_line(topmost_line_changed);
            }

            internal.editor.shape_as_needed(font_system.raw(), false);
        });
    }

    fn highlight<H: Highlighter>(
        &mut self,
        font: Self::Font,
        highlighter: &mut H,
        format_highlight: impl Fn(&H::Highlight) -> highlighter::Format<Self::Font>,
    ) {
        let internal = self.internal();
        let buffer = buffer_from_editor(&internal.editor);

        let last_visible_line = buffer
            .layout_runs()
            .last()
            .map(|run| run.line_i)
            .unwrap_or(buffer.lines.len().saturating_sub(1));

        let current_line = highlighter.current_line();

        if current_line > last_visible_line {
            return;
        }

        let editor =
            self.0.take().expect("Editor should always be initialized");

        let mut internal = Arc::try_unwrap(editor)
            .expect("Editor cannot have multiple strong references");

        let mut font_system =
            text::font_system().write().expect("Write font system");

        let attributes = text::to_attributes(font);

        internal.decorations.truncate(current_line);
        let metrics = buffer_from_editor(&internal.editor).metrics();
        let default_line_height = metrics.line_height / metrics.font_size;

        for line in &mut buffer_mut_from_editor(&mut internal.editor).lines
            [current_line..=last_visible_line]
        {
            let mut list = cosmic_text::AttrsList::new(&attributes);
            let mut decorations = Vec::new();

            for (range, highlight) in highlighter.highlight_line(line.text()) {
                let format = format_highlight(&highlight);
                let size = format.size.unwrap_or(Pixels(metrics.font_size));
                let line_height = format
                    .line_height
                    .unwrap_or(LineHeight::Relative(default_line_height))
                    .to_absolute(size);

                if format.color.is_some()
                    || format.font.is_some()
                    || format.size.is_some()
                    || format.line_height.is_some()
                {
                    let mut span_attributes = if let Some(font) = format.font {
                        text::to_attributes(font)
                    } else {
                        attributes.clone()
                    };

                    if format.size.is_some() || format.line_height.is_some() {
                        span_attributes = span_attributes.metrics(
                            cosmic_text::Metrics::new(size.0, line_height.0),
                        );
                    }

                    list.add_span(
                        range.clone(),
                        &cosmic_text::Attrs {
                            color_opt: format.color.map(text::to_color),
                            ..span_attributes
                        },
                    );
                }

                if let Some(highlight) = format.line_highlight {
                    decorations.push(DecorationSpec {
                        range: range.clone(),
                        highlight,
                        padding: format.padding,
                        height: line_height.0,
                        line: true,
                        strikethrough: false,
                    });
                }
                if let Some(highlight) = format.highlight {
                    decorations.push(DecorationSpec {
                        range: range.clone(),
                        highlight,
                        padding: format.padding,
                        height: line_height.0,
                        line: false,
                        strikethrough: false,
                    });
                }
                if let Some(color) = format.strikethrough {
                    decorations.push(DecorationSpec {
                        range,
                        highlight: Highlight {
                            background: color.into(),
                            border: Default::default(),
                        },
                        padding: Padding::ZERO,
                        height: 1.0,
                        line: false,
                        strikethrough: true,
                    });
                }
            }

            let _ = line.set_attrs_list(list);
            internal.decorations.push(decorations);
        }

        internal.editor.shape_as_needed(font_system.raw(), false);

        self.0 = Some(Arc::new(internal));
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self(Some(Arc::new(Internal::default())))
    }
}

impl PartialEq for Internal {
    fn eq(&self, other: &Self) -> bool {
        self.font == other.font
            && self.bounds == other.bounds
            && buffer_from_editor(&self.editor).metrics()
                == buffer_from_editor(&other.editor).metrics()
    }
}

impl Default for Internal {
    fn default() -> Self {
        Self {
            editor: cosmic_text::Editor::new(cosmic_text::Buffer::new_empty(
                cosmic_text::Metrics {
                    font_size: 1.0,
                    line_height: 1.0,
                },
            )),
            selection: RwLock::new(None),
            font: Font::default(),
            bounds: Size::ZERO,
            topmost_line_changed: None,
            decorations: Vec::new(),
            version: text::Version::default(),
        }
    }
}

impl fmt::Debug for Internal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Internal")
            .field("font", &self.font)
            .field("bounds", &self.bounds)
            .finish()
    }
}

/// A weak reference to an [`Editor`].
#[derive(Debug, Clone)]
pub struct Weak {
    raw: sync::Weak<Internal>,
    /// The bounds of the [`Editor`].
    pub bounds: Size,
}

impl Weak {
    /// Tries to update the reference into an [`Editor`].
    pub fn upgrade(&self) -> Option<Editor> {
        self.raw.upgrade().map(Some).map(Editor)
    }
}

impl PartialEq for Weak {
    fn eq(&self, other: &Self) -> bool {
        match (self.raw.upgrade(), other.raw.upgrade()) {
            (Some(p1), Some(p2)) => p1 == p2,
            _ => false,
        }
    }
}

fn to_motion(motion: Motion) -> cosmic_text::Motion {
    match motion {
        Motion::Left => cosmic_text::Motion::Left,
        Motion::Right => cosmic_text::Motion::Right,
        Motion::Up => cosmic_text::Motion::Up,
        Motion::Down => cosmic_text::Motion::Down,
        Motion::WordLeft => cosmic_text::Motion::LeftWord,
        Motion::WordRight => cosmic_text::Motion::RightWord,
        Motion::Home => cosmic_text::Motion::Home,
        Motion::End => cosmic_text::Motion::End,
        Motion::PageUp => cosmic_text::Motion::PageUp,
        Motion::PageDown => cosmic_text::Motion::PageDown,
        Motion::DocumentStart => cosmic_text::Motion::BufferStart,
        Motion::DocumentEnd => cosmic_text::Motion::BufferEnd,
    }
}

fn buffer_from_editor<'a, 'b>(
    editor: &'a impl cosmic_text::Edit<'b>,
) -> &'a cosmic_text::Buffer
where
    'b: 'a,
{
    match editor.buffer_ref() {
        cosmic_text::BufferRef::Owned(buffer) => buffer,
        cosmic_text::BufferRef::Borrowed(buffer) => buffer,
        cosmic_text::BufferRef::Arc(buffer) => buffer,
    }
}

fn buffer_mut_from_editor<'a, 'b>(
    editor: &'a mut impl cosmic_text::Edit<'b>,
) -> &'a mut cosmic_text::Buffer
where
    'b: 'a,
{
    match editor.buffer_ref_mut() {
        cosmic_text::BufferRef::Owned(buffer) => buffer,
        cosmic_text::BufferRef::Borrowed(buffer) => buffer,
        cosmic_text::BufferRef::Arc(_buffer) => unreachable!(),
    }
}

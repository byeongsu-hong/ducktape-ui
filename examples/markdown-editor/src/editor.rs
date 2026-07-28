use iced::advanced::text::highlighter::{Format, Highlighter, PlainText};
use iced::advanced::text::{Highlight as TextHighlight, LineHeight};
use iced::font::{Family, Style as FontStyle, Weight};
use iced::widget::text_editor::{Action, Content, Cursor, Edit, Position, TextEditor};
use iced::{Border, Color, Element, Font, Padding, Pixels, Theme};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};

const HISTORY_LIMIT: usize = 1_000;
const HISTORY_BYTES: usize = 16 * 1024 * 1024;
const COALESCE_WINDOW: Duration = Duration::from_millis(750);
const BODY_SIZE: f32 = 16.0;
const BODY_LINE_HEIGHT: f32 = 1.6;
const HEADING_SCALE: [f32; 6] = [1.875, 1.5, 1.375, 1.25, 1.125, 1.0];
const HEADING_LINE_HEIGHT: f32 = 1.4;
const CODE_BLOCK_SCALE: f32 = 0.9;
const INLINE_CODE_SCALE: f32 = 1.0;
const CODE_BLOCK_PADDING: f32 = BODY_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caret {
    line: usize,
    column: usize,
}

impl Caret {
    fn new(line: i64, column: i64) -> Self {
        Self {
            line: usize::try_from(line).unwrap_or_default(),
            column: usize::try_from(column).unwrap_or_default(),
        }
    }
}

#[derive(Debug)]
pub struct MarkdownHighlighter {
    current_line: usize,
    fences: Vec<Option<Fence>>,
    code: HashMap<usize, iced_highlighter::Highlighter>,
    caret: Caret,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fence {
    marker: u8,
    length: usize,
    start_line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MarkdownHighlight {
    HiddenMarker,
    Marker,
    HiddenFence,
    Fence,
    Heading(u8),
    Strong,
    Emphasis,
    InlineCode,
    CodeBlock,
    CodeToken {
        color: Option<Color>,
        font: Option<Font>,
    },
    Link,
    Quote,
    ListMarker,
    Strikethrough,
}

pub fn markdown_highlight<'a, Message: 'a>(
    editor: TextEditor<'a, PlainText, Message>,
    line: i64,
    column: i64,
) -> Element<'a, Message> {
    editor
        .padding(Padding {
            top: BODY_SIZE,
            right: CODE_BLOCK_PADDING,
            bottom: 0.0,
            left: CODE_BLOCK_PADDING,
        })
        .highlight_with::<MarkdownHighlighter>(Caret::new(line, column), markdown_format)
        .into()
}

fn geist(weight: Weight, style: FontStyle) -> Font {
    Font {
        family: Family::Name("Geist"),
        weight,
        style,
        ..Font::DEFAULT
    }
}

fn geist_mono() -> Font {
    Font {
        family: Family::Name("Geist Mono"),
        ..Font::DEFAULT
    }
}

fn markdown_format(highlight: &MarkdownHighlight, theme: &Theme) -> Format<Font> {
    let palette = theme.palette();
    let subdued = Color {
        a: 0.38,
        ..palette.text
    };

    let code_background = TextHighlight {
        background: Color {
            a: 0.03,
            ..palette.text
        }
        .into(),
        border: Border {
            color: Color {
                a: 0.1,
                ..palette.text
            },
            width: 1.0,
            radius: 3.0.into(),
        },
    };
    let inline_code_background = TextHighlight {
        background: Color {
            a: 0.03,
            ..palette.text
        }
        .into(),
        border: Border::default().rounded(3.0),
    };

    match *highlight {
        MarkdownHighlight::HiddenMarker => Format {
            color: Some(Color::TRANSPARENT),
            font: Some(geist(Weight::Normal, FontStyle::Normal)),
            size: Some(Pixels(0.01)),
            ..Format::default()
        },
        MarkdownHighlight::Marker => Format {
            color: Some(subdued),
            font: Some(geist(Weight::Normal, FontStyle::Normal)),
            ..Format::default()
        },
        MarkdownHighlight::HiddenFence => Format {
            color: Some(Color::TRANSPARENT),
            font: Some(geist_mono()),
            size: Some(Pixels(0.01)),
            line_height: Some(LineHeight::Absolute(Pixels(BODY_SIZE))),
            line_highlight: Some(code_background),
            padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
            ..Format::default()
        },
        MarkdownHighlight::Fence => Format {
            color: Some(subdued),
            font: Some(geist_mono()),
            size: Some(Pixels(BODY_SIZE * CODE_BLOCK_SCALE)),
            line_height: Some(LineHeight::Absolute(Pixels(
                BODY_SIZE * CODE_BLOCK_SCALE * BODY_LINE_HEIGHT,
            ))),
            line_highlight: Some(code_background),
            padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
            ..Format::default()
        },
        MarkdownHighlight::Heading(level) => Format {
            color: Some(palette.text),
            font: Some(geist(Weight::Bold, FontStyle::Normal)),
            size: Some(Pixels(BODY_SIZE * HEADING_SCALE[level as usize - 1])),
            line_height: Some(LineHeight::Absolute(Pixels(
                BODY_SIZE * HEADING_SCALE[level as usize - 1] * HEADING_LINE_HEIGHT,
            ))),
            ..Format::default()
        },
        MarkdownHighlight::Strong => Format {
            color: Some(palette.text),
            font: Some(geist(Weight::Bold, FontStyle::Normal)),
            ..Format::default()
        },
        MarkdownHighlight::Emphasis => Format {
            color: Some(palette.text),
            font: Some(geist(Weight::Normal, FontStyle::Italic)),
            ..Format::default()
        },
        MarkdownHighlight::Quote => Format {
            color: Some(subdued),
            font: Some(geist(Weight::Normal, FontStyle::Normal)),
            ..Format::default()
        },
        MarkdownHighlight::InlineCode => Format {
            color: Some(palette.text),
            font: Some(geist_mono()),
            size: Some(Pixels(BODY_SIZE * INLINE_CODE_SCALE)),
            line_height: Some(LineHeight::Absolute(Pixels(
                BODY_SIZE * INLINE_CODE_SCALE * BODY_LINE_HEIGHT,
            ))),
            highlight: Some(inline_code_background),
            padding: Padding {
                top: BODY_SIZE * INLINE_CODE_SCALE * 0.2,
                right: BODY_SIZE * INLINE_CODE_SCALE * 0.4,
                bottom: BODY_SIZE * INLINE_CODE_SCALE * 0.2,
                left: BODY_SIZE * INLINE_CODE_SCALE * 0.4,
            },
            ..Format::default()
        },
        MarkdownHighlight::CodeBlock => Format {
            color: Some(palette.text),
            font: Some(geist_mono()),
            size: Some(Pixels(BODY_SIZE * CODE_BLOCK_SCALE)),
            line_height: Some(LineHeight::Absolute(Pixels(
                BODY_SIZE * CODE_BLOCK_SCALE * BODY_LINE_HEIGHT,
            ))),
            line_highlight: Some(code_background),
            padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
            ..Format::default()
        },
        MarkdownHighlight::CodeToken { color, font } => Format {
            color: Some(color.unwrap_or(palette.text)),
            font: Some(font.map_or_else(geist_mono, |font| Font {
                family: Family::Name("Geist Mono"),
                ..font
            })),
            size: Some(Pixels(BODY_SIZE * CODE_BLOCK_SCALE)),
            line_height: Some(LineHeight::Absolute(Pixels(
                BODY_SIZE * CODE_BLOCK_SCALE * BODY_LINE_HEIGHT,
            ))),
            ..Format::default()
        },
        MarkdownHighlight::Link => Format {
            color: Some(palette.primary),
            font: Some(geist(Weight::Semibold, FontStyle::Normal)),
            ..Format::default()
        },
        MarkdownHighlight::ListMarker => Format {
            color: Some(palette.text),
            font: Some(geist(Weight::Normal, FontStyle::Normal)),
            ..Format::default()
        },
        MarkdownHighlight::Strikethrough => Format {
            color: Some(palette.text),
            font: Some(geist(Weight::Normal, FontStyle::Normal)),
            strikethrough: Some(palette.text),
            ..Format::default()
        },
    }
}

impl Highlighter for MarkdownHighlighter {
    type Settings = Caret;
    type Highlight = MarkdownHighlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Self::Highlight)>;

    fn new(caret: &Self::Settings) -> Self {
        Self {
            current_line: 0,
            fences: vec![None],
            code: HashMap::new(),
            caret: *caret,
        }
    }

    fn update(&mut self, caret: &Self::Settings) {
        let changed_line = self.caret.line.min(caret.line);
        self.caret = *caret;
        self.change_line(changed_line);
    }

    fn change_line(&mut self, line: usize) {
        if line >= self.fences.len() {
            self.fences.truncate(1);
            self.code.clear();
            self.current_line = 0;
            return;
        }

        if let Some(fence) = self.fences[line]
            && let Some(code) = self.code.get_mut(&fence.start_line)
        {
            code.change_line(line.saturating_sub(fence.start_line + 1));
            let resume = fence.start_line + 1 + code.current_line();
            self.fences.truncate(resume + 1);
            self.code.retain(|start, _| *start <= fence.start_line);
            self.current_line = resume;
            return;
        }

        self.fences.truncate(line + 1);
        self.code.retain(|start, _| *start < line);
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let line_index = self.current_line;
        let fence = self.fences[line_index];
        let (highlights, next_fence) = highlight_line(
            line,
            line_index,
            fence,
            (line_index == self.caret.line).then_some(self.caret.column),
            &mut self.code,
        );

        self.current_line += 1;
        if self.fences.len() == self.current_line {
            self.fences.push(next_fence);
        } else {
            self.fences[self.current_line] = next_fence;
        }

        highlights.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

fn highlight_line(
    line: &str,
    line_index: usize,
    active_fence: Option<Fence>,
    caret: Option<usize>,
    code: &mut HashMap<usize, iced_highlighter::Highlighter>,
) -> (Vec<(Range<usize>, MarkdownHighlight)>, Option<Fence>) {
    let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
    let delimiter = fence_delimiter(&line[leading..]);

    if let Some(fence) = active_fence {
        let closes = delimiter.is_some_and(|candidate| {
            candidate.marker == fence.marker && candidate.length >= fence.length
        }) && delimiter
            .is_some_and(|candidate| line[leading + candidate.length..].trim().is_empty());

        if !closes {
            let mut highlights = vec![(0..line.len(), MarkdownHighlight::CodeBlock)];
            if let Some(highlighter) = code.get_mut(&fence.start_line) {
                highlights.extend(highlighter.highlight_line(line).map(|(range, highlight)| {
                    (
                        range,
                        MarkdownHighlight::CodeToken {
                            color: highlight.color(),
                            font: highlight.font(),
                        },
                    )
                }));
            }
            return (highlights, Some(fence));
        }

        return (
            vec![(
                0..line.len(),
                if closes {
                    if caret.is_some() {
                        MarkdownHighlight::Fence
                    } else {
                        MarkdownHighlight::HiddenFence
                    }
                } else {
                    MarkdownHighlight::CodeBlock
                },
            )],
            if closes { None } else { Some(fence) },
        );
    }

    if let Some(delimiter) = delimiter {
        let token = line[leading + delimiter.length..]
            .split_whitespace()
            .next()
            .unwrap_or("txt")
            .to_owned();
        code.insert(
            line_index,
            iced_highlighter::Highlighter::new(&iced_highlighter::Settings {
                theme: iced_highlighter::Theme::InspiredGitHub,
                token,
            }),
        );
        let fence = Fence {
            marker: delimiter.marker,
            length: delimiter.length,
            start_line: line_index,
        };
        return (
            vec![(
                leading..line.len(),
                if caret.is_some() {
                    MarkdownHighlight::Fence
                } else {
                    MarkdownHighlight::HiddenFence
                },
            )],
            Some(fence),
        );
    }

    (inline_highlights(line, caret), None)
}

fn fence_delimiter(line: &str) -> Option<Fence> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }

    let length = line.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then_some(Fence {
        marker,
        length,
        start_line: 0,
    })
}

fn inline_highlights(line: &str, caret: Option<usize>) -> Vec<(Range<usize>, MarkdownHighlight)> {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let events = Parser::new_ext(line, options)
        .into_offset_iter()
        .collect::<Vec<_>>();
    let mut active = Vec::<(TagEnd, MarkdownHighlight)>::new();
    let mut active_scopes = Vec::<Range<usize>>::new();
    let mut covered = Vec::<Range<usize>>::new();
    let mut highlights = Vec::new();

    if let Some((range, style)) = structural_marker(line) {
        covered.push(range.clone());
        highlights.push((range, style));
    }

    for (event, range) in events {
        match event {
            Event::Start(tag) => {
                if let Some(style) = tag_style(&tag) {
                    if caret.is_some_and(|caret| range.start <= caret && caret <= range.end) {
                        active_scopes.push(range);
                    }
                    active.push((tag.to_end(), style));
                }
            }
            Event::End(end) => {
                if let Some(index) = active.iter().rposition(|(tag, _)| *tag == end) {
                    active.remove(index);
                }
            }
            Event::Text(_) => push_content(
                range,
                active.last().map(|(_, style)| *style),
                &mut covered,
                &mut highlights,
            ),
            Event::Code(_) => {
                let content = delimited_content(line, range.clone(), b'`');
                if caret.is_some_and(|caret| content.start <= caret && caret <= content.end) {
                    active_scopes.push(range);
                }
                push_content(
                    content,
                    Some(MarkdownHighlight::InlineCode),
                    &mut covered,
                    &mut highlights,
                );
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                let content = delimited_content(line, range.clone(), b'$');
                if caret.is_some_and(|caret| content.start <= caret && caret <= content.end) {
                    active_scopes.push(range);
                }
                push_content(
                    content,
                    Some(MarkdownHighlight::InlineCode),
                    &mut covered,
                    &mut highlights,
                );
            }
            Event::Html(_) | Event::InlineHtml(_) => push_content(
                range,
                Some(MarkdownHighlight::InlineCode),
                &mut covered,
                &mut highlights,
            ),
            Event::FootnoteReference(_) => push_content(
                range,
                Some(MarkdownHighlight::Link),
                &mut covered,
                &mut highlights,
            ),
            Event::TaskListMarker(_) => push_content(
                range,
                Some(MarkdownHighlight::ListMarker),
                &mut covered,
                &mut highlights,
            ),
            Event::Rule => push_content(
                range,
                Some(MarkdownHighlight::Marker),
                &mut covered,
                &mut highlights,
            ),
            Event::SoftBreak | Event::HardBreak => covered.push(range),
        }
    }

    covered.sort_unstable_by_key(|range| range.start);
    let mut cursor = 0;
    for range in covered {
        if cursor < range.start {
            push_marker(cursor..range.start, &active_scopes, &mut highlights);
        }
        cursor = cursor.max(range.end);
    }
    if cursor < line.len() {
        push_marker(cursor..line.len(), &active_scopes, &mut highlights);
    }

    highlights.sort_unstable_by_key(|(range, _)| range.start);
    highlights
}

fn structural_marker(line: &str) -> Option<(Range<usize>, MarkdownHighlight)> {
    let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
    let rest = &line[leading..];
    let length = if rest.starts_with("> ")
        || rest.starts_with("- ")
        || rest.starts_with("+ ")
        || rest.starts_with("* ")
    {
        2
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && rest[digits..].starts_with(". ") {
            digits + 2
        } else {
            0
        }
    };

    (length > 0).then_some((leading..leading + length, MarkdownHighlight::ListMarker))
}

fn push_marker(
    range: Range<usize>,
    active_scopes: &[Range<usize>],
    highlights: &mut Vec<(Range<usize>, MarkdownHighlight)>,
) {
    let visible = active_scopes
        .iter()
        .any(|scope| scope.start <= range.start && range.end <= scope.end);
    highlights.push((
        range,
        if visible {
            MarkdownHighlight::Marker
        } else {
            MarkdownHighlight::HiddenMarker
        },
    ));
}

fn tag_style(tag: &Tag<'_>) -> Option<MarkdownHighlight> {
    match tag {
        Tag::Heading { level, .. } => Some(MarkdownHighlight::Heading(match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        })),
        Tag::BlockQuote(_) => Some(MarkdownHighlight::Quote),
        Tag::Strong => Some(MarkdownHighlight::Strong),
        Tag::Emphasis => Some(MarkdownHighlight::Emphasis),
        Tag::Strikethrough => Some(MarkdownHighlight::Strikethrough),
        Tag::Link { .. } | Tag::Image { .. } => Some(MarkdownHighlight::Link),
        _ => None,
    }
}

fn push_content(
    range: Range<usize>,
    style: Option<MarkdownHighlight>,
    covered: &mut Vec<Range<usize>>,
    highlights: &mut Vec<(Range<usize>, MarkdownHighlight)>,
) {
    if range.is_empty() {
        return;
    }
    covered.push(range.clone());
    if let Some(style) = style {
        highlights.push((range, style));
    }
}

fn delimited_content(line: &str, range: Range<usize>, marker: u8) -> Range<usize> {
    let bytes = line[range.clone()].as_bytes();
    let leading = bytes.iter().take_while(|byte| **byte == marker).count();
    let trailing = bytes
        .iter()
        .rev()
        .take_while(|byte| **byte == marker)
        .count();

    if leading > 0 && trailing >= leading {
        range.start + leading..range.end - leading
    } else {
        range
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditKind {
    Insert,
    Backspace,
    Delete,
    Other,
}

#[derive(Clone, Debug)]
enum ChangeData {
    Replace {
        start: Position,
        removed: String,
        inserted: String,
    },
    Snapshot {
        before: String,
        after: String,
    },
}

#[derive(Clone, Debug)]
struct Change {
    data: ChangeData,
    before: Cursor,
    after: Cursor,
    before_id: u64,
    after_id: u64,
    kind: EditKind,
    changed_at: Instant,
}

impl Change {
    fn bytes(&self) -> usize {
        match &self.data {
            ChangeData::Replace {
                removed, inserted, ..
            } => removed.len() + inserted.len(),
            ChangeData::Snapshot { before, after } => before.len() + after.len(),
        }
    }
}

#[derive(Debug)]
struct History {
    undo: Vec<Change>,
    redo: Vec<Change>,
    bytes: usize,
    current_id: u64,
    saved_id: u64,
    next_id: u64,
}

impl Default for History {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            bytes: 0,
            current_id: 0,
            saved_id: 0,
            next_id: 1,
        }
    }
}

impl History {
    fn record(&mut self, mut change: Change) {
        self.bytes -= self.redo.iter().map(Change::bytes).sum::<usize>();
        self.redo.clear();

        if self.current_id != self.saved_id
            && let Some(previous) = self.undo.last_mut()
            && coalesce(previous, &change)
        {
            previous.after_id = self.next_id;
            self.current_id = self.next_id;
            self.next_id = self.next_id.saturating_add(1);
            self.bytes += change.bytes();
            self.trim();
            return;
        }

        change.before_id = self.current_id;
        change.after_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.current_id = change.after_id;
        self.bytes += change.bytes();
        self.undo.push(change);
        self.trim();
    }

    fn trim(&mut self) {
        while self.undo.len() > HISTORY_LIMIT || self.bytes > HISTORY_BYTES {
            if self.undo.len() <= 1 {
                break;
            }
            self.bytes = self.bytes.saturating_sub(self.undo.remove(0).bytes());
        }
    }
}

// ponytail: one process-wide history matches this single-document example.
static HISTORY: LazyLock<Mutex<History>> = LazyLock::new(|| Mutex::new(History::default()));

fn history() -> MutexGuard<'static, History> {
    HISTORY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub fn test_history_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn track_action(content: &mut Content, action: Action) {
    let Action::Edit(edit) = &action else {
        content.perform(action);
        return;
    };

    if matches!(edit, Edit::Enter) && complete_fence(content) {
        return;
    }

    let before = content.cursor();
    if matches!(edit, Edit::Indent | Edit::Unindent) {
        // ponytail: Iced exposes no indent delta; snapshot only this rare block action.
        let before_text = content.text();
        content.perform(action);
        let change = Change {
            data: ChangeData::Snapshot {
                before: before_text,
                after: content.text(),
            },
            before,
            after: content.cursor(),
            before_id: 0,
            after_id: 0,
            kind: EditKind::Other,
            changed_at: Instant::now(),
        };
        history().record(change);
        return;
    }

    let Some((start, removed, inserted, kind)) = describe_edit(content, edit) else {
        content.perform(action);
        return;
    };
    content.perform(action);
    let change = Change {
        data: ChangeData::Replace {
            start,
            removed,
            inserted,
        },
        before,
        after: content.cursor(),
        before_id: 0,
        after_id: 0,
        kind,
        changed_at: Instant::now(),
    };
    history().record(change);
}

fn complete_fence(content: &mut Content) -> bool {
    let before = content.cursor();
    if before.selection.is_some() {
        return false;
    }

    let Some(line) = content.line(before.position.line) else {
        return false;
    };
    let text = line.text.into_owned();
    if before.position.column != text.len() || inside_fence_before(content, before.position.line) {
        return false;
    }

    let leading = text.len() - text.trim_start_matches([' ', '\t']).len();
    if leading > 3 {
        return false;
    }
    let Some(fence) = fence_delimiter(&text[leading..]) else {
        return false;
    };
    if fence.marker == b'`' && text[leading + fence.length..].contains('`') {
        return false;
    }
    if has_closing_fence_after(content, before.position.line, fence) {
        return false;
    }

    let ending = line_ending(content, before.position.line);
    let indent = &text[..leading];
    let marker = char::from(fence.marker).to_string().repeat(fence.length);
    let inserted = format!("{ending}{indent}{ending}{indent}{marker}");
    replace_range(content, before.position, before.position, &inserted);
    let after = Cursor {
        position: Position {
            line: before.position.line + 1,
            column: leading,
        },
        selection: None,
    };
    content.move_to(after);
    history().record(Change {
        data: ChangeData::Replace {
            start: before.position,
            removed: String::new(),
            inserted,
        },
        before,
        after,
        before_id: 0,
        after_id: 0,
        kind: EditKind::Other,
        changed_at: Instant::now(),
    });
    true
}

fn inside_fence_before(content: &Content, line: usize) -> bool {
    let mut active: Option<Fence> = None;

    for line_index in 0..line {
        let Some(line) = content.line(line_index) else {
            break;
        };
        let leading = line.text.len() - line.text.trim_start_matches([' ', '\t']).len();
        let Some(candidate) = fence_delimiter(&line.text[leading..]) else {
            continue;
        };

        if let Some(fence) = active {
            if candidate.marker == fence.marker
                && candidate.length >= fence.length
                && line.text[leading + candidate.length..].trim().is_empty()
            {
                active = None;
            }
        } else {
            active = Some(candidate);
        }
    }

    active.is_some()
}

fn has_closing_fence_after(content: &Content, line: usize, fence: Fence) -> bool {
    ((line + 1)..content.line_count()).any(|line_index| {
        let Some(line) = content.line(line_index) else {
            return false;
        };
        let leading = line.text.len() - line.text.trim_start_matches([' ', '\t']).len();
        fence_delimiter(&line.text[leading..]).is_some_and(|candidate| {
            candidate.marker == fence.marker
                && candidate.length >= fence.length
                && line.text[leading + candidate.length..].trim().is_empty()
        })
    })
}

fn describe_edit(content: &Content, edit: &Edit) -> Option<(Position, String, String, EditKind)> {
    let cursor = content.cursor();
    if let Some(anchor) = cursor.selection {
        return Some((
            min_position(cursor.position, anchor),
            content.selection().unwrap_or_default(),
            inserted_text(content, edit),
            edit_kind(edit),
        ));
    }

    let position = cursor.position;
    match edit {
        Edit::Insert(character) => Some((
            position,
            String::new(),
            character.to_string(),
            EditKind::Insert,
        )),
        Edit::Paste(text) => Some((
            position,
            String::new(),
            text.as_ref().clone(),
            EditKind::Other,
        )),
        Edit::Enter => Some((
            position,
            String::new(),
            line_ending(content, position.line).to_owned(),
            EditKind::Other,
        )),
        Edit::Backspace => removed_before(content, position)
            .map(|(start, removed)| (start, removed, String::new(), EditKind::Backspace)),
        Edit::Delete => removed_after(content, position)
            .map(|removed| (position, removed, String::new(), EditKind::Delete)),
        Edit::Indent | Edit::Unindent => None,
    }
}

fn inserted_text(content: &Content, edit: &Edit) -> String {
    match edit {
        Edit::Insert(character) => character.to_string(),
        Edit::Paste(text) => text.as_ref().clone(),
        Edit::Enter => line_ending(content, content.cursor().position.line).to_owned(),
        Edit::Backspace | Edit::Delete | Edit::Indent | Edit::Unindent => String::new(),
    }
}

fn edit_kind(edit: &Edit) -> EditKind {
    match edit {
        Edit::Insert(_) => EditKind::Insert,
        Edit::Backspace => EditKind::Backspace,
        Edit::Delete => EditKind::Delete,
        _ => EditKind::Other,
    }
}

fn removed_before(content: &Content, position: Position) -> Option<(Position, String)> {
    let line = content.line(position.line)?;
    if position.column > 0 {
        let start = line.text[..position.column].char_indices().next_back()?.0;
        return Some((
            Position {
                line: position.line,
                column: start,
            },
            line.text[start..position.column].to_owned(),
        ));
    }
    if position.line == 0 {
        return None;
    }
    let previous = content.line(position.line - 1)?;
    Some((
        Position {
            line: position.line - 1,
            column: previous.text.len(),
        },
        normalized_ending(previous.ending).to_owned(),
    ))
}

fn removed_after(content: &Content, position: Position) -> Option<String> {
    let line = content.line(position.line)?;
    if position.column < line.text.len() {
        let end = line.text[position.column..]
            .char_indices()
            .nth(1)
            .map_or(line.text.len(), |(offset, _)| position.column + offset);
        return Some(line.text[position.column..end].to_owned());
    }
    (position.line + 1 < content.line_count()).then(|| normalized_ending(line.ending).to_owned())
}

fn line_ending(content: &Content, line: usize) -> &'static str {
    content
        .line(line)
        .map_or("\n", |line| normalized_ending(line.ending))
}

fn normalized_ending(ending: iced::widget::text_editor::LineEnding) -> &'static str {
    let ending = ending.as_str();
    if ending.is_empty() { "\n" } else { ending }
}

fn coalesce(previous: &mut Change, next: &Change) -> bool {
    if previous.kind != next.kind
        || next.changed_at.duration_since(previous.changed_at) > COALESCE_WINDOW
    {
        return false;
    }
    let (
        ChangeData::Replace {
            start: previous_start,
            removed: previous_removed,
            inserted: previous_inserted,
        },
        ChangeData::Replace {
            start: next_start,
            removed: next_removed,
            inserted: next_inserted,
        },
    ) = (&mut previous.data, &next.data)
    else {
        return false;
    };

    let merged = match previous.kind {
        EditKind::Insert
            if previous_removed.is_empty()
                && next_removed.is_empty()
                && same_position(previous.after.position, *next_start)
                && !previous_inserted.ends_with(char::is_whitespace)
                && !next_inserted.starts_with(char::is_whitespace) =>
        {
            previous_inserted.push_str(next_inserted);
            true
        }
        EditKind::Backspace
            if previous_inserted.is_empty()
                && next_inserted.is_empty()
                && same_position(next.after.position, *previous_start) =>
        {
            *previous_start = *next_start;
            previous_removed.insert_str(0, next_removed);
            true
        }
        EditKind::Delete
            if previous_inserted.is_empty()
                && next_inserted.is_empty()
                && same_position(*previous_start, *next_start) =>
        {
            previous_removed.push_str(next_removed);
            true
        }
        _ => false,
    };

    if merged {
        previous.after = next.after;
        previous.changed_at = next.changed_at;
    }
    merged
}

pub fn reset_document(source: String) -> Content {
    *history() = History::default();
    Content::with_text(&source)
}

pub fn undo_document(mut content: Content) -> Content {
    {
        let mut history = history();
        let Some(change) = history.undo.pop() else {
            return content;
        };
        apply_change(&mut content, &change, false);
        history.current_id = change.before_id;
        history.redo.push(change);
    }
    content
}

pub fn redo_document(mut content: Content) -> Content {
    {
        let mut history = history();
        let Some(change) = history.redo.pop() else {
            return content;
        };
        apply_change(&mut content, &change, true);
        history.current_id = change.after_id;
        history.undo.push(change);
    }
    content
}

fn apply_change(content: &mut Content, change: &Change, forward: bool) {
    match &change.data {
        ChangeData::Replace {
            start,
            removed,
            inserted,
        } => {
            let (selected, replacement, cursor) = if forward {
                (removed, inserted, &change.after)
            } else {
                (inserted, removed, &change.before)
            };
            replace_range(
                content,
                *start,
                position_after(*start, selected),
                replacement,
            );
            content.move_to(*cursor);
        }
        ChangeData::Snapshot { before, after } => {
            *content = Content::with_text(if forward { after } else { before });
            content.move_to(if forward { change.after } else { change.before });
        }
    }
}

fn replace_range(content: &mut Content, start: Position, end: Position, replacement: &str) {
    content.move_to(Cursor {
        position: end,
        selection: Some(start),
    });
    content.perform(Action::Edit(if replacement.is_empty() {
        Edit::Delete
    } else {
        Edit::Paste(Arc::new(replacement.to_owned()))
    }));
}

pub fn format_document(mut content: Content, command: String) -> Content {
    let (prefix, suffix, select_suffix) = match command.as_str() {
        "bold" => ("**", "**", false),
        "italic" => ("*", "*", false),
        "code" => ("`", "`", false),
        "link" => ("[", "](https://)", true),
        _ => return content,
    };
    let before = content.cursor();
    if let Some((raw, inner)) = formatted_range(&content, &command) {
        let line = before.position.line;
        let Some(source) = content.line(line).map(|line| line.text.into_owned()) else {
            return content;
        };
        let removed = source[raw.clone()].to_owned();
        let inserted = source[inner.clone()].to_owned();
        let start = Position {
            line,
            column: raw.start,
        };
        replace_range(
            &mut content,
            start,
            Position {
                line,
                column: raw.end,
            },
            &inserted,
        );
        let map_position = |position: Position| Position {
            line,
            column: raw.start + position.column.saturating_sub(inner.start).min(inner.len()),
        };
        let after = Cursor {
            position: map_position(before.position),
            selection: before.selection.map(map_position),
        };
        content.move_to(after);
        history().record(Change {
            data: ChangeData::Replace {
                start,
                removed,
                inserted,
            },
            before,
            after,
            before_id: 0,
            after_id: 0,
            kind: EditKind::Other,
            changed_at: Instant::now(),
        });
        return content;
    }

    let start = before.selection.map_or(before.position, |anchor| {
        min_position(before.position, anchor)
    });
    let removed = content.selection().unwrap_or_default();
    let label = if command == "link" && removed.is_empty() {
        "link"
    } else {
        removed.as_str()
    };
    let inserted = format!("{prefix}{label}{suffix}");
    let selected_end = before.selection.map_or(before.position, |anchor| {
        max_position(before.position, anchor)
    });
    replace_range(&mut content, start, selected_end, &inserted);

    let inner_start = position_after(start, prefix);
    let inner_end = position_after(inner_start, label);
    let after = if select_suffix {
        let url_start = position_after(inner_end, "](");
        let url_end = position_after(url_start, "https://");
        Cursor {
            position: url_end,
            selection: Some(url_start),
        }
    } else if label.is_empty() {
        Cursor {
            position: inner_start,
            selection: None,
        }
    } else {
        Cursor {
            position: inner_end,
            selection: Some(inner_start),
        }
    };
    content.move_to(after);
    history().record(Change {
        data: ChangeData::Replace {
            start,
            removed,
            inserted,
        },
        before,
        after,
        before_id: 0,
        after_id: 0,
        kind: EditKind::Other,
        changed_at: Instant::now(),
    });
    content
}

fn formatted_range(content: &Content, command: &str) -> Option<(Range<usize>, Range<usize>)> {
    let cursor = content.cursor();
    let anchor = cursor.selection.unwrap_or(cursor.position);
    if cursor.position.line != anchor.line {
        return None;
    }

    let start = cursor.position.column.min(anchor.column);
    let end = cursor.position.column.max(anchor.column);
    let line = content.line(cursor.position.line)?;
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;

    Parser::new_ext(&line.text, options)
        .into_offset_iter()
        .filter_map(|(event, raw)| {
            let inner = match (&event, command) {
                (Event::Start(Tag::Strong), "bold") if raw.len() >= 4 => raw.start + 2..raw.end - 2,
                (Event::Start(Tag::Emphasis), "italic") if raw.len() >= 2 => {
                    raw.start + 1..raw.end - 1
                }
                (Event::Code(_), "code") => delimited_content(&line.text, raw.clone(), b'`'),
                _ => return None,
            };
            ((inner.start <= start && end <= inner.end) || (raw.start == start && end == raw.end))
                .then_some((raw, inner))
        })
        .min_by_key(|(raw, _)| raw.len())
}

pub fn find_document(mut content: Content, query: String, reverse: bool) -> Content {
    if query.is_empty() {
        return content;
    }
    let text = content.text();
    let cursor = global_offset(&content, content.cursor().position);
    let found = if reverse {
        text[..cursor.min(text.len())]
            .rfind(&query)
            .or_else(|| text.rfind(&query))
    } else {
        text[cursor.min(text.len())..]
            .find(&query)
            .map(|offset| cursor.min(text.len()) + offset)
            .or_else(|| text.find(&query))
    };
    if let Some(start) = found {
        content.move_to(Cursor {
            position: position_at(&content, start + query.len()),
            selection: Some(position_at(&content, start)),
        });
    }
    content
}

pub fn can_undo() -> bool {
    !history().undo.is_empty()
}

pub fn can_redo() -> bool {
    !history().redo.is_empty()
}

pub fn is_dirty() -> bool {
    let history = history();
    history.current_id != history.saved_id
}

pub fn revision() -> i64 {
    i64::try_from(history().current_id).unwrap_or(i64::MAX)
}

pub fn mark_saved(revision: i64) {
    let Ok(revision) = u64::try_from(revision) else {
        return;
    };
    let mut history = history();
    if history.current_id == revision {
        history.saved_id = revision;
    }
}

fn min_position(left: Position, right: Position) -> Position {
    if compare_position(left, right).is_gt() {
        right
    } else {
        left
    }
}

fn max_position(left: Position, right: Position) -> Position {
    if compare_position(left, right).is_lt() {
        right
    } else {
        left
    }
}

fn compare_position(left: Position, right: Position) -> Ordering {
    (left.line, left.column).cmp(&(right.line, right.column))
}

fn same_position(left: Position, right: Position) -> bool {
    left.line == right.line && left.column == right.column
}

fn position_after(mut position: Position, text: &str) -> Position {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if matches!(bytes[index], b'\r' | b'\n') {
            let first = bytes[index];
            index += 1;
            if index < bytes.len()
                && matches!((first, bytes[index]), (b'\r', b'\n') | (b'\n', b'\r'))
            {
                index += 1;
            }
            position.line += 1;
            position.column = 0;
        } else {
            let width = text[index..].chars().next().map_or(1, char::len_utf8);
            position.column += width;
            index += width;
        }
    }
    position
}

fn global_offset(content: &Content, position: Position) -> usize {
    let mut offset = 0;
    for index in 0..position.line {
        if let Some(line) = content.line(index) {
            offset += line.text.len() + normalized_ending(line.ending).len();
        }
    }
    offset + position.column
}

fn position_at(content: &Content, mut offset: usize) -> Position {
    for (line_index, line) in content.lines().enumerate() {
        if offset <= line.text.len() {
            return Position {
                line: line_index,
                column: offset,
            };
        }
        offset = offset.saturating_sub(line.text.len() + normalized_ending(line.ending).len());
    }
    let line = content.line_count().saturating_sub(1);
    Position {
        line,
        column: content.line(line).map_or(0, |line| line.text.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MarkdownHighlight, MarkdownHighlighter, can_redo, can_undo, format_document, is_dirty,
        mark_saved, redo_document, reset_document, revision, track_action, undo_document,
    };
    use iced::advanced::text::Highlighter;
    use iced::widget::text_editor::{Action, Edit};

    #[test]
    fn hides_markers_until_the_caret_enters_the_span() {
        let line = "as**df**";
        let mut outside = MarkdownHighlighter::new(&super::Caret { line: 0, column: 1 });
        let outside = outside.highlight_line(line).collect::<Vec<_>>();
        let mut inside = MarkdownHighlighter::new(&super::Caret { line: 0, column: 5 });
        let inside = inside.highlight_line(line).collect::<Vec<_>>();
        let mut marker = MarkdownHighlighter::new(&super::Caret { line: 0, column: 2 });
        let marker = marker.highlight_line(line).collect::<Vec<_>>();

        assert!(outside.iter().any(|(range, style)| {
            *range == (2..4) && *style == MarkdownHighlight::HiddenMarker
        }));
        assert!(
            inside
                .iter()
                .any(|(range, style)| { *range == (2..4) && *style == MarkdownHighlight::Marker })
        );
        assert!(
            inside
                .iter()
                .any(|(range, style)| { *range == (4..6) && *style == MarkdownHighlight::Strong })
        );
        assert!(
            marker
                .iter()
                .any(|(range, style)| { *range == (2..4) && *style == MarkdownHighlight::Marker })
        );
    }

    #[test]
    fn resumes_fenced_code_at_the_changed_line() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret { line: 0, column: 0 });
        for line in ["before", "```rust", "fn main() {}", "```", "after"] {
            let _ = highlighter.highlight_line(line).count();
        }
        highlighter.change_line(2);
        let changed = highlighter
            .highlight_line("let value = 1;")
            .collect::<Vec<_>>();

        assert_eq!(highlighter.current_line(), 3);
        assert_eq!(
            changed.first(),
            Some(&(0..14, MarkdownHighlight::CodeBlock))
        );
        assert!(
            changed
                .iter()
                .any(|(_, style)| matches!(style, MarkdownHighlight::CodeToken { .. }))
        );
    }

    #[test]
    fn gives_headings_distinct_metrics_and_code_blocks_a_surface() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret { line: 0, column: 0 });
        let headings = (1..=6)
            .map(|level| {
                let line = format!("{} heading", "#".repeat(level));
                highlighter
                    .highlight_line(&line)
                    .find_map(|(_, style)| match style {
                        MarkdownHighlight::Heading(level) => {
                            super::markdown_format(&style, &iced::Theme::Light)
                                .size
                                .map(|size| (level, size.0))
                        }
                        _ => None,
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            headings,
            [
                (1, 30.0),
                (2, 24.0),
                (3, 22.0),
                (4, 20.0),
                (5, 18.0),
                (6, 16.0)
            ]
        );
        let code = super::markdown_format(&MarkdownHighlight::CodeBlock, &iced::Theme::Light);
        assert!((code.size.unwrap().0 - 14.4).abs() < 0.01);
        let iced::advanced::text::LineHeight::Absolute(code_line_height) =
            code.line_height.unwrap()
        else {
            panic!("code line height must be absolute");
        };
        assert!((code_line_height.0 - 23.04).abs() < 0.01);
        assert_eq!(code.line_highlight.unwrap().border.width, 1.0);

        let inline = super::markdown_format(&MarkdownHighlight::InlineCode, &iced::Theme::Light);
        assert!((inline.size.unwrap().0 - 16.0).abs() < 0.01);
        assert!((inline.padding.top - 3.2).abs() < 0.01);
        assert!((inline.padding.left - 6.4).abs() < 0.01);
    }

    #[test]
    fn highlights_fenced_code_with_the_declared_language() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret { line: 0, column: 0 });
        let _ = highlighter.highlight_line("```rust").count();
        let code = highlighter
            .highlight_line("fn main() { let answer = 42; }")
            .collect::<Vec<_>>();

        assert!(code.iter().any(|(_, style)| {
            matches!(style, MarkdownHighlight::CodeToken { color: Some(_), .. })
        }));
    }

    #[test]
    fn platform_word_backspace_matches_the_standard_editor_keymap() {
        use iced::keyboard::key::{Code, Named, Physical};
        use iced::keyboard::{Key, Modifiers};
        use iced::widget::text_editor::{Binding, KeyPress, Motion, Status};

        let key = Key::Named(Named::Backspace);
        let word_modifier = if cfg!(target_os = "macos") {
            Modifiers::ALT
        } else {
            Modifiers::CTRL
        };
        let binding = Binding::<()>::from_key_press(KeyPress {
            key: key.clone(),
            modified_key: key,
            physical_key: Physical::Code(Code::Backspace),
            modifiers: word_modifier,
            text: None,
            status: Status::Focused { is_hovered: true },
        });

        assert_eq!(
            binding,
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::WordLeft),
                Binding::Backspace,
            ]))
        );
    }

    #[test]
    fn mixed_markdown_metrics_drive_native_caret_geometry() {
        use iced::advanced::text::editor::Editor as _;
        use iced::advanced::text::{LineHeight, Wrapping};
        use iced::widget::text_editor::{Cursor, Position};
        use iced::{Font, Pixels, Size};

        let mut editor = iced::advanced::graphics::text::Editor::with_text(
            "# Heading\n~~old~~\n\n```\ncode\n```",
        );
        let mut highlighter = MarkdownHighlighter::new(&super::Caret { line: 0, column: 0 });
        editor.update(
            Size::new(300.0, 200.0),
            Font::default(),
            Pixels(16.0),
            LineHeight::Absolute(Pixels(24.0)),
            Wrapping::None,
            &mut highlighter,
        );
        editor.highlight(Font::default(), &mut highlighter, |highlight| {
            super::markdown_format(highlight, &iced::Theme::Light)
        });

        assert_eq!(editor.caret_height(), Pixels(42.0));
        editor.move_to(Cursor {
            position: Position { line: 4, column: 0 },
            selection: None,
        });
        assert!((editor.caret_height().0 - 23.04).abs() < 0.01);
        let decorations = editor.decorations();
        assert!(decorations.iter().any(|decoration| {
            decoration.bounds.height == 1.0 && decoration.bounds.width > 0.0
        }));
        assert!(
            decorations
                .iter()
                .any(|decoration| (decoration.bounds.height - 55.04).abs() < 0.01)
        );
    }

    #[test]
    fn empty_fenced_code_line_keeps_code_metrics_and_one_surface() {
        use iced::advanced::text::editor::Editor as _;
        use iced::advanced::text::{LineHeight, Wrapping};
        use iced::widget::text_editor::{Cursor, Position};
        use iced::{Font, Pixels, Size};

        let mut editor = iced::advanced::graphics::text::Editor::with_text("```\n\n```");
        let mut highlighter = MarkdownHighlighter::new(&super::Caret { line: 1, column: 0 });
        editor.update(
            Size::new(300.0, 200.0),
            Font::default(),
            Pixels(16.0),
            LineHeight::Absolute(Pixels(25.6)),
            Wrapping::None,
            &mut highlighter,
        );
        editor.highlight(Font::default(), &mut highlighter, |highlight| {
            super::markdown_format(highlight, &iced::Theme::Light)
        });
        editor.move_to(Cursor {
            position: Position { line: 1, column: 0 },
            selection: None,
        });

        assert!((editor.caret_height().0 - 23.04).abs() < 0.01);
        assert!(
            editor
                .decorations()
                .iter()
                .any(|decoration| { (decoration.bounds.height - 55.04).abs() < 0.01 })
        );
    }

    #[test]
    fn resumes_near_the_end_of_a_large_document() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret { line: 0, column: 0 });
        for _ in 0..10_000 {
            let _ = highlighter.highlight_line("plain text").count();
        }
        highlighter.change_line(9_999);
        let _ = highlighter.highlight_line("**changed**").count();

        assert_eq!(highlighter.current_line(), 10_000);
        assert_eq!(highlighter.fences.len(), 10_001);
    }

    #[test]
    fn resumes_from_a_syntax_checkpoint_in_a_large_code_block() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret { line: 0, column: 0 });
        let _ = highlighter.highlight_line("```rust").count();
        for _ in 0..10_000 {
            let _ = highlighter.highlight_line("let value = 42;").count();
        }

        highlighter.change_line(9_999);

        assert!(highlighter.current_line() >= 9_950);
        assert!(highlighter.current_line() <= 9_999);
        assert!(
            highlighter
                .highlight_line("let changed = true;")
                .any(|(_, style)| matches!(style, MarkdownHighlight::CodeToken { .. }))
        );
    }

    #[test]
    fn produces_valid_ranges_for_unicode_and_incomplete_markup() {
        let line = "한글 **강조** and [unfinished";
        let mut highlighter = MarkdownHighlighter::new(&super::Caret { line: 0, column: 7 });

        for (range, _) in highlighter.highlight_line(line) {
            assert!(line.is_char_boundary(range.start));
            assert!(line.is_char_boundary(range.end));
            assert!(range.end <= line.len());
        }
    }

    #[test]
    fn undo_redo_tracks_deltas_and_saved_state() {
        let _lock = super::test_history_lock();
        let mut document = reset_document("hello".into());
        mark_saved(revision());
        track_action(&mut document, Action::Edit(Edit::Insert('!')));
        assert_eq!(document.text(), "!hello");
        assert!(is_dirty());
        assert!(can_undo());
        let stale_revision = revision();

        track_action(&mut document, Action::Edit(Edit::Insert('?')));
        mark_saved(stale_revision);
        assert_eq!(document.text(), "!?hello");
        assert!(is_dirty());

        document = undo_document(document);
        assert_eq!(document.text(), "hello");
        assert!(!is_dirty());
        assert!(can_redo());

        document = redo_document(document);
        assert_eq!(document.text(), "!?hello");
    }

    #[test]
    fn formatting_preserves_a_selected_edit_target() {
        let _lock = super::test_history_lock();
        let mut document = reset_document("text".into());
        document.move_to(iced::widget::text_editor::Cursor {
            position: iced::widget::text_editor::Position { line: 0, column: 4 },
            selection: Some(iced::widget::text_editor::Position { line: 0, column: 0 }),
        });
        let document = format_document(document, "bold".into());

        assert_eq!(document.text(), "**text**");
        assert_eq!(document.selection().as_deref(), Some("text"));
    }

    #[test]
    fn bold_command_toggles_existing_strong_markup() {
        let _lock = super::test_history_lock();
        let mut document = reset_document("**word**".into());
        document.move_to(iced::widget::text_editor::Cursor {
            position: iced::widget::text_editor::Position { line: 0, column: 4 },
            selection: None,
        });
        let document = format_document(document, "bold".into());

        assert_eq!(document.text(), "word");
        assert_eq!(document.cursor().position.column, 2);
    }

    #[test]
    fn enter_after_an_opening_fence_inserts_an_atomic_code_block() {
        let _lock = super::test_history_lock();
        let mut document = reset_document("```rust".into());
        document.move_to(iced::widget::text_editor::Cursor {
            position: iced::widget::text_editor::Position { line: 0, column: 7 },
            selection: None,
        });

        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "```rust\n\n```");
        assert_eq!(
            document.cursor().position,
            iced::widget::text_editor::Position { line: 1, column: 0 }
        );

        document = undo_document(document);
        assert_eq!(document.text(), "```rust");
    }

    #[test]
    fn enter_after_a_closing_fence_stays_a_plain_newline() {
        let _lock = super::test_history_lock();
        let mut document = reset_document("```\ncode\n```".into());
        document.move_to(iced::widget::text_editor::Cursor {
            position: iced::widget::text_editor::Position { line: 2, column: 3 },
            selection: None,
        });

        track_action(&mut document, Action::Edit(Edit::Enter));

        assert_eq!(document.text(), "```\ncode\n```\n");
    }

    #[test]
    fn enter_reuses_an_existing_closing_fence() {
        let _lock = super::test_history_lock();
        let mut document = reset_document("```rust\n```".into());
        document.move_to(iced::widget::text_editor::Cursor {
            position: iced::widget::text_editor::Position { line: 0, column: 7 },
            selection: None,
        });

        track_action(&mut document, Action::Edit(Edit::Enter));

        assert_eq!(document.text(), "```rust\n\n```");
        assert_eq!(document.cursor().position.line, 1);
    }
}

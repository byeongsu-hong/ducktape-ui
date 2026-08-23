use iced::advanced::text::{Highlight as TextHighlight, Highlighter, LineHeight};
use iced::font::{Family, Style as FontStyle, Weight};
use iced::widget::text_editor::{Action, Content, Cursor, Edit, Motion, Position};
use iced::{Border, Color, Element, Font, Padding, Pixels, Theme, mouse};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};
use ui_lang_runtime::rich_text_editor::{ContentVersion, EditorChange, Format, RichTextEditor};
use unicode_segmentation::UnicodeSegmentation;

pub use ui_lang_runtime::rich_text_editor::Action as RichEditorAction;

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
const INLINE_CODE_PADDING_X: f32 = BODY_SIZE * 0.4;
// Advance of the bundled Monoplex KR backtick, verified by the shaping test.
const MONOPLEX_MARKER_EM: f32 = 0.528;
const CODE_BACKGROUND_ALPHA: f32 = 0.08;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caret {
    line: usize,
    column: usize,
    dark: bool,
}

impl Caret {
    fn new(line: i64, column: i64, dark: bool) -> Self {
        Self {
            line: usize::try_from(line).unwrap_or_default(),
            column: usize::try_from(column).unwrap_or_default(),
            dark,
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
    Marker {
        hidden: bool,
        style: SpanStyle,
        marker_count: usize,
    },
    HiddenFence,
    Fence,
    Span(SpanStyle),
    CodeBlock,
    CodeToken {
        color: Option<Color>,
        font: Option<Font>,
    },
    ListMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpanStyle {
    heading: Option<u8>,
    strong: bool,
    emphasis: bool,
    code: bool,
    link: bool,
    quote: bool,
    strikethrough: bool,
}

impl SpanStyle {
    fn merge(mut self, other: Self) -> Self {
        self.heading = other.heading.or(self.heading);
        self.strong |= other.strong;
        self.emphasis |= other.emphasis;
        self.code |= other.code;
        self.link |= other.link;
        self.quote |= other.quote;
        self.strikethrough |= other.strikethrough;
        self
    }
}

pub fn markdown_editor<'a>(
    document: &'a Content,
    dark: bool,
    disabled: bool,
    focused: bool,
) -> Element<'a, RichEditorAction> {
    let cursor = document.cursor().position;
    let format_theme = if dark { Theme::Dark } else { Theme::Light };
    let (content_version, change_hint) = current_editor_state();
    let editor = RichTextEditor::new(document, content_version);
    let editor = if let Some(change) = change_hint {
        editor.change_hint(change)
    } else {
        editor
    };
    let editor = editor
        .id("markdown-editor")
        .placeholder("Start writing…")
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .focus_enabled(focused && !disabled)
        .min_height(320.0)
        .font(body_font(Weight::Normal, FontStyle::Normal))
        .size(BODY_SIZE)
        .line_height(BODY_LINE_HEIGHT)
        .wrapping(iced::advanced::text::Wrapping::Word)
        .padding(Padding {
            top: BODY_SIZE,
            right: CODE_BLOCK_PADDING,
            bottom: 0.0,
            left: CODE_BLOCK_PADDING,
        })
        .highlight_with::<MarkdownHighlighter>(
            Caret::new(cursor.line as i64, cursor.column as i64, dark),
            u64::from(dark),
            move |highlight| markdown_format(highlight, &format_theme),
        )
        .mouse_interaction(|line, position| {
            if crate::document::link_at(line, position.column).is_empty() {
                mouse::Interaction::Text
            } else {
                mouse::Interaction::Pointer
            }
        })
        .style(move |_theme, status| {
            let (value, muted, selection) = if dark {
                (
                    Color::from_rgb8(0xd7, 0xda, 0xe0),
                    Color::from_rgb8(0x9d, 0xa5, 0xb4),
                    Color::from_rgb8(0x46, 0x54, 0x74),
                )
            } else {
                (
                    Color::from_rgb8(0x29, 0x28, 0x24),
                    Color::from_rgb8(0x81, 0x7f, 0x77),
                    Color::from_rgb8(0xb9, 0xcd, 0xf4),
                )
            };
            iced::widget::text_editor::Style {
                background: Color::TRANSPARENT.into(),
                border: Border::default(),
                placeholder: muted,
                value: if matches!(status, iced::widget::text_editor::Status::Disabled) {
                    muted
                } else {
                    value
                },
                selection,
            }
        });

    if disabled {
        editor.into()
    } else {
        editor.on_action(|action| action).into()
    }
}

fn body_font(weight: Weight, style: FontStyle) -> Font {
    Font {
        family: Family::Name(if style == FontStyle::Italic {
            "IBM Plex Sans"
        } else {
            "IBM Plex Sans KR"
        }),
        weight,
        style,
        ..Font::DEFAULT
    }
}

fn code_font() -> Font {
    Font {
        family: Family::Name("Monoplex KR"),
        ..Font::DEFAULT
    }
}

fn markdown_format(highlight: &MarkdownHighlight, theme: &Theme) -> Format {
    let palette = theme.palette();
    let subdued = Color {
        a: 0.38,
        ..palette.text
    };

    let code_background = TextHighlight {
        background: Color {
            a: CODE_BACKGROUND_ALPHA,
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
    match *highlight {
        MarkdownHighlight::Marker {
            hidden,
            style,
            marker_count,
        } => {
            let mut format = span_format(style, theme);
            format.color = Some(if hidden { Color::TRANSPARENT } else { subdued });
            format.highlight = None;
            format.padding = Padding::ZERO;
            if hidden {
                if style.code {
                    format.size = Some(Pixels(
                        INLINE_CODE_PADDING_X / (MONOPLEX_MARKER_EM * marker_count.max(1) as f32),
                    ));
                    format.line_height =
                        Some(LineHeight::Absolute(Pixels(BODY_SIZE * BODY_LINE_HEIGHT)));
                } else {
                    format.size = Some(Pixels(0.01));
                    format.line_height = None;
                }
            }
            format
        }
        MarkdownHighlight::HiddenFence => Format {
            color: Some(Color::TRANSPARENT),
            font: Some(code_font()),
            size: Some(Pixels(0.01)),
            line_height: Some(LineHeight::Absolute(Pixels(BODY_SIZE))),
            line_highlight: Some(code_background),
            line_padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
            ..Format::default()
        },
        MarkdownHighlight::Fence => Format {
            color: Some(subdued),
            font: Some(code_font()),
            size: Some(Pixels(BODY_SIZE * CODE_BLOCK_SCALE)),
            line_height: Some(LineHeight::Absolute(Pixels(
                BODY_SIZE * CODE_BLOCK_SCALE * BODY_LINE_HEIGHT,
            ))),
            line_highlight: Some(code_background),
            line_padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
            ..Format::default()
        },
        MarkdownHighlight::Span(style) => span_format(style, theme),
        MarkdownHighlight::CodeBlock => Format {
            color: Some(palette.text),
            font: Some(code_font()),
            size: Some(Pixels(BODY_SIZE * CODE_BLOCK_SCALE)),
            line_height: Some(LineHeight::Absolute(Pixels(
                BODY_SIZE * CODE_BLOCK_SCALE * BODY_LINE_HEIGHT,
            ))),
            line_highlight: Some(code_background),
            line_padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
            ..Format::default()
        },
        MarkdownHighlight::CodeToken { color, font } => Format {
            color: Some(color.unwrap_or(palette.text)),
            font: Some(font.map_or_else(code_font, |font| Font {
                family: Family::Name("Monoplex KR"),
                ..font
            })),
            size: Some(Pixels(BODY_SIZE * CODE_BLOCK_SCALE)),
            line_height: Some(LineHeight::Absolute(Pixels(
                BODY_SIZE * CODE_BLOCK_SCALE * BODY_LINE_HEIGHT,
            ))),
            ..Format::default()
        },
        MarkdownHighlight::ListMarker => Format {
            color: Some(palette.text),
            font: Some(body_font(Weight::Normal, FontStyle::Normal)),
            ..Format::default()
        },
    }
}

fn span_format(style: SpanStyle, theme: &Theme) -> Format {
    let palette = theme.palette();
    let subdued = Color {
        a: 0.38,
        ..palette.text
    };
    let weight = if style.strong || style.heading.is_some() {
        Weight::Bold
    } else if style.link {
        Weight::Semibold
    } else {
        Weight::Normal
    };
    let font_style = if style.emphasis {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };
    let mut format = Format {
        color: Some(if style.link {
            palette.primary
        } else if style.quote {
            subdued
        } else {
            palette.text
        }),
        font: Some(if style.code {
            code_font()
        } else {
            body_font(weight, font_style)
        }),
        strikethrough: style.strikethrough.then_some(palette.text),
        ..Format::default()
    };

    if let Some(level) = style.heading {
        let size = BODY_SIZE * HEADING_SCALE[level as usize - 1];
        format.size = Some(Pixels(size));
        format.line_height = Some(LineHeight::Absolute(Pixels(size * HEADING_LINE_HEIGHT)));
    } else if style.code {
        let size = BODY_SIZE * INLINE_CODE_SCALE;
        format.size = Some(Pixels(size));
        format.line_height = Some(LineHeight::Absolute(Pixels(size * BODY_LINE_HEIGHT)));
        format.highlight = Some(TextHighlight {
            background: Color {
                a: CODE_BACKGROUND_ALPHA,
                ..palette.text
            }
            .into(),
            border: Border::default().rounded(3.0),
        });
        format.padding = Padding {
            top: 0.0,
            right: INLINE_CODE_PADDING_X,
            bottom: 0.0,
            left: INLINE_CODE_PADDING_X,
        };
    }

    format
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
        if self.caret.dark != caret.dark {
            self.caret = *caret;
            self.fences.truncate(1);
            self.code.clear();
            self.current_line = 0;
            return;
        }
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
            self.caret.dark,
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
    dark: bool,
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
                theme: if dark {
                    iced_highlighter::Theme::Base16Ocean
                } else {
                    iced_highlighter::Theme::InspiredGitHub
                },
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListKind {
    Bullet(u8),
    Task(u8),
    Ordered { number: u64, delimiter: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ListItem {
    indent: usize,
    marker: Range<usize>,
    content: usize,
    number: Option<Range<usize>>,
    kind: ListKind,
}

impl ListItem {
    fn next_prefix(&self, line: &str) -> String {
        let marker = match self.kind {
            ListKind::Bullet(marker) => format!("{} ", char::from(marker)),
            ListKind::Task(marker) => format!("{} [ ] ", char::from(marker)),
            ListKind::Ordered { number, delimiter } => {
                format!("{}{} ", number.saturating_add(1), char::from(delimiter))
            }
        };
        format!("{}{marker}", &line[..self.indent])
    }
}

fn list_item(line: &str) -> Option<ListItem> {
    let bytes = line.as_bytes();
    let indent = bytes
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let mut cursor = indent;
    let mut number = None;

    let kind = match *bytes.get(cursor)? {
        marker @ (b'-' | b'+' | b'*') => {
            cursor += 1;
            if !bytes
                .get(cursor)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                return None;
            }
            while bytes
                .get(cursor)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                cursor += 1;
            }
            if matches!(
                bytes.get(cursor..cursor + 3),
                Some(b"[ ]" | b"[x]" | b"[X]")
            ) && bytes
                .get(cursor + 3)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                cursor += 3;
                while bytes
                    .get(cursor)
                    .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
                {
                    cursor += 1;
                }
                ListKind::Task(marker)
            } else {
                ListKind::Bullet(marker)
            }
        }
        byte if byte.is_ascii_digit() => {
            let start = cursor;
            while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                cursor += 1;
            }
            let end = cursor;
            let delimiter = *bytes.get(cursor)?;
            if !matches!(delimiter, b'.' | b')') {
                return None;
            }
            cursor += 1;
            if !bytes
                .get(cursor)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                return None;
            }
            while bytes
                .get(cursor)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                cursor += 1;
            }
            let value = line[start..end].parse().ok()?;
            number = Some(start..end);
            ListKind::Ordered {
                number: value,
                delimiter,
            }
        }
        _ => return None,
    };

    Some(ListItem {
        indent,
        marker: indent..cursor,
        content: cursor,
        number,
        kind,
    })
}

fn inline_highlights(line: &str, caret: Option<usize>) -> Vec<(Range<usize>, MarkdownHighlight)> {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let events = Parser::new_ext(line, options)
        .into_offset_iter()
        .collect::<Vec<_>>();
    let mut active = Vec::<(TagEnd, SpanStyle)>::new();
    let mut scopes = Vec::<(Range<usize>, SpanStyle)>::new();
    let mut visible_scopes = Vec::<Range<usize>>::new();
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
                        visible_scopes.push(range.clone());
                    }
                    scopes.push((range.clone(), style));
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
                (!active.is_empty()).then(|| {
                    active
                        .iter()
                        .fold(SpanStyle::default(), |style, (_, next)| style.merge(*next))
                }),
                &mut covered,
                &mut highlights,
            ),
            Event::Code(_) => {
                let content = delimited_content(line, range.clone(), b'`');
                if caret.is_some_and(|caret| range.start <= caret && caret <= range.end) {
                    visible_scopes.push(range.clone());
                }
                let style = SpanStyle {
                    code: true,
                    ..SpanStyle::default()
                };
                scopes.push((range, style));
                push_content(content, Some(style), &mut covered, &mut highlights);
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                let content = delimited_content(line, range.clone(), b'$');
                if caret.is_some_and(|caret| range.start <= caret && caret <= range.end) {
                    visible_scopes.push(range.clone());
                }
                let style = SpanStyle {
                    code: true,
                    ..SpanStyle::default()
                };
                scopes.push((range, style));
                push_content(content, Some(style), &mut covered, &mut highlights);
            }
            Event::Html(_) | Event::InlineHtml(_) => push_content(
                range,
                Some(SpanStyle {
                    code: true,
                    ..SpanStyle::default()
                }),
                &mut covered,
                &mut highlights,
            ),
            Event::FootnoteReference(_) => push_content(
                range,
                Some(SpanStyle {
                    link: true,
                    ..SpanStyle::default()
                }),
                &mut covered,
                &mut highlights,
            ),
            Event::TaskListMarker(_) => {
                covered.push(range.clone());
                highlights.push((range, MarkdownHighlight::ListMarker));
            }
            Event::Rule => {
                let marker_count = range.len();
                covered.push(range.clone());
                highlights.push((
                    range,
                    MarkdownHighlight::Marker {
                        hidden: false,
                        style: SpanStyle::default(),
                        marker_count,
                    },
                ));
            }
            Event::SoftBreak | Event::HardBreak => covered.push(range),
        }
    }

    covered.sort_unstable_by_key(|range| range.start);
    let mut cursor = 0;
    for range in covered {
        if cursor < range.start {
            push_uncovered(
                line,
                cursor..range.start,
                &scopes,
                &visible_scopes,
                &mut highlights,
            );
        }
        cursor = cursor.max(range.end);
    }
    if cursor < line.len() {
        push_uncovered(
            line,
            cursor..line.len(),
            &scopes,
            &visible_scopes,
            &mut highlights,
        );
    }

    highlights.sort_unstable_by_key(|(range, _)| range.start);
    highlights
}

fn structural_marker(line: &str) -> Option<(Range<usize>, MarkdownHighlight)> {
    let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
    let rest = &line[leading..];
    if rest.starts_with("> ") {
        return Some((
            leading..leading + 2,
            MarkdownHighlight::Span(SpanStyle {
                quote: true,
                ..SpanStyle::default()
            }),
        ));
    }

    list_item(line).map(|item| (item.marker, MarkdownHighlight::ListMarker))
}

fn push_uncovered(
    line: &str,
    range: Range<usize>,
    scopes: &[(Range<usize>, SpanStyle)],
    visible_scopes: &[Range<usize>],
    highlights: &mut Vec<(Range<usize>, MarkdownHighlight)>,
) {
    if line[range.clone()].chars().all(char::is_whitespace) {
        let style = scopes
            .iter()
            .filter(|(scope, _)| scope.start <= range.start && range.end <= scope.end)
            .fold(SpanStyle::default(), |style, (_, next)| style.merge(*next));
        if style != SpanStyle::default() {
            highlights.push((range, MarkdownHighlight::Span(style)));
        }
        return;
    }

    push_marker(range, scopes, visible_scopes, highlights);
}

fn push_marker(
    range: Range<usize>,
    scopes: &[(Range<usize>, SpanStyle)],
    visible_scopes: &[Range<usize>],
    highlights: &mut Vec<(Range<usize>, MarkdownHighlight)>,
) {
    let marker_count = range.len();
    let hidden = !visible_scopes
        .iter()
        .any(|scope| scope.start <= range.start && range.end <= scope.end);
    let style = scopes
        .iter()
        .filter(|(scope, _)| scope.start <= range.start && range.end <= scope.end)
        .fold(SpanStyle::default(), |style, (_, next)| style.merge(*next));
    highlights.push((
        range,
        MarkdownHighlight::Marker {
            hidden,
            style,
            marker_count,
        },
    ));
}

fn tag_style(tag: &Tag<'_>) -> Option<SpanStyle> {
    match tag {
        Tag::Heading { level, .. } => Some(SpanStyle {
            heading: Some(match level {
                HeadingLevel::H1 => 1,
                HeadingLevel::H2 => 2,
                HeadingLevel::H3 => 3,
                HeadingLevel::H4 => 4,
                HeadingLevel::H5 => 5,
                HeadingLevel::H6 => 6,
            }),
            ..SpanStyle::default()
        }),
        Tag::BlockQuote(_) => Some(SpanStyle {
            quote: true,
            ..SpanStyle::default()
        }),
        Tag::Strong => Some(SpanStyle {
            strong: true,
            ..SpanStyle::default()
        }),
        Tag::Emphasis => Some(SpanStyle {
            emphasis: true,
            ..SpanStyle::default()
        }),
        Tag::Strikethrough => Some(SpanStyle {
            strikethrough: true,
            ..SpanStyle::default()
        }),
        Tag::Link { .. } | Tag::Image { .. } => Some(SpanStyle {
            link: true,
            ..SpanStyle::default()
        }),
        _ => None,
    }
}

fn push_content(
    range: Range<usize>,
    style: Option<SpanStyle>,
    covered: &mut Vec<Range<usize>>,
    highlights: &mut Vec<(Range<usize>, MarkdownHighlight)>,
) {
    if range.is_empty() {
        return;
    }
    covered.push(range.clone());
    if let Some(style) = style {
        highlights.push((range, MarkdownHighlight::Span(style)));
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
    Batch(Vec<Replacement>),
    Snapshot {
        before: String,
        after: String,
    },
}

#[derive(Clone, Debug)]
struct Replacement {
    start: Position,
    removed: String,
    inserted: String,
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
            ChangeData::Batch(replacements) => replacements
                .iter()
                .map(|replacement| replacement.removed.len() + replacement.inserted.len())
                .sum(),
            ChangeData::Snapshot { before, after } => before.len() + after.len(),
        }
    }
}

#[derive(Debug)]
struct History {
    undo: Vec<Change>,
    redo: Vec<Change>,
    bytes: usize,
    document_id: u64,
    current_id: u64,
    saved_id: u64,
    next_id: u64,
    pending_change: Option<EditorChange>,
}

impl Default for History {
    fn default() -> Self {
        static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            bytes: 0,
            document_id: NEXT_DOCUMENT_ID.fetch_add(1, AtomicOrdering::Relaxed),
            current_id: 0,
            saved_id: 0,
            next_id: 1,
            pending_change: None,
        }
    }
}

impl History {
    fn record(&mut self, mut change: Change) {
        self.bytes -= self.redo.iter().map(Change::bytes).sum::<usize>();
        self.redo.clear();

        let before = self.content_version();
        let after_id = self.next_id;
        let after = ContentVersion::new(self.document_id, after_id);
        let pending_change = change.data.editor_change(before, after, true);

        if self.current_id != self.saved_id
            && let Some(previous) = self.undo.last_mut()
            && coalesce(previous, &change)
        {
            previous.after_id = after_id;
            self.current_id = after_id;
            self.next_id = self.next_id.saturating_add(1);
            self.pending_change = pending_change;
            self.bytes += change.bytes();
            self.trim();
            return;
        }

        change.before_id = self.current_id;
        change.after_id = after_id;
        self.next_id = self.next_id.saturating_add(1);
        self.current_id = change.after_id;
        self.pending_change = pending_change;
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

    fn content_version(&self) -> ContentVersion {
        ContentVersion::new(self.document_id, self.current_id)
    }
}

impl ChangeData {
    fn editor_change(
        &self,
        from: ContentVersion,
        to: ContentVersion,
        forward: bool,
    ) -> Option<EditorChange> {
        let Self::Replace {
            start,
            removed,
            inserted,
        } = self
        else {
            // Batch edits and full snapshots deliberately use exact discovery;
            // their independently positioned replacements do not form one
            // trustworthy contiguous line span.
            return None;
        };
        let (removed, inserted) = if forward {
            (removed, inserted)
        } else {
            (inserted, removed)
        };
        Some(EditorChange::new(
            from,
            to,
            start.line,
            logical_line_span(removed),
            logical_line_span(inserted),
        ))
    }
}

fn logical_line_span(text: &str) -> usize {
    position_after(Position { line: 0, column: 0 }, text)
        .line
        .saturating_add(1)
}

thread_local! {
    static HISTORY: RefCell<History> = RefCell::new(History::default());
}

fn with_history<T>(f: impl FnOnce(&History) -> T) -> T {
    HISTORY.with(|history| f(&history.borrow()))
}

fn with_history_mut<T>(f: impl FnOnce(&mut History) -> T) -> T {
    HISTORY.with(|history| f(&mut history.borrow_mut()))
}

fn record_change(change: Change) {
    with_history_mut(|history| history.record(change));
}

#[cfg(not(test))]
fn change_time() -> Instant {
    Instant::now()
}

#[cfg(test)]
fn change_time() -> Instant {
    with_history(|history| {
        history.undo.last().map_or_else(Instant::now, |change| {
            change.changed_at + Duration::from_millis(1)
        })
    })
}

pub fn track_action(content: &mut Content, action: Action) {
    let Action::Edit(edit) = &action else {
        content.perform(action);
        return;
    };

    if matches!(edit, Edit::Enter) && complete_fence(content) {
        return;
    }

    if matches!(edit, Edit::Enter) && continue_list(content) {
        return;
    }
    if matches!(edit, Edit::Backspace) && remove_list_marker(content) {
        return;
    }
    if matches!(edit, Edit::Indent | Edit::Unindent) && edit_list_indent(content, edit) {
        return;
    }

    let before = content.cursor();
    if matches!(edit, Edit::Indent | Edit::Unindent) {
        let before_text = content.text();
        edit_plain_indent(content, edit);
        let after_text = content.text();
        if before_text == after_text {
            return;
        }
        let change = Change {
            data: ChangeData::Snapshot {
                before: before_text,
                after: after_text,
            },
            before,
            after: content.cursor(),
            before_id: 0,
            after_id: 0,
            kind: EditKind::Other,
            changed_at: change_time(),
        };
        record_change(change);
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
        changed_at: change_time(),
    };
    record_change(change);
}

pub fn apply_rich_action(mut content: Content, action: RichEditorAction) -> Content {
    match action {
        RichEditorAction::Edit(action) => track_action(&mut content, action),
        RichEditorAction::MoveTo(cursor) => {
            if cursor.selection.is_none() {
                content = clear_editor_selection(content);
            }
            content.move_to(cursor);
        }
    }
    content
}

pub fn clear_editor_selection(mut content: Content) -> Content {
    let cursor = content.cursor();
    if cursor.selection.is_some() {
        content.perform(Action::Move(Motion::Left));
        content.move_to(Cursor {
            position: cursor.position,
            selection: None,
        });
    }
    content
}

fn continue_list(content: &mut Content) -> bool {
    let before = content.cursor();
    if before.selection.is_some() {
        return false;
    }
    let Some(line) = content.line(before.position.line) else {
        return false;
    };
    let text = line.text.into_owned();
    let Some(item) = list_item(&text) else {
        return false;
    };
    if before.position.column < item.content
        || before.position.column > text.len()
        || inside_fence_before(content, before.position.line)
    {
        return false;
    }

    if text[item.content..].trim().is_empty() {
        let inserted =
            parent_list_prefix(content, before.position.line, item.indent).unwrap_or_default();
        let start = Position {
            line: before.position.line,
            column: 0,
        };
        let end = Position {
            line: before.position.line,
            column: text.len(),
        };
        replace_range(content, start, end, &inserted);
        let after = Cursor {
            position: Position {
                line: before.position.line,
                column: inserted.len(),
            },
            selection: None,
        };
        let mut replacements = vec![Replacement {
            start,
            removed: text,
            inserted,
        }];
        renumber_list(content, before.position.line, None, &mut replacements);
        content.move_to(after);
        record_change(Change {
            data: replacement_change(replacements),
            before,
            after,
            before_id: 0,
            after_id: 0,
            kind: EditKind::Other,
            changed_at: change_time(),
        });
        return true;
    }

    let start_column = text[..before.position.column]
        .trim_end_matches([' ', '\t'])
        .len();
    let start = Position {
        line: before.position.line,
        column: start_column,
    };
    let removed = text[start_column..before.position.column].to_owned();
    let inserted = format!(
        "{}{}",
        normalized_ending(line.ending),
        item.next_prefix(&text)
    );
    replace_range(content, start, before.position, &inserted);
    let after = Cursor {
        position: position_after(start, &inserted),
        selection: None,
    };
    content.move_to(after);

    let mut replacements = vec![Replacement {
        start,
        removed,
        inserted,
    }];
    renumber_list(content, before.position.line + 1, None, &mut replacements);
    content.move_to(after);
    record_change(Change {
        data: replacement_change(replacements),
        before,
        after,
        before_id: 0,
        after_id: 0,
        kind: EditKind::Other,
        changed_at: change_time(),
    });
    true
}

fn parent_list_prefix(content: &Content, line: usize, indent: usize) -> Option<String> {
    list_items_above(content, line)
        .find(|(_, item)| item.indent < indent)
        .map(|(text, item)| item.next_prefix(&text))
}

/// The list items above `line` in the same block, nearest first; a blank line
/// ends the block and plain lines are skipped.
fn list_items_above(content: &Content, line: usize) -> impl Iterator<Item = (String, ListItem)> {
    (0..line)
        .rev()
        .map_while(|index| content.line(index).map(|line| line.text.into_owned()))
        .take_while(|text| !text.trim().is_empty())
        .filter_map(|text| list_item(&text).map(|item| (text, item)))
}

fn replacement_change(mut replacements: Vec<Replacement>) -> ChangeData {
    if replacements.len() == 1 {
        let replacement = replacements.pop().expect("one list replacement");
        ChangeData::Replace {
            start: replacement.start,
            removed: replacement.removed,
            inserted: replacement.inserted,
        }
    } else {
        ChangeData::Batch(replacements)
    }
}

/// Renumbers every ordered item in the list block around `line`: siblings at
/// one indent count up from the run's first number, a bullet or a shallower
/// item ends the run, and `restart` is the item just nested, which starts at 1.
/// The block is the list items around `line` with any blank lines between
/// them, so a loose list is one block; the edited line itself is skipped when
/// it is no longer an item, since a plain line there still renders as part of
/// the list.
fn renumber_list(
    content: &mut Content,
    line: usize,
    restart: Option<usize>,
    replacements: &mut Vec<Replacement>,
) {
    let first = (0..line)
        .rev()
        .take_while(|&index| {
            content
                .line(index)
                .is_some_and(|line| line.text.trim().is_empty() || list_item(&line.text).is_some())
        })
        .last()
        .unwrap_or(line);
    let mut runs: Vec<(usize, u64)> = Vec::new();
    for index in first..content.line_count() {
        let Some(text) = content.line(index).map(|line| line.text.into_owned()) else {
            break;
        };
        let Some(item) = list_item(&text) else {
            if index == line || text.trim().is_empty() {
                continue;
            }
            break;
        };
        let ListKind::Ordered { number, .. } = item.kind else {
            runs.retain(|&(indent, _)| indent < item.indent);
            continue;
        };
        runs.retain(|&(indent, _)| indent <= item.indent);
        let number = match runs.last_mut() {
            Some((indent, last)) if *indent == item.indent => {
                *last = last.saturating_add(1);
                *last
            }
            _ => {
                let number = if restart == Some(index) { 1 } else { number };
                runs.push((item.indent, number));
                number
            }
        };
        let range = item.number.expect("ordered lists have a number");
        let inserted = number.to_string();
        if text[range.clone()] == inserted {
            continue;
        }
        let replacement = Replacement {
            start: Position {
                line: index,
                column: range.start,
            },
            removed: text[range.clone()].to_owned(),
            inserted,
        };
        replace_range(
            content,
            replacement.start,
            Position {
                line: index,
                column: range.end,
            },
            &replacement.inserted,
        );
        replacements.push(replacement);
    }
}

fn remove_list_marker(content: &mut Content) -> bool {
    let before = content.cursor();
    if before.selection.is_some() {
        return false;
    }
    let Some(line) = content.line(before.position.line) else {
        return false;
    };
    let text = line.text.into_owned();
    let Some(item) = list_item(&text) else {
        return false;
    };
    if before.position.column != item.content || inside_fence_before(content, before.position.line)
    {
        return false;
    }
    let inserted = text[..item.indent].to_owned();
    let removed = text[..item.content].to_owned();
    let start = Position {
        line: before.position.line,
        column: 0,
    };
    replace_range(content, start, before.position, &inserted);
    let after = Cursor {
        position: Position {
            line: before.position.line,
            column: inserted.len(),
        },
        selection: None,
    };
    let mut replacements = vec![Replacement {
        start,
        removed,
        inserted,
    }];
    renumber_list(content, before.position.line, None, &mut replacements);
    content.move_to(after);
    record_change(Change {
        data: replacement_change(replacements),
        before,
        after,
        before_id: 0,
        after_id: 0,
        kind: EditKind::Other,
        changed_at: change_time(),
    });
    true
}

fn edit_list_indent(content: &mut Content, edit: &Edit) -> bool {
    let before = content.cursor();
    if before.selection.is_some() {
        return false;
    }
    let Some(line) = content.line(before.position.line) else {
        return false;
    };
    let text = line.text.into_owned();
    let Some(item) = list_item(&text) else {
        return false;
    };
    if inside_fence_before(content, before.position.line) {
        return false;
    }

    let line = before.position.line;
    let (removed, inserted) = match edit {
        Edit::Indent => {
            let Some(content_column) = list_items_above(content, line)
                .take_while(|(_, sibling)| sibling.indent >= item.indent)
                .find(|(_, sibling)| sibling.indent == item.indent)
                .map(|(_, sibling)| sibling.content)
            else {
                return false;
            };
            (String::new(), " ".repeat(content_column - item.indent))
        }
        Edit::Unindent if item.indent > 0 => {
            let ancestor = list_items_above(content, line)
                .find(|(_, ancestor)| ancestor.indent < item.indent)
                .map_or(0, |(_, ancestor)| ancestor.indent);
            (text[..item.indent - ancestor].to_owned(), String::new())
        }
        Edit::Unindent => return true,
        _ => return false,
    };
    let start = Position {
        line: before.position.line,
        column: 0,
    };
    let end = Position {
        line: before.position.line,
        column: removed.len(),
    };
    replace_range(content, start, end, &inserted);
    let delta = inserted.len() as isize - removed.len() as isize;
    let after = Cursor {
        position: Position {
            line: before.position.line,
            column: before.position.column.saturating_add_signed(delta),
        },
        selection: None,
    };
    let mut replacements = vec![Replacement {
        start,
        removed,
        inserted,
    }];
    renumber_list(
        content,
        line,
        matches!(edit, Edit::Indent).then_some(line),
        &mut replacements,
    );
    content.move_to(after);
    record_change(Change {
        data: replacement_change(replacements),
        before,
        after,
        before_id: 0,
        after_id: 0,
        kind: EditKind::Other,
        changed_at: change_time(),
    });
    true
}

fn edit_plain_indent(content: &mut Content, edit: &Edit) {
    const TAB_WIDTH: usize = 4;

    let before = content.cursor();
    let (start, end) = before
        .selection
        .map_or((before.position, before.position), |anchor| {
            (
                min_position(before.position, anchor),
                max_position(before.position, anchor),
            )
        });
    let has_selection = before.selection.is_some();
    let mut replacements = Vec::new();

    for line_index in start.line..=end.line {
        let Some(line) = content.line(line_index) else {
            break;
        };
        let text = line.text;

        match edit {
            Edit::Indent if !has_selection => {
                let column = before.position.column.min(text.len());
                let trailing_whitespace = text[..column]
                    .chars()
                    .rev()
                    .take_while(|character| character.is_whitespace())
                    .count();
                let width = TAB_WIDTH - trailing_whitespace % TAB_WIDTH;
                replacements.push(Replacement {
                    start: Position {
                        line: line_index,
                        column,
                    },
                    removed: String::new(),
                    inserted: " ".repeat(width),
                });
            }
            Edit::Indent => {
                let leading = text
                    .char_indices()
                    .find(|(_, character)| !character.is_whitespace())
                    .map_or(text.len(), |(index, _)| index);
                let leading_characters = text[..leading].chars().count();
                let width = TAB_WIDTH - leading_characters % TAB_WIDTH;
                replacements.push(Replacement {
                    start: Position {
                        line: line_index,
                        column: leading,
                    },
                    removed: String::new(),
                    inserted: " ".repeat(width),
                });
            }
            Edit::Unindent => {
                let leading = text
                    .char_indices()
                    .find(|(_, character)| !character.is_whitespace())
                    .map_or(text.len(), |(index, _)| index);
                if leading == 0 {
                    continue;
                }
                let leading_characters = text[..leading].chars().count();
                let remove_characters = if leading_characters % TAB_WIDTH == 0 {
                    TAB_WIDTH.min(leading_characters)
                } else {
                    leading_characters % TAB_WIDTH
                };
                let remove_start = text[..leading]
                    .char_indices()
                    .rev()
                    .nth(remove_characters.saturating_sub(1))
                    .map_or(0, |(index, _)| index);
                replacements.push(Replacement {
                    start: Position {
                        line: line_index,
                        column: remove_start,
                    },
                    removed: text[remove_start..leading].to_owned(),
                    inserted: String::new(),
                });
            }
            _ => return,
        }
    }

    let map_position = |mut position: Position| {
        for replacement in &replacements {
            if position.line != replacement.start.line {
                continue;
            }
            let removed_end = replacement.start.column + replacement.removed.len();
            position.column = if position.column < replacement.start.column {
                position.column
            } else if position.column <= removed_end {
                replacement.start.column + replacement.inserted.len()
            } else {
                position.column.saturating_add_signed(
                    replacement.inserted.len() as isize - replacement.removed.len() as isize,
                )
            };
        }
        position
    };
    let after = Cursor {
        position: map_position(before.position),
        selection: before.selection.map(map_position),
    };

    for replacement in replacements.iter().rev() {
        replace_range(
            content,
            replacement.start,
            position_after(replacement.start, &replacement.removed),
            &replacement.inserted,
        );
    }
    content.move_to(after);
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
    if before.position.column != text.len() {
        return false;
    }

    let leading = text.len() - text.trim_start_matches([' ', '\t']).len();
    if leading > 3 {
        return false;
    }
    let Some(fence) = fence_delimiter(&text[leading..]) else {
        return false;
    };
    if inside_fence_before(content, before.position.line) {
        return false;
    }
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
    record_change(Change {
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
        changed_at: change_time(),
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
        return line.text[position.column..]
            .graphemes(true)
            .next()
            .map(str::to_owned);
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
    with_history_mut(|history| *history = History::default());
    Content::with_text(&source)
}

pub fn undo_document(mut content: Content) -> Content {
    let changed = with_history_mut(|history| {
        let Some(change) = history.undo.pop() else {
            return false;
        };
        let from = history.content_version();
        let to = ContentVersion::new(history.document_id, change.before_id);
        let pending_change = change.data.editor_change(from, to, false);
        apply_change(&mut content, &change, false);
        history.current_id = change.before_id;
        history.pending_change = pending_change;
        history.redo.push(change);
        true
    });
    if !changed {
        return content;
    }
    content
}

pub fn redo_document(mut content: Content) -> Content {
    let changed = with_history_mut(|history| {
        let Some(change) = history.redo.pop() else {
            return false;
        };
        let from = history.content_version();
        let to = ContentVersion::new(history.document_id, change.after_id);
        let pending_change = change.data.editor_change(from, to, true);
        apply_change(&mut content, &change, true);
        history.current_id = change.after_id;
        history.pending_change = pending_change;
        history.undo.push(change);
        true
    });
    if !changed {
        return content;
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
        ChangeData::Batch(replacements) => {
            if forward {
                for replacement in replacements {
                    apply_replacement(content, replacement, true);
                }
                content.move_to(change.after);
            } else {
                for replacement in replacements.iter().rev() {
                    apply_replacement(content, replacement, false);
                }
                content.move_to(change.before);
            }
        }
        ChangeData::Snapshot { before, after } => {
            *content = Content::with_text(if forward { after } else { before });
            content.move_to(if forward { change.after } else { change.before });
        }
    }
}

fn apply_replacement(content: &mut Content, replacement: &Replacement, forward: bool) {
    let (selected, inserted) = if forward {
        (&replacement.removed, &replacement.inserted)
    } else {
        (&replacement.inserted, &replacement.removed)
    };
    replace_range(
        content,
        replacement.start,
        position_after(replacement.start, selected),
        inserted,
    );
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
        record_change(Change {
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
            changed_at: change_time(),
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
    record_change(Change {
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
        changed_at: change_time(),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorStatus {
    pub can_undo: bool,
    pub can_redo: bool,
    pub dirty: bool,
    pub revision: i64,
}

pub fn editor_status() -> EditorStatus {
    with_history(editor_status_from)
}

fn editor_status_from(history: &History) -> EditorStatus {
    EditorStatus {
        can_undo: !history.undo.is_empty(),
        can_redo: !history.redo.is_empty(),
        dirty: history.current_id != history.saved_id,
        revision: i64::try_from(history.current_id).unwrap_or(i64::MAX),
    }
}

#[cfg(test)]
fn current_content_version() -> ContentVersion {
    current_editor_state().0
}

fn current_editor_state() -> (ContentVersion, Option<EditorChange>) {
    with_history(|history| (history.content_version(), history.pending_change))
}

pub fn mark_saved(revision: i64) -> EditorStatus {
    with_history_mut(|history| {
        if let Ok(revision) = u64::try_from(revision)
            && history.current_id == revision
        {
            history.saved_id = revision;
        }
        editor_status_from(history)
    })
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
        MarkdownHighlight, MarkdownHighlighter, clear_editor_selection, current_content_version,
        current_editor_state, editor_status, format_document, inline_highlights, mark_saved,
        redo_document, reset_document, track_action, undo_document,
    };
    use iced::advanced::text::Highlighter;
    use iced::widget::text_editor::{Action, Content, Cursor, Edit, Motion, Position};
    use std::sync::Arc;

    #[test]
    fn keeps_unparsed_whitespace_at_body_metrics() {
        for line in ["안녕하세요, ", "plain text ", "안녕하세요,\t"] {
            let whitespace_start = line.trim_end_matches(char::is_whitespace).len();
            for caret in [None, Some(whitespace_start), Some(line.len())] {
                assert!(
                    inline_highlights(line, caret)
                        .iter()
                        .all(|(_, highlight)| !matches!(
                            highlight,
                            MarkdownHighlight::Marker { .. }
                        )),
                    "{line:?} at {caret:?} must not turn trailing whitespace into a hidden marker"
                );
            }
        }

        let markdown = "**안녕** ";
        let highlights = inline_highlights(markdown, Some(markdown.len()));
        assert!(highlights.iter().any(|(range, highlight)| {
            *range == (0..2) && matches!(highlight, MarkdownHighlight::Marker { hidden: true, .. })
        }));
        assert!(
            highlights
                .iter()
                .all(|(range, _)| *range != (markdown.len() - 1..markdown.len()))
        );
    }

    #[test]
    fn bundled_body_font_keeps_korean_ime_stages_on_one_baseline() {
        use iced::advanced::graphics::text::{Paragraph, font_system};
        use iced::advanced::text::{Paragraph as _, Shaping, Text, Wrapping};
        use iced::{Pixels, Size, alignment};
        use std::borrow::Cow;

        font_system()
            .write()
            .expect("font system")
            .load_font(Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
            )));

        let metrics = |stage: &str| {
            let content = format!("앞{stage}뒤");
            let paragraph = Paragraph::with_text(Text {
                content: &content,
                bounds: Size::new(620.0, 100.0),
                size: Pixels(super::BODY_SIZE),
                line_height: iced::advanced::text::LineHeight::Relative(super::BODY_LINE_HEIGHT),
                font: super::body_font(iced::font::Weight::Normal, iced::font::Style::Normal),
                align_x: iced::advanced::text::Alignment::Default,
                align_y: alignment::Vertical::Top,
                shaping: Shaping::Advanced,
                wrapping: Wrapping::Word,
            });
            let run = paragraph
                .buffer()
                .layout_runs()
                .next()
                .expect("one visual line");
            let glyph = run
                .glyphs
                .iter()
                .find(|glyph| glyph.start == "앞".len())
                .expect("composition glyph");

            (
                run.line_y.to_bits(),
                run.line_height.to_bits(),
                glyph.font_id,
                glyph.y.to_bits(),
                glyph.y_offset.to_bits(),
                glyph.font_size.to_bits(),
                glyph.w.to_bits(),
            )
        };

        let ieung = metrics("ㅇ");
        assert_eq!(metrics("으"), ieung);
        assert_eq!(metrics("응"), ieung);
    }

    #[test]
    fn hides_markers_until_the_caret_enters_the_span() {
        let line = "as**df**";
        let mut outside = MarkdownHighlighter::new(&super::Caret::new(0, 1, false));
        let outside = outside.highlight_line(line).collect::<Vec<_>>();
        let mut inside = MarkdownHighlighter::new(&super::Caret::new(0, 5, false));
        let inside = inside.highlight_line(line).collect::<Vec<_>>();
        let mut marker = MarkdownHighlighter::new(&super::Caret::new(0, 2, false));
        let marker = marker.highlight_line(line).collect::<Vec<_>>();

        assert!(outside.iter().any(|(range, style)| {
            *range == (2..4)
                && matches!(
                    style,
                    MarkdownHighlight::Marker {
                        hidden: true,
                        style,
                        ..
                    } if style.strong
                )
        }));
        assert!(inside.iter().any(|(range, style)| {
            *range == (2..4) && matches!(style, MarkdownHighlight::Marker { hidden: false, .. })
        }));
        assert!(inside.iter().any(|(range, style)| {
            *range == (4..6) && matches!(style, MarkdownHighlight::Span(style) if style.strong)
        }));
        assert!(marker.iter().any(|(range, style)| {
            *range == (2..4) && matches!(style, MarkdownHighlight::Marker { hidden: false, .. })
        }));

        let code = "x `code` y";
        let mut outside = MarkdownHighlighter::new(&super::Caret::new(0, 0, false));
        let outside = outside.highlight_line(code).collect::<Vec<_>>();
        let mut marker = MarkdownHighlighter::new(&super::Caret::new(0, 2, false));
        let marker = marker.highlight_line(code).collect::<Vec<_>>();

        assert!(outside.iter().any(|(range, style)| {
            *range == (2..3) && matches!(style, MarkdownHighlight::Marker { hidden: true, .. })
        }));
        assert!(marker.iter().any(|(range, style)| {
            *range == (2..3) && matches!(style, MarkdownHighlight::Marker { hidden: false, .. })
        }));
    }

    #[test]
    fn resumes_fenced_code_at_the_changed_line() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret::new(0, 0, false));
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
        let mut highlighter = MarkdownHighlighter::new(&super::Caret::new(0, 0, false));
        let headings = (1..=6)
            .map(|level| {
                let line = format!("{} heading", "#".repeat(level));
                highlighter
                    .highlight_line(&line)
                    .find_map(|(_, style)| match style {
                        MarkdownHighlight::Span(style) if style.heading.is_some() => {
                            let level = style.heading.expect("heading level");
                            super::markdown_format(
                                &MarkdownHighlight::Span(style),
                                &iced::Theme::Light,
                            )
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
        assert_eq!(
            code.line_padding,
            iced::Padding::from([0.0, super::CODE_BLOCK_PADDING])
        );
        assert_eq!(code.padding, iced::Padding::ZERO);

        let inline = super::markdown_format(
            &MarkdownHighlight::Span(super::SpanStyle {
                code: true,
                ..super::SpanStyle::default()
            }),
            &iced::Theme::Light,
        );
        assert!((inline.size.unwrap().0 - 16.0).abs() < 0.01);
        assert_eq!(inline.padding.top, 0.0);
        assert!((inline.padding.left - 6.4).abs() < 0.01);
        assert_eq!(
            inline.highlight.unwrap().background,
            iced::Background::Color(iced::Color {
                a: super::CODE_BACKGROUND_ALPHA,
                ..iced::Theme::Light.palette().text
            })
        );
    }

    #[test]
    fn hidden_inline_code_delimiters_reserve_the_highlight_margin() {
        use iced::advanced::graphics::text::{Paragraph, font_system};
        use iced::advanced::text::{Alignment, Paragraph as _, Shaping, Span, Text, Wrapping};
        use iced::{Pixels, Size, alignment};
        use std::borrow::Cow;

        font_system()
            .write()
            .expect("font system")
            .load_font(Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/MonoplexKR-Regular.ttf"
            )));
        let markers = ["before `code` after", "before ``code`` after"]
            .into_iter()
            .flat_map(|line| inline_highlights(line, None))
            .filter_map(|(_, highlight)| match highlight {
                MarkdownHighlight::Marker {
                    hidden: true,
                    style,
                    marker_count,
                } if style.code => Some((
                    marker_count,
                    super::markdown_format(
                        &MarkdownHighlight::Marker {
                            hidden: true,
                            style,
                            marker_count,
                        },
                        &iced::Theme::Light,
                    ),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(markers.len(), 4);
        for (marker_count, format) in markers {
            let source = "`".repeat(marker_count);
            let mut span: Span<'_, (), iced::Font> = Span::new(source.as_str());
            span.font = format.font;
            span.size = format.size;
            span.line_height = format.line_height;
            span.color = format.color;
            let spans = [span];
            let paragraph = Paragraph::with_spans(Text {
                content: spans.as_slice(),
                bounds: Size::new(100.0, 100.0),
                size: Pixels(super::BODY_SIZE),
                line_height: iced::advanced::text::LineHeight::Relative(super::BODY_LINE_HEIGHT),
                font: super::body_font(iced::font::Weight::Normal, iced::font::Style::Normal),
                align_x: Alignment::Default,
                align_y: alignment::Vertical::Top,
                shaping: Shaping::Advanced,
                wrapping: Wrapping::None,
            });
            let width = paragraph
                .buffer()
                .layout_runs()
                .next()
                .expect("one marker run")
                .line_w;

            assert!(
                (width - super::INLINE_CODE_PADDING_X).abs() < 0.1,
                "marker width {width} != {}",
                super::INLINE_CODE_PADDING_X
            );
        }
    }

    #[test]
    fn highlights_fenced_code_with_the_declared_language() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret::new(0, 0, false));
        let _ = highlighter.highlight_line("```rust").count();
        let code = highlighter
            .highlight_line("fn main() { let answer = 42; }")
            .collect::<Vec<_>>();

        assert!(code.iter().any(|(_, style)| {
            matches!(style, MarkdownHighlight::CodeToken { color: Some(_), .. })
        }));
    }

    #[test]
    fn theme_change_restarts_fenced_code_highlighting() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret::new(0, 0, false));
        let _ = highlighter.highlight_line("```rust").count();
        let _ = highlighter.highlight_line("fn main() {}").count();
        assert!(!highlighter.code.is_empty());

        highlighter.update(&super::Caret::new(0, 0, true));

        assert_eq!(highlighter.current_line(), 0);
        assert!(highlighter.code.is_empty());
        assert!(highlighter.caret.dark);
    }

    #[test]
    fn resumes_near_the_end_of_a_large_document() {
        let mut highlighter = MarkdownHighlighter::new(&super::Caret::new(0, 0, false));
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
        let mut highlighter = MarkdownHighlighter::new(&super::Caret::new(0, 0, false));
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
        let mut highlighter = MarkdownHighlighter::new(&super::Caret::new(0, 7, false));

        for (range, _) in highlighter.highlight_line(line) {
            assert!(line.is_char_boundary(range.start));
            assert!(line.is_char_boundary(range.end));
            assert!(range.end <= line.len());
        }
    }

    #[test]
    fn undo_redo_tracks_deltas_and_saved_state() {
        let mut document = reset_document("hello".into());
        mark_saved(editor_status().revision);
        track_action(&mut document, Action::Edit(Edit::Insert('!')));
        assert_eq!(document.text(), "!hello");
        let status = editor_status();
        assert!(status.dirty);
        assert!(status.can_undo);
        let stale_revision = status.revision;

        track_action(&mut document, Action::Edit(Edit::Insert('?')));
        mark_saved(stale_revision);
        assert_eq!(document.text(), "!?hello");
        assert!(editor_status().dirty);

        document = undo_document(document);
        assert_eq!(document.text(), "hello");
        let status = editor_status();
        assert!(!status.dirty);
        assert!(status.can_redo);

        document = redo_document(document);
        assert_eq!(document.text(), "!?hello");
    }

    #[test]
    fn content_version_tracks_text_states_and_document_replacement() {
        let mut document = reset_document("hello".into());
        let initial = current_content_version();

        track_action(&mut document, Action::Move(Motion::Right));
        assert_eq!(current_content_version(), initial);

        track_action(&mut document, Action::Edit(Edit::Insert('!')));
        let edited = current_content_version();
        assert_eq!(edited.document(), initial.document());
        assert_ne!(edited.revision(), initial.revision());

        document = undo_document(document);
        assert_eq!(document.text(), "hello");
        assert_eq!(current_content_version(), initial);

        let replacement = reset_document("hello".into());
        let replaced = current_content_version();
        assert_eq!(replacement.text(), document.text());
        assert_ne!(replaced.document(), initial.document());
        assert_eq!(replaced.revision(), initial.revision());
    }

    #[test]
    fn production_history_emits_exact_editor_change_transitions() {
        let mut document = reset_document("hello".into());
        let initial = current_content_version();
        track_action(&mut document, Action::Edit(Edit::Insert('!')));
        let (inserted, insertion) = current_editor_state();
        let insertion = insertion.expect("ordinary edit transition");
        assert_eq!(insertion.from(), initial);
        assert_eq!(insertion.to(), inserted);
        assert_eq!(insertion.first_changed_line, 0);
        assert_eq!(insertion.removed_lines, 1);
        assert_eq!(insertion.inserted_lines, 1);

        document = undo_document(document);
        let (undone, undo) = current_editor_state();
        let undo = undo.expect("undo transition");
        assert_eq!(undone, initial);
        assert_eq!(undo.from(), inserted);
        assert_eq!(undo.to(), initial);
        assert_eq!(undo.removed_lines, 1);
        assert_eq!(undo.inserted_lines, 1);

        document = redo_document(document);
        assert_eq!(document.text(), "!hello");
        let (redone, redo) = current_editor_state();
        let redo = redo.expect("redo transition");
        assert_eq!(redone, inserted);
        assert_eq!(redo.from(), initial);
        assert_eq!(redo.to(), inserted);

        let mut document = reset_document("alpha\nbeta\ngamma".into());
        let before_selection = current_content_version();
        document.move_to(Cursor {
            position: Position { line: 2, column: 3 },
            selection: Some(Position { line: 0, column: 2 }),
        });
        track_action(
            &mut document,
            Action::Edit(Edit::Paste(Arc::new("응\n답".into()))),
        );
        let (after_selection, selection) = current_editor_state();
        let selection = selection.expect("selection replacement transition");
        assert_eq!(selection.from(), before_selection);
        assert_eq!(selection.to(), after_selection);
        assert_eq!(selection.first_changed_line, 0);
        assert_eq!(selection.removed_lines, 3);
        assert_eq!(selection.inserted_lines, 2);

        let mut document = reset_document("앞 ".into());
        document.move_to(Cursor {
            position: Position { line: 0, column: 4 },
            selection: None,
        });
        let before_ime = current_content_version();
        // RichTextEditor publishes IME commits as a native Paste action.
        track_action(
            &mut document,
            Action::Edit(Edit::Paste(Arc::new("응".into()))),
        );
        let (after_ime, ime) = current_editor_state();
        let ime = ime.expect("IME commit transition");
        assert_eq!(ime.from(), before_ime);
        assert_eq!(ime.to(), after_ime);
        assert_eq!(ime.first_changed_line, 0);
        assert_eq!(ime.removed_lines, 1);
        assert_eq!(ime.inserted_lines, 1);
    }

    #[test]
    fn batched_frames_expose_only_the_latest_exact_transition() {
        let mut document = reset_document("first\nsecond".into());
        let rendered = current_content_version();

        track_action(&mut document, Action::Edit(Edit::Insert('A')));
        let intermediate = current_content_version();
        document.move_to(Cursor {
            position: Position { line: 1, column: 0 },
            selection: None,
        });
        track_action(&mut document, Action::Edit(Edit::Insert('B')));

        let (current, latest) = current_editor_state();
        let latest = latest.expect("latest edit transition");
        assert_eq!(latest.from(), intermediate);
        assert_eq!(latest.to(), current);
        assert_ne!(latest.from(), rendered);
        assert_eq!(latest.first_changed_line, 1);
    }

    #[test]
    fn a_new_edit_after_undo_discards_redo() {
        let mut document = reset_document("hello".into());
        track_action(&mut document, Action::Edit(Edit::Insert('!')));
        document = undo_document(document);
        assert!(editor_status().can_redo);

        track_action(&mut document, Action::Edit(Edit::Insert('?')));

        assert_eq!(document.text(), "?hello");
        assert!(!editor_status().can_redo);
    }

    #[test]
    fn history_records_native_delete_units() {
        use iced::widget::text_editor::{Cursor, Position};

        let original = ")\u{301}🙂";
        let cases = [
            (Position { line: 0, column: 0 }, Edit::Delete, "🙂"),
            (
                Position {
                    line: 0,
                    column: ")\u{301}".len(),
                },
                Edit::Backspace,
                ")🙂",
            ),
        ];

        for (position, edit, expected) in cases {
            let mut document = reset_document(original.into());
            document.move_to(Cursor {
                position,
                selection: None,
            });
            assert_eq!(document.cursor().position, position);
            track_action(&mut document, Action::Edit(edit));
            assert_eq!(document.text(), expected);

            document = undo_document(document);
            assert_eq!(document.text(), original);
            document = redo_document(document);
            assert_eq!(document.text(), expected);
        }
    }

    #[test]
    fn generated_unicode_edits_round_trip_through_history() {
        use iced::widget::text_editor::{Content, Cursor, Position};
        use std::sync::Arc;
        use unicode_segmentation::UnicodeSegmentation;

        fn next(state: &mut usize) -> usize {
            *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *state
        }

        fn positions(content: &Content) -> Vec<Position> {
            content
                .lines()
                .enumerate()
                .flat_map(|(line, value)| {
                    value
                        .text
                        .grapheme_indices(true)
                        .map(|(column, _)| Position { line, column })
                        .chain(std::iter::once(Position {
                            line,
                            column: value.text.len(),
                        }))
                        .collect::<Vec<_>>()
                })
                .collect()
        }

        fn assert_cursor_boundaries(content: &Content) {
            for position in
                std::iter::once(content.cursor().position).chain(content.cursor().selection)
            {
                let line = content
                    .line(position.line)
                    .unwrap_or_else(|| panic!("cursor line {} is absent", position.line));
                assert!(position.column <= line.text.len());
                assert!(line.text.is_char_boundary(position.column));
                assert!(
                    position.column == line.text.len()
                        || line
                            .text
                            .grapheme_indices(true)
                            .any(|(column, _)| column == position.column)
                );
            }
        }

        let mut document = reset_document("초기🙂\nsecond e\u{301}\n- list\n```\n코드\n```".into());
        let original = document.text();
        let mut state = 0x4d59_5df4_usize;
        let inserted = ['a', '한', '🙂', '\u{301}', '*', ' '];
        let pasted = ["붙여넣기", "🙂e\u{301}", "\n둘", "a\r\nb", "**x**"];
        let formats = ["bold", "italic", "code", "link"];

        for _ in 0..256 {
            let available = positions(&document);
            let position = available[next(&mut state) % available.len()];
            let selection = next(&mut state).is_multiple_of(3).then(|| {
                let available = positions(&document);
                available[next(&mut state) % available.len()]
            });
            document.move_to(Cursor {
                position,
                selection,
            });

            let operation = next(&mut state) % 9;
            match operation {
                0 => track_action(
                    &mut document,
                    Action::Edit(Edit::Insert(inserted[next(&mut state) % inserted.len()])),
                ),
                1 => track_action(
                    &mut document,
                    Action::Edit(Edit::Paste(Arc::new(
                        pasted[next(&mut state) % pasted.len()].to_owned(),
                    ))),
                ),
                2 => track_action(&mut document, Action::Edit(Edit::Enter)),
                3 => track_action(&mut document, Action::Edit(Edit::Backspace)),
                4 => track_action(&mut document, Action::Edit(Edit::Delete)),
                5 => track_action(&mut document, Action::Edit(Edit::Indent)),
                6 => track_action(&mut document, Action::Edit(Edit::Unindent)),
                _ => {
                    document =
                        format_document(document, formats[next(&mut state) % formats.len()].into());
                }
            }
            assert_cursor_boundaries(&document);
            for line in document.lines() {
                for (range, _) in inline_highlights(&line.text, None) {
                    assert!(line.text.is_char_boundary(range.start));
                    assert!(line.text.is_char_boundary(range.end));
                    assert!(range.end <= line.text.len());
                }
            }
        }

        let final_text = document.text();
        let mut undo_count = 0;
        while editor_status().can_undo {
            document = undo_document(document);
            assert_cursor_boundaries(&document);
            undo_count += 1;
            assert!(undo_count <= 256);
        }
        assert_eq!(document.text(), original);

        let mut redo_count = 0;
        while editor_status().can_redo {
            document = redo_document(document);
            assert_cursor_boundaries(&document);
            redo_count += 1;
            assert!(redo_count <= undo_count);
        }
        assert_eq!(redo_count, undo_count);
        assert_eq!(document.text(), final_text);
    }

    #[test]
    fn formatting_preserves_a_selected_edit_target() {
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
    fn clearing_selection_preserves_text_and_caret() {
        let mut document = Content::with_text("selected text");
        let position = Position { line: 0, column: 8 };
        document.move_to(Cursor {
            position,
            selection: Some(Position { line: 0, column: 0 }),
        });

        let document = clear_editor_selection(document);

        assert_eq!(document.text(), "selected text");
        assert_eq!(document.cursor().position, position);
        assert_eq!(document.cursor().selection, None);
    }

    #[test]
    fn moving_without_selection_clears_previous_selection() {
        let mut document = Content::with_text("selected text");
        document.move_to(Cursor {
            position: Position { line: 0, column: 8 },
            selection: Some(Position { line: 0, column: 0 }),
        });

        let document = super::apply_rich_action(
            document,
            super::RichEditorAction::MoveTo(Cursor {
                position: Position { line: 0, column: 4 },
                selection: None,
            }),
        );

        assert_eq!(document.text(), "selected text");
        assert_eq!(document.cursor().position, Position { line: 0, column: 4 });
        assert_eq!(document.cursor().selection, None);
    }

    #[test]
    fn bold_command_toggles_existing_strong_markup() {
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
        let mut document = reset_document("```rust\n```".into());
        document.move_to(iced::widget::text_editor::Cursor {
            position: iced::widget::text_editor::Position { line: 0, column: 7 },
            selection: None,
        });

        track_action(&mut document, Action::Edit(Edit::Enter));

        assert_eq!(document.text(), "```rust\n\n```");
        assert_eq!(document.cursor().position.line, 1);
    }

    #[test]
    fn list_editing_continues_exits_renumbers_and_nests() {
        use iced::widget::text_editor::{Cursor, Position};

        let caret = |line, column| Cursor {
            position: Position { line, column },
            selection: None,
        };

        let mut document = reset_document("- first".into());
        document.move_to(caret(0, 7));
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "- first\n- ");
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "- first\n");

        document = reset_document("- [x] done".into());
        document.move_to(caret(0, 10));
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "- [x] done\n- [ ] ");

        document = reset_document("1. one\n2. two\n3. three".into());
        document.move_to(caret(0, 6));
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "1. one\n2. \n3. two\n4. three");
        document = undo_document(document);
        assert_eq!(document.text(), "1. one\n2. two\n3. three");
        document = redo_document(document);
        assert_eq!(document.text(), "1. one\n2. \n3. two\n4. three");

        document = reset_document("- item".into());
        document.move_to(caret(0, 2));
        track_action(&mut document, Action::Edit(Edit::Backspace));
        assert_eq!(document.text(), "item");

        document = reset_document("- a\n- b".into());
        document.move_to(caret(1, 3));
        track_action(&mut document, Action::Edit(Edit::Indent));
        assert_eq!(document.text(), "- a\n  - b");
        track_action(&mut document, Action::Edit(Edit::Unindent));
        assert_eq!(document.text(), "- a\n- b");
    }

    #[test]
    fn ordered_lists_renumber_when_items_nest_lift_and_continue() {
        use iced::widget::text_editor::{Cursor, Position};

        let caret = |line, column| Cursor {
            position: Position { line, column },
            selection: None,
        };

        let mut document = reset_document("1. one\n2. two\n3. three".into());
        document.move_to(caret(1, 6));
        track_action(&mut document, Action::Edit(Edit::Indent));
        assert_eq!(document.text(), "1. one\n   1. two\n2. three");
        document = undo_document(document);
        assert_eq!(document.text(), "1. one\n2. two\n3. three");
        document = redo_document(document);
        assert_eq!(document.text(), "1. one\n   1. two\n2. three");
        track_action(&mut document, Action::Edit(Edit::Unindent));
        assert_eq!(document.text(), "1. one\n2. two\n3. three");

        document = reset_document("1. a\n5. b\n2. c".into());
        document.move_to(caret(0, 4));
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "1. a\n2. \n3. b\n4. c");

        document = reset_document("1. a\n- x\n7. b".into());
        document.move_to(caret(0, 4));
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "1. a\n2. \n- x\n7. b");

        document = reset_document("3. a\n9. b".into());
        document.move_to(caret(0, 4));
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "3. a\n4. \n5. b");

        document = reset_document("1. x\n   1. a\n   2. \n2. y".into());
        document.move_to(caret(2, 6));
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "1. x\n   1. a\n2. \n3. y");

        document = reset_document("1. a\n2. b\n3. c".into());
        document.move_to(caret(1, 3));
        track_action(&mut document, Action::Edit(Edit::Backspace));
        assert_eq!(document.text(), "1. a\nb\n2. c");

        document = reset_document("1. a\n\n2. b\n\n3. c\n\npara\n\n7. d".into());
        document.move_to(caret(2, 4));
        track_action(&mut document, Action::Edit(Edit::Enter));
        assert_eq!(document.text(), "1. a\n\n2. b\n3. \n\n4. c\n\npara\n\n7. d");
    }

    #[test]
    fn tab_in_a_code_block_inserts_four_spaces() {
        use iced::widget::text_editor::{Cursor, Position};

        let mut document = reset_document("```\ncode\n```".into());
        document.move_to(Cursor {
            position: Position { line: 1, column: 4 },
            selection: None,
        });
        track_action(&mut document, Action::Edit(Edit::Indent));

        assert_eq!(document.text(), "```\ncode    \n```");
    }

    #[test]
    fn tab_indents_every_selected_plain_line_and_preserves_the_selection() {
        use iced::widget::text_editor::{Cursor, Position};

        let mut document = reset_document("one\ntwo\nthree".into());
        document.move_to(Cursor {
            position: Position { line: 2, column: 5 },
            selection: Some(Position { line: 0, column: 1 }),
        });
        track_action(&mut document, Action::Edit(Edit::Indent));

        assert_eq!(document.text(), "    one\n    two\n    three");
        assert_eq!(
            document.cursor(),
            Cursor {
                position: Position { line: 2, column: 9 },
                selection: Some(Position { line: 0, column: 5 }),
            }
        );
    }
}

use iced::advanced::text::{Highlight as TextHighlight, Highlighter, LineHeight};
use iced::font::{Family, Style as FontStyle, Weight};
use iced::widget::text_editor::{Action, Content, Cursor, Edit, Position};
use iced::{Border, Color, Element, Font, Padding, Pixels, Theme, mouse};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use ui_lang_runtime::rich_text_editor::{Format, RichTextEditor};

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
) -> Element<'a, RichEditorAction> {
    let cursor = document.cursor().position;
    let format_theme = if dark { Theme::Dark } else { Theme::Light };
    let editor = RichTextEditor::new(document)
        .id("markdown-editor")
        .placeholder("Start writing…")
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
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
        MarkdownHighlight::Marker { hidden, style } => {
            let mut format = span_format(style, theme);
            format.color = Some(if hidden { Color::TRANSPARENT } else { subdued });
            format.highlight = None;
            format.padding = Padding::ZERO;
            if hidden {
                format.size = Some(Pixels(0.01));
                format.line_height = None;
            }
            format
        }
        MarkdownHighlight::HiddenFence => Format {
            color: Some(Color::TRANSPARENT),
            font: Some(code_font()),
            size: Some(Pixels(0.01)),
            line_height: Some(LineHeight::Absolute(Pixels(BODY_SIZE))),
            line_highlight: Some(code_background),
            padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
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
            padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
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
            padding: Padding::from([0.0, CODE_BLOCK_PADDING]),
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
            top: size * 0.2,
            right: size * 0.4,
            bottom: size * 0.2,
            left: size * 0.4,
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
                if caret.is_some_and(|caret| content.start <= caret && caret <= content.end) {
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
                if caret.is_some_and(|caret| content.start <= caret && caret <= content.end) {
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
                covered.push(range.clone());
                highlights.push((
                    range,
                    MarkdownHighlight::Marker {
                        hidden: false,
                        style: SpanStyle::default(),
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
    let hidden = !visible_scopes
        .iter()
        .any(|scope| scope.start <= range.start && range.end <= scope.end);
    let style = scopes
        .iter()
        .filter(|(scope, _)| scope.start <= range.start && range.end <= scope.end)
        .fold(SpanStyle::default(), |style, (_, next)| style.merge(*next));
    highlights.push((range, MarkdownHighlight::Marker { hidden, style }));
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

pub fn apply_rich_action(mut content: Content, action: RichEditorAction) -> Content {
    match action {
        RichEditorAction::Edit(action) => track_action(&mut content, action),
        RichEditorAction::MoveTo(cursor) => content.move_to(cursor),
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
        content.move_to(after);
        history().record(Change {
            data: ChangeData::Replace {
                start,
                removed: text,
                inserted,
            },
            before,
            after,
            before_id: 0,
            after_id: 0,
            kind: EditKind::Other,
            changed_at: Instant::now(),
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
    renumber_ordered_tail(content, before.position.line + 2, item, &mut replacements);
    content.move_to(after);
    history().record(Change {
        data: if replacements.len() == 1 {
            let replacement = replacements.pop().expect("one list replacement");
            ChangeData::Replace {
                start: replacement.start,
                removed: replacement.removed,
                inserted: replacement.inserted,
            }
        } else {
            ChangeData::Batch(replacements)
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

fn parent_list_prefix(content: &Content, line: usize, indent: usize) -> Option<String> {
    for index in (0..line).rev() {
        let line = content.line(index)?;
        if line.text.trim().is_empty() {
            break;
        }
        let Some(item) = list_item(&line.text) else {
            continue;
        };
        if item.indent < indent {
            return Some(item.next_prefix(&line.text));
        }
    }
    None
}

fn renumber_ordered_tail(
    content: &mut Content,
    first_line: usize,
    inserted_item: ListItem,
    replacements: &mut Vec<Replacement>,
) {
    let ListKind::Ordered {
        mut number,
        delimiter,
    } = inserted_item.kind
    else {
        return;
    };
    let indent = inserted_item.indent;

    for line_index in first_line..content.line_count() {
        let Some(line) = content.line(line_index) else {
            break;
        };
        if line.text.trim().is_empty() {
            continue;
        }
        let Some(item) = list_item(&line.text) else {
            break;
        };
        if item.indent > indent {
            continue;
        }
        if item.indent < indent {
            break;
        }
        let ListKind::Ordered {
            number: candidate,
            delimiter: candidate_delimiter,
        } = item.kind
        else {
            break;
        };
        number = number.saturating_add(1);
        if candidate != number || candidate_delimiter != delimiter {
            break;
        }
        let range = item.number.expect("ordered lists have a number");
        let replacement = Replacement {
            start: Position {
                line: line_index,
                column: range.start,
            },
            removed: line.text[range.clone()].to_owned(),
            inserted: number.saturating_add(1).to_string(),
        };
        replace_range(
            content,
            replacement.start,
            Position {
                line: line_index,
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

    let (removed, inserted) = match edit {
        Edit::Indent if has_previous_list_item(content, before.position.line, item.indent) => {
            (String::new(), "  ".to_owned())
        }
        Edit::Indent => return false,
        Edit::Unindent if item.indent > 0 => {
            let width = if text.starts_with('\t') {
                1
            } else {
                item.indent.min(2)
            };
            (text[..width].to_owned(), String::new())
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

fn has_previous_list_item(content: &Content, line: usize, indent: usize) -> bool {
    for index in (0..line).rev() {
        let Some(line) = content.line(index) else {
            return false;
        };
        if line.text.trim().is_empty() {
            return false;
        }
        let Some(item) = list_item(&line.text) else {
            continue;
        };
        if item.indent < indent {
            return false;
        }
        if item.indent == indent {
            return true;
        }
    }
    false
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
        MarkdownHighlight, MarkdownHighlighter, can_redo, can_undo, format_document,
        inline_highlights, is_dirty, mark_saved, redo_document, reset_document, revision,
        track_action, undo_document,
    };
    use iced::advanced::text::Highlighter;
    use iced::widget::text_editor::{Action, Edit};

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
                "../assets/fonts/IBMPlexSansKR-Regular.ttf"
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
                        style
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

        let inline = super::markdown_format(
            &MarkdownHighlight::Span(super::SpanStyle {
                code: true,
                ..super::SpanStyle::default()
            }),
            &iced::Theme::Light,
        );
        assert!((inline.size.unwrap().0 - 16.0).abs() < 0.01);
        assert!((inline.padding.top - 3.2).abs() < 0.01);
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
    fn a_new_edit_after_undo_discards_redo() {
        let _lock = super::test_history_lock();
        let mut document = reset_document("hello".into());
        track_action(&mut document, Action::Edit(Edit::Insert('!')));
        document = undo_document(document);
        assert!(can_redo());

        track_action(&mut document, Action::Edit(Edit::Insert('?')));

        assert_eq!(document.text(), "?hello");
        assert!(!can_redo());
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

    #[test]
    fn list_editing_continues_exits_renumbers_and_nests() {
        use iced::widget::text_editor::{Cursor, Position};

        let _lock = super::test_history_lock();
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
    fn tab_in_a_code_block_inserts_four_spaces() {
        use iced::widget::text_editor::{Cursor, Position};

        let _lock = super::test_history_lock();
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

        let _lock = super::test_history_lock();
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

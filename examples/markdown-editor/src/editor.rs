use iced::advanced::text::highlighter::{Format, Highlighter, PlainText};
use iced::font::{Style as FontStyle, Weight};
use iced::widget::text_editor::TextEditor;
use iced::{Color, Element, Font, Theme};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

#[derive(Debug)]
pub struct MarkdownHighlighter {
    current_line: usize,
    fences: Vec<Option<Fence>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fence {
    marker: u8,
    length: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownHighlight {
    Marker,
    Heading,
    Strong,
    Emphasis,
    Code,
    Link,
    Quote,
    ListMarker,
    Strikethrough,
}

pub fn markdown_highlight<'a, Message: 'a>(
    editor: TextEditor<'a, PlainText, Message>,
) -> Element<'a, Message> {
    editor
        .highlight_with::<MarkdownHighlighter>((), markdown_format)
        .into()
}

fn markdown_format(highlight: &MarkdownHighlight, theme: &Theme) -> Format<Font> {
    let palette = theme.palette();
    let subdued = Color {
        a: 0.32,
        ..palette.text
    };

    match highlight {
        MarkdownHighlight::Marker => Format {
            color: Some(subdued),
            font: None,
        },
        MarkdownHighlight::Heading | MarkdownHighlight::Strong => Format {
            color: Some(palette.text),
            font: Some(Font {
                weight: Weight::Bold,
                ..Font::DEFAULT
            }),
        },
        MarkdownHighlight::Emphasis | MarkdownHighlight::Quote => Format {
            color: Some(if matches!(highlight, MarkdownHighlight::Quote) {
                palette.primary
            } else {
                palette.text
            }),
            font: Some(Font {
                style: FontStyle::Italic,
                ..Font::DEFAULT
            }),
        },
        MarkdownHighlight::Code => Format {
            color: Some(palette.primary),
            font: Some(Font::MONOSPACE),
        },
        MarkdownHighlight::Link | MarkdownHighlight::ListMarker => Format {
            color: Some(palette.primary),
            font: Some(Font {
                weight: Weight::Semibold,
                ..Font::DEFAULT
            }),
        },
        MarkdownHighlight::Strikethrough => Format {
            color: Some(subdued),
            font: None,
        },
    }
}

impl Highlighter for MarkdownHighlighter {
    type Settings = ();
    type Highlight = MarkdownHighlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Self::Highlight)>;

    fn new((): &Self::Settings) -> Self {
        Self {
            current_line: 0,
            fences: vec![None],
        }
    }

    fn update(&mut self, (): &Self::Settings) {}

    fn change_line(&mut self, line: usize) {
        if line < self.fences.len() {
            self.fences.truncate(line + 1);
            self.current_line = line;
        } else {
            self.fences.truncate(1);
            self.current_line = 0;
        }
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let fence = self.fences[self.current_line];
        let (mut highlights, next_fence) = highlight_line(line, fence);
        highlights.retain(|(range, _)| !range.is_empty());

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
    active_fence: Option<Fence>,
) -> (Vec<(Range<usize>, MarkdownHighlight)>, Option<Fence>) {
    let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
    let delimiter = fence_delimiter(&line[leading..]);

    if let Some(fence) = active_fence {
        let closes = delimiter.is_some_and(|candidate| {
            candidate.marker == fence.marker && candidate.length >= fence.length
        });
        return (
            vec![(
                0..line.len(),
                if closes {
                    MarkdownHighlight::Marker
                } else {
                    MarkdownHighlight::Code
                },
            )],
            if closes { None } else { Some(fence) },
        );
    }

    if let Some(fence) = delimiter {
        return (
            vec![(leading..line.len(), MarkdownHighlight::Marker)],
            Some(fence),
        );
    }

    (inline_highlights(line), None)
}

fn fence_delimiter(line: &str) -> Option<Fence> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }

    let length = line.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then_some(Fence { marker, length })
}

fn inline_highlights(line: &str) -> Vec<(Range<usize>, MarkdownHighlight)> {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut active = Vec::<(TagEnd, MarkdownHighlight)>::new();
    let mut covered = Vec::<Range<usize>>::new();
    let mut highlights = Vec::new();

    for (event, range) in Parser::new_ext(line, options).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if let Some(style) = tag_style(&tag) {
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
            Event::Code(_) => push_content(
                delimited_content(line, range, b'`'),
                Some(MarkdownHighlight::Code),
                &mut covered,
                &mut highlights,
            ),
            Event::InlineMath(_) | Event::DisplayMath(_) => push_content(
                delimited_content(line, range, b'$'),
                Some(MarkdownHighlight::Code),
                &mut covered,
                &mut highlights,
            ),
            Event::Html(_) | Event::InlineHtml(_) => push_content(
                range,
                Some(MarkdownHighlight::Code),
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
            highlights.push((cursor..range.start, MarkdownHighlight::Marker));
        }
        cursor = cursor.max(range.end);
    }
    if cursor < line.len() {
        highlights.push((cursor..line.len(), MarkdownHighlight::Marker));
    }

    highlights.sort_unstable_by_key(|(range, _)| range.start);
    highlights
}

fn tag_style(tag: &Tag<'_>) -> Option<MarkdownHighlight> {
    match tag {
        Tag::Heading { .. } => Some(MarkdownHighlight::Heading),
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

#[cfg(test)]
mod tests {
    use super::{MarkdownHighlight, MarkdownHighlighter};
    use iced::advanced::text::Highlighter;

    #[test]
    fn styles_common_markdown_inline() {
        let mut highlighter = MarkdownHighlighter::new(&());
        let highlights = highlighter
            .highlight_line("Write **bold**, *italic*, `code`, and [links](https://example.com).")
            .collect::<Vec<_>>();

        for expected in [
            MarkdownHighlight::Marker,
            MarkdownHighlight::Strong,
            MarkdownHighlight::Emphasis,
            MarkdownHighlight::Code,
            MarkdownHighlight::Link,
        ] {
            assert!(highlights.iter().any(|(_, style)| *style == expected));
        }
    }

    #[test]
    fn resumes_fenced_code_at_the_changed_line() {
        let mut highlighter = MarkdownHighlighter::new(&());
        for line in ["before", "```rust", "fn main() {}", "```", "after"] {
            let _ = highlighter.highlight_line(line).count();
        }
        assert_eq!(highlighter.current_line(), 5);

        highlighter.change_line(2);
        let changed = highlighter
            .highlight_line("let value = 1;")
            .collect::<Vec<_>>();

        assert_eq!(highlighter.current_line(), 3);
        assert_eq!(changed, vec![(0..14, MarkdownHighlight::Code)]);
    }

    #[test]
    fn resumes_near_the_end_of_a_large_document() {
        let mut highlighter = MarkdownHighlighter::new(&());
        for _ in 0..10_000 {
            let _ = highlighter.highlight_line("plain text").count();
        }

        highlighter.change_line(9_999);
        let _ = highlighter.highlight_line("**changed**").count();

        assert_eq!(highlighter.current_line(), 10_000);
        assert_eq!(highlighter.fences.len(), 10_001);
    }

    #[test]
    fn produces_valid_ranges_for_unicode_and_incomplete_markup() {
        let line = "한글 **강조** and [unfinished";
        let mut highlighter = MarkdownHighlighter::new(&());

        for (range, _) in highlighter.highlight_line(line) {
            assert!(line.is_char_boundary(range.start));
            assert!(line.is_char_boundary(range.end));
            assert!(range.end <= line.len());
        }
    }
}

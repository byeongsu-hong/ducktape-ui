use iced::advanced::graphics::text::{Paragraph as GraphicsParagraph, cosmic_text};
use iced::advanced::text::{self, Paragraph as _, Renderer as _, Span, Text};
use iced::alignment;
use iced::widget::text_editor::Position;
use iced::{Color, Font, Padding, Pixels, Point, Rectangle, Size, Vector};
use std::ops::Range;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use super::EditorChange;

/// Visual formatting for a highlighted source range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Format {
    /// Text color override.
    pub color: Option<Color>,
    /// Font override.
    pub font: Option<Font>,
    /// Font size override.
    pub size: Option<Pixels>,
    /// Line height override.
    pub line_height: Option<text::LineHeight>,
    /// Background drawn around the formatted span.
    pub highlight: Option<text::Highlight>,
    /// Background drawn across every visual line containing the range.
    pub line_highlight: Option<text::Highlight>,
    /// Layout padding inside [`Self::line_highlight`].
    pub line_padding: Padding,
    /// Strikethrough color.
    pub strikethrough: Option<Color>,
    /// Extra paint-only padding around [`Self::highlight`].
    pub padding: Padding,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            color: None,
            font: None,
            size: None,
            line_height: None,
            highlight: None,
            line_highlight: None,
            line_padding: Padding::ZERO,
            strikethrough: None,
            padding: Padding::ZERO,
        }
    }
}

impl Format {
    fn overlay(self, overlay: Self) -> Self {
        Self {
            color: overlay.color.or(self.color),
            font: overlay.font.or(self.font),
            size: overlay.size.or(self.size),
            line_height: overlay.line_height.or(self.line_height),
            highlight: overlay.highlight.or(self.highlight),
            line_highlight: overlay.line_highlight.or(self.line_highlight),
            line_padding: if overlay.line_padding == Padding::ZERO {
                self.line_padding
            } else {
                overlay.line_padding
            },
            strikethrough: overlay.strikethrough.or(self.strikethrough),
            padding: if overlay.padding == Padding::ZERO {
                self.padding
            } else {
                overlay.padding
            },
        }
    }
}

#[derive(Default)]
pub(super) struct DocumentLayout {
    pub(super) lines: Vec<DocumentLine>,
    pub(super) height: f32,
}

pub(super) struct DocumentLine {
    pub(super) signature: StyledLine,
    pub(super) paragraph: GraphicsParagraph,
    pub(super) spans: Vec<Span<'static, (), Font>>,
    pub(super) strikethroughs: Vec<Option<Color>>,
    pub(super) top: f32,
    pub(super) height: f32,
    #[cfg(test)]
    pub(super) identity: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct StyledLine {
    pub(super) text: String,
    pub(super) segments: Vec<Segment>,
    pub(super) empty_format: Format,
    pub(super) line_highlight: Option<text::Highlight>,
    pub(super) line_padding: Padding,
}

#[derive(Debug, PartialEq)]
struct StyledLineFormat {
    segments: Vec<Segment>,
    empty_format: Format,
    line_highlight: Option<text::Highlight>,
    line_padding: Padding,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LineLayoutStyle {
    pub(super) width: f32,
    pub(super) font: Font,
    pub(super) text_size: Pixels,
    pub(super) line_height: text::LineHeight,
    pub(super) wrapping: text::Wrapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LayoutUpdate {
    pub(super) mapping_line_comparisons: usize,
    pub(super) styled_signature_comparisons: usize,
    pub(super) newly_owned_styled_texts: usize,
    pub(super) newly_owned_styled_text_bytes: usize,
    pub(super) line_vector_slots_prepared: usize,
    pub(super) rebuilt_lines: usize,
    pub(super) shaped_paragraphs: usize,
    pub(super) highlighted_lines: usize,
    pub(super) change_hint_used: bool,
    pub(super) change_hint_rejected: bool,
}

#[derive(Debug, Clone, Copy)]
struct LineMapping {
    common_prefix: usize,
    common_suffix: usize,
    changed_overlap: usize,
    mapping_line_comparisons: usize,
    change_hint_used: bool,
    change_hint_rejected: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum DocumentChange {
    Unchanged,
    Discover,
    Hint(EditorChange),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DocumentUpdate {
    pub(super) change: DocumentChange,
    pub(super) geometry_changed: bool,
    pub(super) format_changed: bool,
}

impl DocumentUpdate {
    #[cfg(test)]
    pub(super) const fn text(change: DocumentChange) -> Self {
        Self {
            change,
            geometry_changed: false,
            format_changed: false,
        }
    }
}

pub(super) fn ordered_positions(left: Position, right: Position) -> (Position, Position) {
    if (left.line, left.column) <= (right.line, right.column) {
        (left, right)
    } else {
        (right, left)
    }
}

impl DocumentLayout {
    pub(super) fn update<H>(
        &mut self,
        texts: &[String],
        highlighter: &mut H,
        format: &dyn Fn(&H::Highlight) -> Format,
        style: LineLayoutStyle,
        update: DocumentUpdate,
    ) -> LayoutUpdate
    where
        H: text::Highlighter,
    {
        let old_len = self.lines.len();
        let new_len = texts.len();
        let DocumentUpdate {
            change,
            geometry_changed,
            format_changed,
        } = update;
        let LineMapping {
            common_prefix,
            common_suffix,
            changed_overlap,
            mapping_line_comparisons,
            change_hint_used,
            change_hint_rejected,
        } = match change {
            DocumentChange::Unchanged => LineMapping {
                common_prefix: old_len.min(new_len),
                common_suffix: 0,
                changed_overlap: 0,
                mapping_line_comparisons: 0,
                change_hint_used: false,
                change_hint_rejected: false,
            },
            DocumentChange::Discover => discover_mapping(&self.lines, texts),
            DocumentChange::Hint(change) => hinted_mapping(change, old_len, new_len)
                .unwrap_or_else(|| {
                    let mut mapping = discover_mapping(&self.lines, texts);
                    mapping.change_hint_rejected = true;
                    mapping
                }),
        };
        let text_changed = common_prefix < old_len || common_prefix < new_len;

        let mut scan_start = highlighter.current_line().min(new_len);
        if text_changed {
            scan_start = scan_start.min(common_prefix);
        }
        if format_changed {
            scan_start = 0;
        }
        if scan_start < new_len {
            highlighter.change_line(scan_start);
        }

        let mut old = std::mem::take(&mut self.lines)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        let new_suffix_start = new_len.saturating_sub(common_suffix);
        let old_suffix_start = old_len.saturating_sub(common_suffix);
        let mut lines = Vec::with_capacity(new_len);
        let mut rebuilt = 0;
        let mut highlighted = 0;
        let mut styled_signature_comparisons = 0;
        let mut newly_owned_styled_texts = 0;
        let mut newly_owned_styled_text_bytes = 0;

        for (index, text) in texts.iter().enumerate() {
            let candidate = if index < common_prefix {
                Some(index)
            } else if index >= new_suffix_start {
                Some(old_suffix_start + index - new_suffix_start)
            } else if index < old_len
                && (!change_hint_used || index < common_prefix + changed_overlap)
            {
                Some(index)
            } else {
                None
            };

            if index < scan_start {
                let mut line = candidate
                    .and_then(|candidate| old.get_mut(candidate))
                    .and_then(Option::take)
                    .expect("unchanged rich line");
                if geometry_changed {
                    line = DocumentLine::new(line.into_signature(), style);
                    rebuilt += 1;
                }
                lines.push(line);
                continue;
            }

            highlighted += 1;
            let styled_format = styled_line_format(text, highlighter, format);
            let reusable = candidate
                .and_then(|candidate| old.get_mut(candidate))
                .and_then(|line| {
                    if !geometry_changed
                        && line.as_ref().is_some_and(|line| {
                            styled_signature_comparisons += 1;
                            line.signature.matches(text, &styled_format)
                        })
                    {
                        line.take()
                    } else {
                        None
                    }
                });
            let line = reusable.unwrap_or_else(|| {
                rebuilt += 1;
                let reused_text = candidate
                    .and_then(|candidate| old.get_mut(candidate))
                    .and_then(|line| {
                        if line
                            .as_ref()
                            .is_some_and(|line| line.signature.text == text.as_str())
                        {
                            line.take().map(DocumentLine::into_text)
                        } else {
                            None
                        }
                    });
                let text = reused_text.unwrap_or_else(|| {
                    newly_owned_styled_texts += 1;
                    newly_owned_styled_text_bytes += text.len();
                    text.to_owned()
                });
                DocumentLine::new(styled_format.with_text(text), style)
            });
            lines.push(line);
        }

        let mut top = 0.0;
        for line in &mut lines {
            line.top = top;
            top += line.height;
        }
        self.lines = lines;
        self.height = top.max(style.line_height.to_absolute(style.text_size).0);
        LayoutUpdate {
            mapping_line_comparisons,
            styled_signature_comparisons,
            newly_owned_styled_texts,
            newly_owned_styled_text_bytes,
            line_vector_slots_prepared: old_len.saturating_add(new_len),
            rebuilt_lines: rebuilt,
            shaped_paragraphs: rebuilt,
            highlighted_lines: highlighted,
            change_hint_used,
            change_hint_rejected,
        }
    }

    pub(super) fn caret(&self, position: Position) -> Rectangle {
        let Some(line) = self.line(position.line) else {
            return Rectangle::new(Point::ORIGIN, Size::new(1.0, 0.0));
        };
        let caret = caret_rectangle(
            line.paragraph.buffer(),
            Position {
                line: 0,
                column: position.column.min(line.signature.text.len()),
            },
        );
        caret
            + Vector::new(
                line.signature.line_padding.left,
                line.top + line.signature.line_padding.top,
            )
    }

    pub(super) fn hit(&self, point: Point) -> Position {
        let Some(last) = self.lines.len().checked_sub(1) else {
            return Position { line: 0, column: 0 };
        };
        let line_index = self
            .lines
            .partition_point(|line| line.top + line.height <= point.y)
            .min(last);
        let line = &self.lines[line_index];
        let local = hit_position(
            line.paragraph.buffer(),
            Point::new(
                point.x - line.signature.line_padding.left,
                point.y - line.top - line.signature.line_padding.top,
            ),
        );
        Position {
            line: line_index,
            column: local.column.min(line.signature.text.len()),
        }
    }

    pub(super) fn hit_test(&self, point: Point) -> Option<Position> {
        if point.y < 0.0 || point.y >= self.height {
            return None;
        }
        let line_index = self
            .lines
            .partition_point(|line| line.top + line.height <= point.y);
        let line = self.lines.get(line_index)?;
        let local = Point::new(
            point.x - line.signature.line_padding.left,
            point.y - line.top - line.signature.line_padding.top,
        );
        if local.x < 0.0 || local.y < 0.0 {
            return None;
        }

        let run = line
            .paragraph
            .buffer()
            .layout_runs()
            .find(|run| run.line_top <= local.y && local.y < run.line_top + run.line_height)?;
        let mut glyphs = run.glyphs.iter();
        let first = glyphs.next()?;
        let (left, right) = glyphs.fold((first.x, first.x + first.w), |(left, right), glyph| {
            (left.min(glyph.x), right.max(glyph.x + glyph.w))
        });
        if local.x < left || local.x > right {
            return None;
        }

        let position = hit_position(line.paragraph.buffer(), local);
        Some(Position {
            line: line_index,
            column: position.column.min(line.signature.text.len()),
        })
    }

    pub(super) fn draw_text(
        &self,
        renderer: &mut iced::Renderer,
        origin: Point,
        color: Color,
        clip: Rectangle,
    ) {
        for line in &self.lines {
            let top = origin.y + line.top;
            if top + line.height < clip.y || top > clip.y + clip.height {
                continue;
            }
            renderer.fill_paragraph(
                &line.paragraph,
                origin
                    + Vector::new(
                        line.signature.line_padding.left,
                        line.top + line.signature.line_padding.top,
                    ),
                color,
                clip,
            );
        }
    }

    pub(super) fn line(&self, index: usize) -> Option<&DocumentLine> {
        let index = index.min(self.lines.len().checked_sub(1)?);
        self.lines.get(index)
    }
}

fn hinted_mapping(change: EditorChange, old_len: usize, new_len: usize) -> Option<LineMapping> {
    let old_suffix_start = change
        .first_changed_line
        .checked_add(change.removed_lines)?;
    let new_suffix_start = change
        .first_changed_line
        .checked_add(change.inserted_lines)?;
    if old_suffix_start > old_len
        || new_suffix_start > new_len
        || old_len - old_suffix_start != new_len - new_suffix_start
    {
        return None;
    }

    Some(LineMapping {
        common_prefix: change.first_changed_line,
        common_suffix: old_len - old_suffix_start,
        changed_overlap: change.removed_lines.min(change.inserted_lines),
        mapping_line_comparisons: 0,
        change_hint_used: true,
        change_hint_rejected: false,
    })
}

fn discover_mapping(lines: &[DocumentLine], texts: &[String]) -> LineMapping {
    let old_len = lines.len();
    let new_len = texts.len();
    let shared_len = old_len.min(new_len);
    let mut mapping_line_comparisons = 0;
    let mut common_prefix = 0;

    while common_prefix < shared_len {
        mapping_line_comparisons += 1;
        if lines[common_prefix].signature.text != texts[common_prefix] {
            break;
        }
        common_prefix += 1;
    }

    let mut common_suffix = 0;
    while common_suffix < shared_len.saturating_sub(common_prefix) {
        mapping_line_comparisons += 1;
        if lines[old_len - common_suffix - 1].signature.text != texts[new_len - common_suffix - 1] {
            break;
        }
        common_suffix += 1;
    }

    LineMapping {
        common_prefix,
        common_suffix,
        changed_overlap: 0,
        mapping_line_comparisons,
        change_hint_used: false,
        change_hint_rejected: false,
    }
}

impl DocumentLine {
    pub(super) fn new(signature: StyledLine, style: LineLayoutStyle) -> Self {
        #[cfg(test)]
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        let mut spans = Vec::new();
        let mut strikethroughs = Vec::new();
        if signature.segments.is_empty() {
            push_span(
                &mut spans,
                &mut strikethroughs,
                String::new(),
                signature.empty_format,
            );
        } else {
            for segment in &signature.segments {
                push_span(
                    &mut spans,
                    &mut strikethroughs,
                    signature.text[segment.range.clone()].to_owned(),
                    segment.format,
                );
            }
        }

        let paragraph = GraphicsParagraph::with_spans(Text {
            content: spans.as_slice(),
            bounds: Size::new(
                (style.width - signature.line_padding.x()).max(1.0),
                i32::MAX as f32,
            ),
            size: style.text_size,
            line_height: style.line_height,
            font: style.font,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: style.wrapping,
        });
        let height = paragraph_height(&paragraph, style.text_size, style.line_height)
            + signature.line_padding.y();

        Self {
            signature,
            paragraph,
            spans,
            strikethroughs,
            top: 0.0,
            height,
            #[cfg(test)]
            identity: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn into_text(self) -> String {
        self.signature.text
    }

    fn into_signature(self) -> StyledLine {
        self.signature
    }
}

impl StyledLine {
    fn matches(&self, text: &str, format: &StyledLineFormat) -> bool {
        self.text == text
            && self.segments == format.segments
            && self.empty_format == format.empty_format
            && self.line_highlight == format.line_highlight
            && self.line_padding == format.line_padding
    }
}

impl StyledLineFormat {
    fn with_text(self, text: String) -> StyledLine {
        StyledLine {
            text,
            segments: self.segments,
            empty_format: self.empty_format,
            line_highlight: self.line_highlight,
            line_padding: self.line_padding,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TextLines {
    starts: Vec<usize>,
    lengths: Vec<usize>,
}

impl TextLines {
    pub(super) fn empty() -> Self {
        Self {
            starts: vec![0],
            lengths: vec![0],
        }
    }

    pub(super) fn parse(source: &str) -> (Vec<String>, Self) {
        let bytes = source.as_bytes();
        let mut lines = Vec::new();
        let mut starts = vec![0];
        let mut lengths = Vec::new();
        let mut line_start = 0;
        let mut index = 0;

        while index < bytes.len() {
            let ending_len = match bytes[index] {
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => 2,
                b'\n' if bytes.get(index + 1) == Some(&b'\r') => 2,
                b'\r' | b'\n' => 1,
                _ => {
                    index += 1;
                    continue;
                }
            };

            lines.push(source[line_start..index].to_owned());
            lengths.push(index - line_start);
            index += ending_len;
            line_start = index;
            starts.push(index);
        }

        lines.push(source[line_start..].to_owned());
        lengths.push(source.len() - line_start);

        (lines, Self { starts, lengths })
    }

    pub(super) fn offset(&self, position: Position) -> usize {
        let line = position.line.min(self.starts.len().saturating_sub(1));
        self.starts.get(line).copied().unwrap_or_default()
            + position
                .column
                .min(self.lengths.get(line).copied().unwrap_or_default())
    }

    pub(super) fn position(&self, offset: usize) -> Position {
        let line = self
            .starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        Position {
            line,
            column: offset
                .saturating_sub(self.starts.get(line).copied().unwrap_or_default())
                .min(self.lengths.get(line).copied().unwrap_or_default()),
        }
    }
}

fn styled_line_format<H>(
    source: &str,
    highlighter: &mut H,
    format: &dyn Fn(&H::Highlight) -> Format,
) -> StyledLineFormat
where
    H: text::Highlighter,
{
    let highlights = highlighter
        .highlight_line(source)
        .map(|(range, highlight)| (range, format(&highlight)))
        .collect::<Vec<_>>();
    let segments = compose_segments(source, &highlights);
    let empty_format = highlights
        .iter()
        .fold(Format::default(), |base, (_, next)| base.overlay(*next));
    let line = highlights
        .iter()
        .filter_map(|(_, format)| {
            format
                .line_highlight
                .map(|highlight| (highlight, format.line_padding))
        })
        .next_back();
    let (line_highlight, line_padding) = line
        .map_or((None, Padding::ZERO), |(highlight, padding)| {
            (Some(highlight), padding)
        });

    StyledLineFormat {
        segments,
        empty_format,
        line_highlight,
        line_padding,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Segment {
    pub(super) range: Range<usize>,
    pub(super) format: Format,
}

pub(super) fn compose_segments(line: &str, highlights: &[(Range<usize>, Format)]) -> Vec<Segment> {
    if line.is_empty() {
        return Vec::new();
    }

    let mut boundaries = vec![0, line.len()];
    for (range, _) in highlights {
        let start = range.start.min(line.len());
        let end = range.end.min(line.len());
        if start <= end && line.is_char_boundary(start) && line.is_char_boundary(end) {
            boundaries.push(start);
            boundaries.push(end);
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut segments: Vec<Segment> = Vec::new();
    for pair in boundaries.windows(2) {
        let range = pair[0]..pair[1];
        if range.is_empty()
            || !line.is_char_boundary(range.start)
            || !line.is_char_boundary(range.end)
        {
            continue;
        }
        let format = highlights
            .iter()
            .filter(|(highlight, _)| highlight.start < range.end && range.start < highlight.end)
            .fold(Format::default(), |base, (_, next)| base.overlay(*next));

        if let Some(previous) = segments.last_mut()
            && previous.range.end == range.start
            && previous.format == format
        {
            previous.range.end = range.end;
        } else {
            segments.push(Segment { range, format });
        }
    }
    segments
}

pub(super) fn to_span(source: String, format: Format) -> Span<'static, (), Font> {
    let mut span = Span::new(source);
    span.color = format.color;
    span.font = format.font;
    span.size = format.size;
    span.line_height = format.line_height;
    span.highlight = format.highlight;
    span.padding = format.padding;
    span.strikethrough = format.strikethrough.is_some();
    span
}

pub(super) fn push_span(
    spans: &mut Vec<Span<'static, (), Font>>,
    strikethroughs: &mut Vec<Option<Color>>,
    source: String,
    format: Format,
) {
    strikethroughs.push(format.strikethrough);
    spans.push(to_span(source, format));
}

pub(super) fn paragraph_height(
    paragraph: &GraphicsParagraph,
    size: Pixels,
    line_height: text::LineHeight,
) -> f32 {
    paragraph
        .buffer()
        .layout_runs()
        .map(|run| run.line_top + run.line_height)
        .reduce(f32::max)
        .unwrap_or_else(|| line_height.to_absolute(size).0)
}

pub(super) fn hit_position(buffer: &cosmic_text::Buffer, point: Point) -> Position {
    let height = buffer
        .layout_runs()
        .map(|run| run.line_top + run.line_height)
        .fold(0.0, f32::max);
    let point = Point::new(
        point.x.max(0.0),
        point.y.clamp(0.0, (height - 0.5).max(0.0)),
    );
    let cursor = buffer.hit(point.x, point.y).unwrap_or_else(|| {
        let line = buffer.lines.len().saturating_sub(1);
        cosmic_text::Cursor::new(
            line,
            buffer.lines.get(line).map_or(0, |line| line.text().len()),
        )
    });
    Position {
        line: cursor.line,
        column: cursor.index,
    }
}

pub(super) fn caret_rectangle(buffer: &cosmic_text::Buffer, position: Position) -> Rectangle {
    let mut previous = None;
    for run in buffer
        .layout_runs()
        .filter(|run| run.line_i == position.line)
    {
        let start = run.glyphs.first().map_or(0, |glyph| glyph.start);
        let end = run.glyphs.last().map_or(start, |glyph| glyph.end);

        if start > position.column {
            return previous.unwrap_or_else(|| {
                Rectangle::new(
                    Point::new(0.0, run.line_top),
                    Size::new(1.0, run.line_height),
                )
            });
        }

        let cursor = cosmic_text::Cursor::new(position.line, position.column);
        if position.column <= end {
            let x = run
                .highlight(cursor, cursor)
                .map_or_else(|| caret_x(run.glyphs, position.column), |(x, _)| x);
            return Rectangle::new(Point::new(x, run.line_top), Size::new(1.0, run.line_height));
        }

        previous = Some(Rectangle::new(
            Point::new(run.line_w, run.line_top),
            Size::new(1.0, run.line_height),
        ));
    }

    previous.unwrap_or_else(|| {
        let metrics = buffer.metrics();
        Rectangle::new(
            Point::new(0.0, position.line as f32 * metrics.line_height),
            Size::new(1.0, metrics.line_height),
        )
    })
}

fn caret_x(glyphs: &[cosmic_text::LayoutGlyph], index: usize) -> f32 {
    glyphs
        .iter()
        .find(|glyph| index <= glyph.start)
        .map_or_else(
            || glyphs.last().map_or(0.0, |glyph| glyph.x + glyph.w),
            |glyph| glyph.x,
        )
}

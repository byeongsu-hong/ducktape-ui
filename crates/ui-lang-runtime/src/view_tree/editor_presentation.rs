//! Resolve copied editor formatting through the host's existing font registry.
use super::*;
use crate::rich_text_editor::{Format, PresentedLine};
use std::sync::Arc;
use wire::editor_presentation::EditorPresentation;

pub(super) fn lines(value: &EditorPresentation, source: &str) -> Option<Arc<[PresentedLine]>> {
    value.validate(source).ok()?;
    let formats: Vec<_> = value.formats.iter().map(format).collect();
    let mut lines: Vec<PresentedLine> = Vec::new();
    for span in &value.spans {
        if lines
            .last()
            .is_none_or(|line| line.line != span.line as usize)
        {
            lines.push(PresentedLine {
                line: span.line as usize,
                spans: Vec::new(),
            });
        }
        lines.last_mut()?.spans.push((
            span.start as usize..span.end as usize,
            formats[usize::from(span.format)],
        ));
    }
    Some(lines.into())
}

fn format(value: &wire::editor_presentation::EditorFormat) -> Format {
    Format {
        color: value.color.map(color),
        font: value.font.as_ref().map(text::named_font),
        size: value.size.map(iced::Pixels),
        line_height: value.line_height.map(|height| match height {
            wire::LineHeight::Relative(value) => widget::text::LineHeight::Relative(value),
            wire::LineHeight::Absolute(value) => widget::text::LineHeight::Absolute(value.into()),
        }),
        highlight: highlight(value.background, value.border),
        line_highlight: highlight(value.line_background, value.line_border),
        line_padding: padding(value.line_padding),
        line_rule: value.line_rule.map(color),
        strikethrough: value.strikethrough.map(color),
        // A span's padding is paint-only (it sizes the highlight quad around
        // the glyph run) and the wire lets it go negative on purpose — a
        // guest draws a box smaller than the line with it. `line_padding`
        // moves layout and keeps its floor through `padding` above.
        padding: span_padding(value.padding),
    }
}

fn span_padding(edges: wire::Edges) -> iced::Padding {
    iced::Padding {
        top: signed_pixels(edges.top),
        right: signed_pixels(edges.right),
        bottom: signed_pixels(edges.bottom),
        left: signed_pixels(edges.left),
    }
}

/// A finite pixel measure that may point either way; NaN reads as 0.
fn signed_pixels(value: f32) -> f32 {
    match value.is_nan() {
        true => 0.0,
        false => value.clamp(-f32::MAX, f32::MAX),
    }
}

fn highlight(
    background: Option<wire::Rgba>,
    edge: Option<wire::Border>,
) -> Option<iced::advanced::text::Highlight> {
    (background.is_some() || edge.is_some()).then(|| iced::advanced::text::Highlight {
        background: background.map(color).unwrap_or(Color::TRANSPARENT).into(),
        border: edge.map(border).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wire::editor_presentation::{EditorFormat, EditorSpan};

    #[test]
    fn a_negative_span_inset_reaches_the_native_highlight_but_line_padding_keeps_its_floor() {
        let value = EditorPresentation {
            formats: vec![EditorFormat {
                padding: wire::Edges {
                    top: -4.55,
                    right: 5.25,
                    bottom: -4.55,
                    left: f32::NAN,
                },
                line_padding: wire::Edges {
                    top: -4.55,
                    right: 5.25,
                    bottom: -4.55,
                    left: 1.0,
                },
                ..Default::default()
            }],
            spans: vec![EditorSpan {
                line: 1,
                start: 0,
                end: 2,
                format: 0,
            }],
            ..Default::default()
        };
        let native = lines(&value, "Title\n- 한글").unwrap();
        let format = native[0].spans[0].1;
        assert_eq!(
            format.padding.top, -4.55,
            "a paint-only inset keeps its sign"
        );
        assert_eq!(format.padding.bottom, -4.55);
        assert_eq!(format.padding.right, 5.25);
        assert_eq!(format.padding.left, 0.0, "NaN reads as 0");
        assert_eq!(
            format.line_padding.top, 0.0,
            "line padding moves layout: never negative"
        );
        assert_eq!(format.line_padding.bottom, 0.0);
        assert_eq!(format.line_padding.right, 5.25);
        assert_eq!(format.line_padding.left, 1.0);
    }

    #[test]
    fn copied_marker_and_line_formats_reach_the_native_highlighter() {
        let background = wire::Rgba([0.1, 0.2, 0.3, 1.0]);
        let value = EditorPresentation {
            formats: vec![EditorFormat {
                size: Some(0.01),
                color: Some(wire::Rgba([0.0; 4])),
                line_background: Some(background),
                line_padding: wire::Edges {
                    left: 22.0,
                    ..Default::default()
                },
                line_height: Some(wire::LineHeight::Relative(1.65)),
                ..Default::default()
            }],
            spans: vec![EditorSpan {
                line: 1,
                start: 0,
                end: 2,
                format: 0,
            }],
            ..Default::default()
        };
        let native = lines(&value, "Title\n- 한글").unwrap();
        assert_eq!(native.len(), 1);
        assert_eq!(native[0].line, 1);
        assert_eq!(native[0].spans[0].0, 0..2);
        let format = native[0].spans[0].1;
        assert_eq!(format.size, Some(iced::Pixels(0.01)));
        assert_eq!(format.line_padding.left, 22.0);
        assert_eq!(
            format.line_height,
            Some(widget::text::LineHeight::Relative(1.65))
        );
        assert_eq!(
            format.line_highlight.unwrap().background,
            iced::Background::Color(color(background))
        );
    }
}

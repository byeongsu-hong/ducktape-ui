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
        padding: padding(value.padding),
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

use super::*;

#[test]
fn overlapping_formats_keep_block_metrics_under_token_colors() {
    let block = Format {
        size: Some(Pixels(14.0)),
        line_height: Some(text::LineHeight::Absolute(Pixels(24.0))),
        line_padding: Padding::from([0.0, 12.0]),
        line_highlight: Some(text::Highlight {
            background: iced::Background::Color(Color::BLACK),
            border: iced::Border::default(),
        }),
        ..Format::default()
    };
    let token = Format {
        color: Some(Color::from_rgb(1.0, 0.0, 0.0)),
        ..Format::default()
    };

    let segments = compose_segments("let value", &[(0..9, block), (4..9, token)]);

    assert_eq!(segments.len(), 2);
    assert_eq!(segments[1].format.size, block.size);
    assert_eq!(segments[1].format.line_height, block.line_height);
    assert_eq!(segments[1].format.color, token.color);
    assert_eq!(segments[1].format.line_highlight, block.line_highlight);
    assert_eq!(segments[1].format.line_padding, block.line_padding);
}

#[test]
fn line_padding_changes_wrapping_caret_and_hit_geometry() {
    let source = Content::with_text("code that wraps");
    let padding = Padding {
        top: 4.0,
        right: 12.0,
        bottom: 6.0,
        left: 12.0,
    };
    let mut document = DocumentLayout::default();
    document.update(
        &content_lines(&source),
        &mut WholeLine::default(),
        &|_| Format {
            line_highlight: Some(text::Highlight {
                background: iced::Background::Color(Color::BLACK),
                border: iced::Border::default(),
            }),
            line_padding: padding,
            ..Format::default()
        },
        test_layout_style(100.0),
        DocumentUpdate::text(DocumentChange::Discover),
    );

    let line = &document.lines[0];
    assert!((line.paragraph.bounds().width - 76.0).abs() < 0.01);
    assert!(
        (line.height
            - paragraph_height(
                &line.paragraph,
                Pixels(16.0),
                text::LineHeight::Relative(1.6),
            )
            - padding.y())
        .abs()
            < 0.01
    );

    let start = document.caret(Position { line: 0, column: 0 });
    assert!((start.x - padding.left).abs() < 0.01);
    assert!((start.y - padding.top).abs() < 0.01);
    assert_eq!(
        document.hit(Point::new(start.x, start.y + start.height / 2.0)),
        Position { line: 0, column: 0 }
    );
    assert_eq!(
        document.hit_test(Point::new(start.x, start.y + start.height / 2.0)),
        Some(Position { line: 0, column: 0 })
    );
    assert_eq!(
        document.hit_test(Point::new(99.0, start.y + start.height / 2.0)),
        None
    );
}

#[test]
fn inline_highlight_padding_cannot_bleed_into_adjacent_lines() {
    let bounds = Rectangle::new(Point::new(20.0, 10.0), Size::new(30.0, 20.0));
    let line = Rectangle::new(Point::new(0.0, 10.0), Size::new(100.0, 20.0));
    let padded = span_highlight_bounds(
        bounds,
        Padding {
            top: 5.0,
            right: 6.0,
            bottom: 5.0,
            left: 6.0,
        },
        line,
    )
    .expect("visible highlight");

    assert_eq!(
        padded,
        Rectangle::new(Point::new(14.0, 10.0), Size::new(42.0, 20.0))
    );
}

#[test]
fn hidden_markers_and_heading_text_share_one_hit_test_layout() {
    let spans = vec![
        to_span(
            "# ".to_owned(),
            Format {
                size: Some(Pixels(0.01)),
                color: Some(Color::TRANSPARENT),
                ..Format::default()
            },
        ),
        to_span(
            "Heading".to_owned(),
            Format {
                size: Some(Pixels(30.0)),
                line_height: Some(text::LineHeight::Absolute(Pixels(42.0))),
                ..Format::default()
            },
        ),
    ];
    let paragraph = GraphicsParagraph::with_spans(Text {
        content: spans.as_slice(),
        bounds: Size::new(500.0, 500.0),
        size: Pixels(16.0),
        line_height: text::LineHeight::Relative(1.6),
        font: Font::DEFAULT,
        align_x: text::Alignment::Default,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::Word,
    });

    let caret = caret_rectangle(paragraph.buffer(), Position { line: 0, column: 2 });
    let hit = hit_position(paragraph.buffer(), Point::new(caret.x, caret.y + 1.0));

    assert_eq!(hit.line, 0);
    assert_eq!(hit.column, 2);
    assert!(caret.height >= 42.0);
}

#[test]
fn line_paragraphs_preserve_whole_document_caret_geometry() {
    let heading = Format {
        size: Some(Pixels(30.0)),
        line_height: Some(text::LineHeight::Absolute(Pixels(42.0))),
        ..Format::default()
    };
    let hidden = Format {
        size: Some(Pixels(0.01)),
        color: Some(Color::TRANSPARENT),
        ..Format::default()
    };
    let code = Format {
        size: Some(Pixels(14.0)),
        line_height: Some(text::LineHeight::Absolute(Pixels(24.0))),
        ..Format::default()
    };
    let signatures = [
        StyledLine {
            text: "# 제목".to_owned(),
            segments: vec![
                Segment {
                    range: 0..2,
                    format: hidden,
                },
                Segment {
                    range: 2.."# 제목".len(),
                    format: heading,
                },
            ],
            empty_format: Format::default(),
            line_highlight: None,
            line_padding: Padding::ZERO,
        },
        StyledLine {
            text: "a body line long enough to wrap".to_owned(),
            segments: vec![Segment {
                range: 0.."a body line long enough to wrap".len(),
                format: Format::default(),
            }],
            empty_format: Format::default(),
            line_highlight: None,
            line_padding: Padding::ZERO,
        },
        StyledLine {
            text: String::new(),
            segments: Vec::new(),
            empty_format: code,
            line_highlight: None,
            line_padding: Padding::ZERO,
        },
        StyledLine {
            text: "let value = 1;".to_owned(),
            segments: vec![Segment {
                range: 0.."let value = 1;".len(),
                format: code,
            }],
            empty_format: Format::default(),
            line_highlight: None,
            line_padding: Padding::ZERO,
        },
    ];
    let style = test_layout_style(120.0);

    let mut document = DocumentLayout::default();
    for signature in signatures.iter().cloned() {
        let mut line = DocumentLine::new(signature, style);
        line.top = document.height;
        document.height += line.height;
        document.lines.push(line);
    }

    let mut reference_spans = Vec::new();
    for (line_index, signature) in signatures.iter().enumerate() {
        let ending = (line_index + 1 < signatures.len()).then_some("\n");
        if signature.segments.is_empty() {
            reference_spans.push(to_span(
                ending.unwrap_or_default().to_owned(),
                signature.empty_format,
            ));
            continue;
        }
        for (segment_index, segment) in signature.segments.iter().enumerate() {
            let mut text = signature.text[segment.range.clone()].to_owned();
            if segment_index + 1 == signature.segments.len()
                && let Some(ending) = ending
            {
                text.push_str(ending);
            }
            reference_spans.push(to_span(text, segment.format));
        }
    }
    let reference = GraphicsParagraph::with_spans(Text {
        content: reference_spans.as_slice(),
        bounds: Size::new(style.width, i32::MAX as f32),
        size: style.text_size,
        line_height: style.line_height,
        font: style.font,
        align_x: text::Alignment::Default,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: style.wrapping,
    });

    let reference_height = paragraph_height(&reference, style.text_size, style.line_height);
    assert!(
        (document.height - reference_height).abs() < 0.01,
        "document height {} != reference height {reference_height}",
        document.height
    );
    for (line, signature) in signatures.iter().enumerate() {
        for column in [0, signature.text.len()] {
            let expected = caret_rectangle(reference.buffer(), Position { line, column });
            let actual = document.caret(Position { line, column });
            assert!(
                (actual.x - expected.x).abs() < 0.01
                    && (actual.y - expected.y).abs() < 0.01
                    && (actual.height - expected.height).abs() < 0.01,
                "caret mismatch at {line}:{column}: {actual:?} != {expected:?}"
            );
            let point = Point::new(expected.x, expected.y + expected.height / 2.0);
            assert_eq!(document.hit(point), hit_position(reference.buffer(), point));
        }
    }
}

#[test]
fn empty_formatted_lines_keep_their_rich_metrics() {
    let content = Content::with_text("\n");
    let format = Format {
        size: Some(Pixels(14.0)),
        line_height: Some(text::LineHeight::Absolute(Pixels(23.0))),
        ..Format::default()
    };
    let mut document = DocumentLayout::default();
    let rebuilt = document.update(
        &content_lines(&content),
        &mut WholeLine::default(),
        &|_| format,
        test_layout_style(500.0),
        DocumentUpdate::text(DocumentChange::Discover),
    );

    assert_eq!(rebuilt.rebuilt_lines, content.line_count());
    assert_eq!(document.lines.len(), content.line_count());
    assert!(
        document
            .lines
            .iter()
            .all(|line| line.spans.len() == 1 && line.spans[0].size == format.size)
    );
    assert!(
        document
            .lines
            .iter()
            .all(|line| line.strikethroughs == [None] && line.height >= 23.0)
    );
}

#[test]
fn strikethrough_keeps_its_explicit_color() {
    let color = Color::from_rgb8(0x12, 0x34, 0x56);
    let mut spans = Vec::new();
    let mut strikethroughs = Vec::new();
    push_span(
        &mut spans,
        &mut strikethroughs,
        "old".to_owned(),
        Format {
            color: Some(Color::WHITE),
            strikethrough: Some(color),
            ..Format::default()
        },
    );

    assert_eq!(strikethroughs, vec![Some(color)]);
    assert!(spans[0].strikethrough);
}

#[test]
fn consecutive_line_highlights_share_one_surface() {
    let code = text::Highlight {
        background: iced::Background::Color(Color::BLACK),
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: Color::WHITE,
        },
    };
    let quote = text::Highlight {
        background: iced::Background::Color(Color::WHITE),
        border: iced::Border::default(),
    };
    let runs = [
        (Some(code), 0.0, 12.0),
        (Some(code), 12.0, 12.0),
        (Some(code), 24.0, 12.0),
        (None, 36.0, 12.0),
        (Some(code), 48.0, 12.0),
        (Some(quote), 60.0, 12.0),
    ];
    let mut groups = Vec::new();

    visit_line_highlight_groups(runs, |group| groups.push(group));

    assert_eq!(
        groups,
        vec![
            LineHighlightGroup {
                top: 0.0,
                height: 36.0,
                highlight: code,
            },
            LineHighlightGroup {
                top: 48.0,
                height: 12.0,
                highlight: code,
            },
            LineHighlightGroup {
                top: 60.0,
                height: 12.0,
                highlight: quote,
            },
        ]
    );

    let highlights = vec![Some(code); 256];
    let runs = highlights
        .iter()
        .copied()
        .enumerate()
        .map(|(line, highlight)| (highlight, line as f32 * 12.0, 12.0));
    groups.clear();

    visit_line_highlight_groups(runs, |group| groups.push(group));

    assert_eq!(
        groups,
        vec![LineHighlightGroup {
            top: 0.0,
            height: highlights.len() as f32 * 12.0,
            highlight: code,
        }]
    );
}

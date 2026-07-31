use super::*;

#[test]
fn ime_stages_rebuild_only_the_changed_line_in_a_long_document() {
    let mut lines = (0..1_000)
        .map(|index| format!("stable line {index}"))
        .collect::<Vec<_>>();
    let mut highlighter = WholeLine::default();
    let mut document = DocumentLayout::default();
    let style = test_layout_style(700.0);

    assert_eq!(
        document.update(
            &lines,
            &mut highlighter,
            &|_| Format::default(),
            style,
            false,
            false,
        ),
        lines.len()
    );

    for stage in ["ㅇ", "으", "응"] {
        lines[500] = format!("stable line 500 {stage}");
        assert_eq!(
            document.update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                false,
                false,
            ),
            1,
            "{stage:?} must not reshape unchanged paragraphs"
        );
    }
}

#[test]
fn line_insertions_reuse_the_unchanged_suffix() {
    let mut lines = vec!["first".to_owned(), "second".to_owned(), "third".to_owned()];
    let mut highlighter = WholeLine::default();
    let mut document = DocumentLayout::default();
    let style = test_layout_style(700.0);

    assert_eq!(
        document.update(
            &lines,
            &mut highlighter,
            &|_| Format::default(),
            style,
            false,
            false,
        ),
        3
    );

    lines.insert(1, "inserted".to_owned());
    assert_eq!(
        document.update(
            &lines,
            &mut highlighter,
            &|_| Format::default(),
            style,
            false,
            false,
        ),
        1
    );

    lines.remove(1);
    assert_eq!(
        document.update(
            &lines,
            &mut highlighter,
            &|_| Format::default(),
            style,
            false,
            false,
        ),
        0
    );
}

#[test]
fn content_version_distinguishes_document_replacement_from_text_revision() {
    let renderer = headless_renderer();
    let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 120.0));
    let mut content = Content::with_text("first document");
    let mut editor = RichTextEditor::<_, ()>::new(&content, ContentVersion::new(7, 0))
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(120.0));
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);

    editor.layout(&mut tree, &renderer, &limits);
    drop(editor);
    tree.state
        .downcast_mut::<State<text::highlighter::PlainText>>()
        .metrics = LayoutMetrics::default();

    content = Content::with_text("second document");
    let mut editor = RichTextEditor::<_, ()>::new(&content, ContentVersion::new(8, 0))
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(120.0));
    editor.layout(&mut tree, &renderer, &limits);

    let state = tree
        .state
        .downcast_ref::<State<text::highlighter::PlainText>>();
    assert_eq!(state.source, "second document");
    assert_eq!(state.metrics.full_text_materializations, 1);
    assert_eq!(state.metrics.rebuilt_lines, 1);
    assert_eq!(state.content_version, Some(ContentVersion::new(8, 0)));
}

#[test]
#[ignore = "large-document performance contract run explicitly in CI"]
fn performance_contract_content_version_skips_large_caret_snapshots() {
    let source = (0..100_000)
        .map(|index| format!("line {index}\n"))
        .collect::<String>();
    let mut content = Content::with_text(&source);
    let renderer = headless_renderer();
    let limits = layout::Limits::new(Size::ZERO, Size::new(800.0, 600.0));
    let version = ContentVersion::new(11, 0);
    let mut editor = RichTextEditor::<_, ()>::new(&content, version)
        .width(Length::Fixed(800.0))
        .height(Length::Fixed(600.0))
        .wrapping(text::Wrapping::None);
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);

    editor.layout(&mut tree, &renderer, &limits);
    drop(editor);
    tree.state
        .downcast_mut::<State<text::highlighter::PlainText>>()
        .metrics = LayoutMetrics::default();

    for event in 0..1_000 {
        let line = event * 97 % 100_000;
        content.move_to(Cursor {
            position: Position {
                line,
                column: event % 8,
            },
            selection: (event % 2 == 0).then_some(Position {
                line: line.saturating_sub(1),
                column: 0,
            }),
        });
        let mut editor = RichTextEditor::<_, ()>::new(&content, version)
            .width(Length::Fixed(800.0))
            .height(Length::Fixed(600.0))
            .wrapping(text::Wrapping::None);
        editor.layout(&mut tree, &renderer, &limits);
    }

    let state = tree
        .state
        .downcast_ref::<State<text::highlighter::PlainText>>();
    assert_eq!(state.document.lines.len(), 100_001);
    assert_eq!(state.metrics.full_text_materializations, 0);
    assert_eq!(state.metrics.rebuilt_lines, 0);

    tree.state
        .downcast_mut::<State<text::highlighter::PlainText>>()
        .metrics = LayoutMetrics::default();
    content.perform(text_editor::Action::Move(Motion::Right));
    content.move_to(Cursor {
        position: Position {
            line: 50_000,
            column: 4,
        },
        selection: None,
    });
    assert_eq!(
        content.line(50_000).map(|line| line.text.into_owned()),
        Some("line 50000".to_owned())
    );
    assert_eq!(
        content.cursor(),
        Cursor {
            position: Position {
                line: 50_000,
                column: 4,
            },
            selection: None,
        }
    );
    content.perform(text_editor::Action::Edit(Edit::Insert('x')));
    assert_eq!(
        content.line(50_000).map(|line| line.text.into_owned()),
        Some("linex 50000".to_owned())
    );
    let mut editor = RichTextEditor::<_, ()>::new(&content, ContentVersion::new(11, 1))
        .width(Length::Fixed(800.0))
        .height(Length::Fixed(600.0))
        .wrapping(text::Wrapping::None);
    editor.layout(&mut tree, &renderer, &limits);

    let state = tree
        .state
        .downcast_ref::<State<text::highlighter::PlainText>>();
    assert_eq!(state.metrics.full_text_materializations, 1);
    assert_eq!(state.metrics.rebuilt_lines, 1);
    assert_eq!(state.document.lines[50_000].signature.text, "linex 50000");
}

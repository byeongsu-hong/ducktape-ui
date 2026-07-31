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
        document
            .update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                DocumentUpdate::text(DocumentChange::Discover),
            )
            .rebuilt_lines,
        lines.len()
    );

    for stage in ["ㅇ", "으", "응"] {
        lines[500] = format!("stable line 500 {stage}");
        assert_eq!(
            document
                .update(
                    &lines,
                    &mut highlighter,
                    &|_| Format::default(),
                    style,
                    DocumentUpdate::text(DocumentChange::Discover),
                )
                .rebuilt_lines,
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
        document
            .update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                DocumentUpdate::text(DocumentChange::Discover),
            )
            .rebuilt_lines,
        3
    );

    lines.insert(1, "inserted".to_owned());
    assert_eq!(
        document
            .update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                DocumentUpdate::text(DocumentChange::Discover),
            )
            .rebuilt_lines,
        1
    );

    lines.remove(1);
    assert_eq!(
        document
            .update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                DocumentUpdate::text(DocumentChange::Discover),
            )
            .rebuilt_lines,
        0
    );
}

#[test]
fn change_hint_maps_replacements_insertions_undo_and_redo_without_line_diffing() {
    let style = test_layout_style(700.0);
    let mut highlighter = WholeLine::default();
    let mut document = DocumentLayout::default();
    let original = vec!["first".to_owned(), "second".to_owned(), "third".to_owned()];
    document.update(
        &original,
        &mut highlighter,
        &|_| Format::default(),
        style,
        DocumentUpdate::text(DocumentChange::Discover),
    );
    let original_ids = document
        .lines
        .iter()
        .map(|line| line.identity)
        .collect::<Vec<_>>();

    let replaced = vec!["first".to_owned(), "SECOND".to_owned(), "third".to_owned()];
    let update = document.update(
        &replaced,
        &mut highlighter,
        &|_| Format::default(),
        style,
        DocumentUpdate::text(DocumentChange::Hint(EditorChange::new(1, 1, 1))),
    );
    assert_eq!(update.compared_lines, 0);
    assert_eq!(update.rebuilt_lines, 1);
    assert_eq!(update.shaped_paragraphs, 1);
    assert_eq!(update.highlighted_lines, 2);
    assert!(update.change_hint_used);
    assert_eq!(document.lines[0].identity, original_ids[0]);
    assert_eq!(document.lines[2].identity, original_ids[2]);
    let replaced_ids = document
        .lines
        .iter()
        .map(|line| line.identity)
        .collect::<Vec<_>>();

    let inserted = vec![
        "first".to_owned(),
        "inserted".to_owned(),
        "SECOND".to_owned(),
        "third".to_owned(),
    ];
    let update = document.update(
        &inserted,
        &mut highlighter,
        &|_| Format::default(),
        style,
        DocumentUpdate::text(DocumentChange::Hint(EditorChange::new(1, 0, 1))),
    );
    assert_eq!(update.compared_lines, 0);
    assert_eq!(update.rebuilt_lines, 1);
    assert!(update.change_hint_used);
    assert_eq!(document.lines[2].identity, replaced_ids[1]);
    assert_eq!(document.lines[3].identity, replaced_ids[2]);

    let update = document.update(
        &replaced,
        &mut highlighter,
        &|_| Format::default(),
        style,
        DocumentUpdate::text(DocumentChange::Hint(EditorChange::new(1, 1, 0))),
    );
    assert_eq!(update.compared_lines, 0);
    assert_eq!(update.rebuilt_lines, 0);
    assert_eq!(document.lines[1].identity, replaced_ids[1]);
    assert_eq!(document.lines[2].identity, replaced_ids[2]);

    let update = document.update(
        &inserted,
        &mut highlighter,
        &|_| Format::default(),
        style,
        DocumentUpdate::text(DocumentChange::Hint(EditorChange::new(1, 0, 1))),
    );
    assert_eq!(update.compared_lines, 0);
    assert_eq!(update.rebuilt_lines, 1);
    assert_eq!(document.lines[2].identity, replaced_ids[1]);
    assert_eq!(document.lines[3].identity, replaced_ids[2]);
}

#[test]
fn invalid_change_hints_fall_back_to_exact_diffing() {
    let style = test_layout_style(700.0);
    for invalid in [
        EditorChange::new(4, 0, 0),
        EditorChange::new(1, 0, 1),
        EditorChange::new(usize::MAX, 1, 1),
    ] {
        let mut highlighter = WholeLine::default();
        let mut document = DocumentLayout::default();
        document.update(
            &["first".to_owned(), "second".to_owned(), "third".to_owned()],
            &mut highlighter,
            &|_| Format::default(),
            style,
            DocumentUpdate::text(DocumentChange::Discover),
        );
        let changed = ["first".to_owned(), "SECOND".to_owned(), "third".to_owned()];
        let update = document.update(
            &changed,
            &mut highlighter,
            &|_| Format::default(),
            style,
            DocumentUpdate::text(DocumentChange::Hint(invalid)),
        );

        assert!(update.change_hint_rejected, "{invalid:?}");
        assert!(!update.change_hint_used, "{invalid:?}");
        assert!(update.compared_lines > 0, "{invalid:?}");
        assert_eq!(update.rebuilt_lines, 1, "{invalid:?}");
        assert_eq!(document.lines[1].signature.text, "SECOND");
    }
}

#[test]
fn insertion_hint_keeps_an_identical_shifted_suffix_line_identity() {
    let style = test_layout_style(700.0);
    let mut highlighter = WholeLine::default();
    let mut document = DocumentLayout::default();
    let original = [
        "first".to_owned(),
        "duplicate".to_owned(),
        "last".to_owned(),
    ];
    document.update(
        &original,
        &mut highlighter,
        &|_| Format::default(),
        style,
        DocumentUpdate::text(DocumentChange::Discover),
    );
    let shifted_identity = document.lines[1].identity;

    let inserted = [
        "first".to_owned(),
        "duplicate".to_owned(),
        "duplicate".to_owned(),
        "last".to_owned(),
    ];
    let update = document.update(
        &inserted,
        &mut highlighter,
        &|_| Format::default(),
        style,
        DocumentUpdate::text(DocumentChange::Hint(EditorChange::new(1, 0, 1))),
    );

    assert_eq!(update.rebuilt_lines, 1);
    assert_ne!(document.lines[1].identity, shifted_identity);
    assert_eq!(document.lines[2].identity, shifted_identity);
}

#[test]
fn change_hint_restarts_stateful_highlighting_at_the_changed_line() {
    let style = test_layout_style(700.0);
    let mut highlighter = <ToggleHighlighter as text::Highlighter>::new(&());
    let mut document = DocumentLayout::default();
    document.update(
        &["before".to_owned(), "middle".to_owned(), "after".to_owned()],
        &mut highlighter,
        &|inside| Format {
            color: inside.then_some(Color::BLACK),
            ..Format::default()
        },
        style,
        DocumentUpdate::text(DocumentChange::Discover),
    );

    let changed = ["before".to_owned(), "toggle".to_owned(), "after".to_owned()];
    let update = document.update(
        &changed,
        &mut highlighter,
        &|inside| Format {
            color: inside.then_some(Color::BLACK),
            ..Format::default()
        },
        style,
        DocumentUpdate::text(DocumentChange::Hint(EditorChange::new(1, 1, 1))),
    );

    assert_eq!(update.compared_lines, 0);
    assert_eq!(update.highlighted_lines, 2);
    assert_eq!(update.rebuilt_lines, 2);
    assert_eq!(
        document.lines[2].signature.segments[0].format.color,
        Some(Color::BLACK)
    );
}

#[test]
fn widget_change_hint_separates_materialization_diff_and_shaping_metrics() {
    let renderer = headless_renderer();
    let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 120.0));
    let content = Content::with_text("first\nsecond\nthird");
    let mut editor = RichTextEditor::<_, ()>::new(&content, ContentVersion::new(9, 0))
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(120.0));
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
    editor.layout(&mut tree, &renderer, &limits);
    drop(editor);
    tree.state
        .downcast_mut::<State<text::highlighter::PlainText>>()
        .metrics = LayoutMetrics::default();

    let content = Content::with_text("first\nseXcond\nthird");
    let mut editor = RichTextEditor::<_, ()>::new(&content, ContentVersion::new(9, 1))
        .change_hint(EditorChange::new(1, 1, 1))
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(120.0));
    editor.layout(&mut tree, &renderer, &limits);

    let state = tree
        .state
        .downcast_ref::<State<text::highlighter::PlainText>>();
    assert_eq!(state.metrics.full_text_materializations, 1);
    assert_eq!(state.metrics.compared_lines, 0);
    assert_eq!(state.metrics.rebuilt_lines, 1);
    assert_eq!(state.metrics.shaped_paragraphs, 1);
    assert_eq!(state.metrics.highlighted_lines, 2);
    assert_eq!(state.metrics.accepted_change_hints, 1);
    assert_eq!(state.metrics.rejected_change_hints, 0);
    assert_eq!(state.document.lines[1].signature.text, "seXcond");
}

#[test]
fn caret_selection_and_viewport_resize_do_not_rediscover_line_changes() {
    let renderer = headless_renderer();
    let content = Content::with_text("first\nsecond\nthird");
    let version = ContentVersion::new(10, 0);
    let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 120.0));
    let mut editor = RichTextEditor::<_, ()>::new(&content, version)
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(120.0));
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
    editor.layout(&mut tree, &renderer, &limits);
    drop(editor);
    tree.state
        .downcast_mut::<State<text::highlighter::PlainText>>()
        .metrics = LayoutMetrics::default();

    let mut content = content;
    content.move_to(Cursor {
        position: Position { line: 2, column: 3 },
        selection: Some(Position { line: 1, column: 1 }),
    });
    let mut editor = RichTextEditor::<_, ()>::new(&content, version)
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(120.0));
    editor.layout(&mut tree, &renderer, &limits);
    assert_eq!(
        tree.state
            .downcast_ref::<State<text::highlighter::PlainText>>()
            .metrics,
        LayoutMetrics::default()
    );
    drop(editor);

    let resized_limits = layout::Limits::new(Size::ZERO, Size::new(320.0, 120.0));
    let mut editor = RichTextEditor::<_, ()>::new(&content, version)
        .width(Length::Fixed(320.0))
        .height(Length::Fixed(120.0));
    editor.layout(&mut tree, &renderer, &resized_limits);
    let state = tree
        .state
        .downcast_ref::<State<text::highlighter::PlainText>>();
    assert_eq!(state.metrics.full_text_materializations, 0);
    assert_eq!(state.metrics.compared_lines, 0);
    assert_eq!(state.metrics.rebuilt_lines, 3);
    assert_eq!(state.metrics.shaped_paragraphs, 3);
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
    assert_eq!(state.metrics.compared_lines, 0);
    assert_eq!(state.metrics.rebuilt_lines, 0);
    assert_eq!(state.metrics.shaped_paragraphs, 0);
    assert_eq!(state.metrics.highlighted_lines, 0);

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
    let started = Instant::now();
    let mut editor = RichTextEditor::<_, ()>::new(&content, ContentVersion::new(11, 1))
        .change_hint(EditorChange::new(50_000, 1, 1))
        .width(Length::Fixed(800.0))
        .height(Length::Fixed(600.0))
        .wrapping(text::Wrapping::None);
    editor.layout(&mut tree, &renderer, &limits);
    let elapsed = started.elapsed();

    let state = tree
        .state
        .downcast_ref::<State<text::highlighter::PlainText>>();
    assert_eq!(state.metrics.full_text_materializations, 1);
    assert_eq!(state.metrics.compared_lines, 0);
    assert_eq!(state.metrics.rebuilt_lines, 1);
    assert_eq!(state.metrics.shaped_paragraphs, 1);
    assert_eq!(state.metrics.highlighted_lines, 50_001);
    assert_eq!(state.metrics.accepted_change_hints, 1);
    assert_eq!(state.document.lines[50_000].signature.text, "linex 50000");
    assert!(
        elapsed < Duration::from_secs(5),
        "hinted 100k-line edit took {elapsed:?}"
    );
    eprintln!("hinted 100k-line edit: {elapsed:?}");
}

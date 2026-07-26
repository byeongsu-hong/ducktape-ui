ui_lang::include_app!("src/ui/showcase.ice");

#[test]
fn qr_data_initializes() {
    let _ = Showcase::__boot();
}

#[test]
fn appends_markdown_and_tracks_image_uris() {
    let (mut app, _) = Showcase::__boot();
    assert!(app.help_images.is_empty());

    assert_eq!(app.__update(__ShowcaseMessage::ExtendMarkdown).units(), 1);
    assert_eq!(app.help_images, ["asset://ice"]);
}

#[test]
fn resizes_a_named_nested_pane_split() {
    let (mut app, _) = Showcase::__boot();
    let split = app.__pane_nested_workspace_splits["editor_stack"];

    let _ = app.__update(__ShowcaseMessage::ResizeNestedEditor);

    let regions =
        app.__pane_nested_workspace
            .layout()
            .split_regions(0.0, 0.0, iced::Size::new(100.0, 100.0));
    assert_eq!(regions[&split].2, 0.45);
}

#[test]
fn constructs_a_native_pane_grid_style() {
    let style = crate::backend::workspace_panes(&iced::Theme::Dark, true);
    assert_eq!(style.hovered_split.width, 5.0);

    let (app, _) = Showcase::__boot();
    let _ = app.__view();
}

#[test]
fn opens_and_renders_a_runtime_pane_template() {
    let (mut app, _) = Showcase::__boot();
    app.tasks = vec![crate::backend::Task {
        id: 1,
        title: "Dynamic pane".into(),
        done: false,
    }];

    let _ = app.__update(__ShowcaseMessage::OpenTaskPane);
    assert!(
        app.__pane_nested_workspace
            .iter()
            .any(|(_, pane)| { matches!(pane, __IcePaneNestedWorkspace::PaneTask(1)) })
    );
    let _ = app.__update(__ShowcaseMessage::MaximizeTaskPane);
    assert!(app.__pane_nested_workspace.maximized().is_some());
    let _ = app.__view();
    app.tasks.clear();
    let _ = app.__view();

    let _ = app.__update(__ShowcaseMessage::CloseTaskPane(1));
    assert!(
        !app.__pane_nested_workspace
            .iter()
            .any(|(_, pane)| { matches!(pane, __IcePaneNestedWorkspace::PaneTask(1)) })
    );

    let _ = app.__update(__ShowcaseMessage::OpenModePane);
    assert!(app.__pane_nested_workspace.iter().any(|(_, pane)| {
        matches!(pane, __IcePaneNestedWorkspace::ModePane(name) if name == "List")
    }));
    let _ = app.__view();
    let _ = app.__update(__ShowcaseMessage::CloseModePane("List".into()));
}

#[test]
fn reads_and_clears_the_editor_text() {
    let (mut app, _) = Showcase::__boot();
    let original = app.notes.text();
    assert!(!original.is_empty());

    // `read_notes` copies the live editor content into `notes_text`.
    let _ = app.__update(__ShowcaseMessage::ReadNotes);
    assert_eq!(app.notes_text, original);

    // `clear_notes` replaces the editor content with an empty document.
    let _ = app.__update(__ShowcaseMessage::ClearNotes);
    assert_eq!(app.notes.text(), "");

    let _ = app.__update(__ShowcaseMessage::ReadNotes);
    assert_eq!(app.notes_text, "");
    let _ = app.__view();
}

#[test]
fn renders_the_showcase_through_the_headless_draw_path() {
    let (mut app, _) = Showcase::__boot();
    app.tasks = vec![crate::backend::Task {
        id: 1,
        title: "Rendered task".into(),
        done: false,
    }];

    let counts = super::draw_headlessly(
        app.__view(),
        &app.__theme(),
        iced::Size::new(1600.0, 4000.0),
    );
    assert!(
        counts.quads > 0,
        "surface widgets must draw quads: {counts:?}"
    );
    assert!(
        counts.primitives > 0,
        "custom widgets must draw primitives: {counts:?}"
    );
    assert!(
        counts.text > 0,
        "text widgets must draw glyph runs: {counts:?}"
    );
}

ui_lang::include_app!("tests/cases/ui/mouse_interaction.ice");

pub fn interaction_round_trip(value: iced::mouse::Interaction) -> iced::mouse::Interaction {
    value
}

#[test]
fn preserves_every_native_mouse_interaction() {
    use iced::mouse::Interaction;

    let (mut app, _) = NativeMouseInteraction::__boot();
    let _ = app.__update(__NativeMouseInteractionMessage::Inspect);

    assert_eq!(app.default_value, Interaction::default());
    assert_eq!(app.returned, Interaction::Pointer);
    assert_eq!(
        app.basic,
        vec![Interaction::None, Interaction::Hidden, Interaction::Idle]
    );
    assert_eq!(
        app.feedback,
        vec![
            Interaction::ContextMenu,
            Interaction::Help,
            Interaction::Progress,
            Interaction::Wait,
        ]
    );
    assert_eq!(
        app.precision,
        vec![Interaction::Cell, Interaction::Crosshair, Interaction::Text]
    );
    assert_eq!(
        app.actions,
        vec![Interaction::Alias, Interaction::Copy, Interaction::Move]
    );
    assert_eq!(
        app.grabbing,
        vec![
            Interaction::NoDrop,
            Interaction::NotAllowed,
            Interaction::Grab,
            Interaction::Grabbing,
        ]
    );
    assert_eq!(
        app.resize_axes,
        vec![
            Interaction::ResizingHorizontally,
            Interaction::ResizingVertically,
        ]
    );
    assert_eq!(
        app.resize_diagonal,
        vec![
            Interaction::ResizingDiagonallyUp,
            Interaction::ResizingDiagonallyDown,
        ]
    );
    assert_eq!(
        app.resize_grid,
        vec![Interaction::ResizingColumn, Interaction::ResizingRow]
    );
    assert_eq!(
        app.navigation,
        vec![
            Interaction::AllScroll,
            Interaction::ZoomIn,
            Interaction::ZoomOut,
        ]
    );
    assert_eq!(app.kind, "pointer");
    assert!(app.values_equal);
    assert!(app.values_ordered);
    let _ = app.__view();
}

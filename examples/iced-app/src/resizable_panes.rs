ui_lang::include_app!("src/ui/resizable_panes.ice");

#[test]
fn drag_deltas_resize_the_left_pane_with_a_min_clamp() {
    let (mut app, _) = ResizablePanes::__boot();
    assert_eq!(app.left_width, 240.0);
    assert!(!app.dragging);

    // Press marks the drag active so the app can react while it lasts.
    let _ = app.__update(__ResizablePanesMessage::DragStarted);
    assert!(app.dragging);

    // A positive horizontal delta widens the left pane by that amount.
    let _ = app.__update(__ResizablePanesMessage::DividerDragged(60.0, 0.0));
    assert_eq!(app.left_width, 300.0);

    // Shrinking is honoured until it would cross the 160px minimum, then refused.
    let _ = app.__update(__ResizablePanesMessage::DividerDragged(-100.0, 0.0));
    assert_eq!(app.left_width, 200.0);
    let _ = app.__update(__ResizablePanesMessage::DividerDragged(-100.0, 0.0));
    assert_eq!(
        app.left_width, 200.0,
        "min clamp holds the pane at >= 160px"
    );

    // Release clears the active-drag flag.
    let _ = app.__update(__ResizablePanesMessage::DragEnded);
    assert!(!app.dragging);
    let _ = app.__view();
}

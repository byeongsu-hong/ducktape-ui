use super::*;
use iced::futures::StreamExt;
use iced_test::runtime::{Action, task};

#[test]
fn window_queue_bounds_duplicates_and_cancellation() {
    let mut queue = WindowEffects::default();
    let focus = wire::encode(&wire::WindowCommand::Focus);
    for id in 0..MAX_WINDOW_REQUESTS as u64 {
        queue.push(id, &focus).unwrap();
    }
    assert!(queue.push(100, &focus).unwrap_err().contains("too many"));
    queue.cancel(0);
    assert!(queue.push(1, &focus).unwrap_err().contains("duplicate"));
    queue.push(100, &focus).unwrap();
    queue.cancel(100);
    assert!(!queue.pending.iter().any(|request| request.id == 100));
    assert!(
        queue
            .push(101, &[0xff; 4])
            .unwrap_err()
            .contains("RequestError")
    );
}

fn outputs<M: Send + 'static>(work: iced::Task<M>) -> Vec<Action<M>> {
    iced::futures::executor::block_on(async {
        match task::into_stream(work) {
            Some(stream) => stream.collect().await,
            None => Vec::new(),
        }
    })
}

fn entry(native: bool) -> CatalogEntry {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/window-effects-native"
    } else {
        "../target/window-effects-wasm"
    });
    crate::catalog::scan_dir(&dir)
        .into_iter()
        .find(|entry| entry.name == "Window effects")
        .expect("bundle the current native and wasm window effect fixtures first")
}

fn running(native: bool) -> Running {
    let entry = entry(native);
    Running {
        id: entry.id.clone(),
        name: entry.name.clone(),
        surface: Surface(Arc::new(Mutex::new(Guest::load(&entry).unwrap()))),
        window: iced::window::Id::unique(),
    }
}

fn prepare(app: &Running) -> WindowEffect {
    let mut prepared = outputs(prepare_window_effects(vec![app.clone()], app.window));
    assert_eq!(
        prepared.len(),
        1,
        "one prepared UI message, no native action yet"
    );
    let Action::Output(effect) = prepared.pop().unwrap() else {
        panic!("prepare emitted a native action")
    };
    effect
}

#[test]
#[ignore = "requires native and wasm window effect fixtures"]
fn bundled_window_effects_reject_cancelled_closed_and_replaced_generations() {
    for native in [false, true] {
        let app = running(native);
        let focus = wire::encode(&wire::WindowCommand::Focus);
        app.surface
            .0
            .lock()
            .unwrap()
            .window_effects
            .push(1, &focus)
            .unwrap();
        let cancelled = prepare(&app);
        app.surface.0.lock().unwrap().cancel(1);
        assert!(
            outputs(commit_window_effect(vec![app.clone()], cancelled)).is_empty(),
            "cancelled request emitted a native action"
        );

        app.surface
            .0
            .lock()
            .unwrap()
            .window_effects
            .push(2, &focus)
            .unwrap();
        let closed = prepare(&app);
        let unrelated = running(native);
        assert!(
            outputs(commit_window_effect(vec![unrelated], closed)).is_empty(),
            "closed/uninstalled guest emitted a native action on another running window"
        );

        app.surface
            .0
            .lock()
            .unwrap()
            .window_effects
            .push(3, &focus)
            .unwrap();
        let stale = prepare(&app);
        let replacement = Guest::load(&entry(native)).unwrap();
        *app.surface.0.lock().unwrap() = replacement;
        app.surface
            .0
            .lock()
            .unwrap()
            .window_effects
            .push(3, &focus)
            .unwrap();
        let fresh = prepare(&app);
        assert!(
            outputs(commit_window_effect(vec![app.clone()], stale)).is_empty(),
            "old generation emitted a native action on the replacement's window"
        );
        assert_eq!(
            outputs(commit_window_effect(vec![app.clone()], fresh)).len(),
            2,
            "the replacement's own colliding request must still submit and complete"
        );
    }
}

/// Execute the real generated store update/task loop. Native window actions
/// are the boundary under test: no OS success acknowledgement is invented.
fn store_actions(
    store: &mut crate::IceStore,
    message: crate::__IceStoreMessage,
) -> Vec<iced_test::runtime::window::Action> {
    let mut messages = std::collections::VecDeque::from([message]);
    let mut windows = Vec::new();
    while let Some(message) = messages.pop_front() {
        for action in outputs(store.__update(message)) {
            match action {
                Action::Output(message) => messages.push_back(message),
                Action::Window(action) => windows.push(action),
                other => panic!("unexpected host action: {other:?}"),
            }
        }
    }
    windows
}

#[test]
#[ignore = "requires native and wasm window effect fixtures"]
fn bundled_window_effects_follow_mounted_clicks_and_keep_store_window() {
    use super::super::layers_tests::{build, click, redraw, renderer};
    use iced_test::runtime::{user_interface, window::Action as WindowAction};
    for native in [false, true] {
        for label in ["Focus guest", "Resize guest", "Close guest", "Exit guest"] {
            let app = running(native);
            let mut renderer = renderer();
            let mut ui = build(
                &app.surface.0,
                user_interface::Cache::default(),
                &mut renderer,
                480.0,
            );
            let mut now = Instant::now();
            ui = redraw(ui, &app.surface.0, &mut renderer, &mut now, 480.0);
            click(&mut ui, &mut renderer, label);
            let mut notices = Vec::new();
            for _ in 0..4 {
                ui = redraw_notices(ui, &app, &mut renderer, &mut now, &mut notices);
            }
            assert!(
                notices.iter().any(|notice| notice == "wake"),
                "a window request must wake the host UI even without a tree change"
            );
            assert!(
                app.surface.0.lock().unwrap().window_effects.queued(),
                "{label} must reach the host window queue (native={native})"
            );
            assert_eq!(
                result_text(app.surface.0.lock().unwrap().frame.root.as_ref().unwrap()),
                Some("waiting"),
                "the guest chain must wait for the host UI acknowledgement"
            );
            let mut store = crate::IceStore::__state();
            let main = iced::window::Id::unique();
            store.store_window = Some(main);
            store.running = vec![app.clone()];
            let mut actions = notices
                .into_iter()
                .flat_map(|notice| {
                    store_actions(
                        &mut store,
                        crate::__IceStoreMessage::GuestChanged(app.window, notice),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(actions.len(), 1, "one scoped native effect for {label}");
            match (label, actions.pop().unwrap()) {
                ("Focus guest", WindowAction::GainFocus(id)) => assert_eq!(id, app.window),
                ("Resize guest", WindowAction::Resize(id, size)) => {
                    assert_eq!(id, app.window);
                    assert_eq!(size, iced::Size::new(600.5, 400.25));
                }
                ("Close guest" | "Exit guest", WindowAction::Close(id)) => {
                    assert_eq!(id, app.window);
                    assert_ne!(id, main, "guest exit must never close the store");
                }
                _ => panic!("wrong native action for {label}"),
            }
            assert_eq!(store.store_window, Some(main));
            assert!(
                app.surface
                    .0
                    .lock()
                    .unwrap()
                    .window_effects
                    .pending
                    .is_empty(),
                "completion must settle the request"
            );
            for _ in 0..4 {
                ui = redraw(ui, &app.surface.0, &mut renderer, &mut now, 480.0);
            }
            let expected = match label {
                "Focus guest" => "focus submitted",
                "Resize guest" => "resize submitted",
                "Close guest" => "close submitted",
                _ => "quit submitted",
            };
            assert_eq!(
                result_text(app.surface.0.lock().unwrap().frame.root.as_ref().unwrap()),
                Some(expected),
                "host acknowledgement must reach the real guest and advance its sequential task"
            );
            if matches!(label, "Close guest" | "Exit guest") {
                assert!(
                    store_actions(
                        &mut store,
                        crate::__IceStoreMessage::WindowClosed(app.window)
                    )
                    .is_empty(),
                    "closing a guest must not emit any store window action or process exit"
                );
                assert!(store.running.is_empty());
                assert_eq!(store.store_window, Some(main));
            }
        }
    }
}

#[test]
#[ignore = "requires native and wasm window effect fixtures"]
fn bundled_window_effects_cancel_same_frame_and_reject_bad_requests() {
    for native in [false, true] {
        let app = running(native);
        let mut guest = app.surface.0.lock().unwrap();
        let now = Instant::now();
        guest.frame.requests = vec![wire::Request {
            id: 700,
            kind: "host.window".into(),
            payload: wire::encode(&wire::WindowCommand::Focus),
        }];
        guest.frame.cancels = vec![700];
        guest.staged_frame = true;
        guest.redraw(now, &mut iced::advanced::clipboard::Null, None);
        assert!(
            guest.window_effects.pending.is_empty(),
            "same-frame cancellation must precede native submission"
        );
        guest.answer(
            now,
            wire::Request {
                id: 701,
                kind: "host.window".into(),
                payload: wire::encode(&wire::WindowCommand::Resize {
                    width: f32::NAN,
                    height: 10.0,
                }),
            },
        );
        assert!(guest.window_effects.pending.is_empty());
        assert!(guest.due.iter().any(|(_, event)| matches!(event, wire::Event::Response { id: 701, result: Err(error), done: true } if error.contains("RequestError"))));
    }
}

fn result_text(node: &wire::Node) -> Option<&str> {
    if let wire::Node::Text { key, content, .. } = node
        && key.ends_with("/result")
    {
        return Some(content);
    }
    node.children().iter().find_map(|node| result_text(node))
}

#[test]
#[ignore = "requires native and wasm window effect fixtures"]
fn bundled_window_effects_do_not_acknowledge_a_replacements_request() {
    for native in [false, true] {
        let app = running(native);
        let focus = wire::encode(&wire::WindowCommand::Focus);
        app.surface
            .0
            .lock()
            .unwrap()
            .window_effects
            .push(9, &focus)
            .unwrap();
        let old = prepare(&app);
        let submitted = outputs(commit_window_effect(vec![app.clone()], old.clone()));
        assert!(matches!(
            submitted.as_slice(),
            [Action::Window(_), Action::Output(_)]
        ));
        assert!(
            outputs(commit_window_effect(vec![app.clone()], old.clone())).is_empty(),
            "duplicate native submission"
        );
        let replacement = Guest::load(&entry(native)).unwrap();
        *app.surface.0.lock().unwrap() = replacement;
        app.surface
            .0
            .lock()
            .unwrap()
            .window_effects
            .push(9, &focus)
            .unwrap();
        let fresh = prepare(&app);
        let submitted = outputs(commit_window_effect(vec![app.clone()], fresh.clone()));
        assert!(matches!(
            submitted.as_slice(),
            [Action::Window(_), Action::Output(_)]
        ));
        assert!(!complete_window_effect(std::slice::from_ref(&app), old));
        assert!(
            app.surface.0.lock().unwrap().due.is_empty(),
            "old completion answered a replacement request ID"
        );
        assert!(complete_window_effect(std::slice::from_ref(&app), fresh));
        assert!(
            app.surface
                .0
                .lock()
                .unwrap()
                .due
                .iter()
                .any(|(_, event)| matches!(
                    event,
                    wire::Event::Response {
                        id: 9,
                        result: Ok(_),
                        done: true
                    }
                ))
        );
    }
}

fn redraw_notices(
    mut ui: super::super::layers_tests::Ui,
    app: &Running,
    renderer: &mut iced::Renderer,
    now: &mut Instant,
    notices: &mut Vec<String>,
) -> super::super::layers_tests::Ui {
    *now += std::time::Duration::from_secs(1);
    let mut messages = Vec::new();
    ui.update(
        &[iced::Event::Window(iced::window::Event::RedrawRequested(
            *now,
        ))],
        iced::mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut messages,
    );
    assert!(app.surface.0.lock().unwrap().fault.is_none());
    let changed = messages.iter().any(|message| message == "wake");
    notices.extend(messages);
    if changed {
        super::super::layers_tests::build(&app.surface.0, ui.into_cache(), renderer, 480.0)
    } else {
        ui
    }
}

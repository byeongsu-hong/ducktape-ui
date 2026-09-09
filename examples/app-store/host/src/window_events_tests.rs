//! Actual backend observations after mounted native input, not OS-dispatch smoke.
use super::layers_tests::{Ui, build, click, focus, redraw, renderer};
use super::reload::finish_reload;
use super::*;
use iced::{Event, mouse, window};
use iced_test::runtime::user_interface;

fn entry(native: bool) -> CatalogEntry {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/window-events-native"
    } else {
        "../target/window-events-wasm"
    });
    crate::catalog::scan_dir(&directory)
        .into_iter()
        .find(|entry| entry.name == "Window events fixture")
        .expect("build both current window event fixtures first")
}
fn read(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Text { key, content, .. } = node
        && key.ends_with(&format!("/{suffix}"))
    {
        return Some(content.clone());
    }
    node.children().iter().find_map(|child| read(child, suffix))
}
fn text(guest: &Arc<Mutex<Guest>>, suffix: &str) -> String {
    read(guest.lock().unwrap().frame.root.as_ref().unwrap(), suffix).unwrap()
}
fn key(guest: &Arc<Mutex<Guest>>, suffix: &str) -> String {
    fn find(node: &wire::Node, suffix: &str) -> Option<String> {
        if let wire::Node::Input { key, .. } = node
            && key.ends_with(&format!("/{suffix}"))
        {
            return Some(key.clone());
        }
        node.children().iter().find_map(|child| find(child, suffix))
    }
    find(guest.lock().unwrap().frame.root.as_ref().unwrap(), suffix).unwrap()
}
fn send(ui: &mut Ui, renderer: &mut iced::Renderer, events: &[Event]) {
    ui.update(
        events,
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn settle(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut Instant,
) -> Ui {
    for _ in 0..4 {
        ui = redraw(ui, guest, renderer, now, 720.0);
    }
    assert!(guest.lock().unwrap().fault.is_none());
    ui
}

#[test]
#[ignore = "requires current native and Wasm window event fixtures"]
fn native_and_wasm_window_and_ime_observations_do_not_duplicate_native_input() {
    use iced::advanced::input_method::Event as I;
    for native in [false, true] {
        let guest = Arc::new(Mutex::new(Guest::load(&entry(native)).unwrap()));
        let mut renderer = renderer();
        let mut now = Instant::now();
        let ui = build(
            &guest,
            user_interface::Cache::default(),
            &mut renderer,
            720.0,
        );
        let mut ui = settle(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            guest.lock().unwrap().frame.event_interest,
            wire::events::Interest::ALL
        );
        send(
            &mut ui,
            &mut renderer,
            &[
                Event::Window(window::Event::Focused),
                Event::Window(window::Event::Unfocused),
                Event::Window(window::Event::FileHovered("/tmp/한글.txt".into())),
                Event::Window(window::Event::FileDropped("/tmp/한글.txt".into())),
                Event::Window(window::Event::FilesHoveredLeft),
            ],
        );
        ui = settle(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "focus"), "1");
        assert_eq!(text(&guest, "blur"), "1");
        assert_eq!(text(&guest, "hover-path"), "/tmp/한글.txt");
        assert_eq!(text(&guest, "drop-path"), "/tmp/한글.txt");
        assert_eq!(text(&guest, "hover-left"), "1");
        assert_eq!(
            text(&guest, "events"),
            "5",
            "generic event source sees the same observations once"
        );
        assert!(focus(&mut ui, &renderer, &key(&guest, "draft-input")));
        send(
            &mut ui,
            &mut renderer,
            &[
                Event::InputMethod(I::Opened),
                Event::InputMethod(I::Preedit("한글".into(), Some(0..6))),
            ],
        );
        ui = settle(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "preedits"), "1");
        assert_eq!(text(&guest, "composition"), "한글");
        assert_eq!(text(&guest, "range"), "true");
        assert_eq!(text(&guest, "draft"), "", "preedit is not a document edit");
        send(
            &mut ui,
            &mut renderer,
            &[
                Event::InputMethod(I::Commit("한글".into())),
                Event::InputMethod(I::Closed),
            ],
        );
        ui = settle(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            text(&guest, "draft"),
            "한글",
            "native commit is applied once"
        );
        assert_eq!(text(&guest, "commits"), "1", "one subscription observation");
        assert_eq!(text(&guest, "captured-commits"), "1");
        assert_eq!(text(&guest, "ime-opens"), "1");
        assert_eq!(text(&guest, "ime-closes"), "1");
        assert_eq!(text(&guest, "events"), "9");

        click(&mut ui, &mut renderer, "Open modal");
        ui = settle(ui, &guest, &mut renderer, &mut now);
        assert!(focus(&mut ui, &renderer, &key(&guest, "modal-input")));
        send(
            &mut ui,
            &mut renderer,
            &[
                Event::InputMethod(I::Opened),
                Event::InputMethod(I::Commit("!".into())),
                Event::InputMethod(I::Closed),
            ],
        );
        ui = settle(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "draft").matches('!').count(), 1);
        assert_eq!(
            text(&guest, "commits"),
            "2",
            "captured overlay does not also forward through base"
        );
        assert_eq!(text(&guest, "captured-commits"), "2");
        click(&mut ui, &mut renderer, "Close modal");
        ui = settle(ui, &guest, &mut renderer, &mut now);
        click(&mut ui, &mut renderer, "Disable observations");
        ui = settle(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            guest.lock().unwrap().frame.event_interest,
            Default::default()
        );
        send(
            &mut ui,
            &mut renderer,
            &[
                Event::Window(window::Event::Focused),
                Event::Window(window::Event::FileDropped("/tmp/other.txt".into())),
                Event::InputMethod(I::Commit("ignored observation".into())),
            ],
        );
        let _ui = settle(ui, &guest, &mut renderer, &mut now);
        assert_eq!(text(&guest, "focus"), "1");
        assert_eq!(text(&guest, "drop-path"), "/tmp/한글.txt");
        assert_eq!(text(&guest, "commits"), "2");
    }
}

#[test]
#[ignore = "requires current native and Wasm window event fixtures"]
fn native_and_wasm_replacement_rejects_old_observations_and_close_cannot_execute_effects() {
    for native in [false, true] {
        let entry = entry(native);
        let guest = Arc::new(Mutex::new(Guest::load(&entry).unwrap()));
        let mut renderer = renderer();
        let mut now = Instant::now();
        let ui = build(
            &guest,
            user_interface::Cache::default(),
            &mut renderer,
            720.0,
        );
        let mut ui = settle(ui, &guest, &mut renderer, &mut now);
        let running = Running {
            id: entry.id.clone(),
            name: entry.name.clone(),
            surface: Surface(guest.clone()),
            window: window::Id::unique(),
        };
        let prepared =
            iced::futures::executor::block_on(prepare_reload(entry, vec![running.clone()], 1));
        finish_reload(std::slice::from_ref(&running), 1, prepared).unwrap();
        send(
            &mut ui,
            &mut renderer,
            &[Event::Window(window::Event::Focused)],
        );
        assert!(
            !guest
                .lock()
                .unwrap()
                .pending
                .iter()
                .any(|event| matches!(event, wire::Event::Observation { .. })),
            "retired widget cannot enqueue into replacement"
        );
        ui = build(&guest, ui.into_cache(), &mut renderer, 720.0);
        send(
            &mut ui,
            &mut renderer,
            &[Event::Window(window::Event::Focused)],
        );
        ui = settle(ui, &guest, &mut renderer, &mut now);
        assert_eq!(
            text(&guest, "focus"),
            "1",
            "new instance gets one new observation, not replay"
        );
        // No redraw can happen after native destruction. The close boundary
        // drains the queued CloseRequested and Closed in one bounded tick.
        send(
            &mut ui,
            &mut renderer,
            &[Event::Window(window::Event::CloseRequested)],
        );
        let mut guest = guest.lock().unwrap();
        let mut frame = guest.window_closed().expect("terminal observation frame");
        merge(&mut guest.frame.root.clone(), &mut frame).unwrap();
        let root = frame
            .root
            .as_ref()
            .expect("closed handler publishes its state");
        assert_eq!(read(root, "closing").as_deref(), Some("1"));
        assert_eq!(read(root, "closed").as_deref(), Some("1"));
        assert!(
            frame
                .requests
                .iter()
                .any(|request| request.kind == "host.window"),
            "fixture tries a post-close focus effect"
        );
        assert!(
            !guest.window_effects.queued(),
            "final tick requests must not execute"
        );
        assert!(!guest.alive.load(Ordering::Relaxed));
        assert!(
            guest.window_closed().is_none(),
            "closed is delivered at most once"
        );
    }
}

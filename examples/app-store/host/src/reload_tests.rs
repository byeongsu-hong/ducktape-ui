//! Two separately compiled modules through the mounted host replacement path.
use super::layers_tests::{Ui, build, focus, redraw, renderer, type_text};
use super::reload::finish_reload;
use super::*;
use iced::advanced::widget::Operation;
use iced::{Event, Rectangle, Vector, mouse};
use iced_test::runtime::user_interface;

fn entry(version: u8) -> CatalogEntry {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../target/reload-v{version}-fixture/app_store_reload_v{version}_fixture.wasm"
    ));
    let bytes = std::fs::read(&path).expect("bundle both reload fixtures first");
    CatalogEntry {
        id: "reload-fixture".into(),
        name: format!("Reload version {version}"),
        description: String::new(),
        capabilities: vec![Capability {
            name: "clock".into(),
        }],
        path: path.to_string_lossy().into_owned(),
        mark: "R".into(),
        hash: sha256_hex(&bytes),
    }
}
fn running() -> Running {
    let guest = Guest::load(&entry(1)).unwrap();
    Running {
        id: "reload-fixture".into(),
        name: "Reload version 1".into(),
        surface: Surface(Arc::new(Mutex::new(guest))),
        window: iced::window::Id::unique(),
    }
}
fn read(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Text { key, content, .. } = node
        && key.ends_with(suffix)
    {
        return Some(content.clone());
    }
    if let wire::Node::Input { key, value, .. } = node
        && key.ends_with(suffix)
    {
        return Some(value.clone());
    }
    node.children().iter().find_map(|node| read(node, suffix))
}
fn value(app: &Running, suffix: &str) -> String {
    read(
        app.surface.0.lock().unwrap().frame.root.as_ref().unwrap(),
        suffix,
    )
    .unwrap()
}
#[derive(Default)]
struct NativeState {
    focused: bool,
    scroll: Option<(Rectangle, f32)>,
}
impl Operation for NativeState {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn focusable(
        &mut self,
        id: Option<&iced::widget::Id>,
        _: Rectangle,
        state: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        if id == Some(&iced::widget::Id::new("ReloadFixture/root/draft")) {
            self.focused = state.is_focused();
        }
    }
    fn scrollable(
        &mut self,
        id: Option<&iced::widget::Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&iced::widget::Id::new("ReloadFixture/root/history")) {
            self.scroll = Some((bounds, translation.y));
        }
    }
}
fn native(ui: &mut Ui, renderer: &iced::Renderer) -> NativeState {
    let mut state = NativeState::default();
    ui.operate(renderer, &mut state);
    state
}
fn prepare(app: &Running, serial: i64) -> Reload {
    iced::futures::executor::block_on(prepare_reload(entry(2), vec![app.clone()], serial))
}

#[test]
#[ignore = "requires two bundled reload fixtures"]
fn bundled_reload_preserves_window_draft_focus_scroll_and_host_resources() {
    assert_ne!(
        entry(1).hash,
        entry(2).hash,
        "the candidate must be a different binary"
    );
    let app = running();
    let guest = app.surface.0.clone();
    let mut renderer = renderer();
    let mut now = Instant::now();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(value(&app, "/version"), "version 1");
    assert!(focus(&mut ui, &renderer, "ReloadFixture/root/draft"));
    for character in "saved draft".chars() {
        type_text(&mut ui, &mut renderer, &character.to_string());
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
    }
    let point = native(&mut ui, &renderer).scroll.unwrap().0.center();
    ui.update(
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -60.0 },
        })],
        mouse::Cursor::Available(point),
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    let before = native(&mut ui, &renderer);
    assert!(before.focused);
    assert!(before.scroll.unwrap().1 > 50.0);
    assert_eq!(value(&app, "/draft"), "saved draft");
    let log = guest.lock().unwrap().log_session.clone();
    let old_instance = guest.lock().unwrap().alive.clone();
    let request = prepare(&app, 1);
    let loaded = finish_reload(std::slice::from_ref(&app), 1, request).unwrap();
    assert_eq!(loaded.surface, app.surface);
    assert_eq!(loaded.hash, entry(2).hash);
    assert!(!old_instance.load(Ordering::Relaxed));
    assert!(Arc::ptr_eq(&log, &guest.lock().unwrap().log_session));
    type_text(&mut ui, &mut renderer, "stale");
    assert!(
        guest.lock().unwrap().pending.is_empty(),
        "old widget keyboard events must be refused"
    );
    let apps = renamed_running(vec![app.clone()], &loaded);
    assert_eq!(apps[0].window, app.window);
    ui = build(&guest, ui.into_cache(), &mut renderer, 600.0);
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(value(&app, "/version"), "version 2");
    assert_eq!(value(&app, "/draft"), "saved draft");
    assert_eq!(value(&app, "/boots"), "1", "restore must not replay mount");

    let after = native(&mut ui, &renderer);
    assert!(after.focused, "native focus must survive replacement");
    assert_eq!(after.scroll.unwrap().1, before.scroll.unwrap().1);
    assert_eq!(
        guest.lock().unwrap().tickers.len(),
        1,
        "one restarted subscription"
    );
    let mut report = None;
    guest
        .lock()
        .unwrap()
        .frame
        .root
        .as_mut()
        .unwrap()
        .for_each_mut(&mut |node| {
            if let wire::Node::Button {
                content: wire::ButtonContent::Label(label),
                on_press,
                ..
            } = node
                && label == "Report initializations"
            {
                report = *on_press;
            }
        });
    guest
        .lock()
        .unwrap()
        .pending
        .push(wire::Event::Message(report.unwrap()));
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(
        value(&app, "/initializations"),
        "0",
        "replacement never runs init"
    );
    type_text(&mut ui, &mut renderer, "!");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    assert_eq!(value(&app, "/draft"), "saved draft!");
}

#[test]
#[ignore = "requires two bundled reload fixtures"]
fn bundled_reload_rejects_closed_superseded_and_edited_instances() {
    let app = running();
    let guest = app.surface.0.clone();
    let mut renderer = renderer();
    let mut now = Instant::now();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    let alive = guest.lock().unwrap().alive.clone();
    assert!(finish_reload(&[], 1, prepare(&app, 1)).is_err());
    assert!(finish_reload(std::slice::from_ref(&app), 2, prepare(&app, 1)).is_err());
    let stale = prepare(&app, 3);
    assert!(focus(&mut ui, &renderer, "ReloadFixture/root/draft"));
    type_text(&mut ui, &mut renderer, "n");
    assert!(finish_reload(std::slice::from_ref(&app), 3, stale).is_err());
    ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    assert_eq!(value(&app, "/draft"), "n");
    assert!(Arc::ptr_eq(&alive, &guest.lock().unwrap().alive));
    assert_eq!(value(&app, "/version"), "version 1");
    let stale = prepare(&app, 4);
    ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    assert!(finish_reload(std::slice::from_ref(&app), 4, stale).is_err());
    assert_eq!(value(&app, "/draft"), "n");
    drop(ui);
}

#[test]
#[ignore = "requires bundled reload and component fixtures"]
fn bundled_reload_bad_hash_and_incompatible_schema_leave_pin_and_guest_intact() {
    let app = running();
    let mut guest = app.surface.0.lock().unwrap();
    let mut now = Instant::now();
    for _ in 0..3 {
        now += Duration::from_secs(1);
        guest.redraw(now, &mut iced::advanced::clipboard::Null, None);
    }
    let alive = guest.alive.clone();
    drop(guest);
    let pin = Installed {
        id: app.id.clone(),
        hash: entry(1).hash,
    };
    let mut invalid = entry(2);
    invalid.hash = "wrong hash".into();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/component-fixture/app_store_component_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle component fixture first");
    let mut incompatible = entry(2);
    incompatible.path = path.to_string_lossy().into_owned();
    incompatible.hash = sha256_hex(&bytes);
    for (candidate, expected) in [
        (invalid, "changed on disk since the catalog was scanned"),
        (incompatible, "snapshot schema mismatch"),
    ] {
        let prepared =
            iced::futures::executor::block_on(prepare_reload(candidate, vec![app.clone()], 7));
        let result = commit_reload(vec![pin.clone()], vec![app.clone()], 7, prepared);
        assert!(
            result.status.contains(expected),
            "expected {expected:?}, got {:?}",
            result.status
        );
        assert_eq!(result.library, vec![pin.clone()]);
        assert_eq!(result.running, vec![app.clone()]);
        assert!(Arc::ptr_eq(&alive, &app.surface.0.lock().unwrap().alive));
        assert_eq!(value(&app, "/version"), "version 1");
    }
}

#[test]
#[ignore = "requires two bundled reload fixtures"]
fn bundled_reload_staged_requests_are_canceled_before_platform_effects() {
    #[derive(Default)]
    struct Clipboard {
        writes: usize,
    }
    impl iced::advanced::Clipboard for Clipboard {
        fn read(&self, _: iced::advanced::clipboard::Kind) -> Option<String> {
            None
        }
        fn write(&mut self, _: iced::advanced::clipboard::Kind, _: String) {
            self.writes += 1;
        }
    }
    let app = running();
    let mut now = Instant::now();
    for _ in 0..3 {
        now += Duration::from_secs(1);
        app.surface
            .0
            .lock()
            .unwrap()
            .redraw(now, &mut iced::advanced::clipboard::Null, None);
    }
    let mut candidate = entry(2);
    candidate.capabilities.push(Capability {
        name: "clipboard".into(),
    });
    let prepared =
        iced::futures::executor::block_on(prepare_reload(candidate, vec![app.clone()], 1));
    finish_reload(std::slice::from_ref(&app), 1, prepared).unwrap();
    let mut guest = app.surface.0.lock().unwrap();
    assert!(guest.staged_frame);
    // Model a request future created and dropped in the raw first frame.
    // Keep one live ticker too, so dropping the entire frame cannot pass.
    guest.frame.requests = vec![
        wire::Request {
            id: 900,
            kind: "clock.ticks".into(),
            payload: 1000_u64.to_le_bytes().to_vec(),
        },
        wire::Request {
            id: 901,
            kind: "clock.ticks".into(),
            payload: 1000_u64.to_le_bytes().to_vec(),
        },
        wire::Request {
            id: 902,
            kind: "clipboard.write".into(),
            payload: wire::encode(&(wire::ClipboardTarget::Standard, "canceled".to_string())),
        },
    ];
    guest.frame.cancels = vec![900, 902];
    let ticks = guest.ticks;
    let mut clipboard = Clipboard::default();
    guest.redraw(now + Duration::from_secs(1), &mut clipboard, None);
    assert_eq!(
        guest.ticks, ticks,
        "staged dispatch must not run a second guest tick"
    );
    assert_eq!(
        guest
            .tickers
            .iter()
            .map(|ticker| ticker.id)
            .collect::<Vec<_>>(),
        vec![901],
        "same-frame cancellation must remove the newly started ticker"
    );
    assert_eq!(
        clipboard.writes, 0,
        "same-frame cancellation must precede clipboard effects"
    );
    assert!(guest.clipboard.is_empty());
}

#[test]
#[ignore = "requires two bundled reload fixtures"]
fn bundled_reload_removed_terminal_capability_drops_its_provider() {
    let mut original = entry(1);
    original.capabilities.push(Capability {
        name: "terminal".into(),
    });
    let guest = Guest::load_with_terminal(&original, None).unwrap();
    assert!(guest.terminal.is_some());
    assert!(guest.surfaces.contains_key("terminal"));
    let app = Running {
        id: original.id,
        name: original.name,
        surface: Surface(Arc::new(Mutex::new(guest))),
        window: iced::window::Id::unique(),
    };
    let mut now = Instant::now();
    for _ in 0..3 {
        now += Duration::from_secs(1);
        app.surface
            .0
            .lock()
            .unwrap()
            .redraw(now, &mut iced::advanced::clipboard::Null, None);
    }
    let loaded = finish_reload(std::slice::from_ref(&app), 1, prepare(&app, 1)).unwrap();
    assert_eq!(loaded.surface, app.surface);
    let guest = app.surface.0.lock().unwrap();
    assert!(
        guest.terminal.is_none(),
        "removed permission must drop the terminal session"
    );
    assert!(
        !guest.surfaces.contains_key("terminal"),
        "removed permission must drop the native terminal provider"
    );
}

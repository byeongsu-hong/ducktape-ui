//! The same installed application through native IPC and a real wasm component.
use super::layers_tests::{build, click, redraw, renderer};
use super::*;
use iced::advanced::renderer::Headless;
use iced_test::runtime::user_interface;

fn catalog(native: bool) -> Vec<CatalogEntry> {
    crate::catalog::scan_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
            "../target/app-store-native-catalog"
        } else {
            "../target/app-store-catalog"
        }),
    )
}
fn entry(native: bool, name: &str) -> CatalogEntry {
    catalog(native)
        .into_iter()
        .find(|entry| entry.name == name)
        .expect("build the current native packages and wasm catalog first")
}
fn text(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Text { key, content, .. } = node
        && key.ends_with(suffix)
    {
        return Some(content.clone());
    }
    node.children().iter().find_map(|node| text(node, suffix))
}

#[test]
#[ignore = "requires all five native packages and bundled wasm counter"]
fn native_and_wasm_counter_share_mounted_route_pixels_and_host_bus() {
    assert_eq!(
        catalog(true).len(),
        5,
        "all shipped apps have a native package"
    );
    let mut screenshots = Vec::new();
    for native in [false, true] {
        let entry = entry(native, "Counter");
        assert_eq!(
            entry
                .capabilities
                .iter()
                .any(|cap| cap.name == "native-code"),
            native
        );
        let guest = Arc::new(Mutex::new(Guest::load(&entry).unwrap()));
        let mut renderer = renderer();
        let mut ui = build(
            &guest,
            user_interface::Cache::default(),
            &mut renderer,
            480.0,
        );
        let mut now = std::time::Instant::now();
        for _ in 0..4 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 480.0);
        }
        assert_eq!(
            text(guest.lock().unwrap().frame.root.as_ref().unwrap(), "/count").as_deref(),
            Some("0")
        );
        click(&mut ui, &mut renderer, "+");
        for _ in 0..4 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 480.0);
        }
        {
            let mut guest = guest.lock().unwrap();
            assert_eq!(
                text(guest.frame.root.as_ref().unwrap(), "/count").as_deref(),
                Some("1"),
                "mounted native click must reach the app handler"
            );
            assert_eq!(
                text(guest.frame.root.as_ref().unwrap(), "/shared").as_deref(),
                Some("Shared on the bus"),
                "host reply must complete the task"
            );
            guest
                .backend
                .snapshot()
                .expect("persistent subscriptions must allow state transfer");
        }
        ui.draw(
            &mut renderer,
            &iced::Theme::Light,
            &iced::advanced::renderer::Style {
                text_color: iced::Color::BLACK,
            },
            iced::mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(iced::Size::new(480, 600), 1.0, iced::Color::WHITE);
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[0] != pixel[1]),
            "real colored app pixels must be painted"
        );
        screenshots.push(pixels);
    }
    assert_eq!(
        screenshots[0], screenshots[1],
        "same state paints the same native widgets"
    );
}

#[test]
#[ignore = "requires native Chaos package"]
fn native_runaway_faults_at_deadline_and_other_app_still_runs() {
    let mut chaos = Guest::load(&entry(true, "Chaos")).unwrap();
    chaos.tick();
    fn route(node: &wire::Node) -> Option<u32> {
        if let wire::Node::Button { key, on_press, .. } = node
            && key.ends_with("/spin")
        {
            return *on_press;
        }
        node.children().iter().find_map(route)
    }
    chaos.pending.push(wire::Event::Message(
        route(chaos.frame.root.as_ref().unwrap()).unwrap(),
    ));
    let start = std::time::Instant::now();
    chaos.tick();
    assert!(
        chaos
            .fault
            .as_deref()
            .is_some_and(|error| error.contains("deadline")),
        "the host must report its exchange deadline"
    );
    assert!(!chaos.alive.load(Ordering::Relaxed));
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "runaway guest must return control"
    );
    drop(chaos);
    let mut counter = Guest::load(&entry(true, "Counter")).unwrap();
    counter.tick();
    assert!(counter.fault.is_none());
    assert_eq!(
        text(counter.frame.root.as_ref().unwrap(), "/count").as_deref(),
        Some("0")
    );
}

#[test]
#[ignore = "requires current native Counter package and bundled wasm Counter"]
fn native_and_wasm_accessibility_click_updates_counter() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    for native in [false, true] {
        let guest = Arc::new(Mutex::new(Guest::load(&entry(native, "Counter")).unwrap()));
        let mut renderer = renderer();
        let mut ui = build(
            &guest,
            user_interface::Cache::default(),
            &mut renderer,
            480.0,
        );
        let mut now = std::time::Instant::now();
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 480.0);
        }
        assert_eq!(
            text(guest.lock().unwrap().frame.root.as_ref().unwrap(), "/count").as_deref(),
            Some("0")
        );
        let mut stream = task::into_stream(ui_lang_runtime::snapshot::<String>("Counter")).unwrap();
        let Some(Action::Widget(mut operation)) = iced::futures::executor::block_on(stream.next())
        else {
            panic!("snapshot operation")
        };
        ui.operate(&renderer, operation.as_mut());
        let _ = operation.finish();
        let Some(Action::Output(snapshot)) = iced::futures::executor::block_on(stream.next())
        else {
            panic!("snapshot output")
        };
        let (id, button) = snapshot
            .update
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("+"))
            .expect("accessible increment button");
        assert_eq!(button.role(), ui_lang_runtime::Role::Button);
        assert!(
            button.supports_action(ui_lang_runtime::Action::Click),
            "mounted guest button must expose accessibility Click to the host message type"
        );
        let mut request = ui_lang_runtime::refresh_request();
        request.action = ui_lang_runtime::Action::Click;
        request.target_node = *id;
        let mut actions =
            task::into_stream(snapshot.dispatch(request)).expect("Click dispatch task");
        while let Some(action) = iced::futures::executor::block_on(actions.next()) {
            if let Action::Widget(mut operation) = action {
                ui.operate(&renderer, operation.as_mut());
                let _ = operation.finish();
            }
        }
        for _ in 0..4 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 480.0);
        }
        assert_eq!(
            text(guest.lock().unwrap().frame.root.as_ref().unwrap(), "/count").as_deref(),
            Some("1"),
            "accessible click must update actual guest, native={native}"
        );
    }
}

fn state_field(guest: &mut Guest, name: &str) -> wire::SnapshotValue {
    let state = wire::Snapshot::decode(&guest.backend.snapshot().unwrap()).unwrap();
    let wire::SnapshotValue::Record { fields, .. } = state.state else {
        panic!("app state record")
    };
    fields
        .into_iter()
        .find(|(field, _)| field == name)
        .expect("state field")
        .1
}

fn pump_theme(
    mut ui: super::layers_tests::Ui,
    surface: &Surface,
    renderer: &mut iced::Renderer,
    now: &mut Instant,
    dark: bool,
) -> super::layers_tests::Ui {
    for _ in 0..4 {
        ui = iced_test::runtime::UserInterface::build(
            wasm_view(surface.clone(), dark),
            iced::Size::new(480.0, 600.0),
            ui.into_cache(),
            renderer,
        );
        *now += Duration::from_secs(1);
        ui.update(
            &[iced::Event::Window(iced::window::Event::RedrawRequested(
                *now,
            ))],
            iced::mouse::Cursor::Unavailable,
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut vec![],
        );
        assert!(surface.0.lock().unwrap().fault.is_none());
    }
    ui
}

#[test]
#[ignore = "requires current native and wasm Counter/Todo bundles and isolated APP_STORE_DATA"]
fn native_and_wasm_subscriptions_preserve_state_and_restart_once_on_reload() {
    assert!(std::env::var_os("APP_STORE_DATA").is_some());
    for native in [false, true] {
        for name in ["Counter", "Todo"] {
            let entry = entry(native, name);
            let surface = Surface(Arc::new(Mutex::new(Guest::load(&entry).unwrap())));
            let running = Running {
                id: entry.id.clone(),
                name: entry.name.clone(),
                surface: surface.clone(),
                window: iced::window::Id::unique(),
            };
            let mut renderer = renderer();
            let mut ui = build(
                &surface.0,
                user_interface::Cache::default(),
                &mut renderer,
                480.0,
            );
            let mut now = Instant::now();
            ui = pump_theme(ui, &surface, &mut renderer, &mut now, false);
            assert_eq!(
                state_field(&mut surface.0.lock().unwrap(), "dark"),
                wire::SnapshotValue::Bool(false)
            );
            if name == "Counter" {
                click(&mut ui, &mut renderer, "+");
            } else {
                assert!(super::layers_tests::focus(
                    &mut ui,
                    &renderer,
                    "Todo/app/content/composer/draft"
                ));
                for character in "unsaved draft".chars() {
                    super::layers_tests::type_text(&mut ui, &mut renderer, &character.to_string());
                    ui = pump_theme(ui, &surface, &mut renderer, &mut now, false);
                }
            }
            ui = pump_theme(ui, &surface, &mut renderer, &mut now, true);
            assert_eq!(
                state_field(&mut surface.0.lock().unwrap(), "dark"),
                wire::SnapshotValue::Bool(true)
            );
            if name == "Counter" {
                let mut guest = surface.0.lock().unwrap();
                let id = guest.theme_subscriptions[0];
                guest.pending.push(wire::Event::Response {
                    id,
                    result: Err("theme temporarily unavailable".into()),
                    done: false,
                });
                drop(guest);
                ui = pump_theme(ui, &surface, &mut renderer, &mut now, true);
                assert_eq!(
                    state_field(&mut surface.0.lock().unwrap(), "answer"),
                    wire::SnapshotValue::Str("theme temporarily unavailable".into()),
                    "fallible subscription error reaches existing handler"
                );
                ui = pump_theme(ui, &surface, &mut renderer, &mut now, false);
                assert_eq!(
                    state_field(&mut surface.0.lock().unwrap(), "dark"),
                    wire::SnapshotValue::Bool(false),
                    "success still arrives after a nonterminal error"
                );
                ui = pump_theme(ui, &surface, &mut renderer, &mut now, true);
            }
            for serial in 1..=3 {
                let before = {
                    let mut guest = surface.0.lock().unwrap();
                    assert_eq!(guest.theme_subscriptions.len(), 1, "one live theme stream");
                    if name == "Counter" {
                        assert_eq!(
                            state_field(&mut guest, "count"),
                            wire::SnapshotValue::I64(1)
                        );
                    } else {
                        assert_eq!(
                            state_field(&mut guest, "draft"),
                            wire::SnapshotValue::Str("unsaved draft".into())
                        );
                    }
                    guest
                        .backend
                        .snapshot()
                        .expect("a live subscription must not block reload")
                };
                let old_alive = surface.0.lock().unwrap().alive.clone();
                let reload = iced::futures::executor::block_on(prepare_reload(
                    entry.clone(),
                    vec![running.clone()],
                    serial,
                ));
                let loaded =
                    super::reload::finish_reload(std::slice::from_ref(&running), serial, reload)
                        .unwrap();
                assert_eq!(loaded.surface, surface);
                assert!(
                    !old_alive.load(Ordering::Relaxed),
                    "retired instance and its streams must be released"
                );
                {
                    let mut guest = surface.0.lock().unwrap();
                    assert_eq!(
                        guest.backend.snapshot().unwrap(),
                        before,
                        "complete count/draft/theme/editor state survives replacement"
                    );
                }
                ui = pump_theme(ui, &surface, &mut renderer, &mut now, true);
                let mut guest = surface.0.lock().unwrap();
                assert_eq!(
                    guest.theme_subscriptions.len(),
                    1,
                    "reload must replace, not accumulate subscriptions"
                );
                assert_eq!(
                    guest.backend.snapshot().unwrap(),
                    before,
                    "restart does not replay mount or duplicate state updates"
                );
            }
            ui = pump_theme(ui, &surface, &mut renderer, &mut now, false);
            assert_eq!(
                state_field(&mut surface.0.lock().unwrap(), "dark"),
                wire::SnapshotValue::Bool(false),
                "replacement receives later host theme changes"
            );
            if name == "Counter" {
                click(&mut ui, &mut renderer, "+");
                let _ui = pump_theme(ui, &surface, &mut renderer, &mut now, false);
                assert_eq!(
                    state_field(&mut surface.0.lock().unwrap(), "count"),
                    wire::SnapshotValue::I64(2),
                    "replacement still routes events"
                );
            }
        }
    }
}

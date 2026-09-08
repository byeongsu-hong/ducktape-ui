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
            assert!(
                guest
                    .backend
                    .snapshot()
                    .unwrap_err()
                    .contains("pending work"),
                "the existing on-mount stream snapshot restriction applies to both backends"
            );
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

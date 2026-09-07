//! The actual wasm guest mounted in GuestView, using native mouse/key events.
use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Operation;
use iced::{Event, Rectangle, Size, mouse, window};
use iced::{Font, Pixels, keyboard};
use iced_test::runtime::{UserInterface, user_interface};
use ui_lang_runtime::view_tree;

fn renderer() -> iced::Renderer {
    iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .unwrap()
}
fn guest() -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/widget-fixture/app_store_widget_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle widget fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Arc::new(Mutex::new(
        Guest::load(&CatalogEntry {
            id: "widget-fixture".into(),
            name: "Widget fixture".into(),
            description: String::new(),
            capabilities: vec![Capability {
                name: "clock".into(),
            }],
            path: path.to_string_lossy().into_owned(),
            mark: "W".into(),
            hash,
        })
        .unwrap(),
    ))
}
type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;
fn build(
    guest: &Arc<Mutex<Guest>>,
    cache: user_interface::Cache,
    renderer: &mut iced::Renderer,
) -> Ui {
    UserInterface::build(
        wasm_view(Surface(guest.clone()), false),
        Size::new(600.0, 1000.0),
        cache,
        renderer,
    )
}
fn redraw(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut std::time::Instant,
) -> Ui {
    *now += std::time::Duration::from_secs(1);
    let mut messages = vec![];
    ui.update(
        &[Event::Window(window::Event::RedrawRequested(*now))],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut messages,
    );
    assert!(
        guest.lock().unwrap().fault.is_none(),
        "guest must remain live"
    );
    if messages.iter().any(|message| message == "wake") {
        build(guest, ui.into_cache(), renderer)
    } else {
        ui
    }
}
fn focused(ui: &mut Ui, renderer: &iced::Renderer, target: &str) -> bool {
    let bytes = view_tree::execute_widget_command(
        ui_lang_wire::WidgetCommand::Focused {
            target: target.into(),
        },
        |operation| ui.operate(renderer, operation),
    )
    .unwrap();
    ui_lang_wire::decode(&bytes).unwrap()
}
struct TextBounds<'a> {
    label: &'a str,
    bounds: Option<Rectangle>,
}
impl Operation for TextBounds<'_> {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn text(&mut self, _id: Option<&iced::widget::Id>, bounds: Rectangle, text: &str) {
        if text == self.label {
            self.bounds = Some(bounds);
        }
    }
}
fn click(ui: &mut Ui, renderer: &mut iced::Renderer, label: &str) {
    let mut find = TextBounds {
        label,
        bounds: None,
    };
    ui.operate(renderer, &mut find);
    let point = find.bounds.expect("rendered button label").center();
    ui.update(
        &[
            Event::Mouse(mouse::Event::CursorMoved { position: point }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
        mouse::Cursor::Available(point),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn type_x(ui: &mut Ui, renderer: &mut iced::Renderer) {
    ui.update(
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("X".into()),
            modified_key: keyboard::Key::Character("X".into()),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: Some("X".into()),
            repeat: false,
        })],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn value(guest: &Arc<Mutex<Guest>>, key: &str) -> String {
    fn find(node: &ui_lang_wire::Node, key: &str) -> Option<String> {
        match node {
            ui_lang_wire::Node::Input {
                key: found, value, ..
            } if found == key => return Some(value.clone()),
            ui_lang_wire::Node::Text {
                key: found,
                content,
                ..
            } if found == key => return Some(content.clone()),
            _ => {}
        }
        node.children().iter().find_map(|child| find(child, key))
    }
    find(guest.lock().unwrap().frame.root.as_ref().unwrap(), key).unwrap()
}

#[test]
#[ignore = "requires bundled widget-fixture wasm"]
fn bundled_widget_tasks_focus_after_mount_and_edit_only_the_requesting_guest() {
    let mut renderer = renderer();
    let first = guest();
    let second = guest();
    let mut a = build(&first, user_interface::Cache::default(), &mut renderer);
    let mut b = build(&second, user_interface::Cache::default(), &mut renderer);
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        a = redraw(a, &first, &mut renderer, &mut now);
        b = redraw(b, &second, &mut renderer, &mut now);
    }
    assert!(
        focused(&mut a, &renderer, "WidgetFixture/first"),
        "boot focus must survive timer-driven frames until mounted"
    );
    assert!(focused(&mut b, &renderer, "WidgetFixture/first"));
    assert!(
        value(&first, "WidgetFixture/pulses")
            .parse::<u64>()
            .unwrap()
            > 0,
        "the timer must actually change frames during boot focus"
    );
    click(&mut a, &mut renderer, "Focus second");
    for _ in 0..3 {
        a = redraw(a, &first, &mut renderer, &mut now);
    }
    assert!(
        focused(&mut a, &renderer, "WidgetFixture/second"),
        "clicked guest focus task must reach its second input"
    );
    assert!(
        focused(&mut b, &renderer, "WidgetFixture/first"),
        "a different guest must keep its own focus"
    );
    assert!(!focused(&mut b, &renderer, "WidgetFixture/second"));
    click(&mut a, &mut renderer, "Focus first");
    for _ in 0..4 {
        a = redraw(a, &first, &mut renderer, &mut now);
    }
    assert_eq!(
        value(&first, "WidgetFixture/focused"),
        "focused",
        "focused query must resume its wasm handler"
    );
    click(&mut a, &mut renderer, "Select range");
    for _ in 0..4 {
        a = redraw(a, &first, &mut renderer, &mut now);
    }
    type_x(&mut a, &mut renderer);
    a = redraw(a, &first, &mut renderer, &mut now);
    assert_eq!(
        value(&first, "WidgetFixture/first"),
        "aXd",
        "wasm selection task must affect subsequent real typing"
    );
    assert_eq!(value(&second, "WidgetFixture/first"), "abcd");
    assert!(focused(&mut a, &renderer, "WidgetFixture/first"));
    click(&mut a, &mut renderer, "Cancel focus");
    for _ in 0..4 {
        a = redraw(a, &first, &mut renderer, &mut now);
    }
    assert!(
        !focused(&mut a, &renderer, "WidgetFixture/second"),
        "a request canceled in the same guest frame must never focus its target"
    );
    for (label, expected) in [("Scroll to", 100.0), ("Scroll by", 76.0)] {
        click(&mut a, &mut renderer, label);
        for _ in 0..3 {
            a = redraw(a, &first, &mut renderer, &mut now);
        }
        let position = scroll_position(&mut a, &renderer);
        assert!(
            (position.0 - expected).abs() < 0.5,
            "wasm {label} must move the native scroll region: {position:?}"
        );
    }
    for (label, fraction) in [("Snap", 0.5), ("Snap end", 1.0)] {
        click(&mut a, &mut renderer, label);
        for _ in 0..3 {
            a = redraw(a, &first, &mut renderer, &mut now);
        }
        let (translation, overflow) = scroll_position(&mut a, &renderer);
        assert!(overflow > 400.0);
        assert!(
            (translation - overflow * fraction).abs() < 0.5,
            "wasm {label} must reach the requested content position"
        );
    }
}

#[derive(Default)]
struct ScrollPosition(Option<(f32, f32)>);
impl Operation for ScrollPosition {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn scrollable(
        &mut self,
        id: Option<&iced::widget::Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: iced::Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&iced::widget::Id::new("WidgetFixture/list")) {
            self.0 = Some((translation.y, (content.height - bounds.height).max(0.0)));
        }
    }
}
fn scroll_position(ui: &mut Ui, renderer: &iced::Renderer) -> (f32, f32) {
    let mut position = ScrollPosition::default();
    ui.operate(renderer, &mut position);
    position.0.expect("mounted scroll region")
}

#[test]
#[ignore = "requires bundled widget-fixture wasm"]
fn bundled_widget_queue_defers_mount_and_rejects_stale_canceled_or_over_budget_work() {
    let owned = guest();
    let mut guest = owned.lock().unwrap();
    let now = std::time::Instant::now();
    let command = || ui_lang_wire::WidgetCommand::Focus {
        target: "WidgetFixture/first".into(),
    };
    let revision = guest.frame_rev;
    guest.widgets.push((901, revision, command()));
    let mut count = 0;
    let mut execute = |_| {
        count += 1;
        Ok(ui_lang_wire::encode(&()))
    };
    let mut mount = Some(MountedWidgets {
        revision: guest.frame_rev + 1,
        execute: &mut execute,
    });
    guest.execute_widgets(now, &mut mount, &mut 16);
    assert_eq!(
        guest.widgets.len(),
        1,
        "unmounted frame must defer native operations"
    );
    guest.cancel(901);
    mount.as_mut().unwrap().revision = guest.frame_rev;
    guest.execute_widgets(now, &mut mount, &mut 16);
    assert!(guest.widgets.is_empty());
    let revision = guest.frame_rev;
    guest.widgets.push((902, revision, command()));
    guest.frame_rev += 1;
    mount.as_mut().unwrap().revision = guest.frame_rev;
    guest.execute_widgets(now, &mut mount, &mut 16);
    let revision = guest.frame_rev;
    guest.widgets.push((903, revision, command()));
    guest.reply_bytes = MAX_REPLY_BYTES_PER_TICK + 1;
    guest.execute_widgets(now, &mut mount, &mut 16);
    assert_eq!(
        count, 0,
        "canceled, stale and over-budget requests must not mutate native widgets"
    );
    for id in [902, 903] {
        assert!(guest.due.iter().any(|(_, event)| matches!(event, wire::Event::Response { id: found, result: Err(_), .. } if *found == id)),
            "rejected widget request must resolve with an error");
    }
}

#[test]
#[ignore = "requires bundled widget-fixture wasm"]
fn bundled_widget_execution_is_charged_to_the_redraw_time_budget() {
    let owned = guest();
    let mut guest = owned.lock().unwrap();
    let mut now = std::time::Instant::now();
    guest.redraw(now, &mut iced::advanced::clipboard::Null, None);
    assert!(!guest.widgets.is_empty(), "boot focus must be queued");
    now += std::time::Duration::from_secs(1);
    let revision = guest.frame_rev;
    let mut execute = |_| {
        std::thread::sleep(TICK_BUDGET * 2);
        Ok(wire::encode(&()))
    };
    guest.redraw(
        now,
        &mut iced::advanced::clipboard::Null,
        Some(MountedWidgets {
            revision,
            execute: &mut execute,
        }),
    );
    assert!(
        guest.resting_until.is_some_and(|until| until > now),
        "native widget work must count toward redraw throttling"
    );
}

#[derive(Default)]
struct FocusStates(Vec<(iced::widget::Id, bool)>);
impl Operation for FocusStates {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn focusable(
        &mut self,
        id: Option<&iced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        if let Some(id) = id
            && ["host-input", "WidgetFixture/first", "WidgetFixture/second"]
                .iter()
                .any(|key| id == &iced::widget::Id::new(key))
        {
            self.0.push((id.clone(), state.is_focused()));
        }
    }
}

#[test]
#[ignore = "requires bundled widget-fixture wasm"]
fn bundled_widget_commands_preserve_siblings_and_host_focus_in_one_window() {
    let mut renderer = renderer();
    let first = guest();
    let second = guest();
    let build_pair = |cache, renderer: &mut iced::Renderer| {
        UserInterface::build(
            iced::widget::column![
                iced::widget::text_input("Host", "host")
                    .id("host-input")
                    .on_input(std::convert::identity),
                iced::widget::row![
                    iced::widget::container(wasm_view(Surface(first.clone()), false)).width(600),
                    iced::widget::container(wasm_view(Surface(second.clone()), false)).width(600),
                ],
            ],
            Size::new(1200.0, 1100.0),
            cache,
            renderer,
        )
    };
    let mut ui = build_pair(user_interface::Cache::default(), &mut renderer);
    view_tree::execute_widget_command(
        wire::WidgetCommand::Focus {
            target: "host-input".into(),
        },
        |operation| ui.operate(&renderer, operation),
    )
    .unwrap();
    let mut now = std::time::Instant::now();
    for frame in 0..8 {
        if frame == 4 {
            let mut guest = first.lock().unwrap();
            let revision = guest.frame_rev;
            guest.widgets.push((
                999,
                revision,
                wire::WidgetCommand::Focus {
                    target: "WidgetFixture/second".into(),
                },
            ));
        }
        now += std::time::Duration::from_secs(1);
        let mut messages = vec![];
        ui.update(
            &[Event::Window(window::Event::RedrawRequested(now))],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        if messages.iter().any(|message| message == "wake") {
            ui = build_pair(ui.into_cache(), &mut renderer);
        }
    }
    assert!(first.lock().unwrap().fault.is_none());
    assert!(second.lock().unwrap().fault.is_none());
    let mut states = FocusStates::default();
    ui.operate(&renderer, &mut states);
    assert_eq!(
        states.0,
        vec![
            (iced::widget::Id::new("host-input"), true),
            (iced::widget::Id::new("WidgetFixture/first"), false),
            (iced::widget::Id::new("WidgetFixture/second"), true),
            (iced::widget::Id::new("WidgetFixture/first"), true),
            (iced::widget::Id::new("WidgetFixture/second"), false),
        ],
        "guest boot commands must preserve siblings and the host in the same UI root"
    );
}

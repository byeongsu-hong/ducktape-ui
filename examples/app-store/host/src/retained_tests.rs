//! The actual wasm guest mounted in GuestView, using native mouse/key events.
use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Operation;
use iced::{Event, Rectangle, Size, mouse, window};
use iced::{Font, Pixels};
use iced_test::runtime::{UserInterface, user_interface};

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
        .join("../target/retained-fixture/app_store_retained_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle widget fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Arc::new(Mutex::new(
        Guest::load(&CatalogEntry {
            preferred_size: None,
            id: "retained-fixture".into(),
            name: "Retained fixture".into(),
            description: String::new(),
            capabilities: vec![],
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
#[derive(Default)]
struct TextBounds<'a> {
    label: &'a str,
    bounds: Option<Rectangle>,
    translation: iced::Vector,
    pending: Option<iced::Vector>,
}
impl Operation for TextBounds<'_> {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        let offset = self.pending.take().unwrap_or(iced::Vector::ZERO);
        self.translation += offset;
        visit(self);
        self.translation -= offset;
    }
    fn scrollable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: Rectangle,
        _content: Rectangle,
        translation: iced::Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.pending = Some(translation);
    }
    fn text(&mut self, _id: Option<&iced::widget::Id>, bounds: Rectangle, text: &str) {
        if text == self.label {
            self.bounds = Some(Rectangle {
                x: bounds.x - self.translation.x,
                y: bounds.y - self.translation.y,
                ..bounds
            });
        }
    }
}
fn click(ui: &mut Ui, renderer: &mut iced::Renderer, label: &str) {
    let mut find = TextBounds {
        label,
        bounds: None,
        ..Default::default()
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

fn row_bounds(ui: &mut Ui, renderer: &iced::Renderer, label: &str) -> Rectangle {
    let mut find = TextBounds {
        label,
        bounds: None,
        ..Default::default()
    };
    ui.operate(renderer, &mut find);
    find.bounds.expect("rendered native log row")
}
fn frames(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut std::time::Instant,
) -> Ui {
    for _ in 0..4 {
        ui = redraw(ui, guest, renderer, now);
    }
    ui
}
fn seed(guest: &Arc<Mutex<Guest>>) -> Arc<crate::surfaces::log::Session> {
    let session = guest.lock().unwrap().log_session.clone();
    for row in 0..100 {
        session.append(&format!("host row {row:03}"));
    }
    session
}
#[test]
#[ignore = "requires bundled retained-fixture wasm"]
fn bundled_retained_log_preserves_interaction_and_releases_mount_without_ending_session() {
    let mut renderer = renderer();
    let guest = guest();
    let session = seed(&guest);
    let baseline = Arc::strong_count(&session);
    let mut now = std::time::Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "RetainedFixture/rows"), "100");
    assert!(
        Arc::strong_count(&session) > baseline,
        "a mounted view must hold its session lease"
    );
    click(&mut ui, &mut renderer, "host row 095");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "RetainedFixture/selected"),
        "96",
        "native row selection must route a typed wasm notice"
    );
    let before = value(&guest, "RetainedFixture/offset")
        .parse::<f32>()
        .unwrap();
    let point = row_bounds(&mut ui, &renderer, "host row 099").center();
    ui.update(
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 4.0 },
        })],
        mouse::Cursor::Available(point),
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert!(
        value(&guest, "RetainedFixture/offset")
            .parse::<f32>()
            .unwrap()
            < before,
        "native wheel must scroll history"
    );
    assert_eq!(value(&guest, "RetainedFixture/following"), "false");
    click(&mut ui, &mut renderer, "Advance");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "RetainedFixture/selected"),
        "96",
        "unrelated guest frames must preserve retained selection"
    );
    session.append("appended while paused");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "RetainedFixture/unread"), "1");
    assert_eq!(value(&guest, "RetainedFixture/following"), "false");
    click(&mut ui, &mut renderer, "Toggle log");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        Arc::strong_count(&session),
        baseline,
        "unmount must release the native view lease"
    );
    session.append("appended while unmounted");
    click(&mut ui, &mut renderer, "Toggle log");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "RetainedFixture/rows"),
        "102",
        "host session must keep receiving data while no view is mounted"
    );
    assert_eq!(
        value(&guest, "RetainedFixture/selected"),
        "-1",
        "remount must create fresh native view state"
    );
    assert_eq!(value(&guest, "RetainedFixture/following"), "true");
    assert!(row_bounds(&mut ui, &renderer, "appended while unmounted").height > 0.0);
    drop(ui);
    drop(guest);
    assert_eq!(
        Arc::strong_count(&session),
        1,
        "dropping the guest must release its registry while the host can keep the session"
    );
    session.append("after guest exit");
}

#[test]
#[ignore = "requires bundled retained-fixture wasm"]
fn bundled_retained_replacement_with_same_keys_cannot_inherit_another_instance_view() {
    let mut renderer = renderer();
    let first = guest();
    let first_session = seed(&first);
    let mut now = std::time::Instant::now();
    let ui = build(&first, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &first, &mut renderer, &mut now);
    click(&mut ui, &mut renderer, "host row 095");
    ui = frames(ui, &first, &mut renderer, &mut now);
    assert_eq!(value(&first, "RetainedFixture/selected"), "96");
    let second = guest();
    // Both views may legitimately share one host session, but never selection.
    {
        let mut next = second.lock().unwrap();
        next.log_session = first_session.clone();
        next.surfaces = crate::surfaces::registry(first_session.clone());
        next.redraw(now, &mut iced::advanced::clipboard::Null, None);
    }
    let replacement = build(&second, ui.into_cache(), &mut renderer);
    let mut replacement = frames(replacement, &second, &mut renderer, &mut now);
    click(&mut replacement, &mut renderer, "Resume tail");
    let replacement = frames(replacement, &second, &mut renderer, &mut now);
    assert_eq!(
        value(&second, "RetainedFixture/selected"),
        "-1",
        "an identical node key in a replacement guest must acquire a new native view"
    );
    assert_eq!(value(&second, "RetainedFixture/rows"), "100");
    assert_eq!(value(&first, "RetainedFixture/selected"), "96");
    drop(replacement);
}

#[test]
#[ignore = "requires bundled retained-fixture wasm"]
fn bundled_retained_two_views_share_session_but_not_selection_in_one_window() {
    let mut renderer = renderer();
    let first = guest();
    let session = seed(&first);
    let second = guest();
    {
        let mut next = second.lock().unwrap();
        next.log_session = session.clone();
        next.surfaces = crate::surfaces::registry(session.clone());
    }
    let build_pair = |cache, renderer: &mut iced::Renderer| {
        UserInterface::build(
            iced::widget::row![
                iced::widget::container(wasm_view(Surface(first.clone()), false)).width(600),
                iced::widget::container(wasm_view(Surface(second.clone()), false)).width(600),
            ],
            Size::new(1200.0, 1000.0),
            cache,
            renderer,
        )
    };
    let mut ui = build_pair(user_interface::Cache::default(), &mut renderer);
    let mut now = std::time::Instant::now();
    for frame in 0..8 {
        if frame == 4 {
            click(&mut ui, &mut renderer, "host row 095");
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
    // TextBounds picks the last matching row, in the second view.
    assert_eq!(value(&second, "RetainedFixture/selected"), "96");
    assert_eq!(
        value(&first, "RetainedFixture/selected"),
        "-1",
        "a sibling native timeline must retain its own selection"
    );
    assert_eq!(value(&first, "RetainedFixture/rows"), "100");
    assert_eq!(value(&second, "RetainedFixture/rows"), "100");
}

#[test]
#[ignore = "requires bundled retained-fixture wasm"]
fn bundled_retained_native_hover_and_layout_consumed_updates_remain_observable() {
    let mut renderer = renderer();
    let guest = guest();
    let session = seed(&guest);
    for row in 100..256 {
        session.append(&format!("host row {row:03}"));
    }
    let mut now = std::time::Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &guest, &mut renderer, &mut now);
    let point = row_bounds(&mut ui, &renderer, "host row 255").center();
    ui.update(
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 10000.0 },
        })],
        mouse::Cursor::Available(point),
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "RetainedFixture/offset")
            .parse::<f32>()
            .unwrap(),
        0.0
    );
    assert_eq!(value(&guest, "RetainedFixture/following"), "false");
    session.append("arrived before relayout");
    ui = build(&guest, ui.into_cache(), &mut renderer);
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(
        value(&guest, "RetainedFixture/unread"),
        "1",
        "layout must not swallow the session update notice"
    );
    let point = row_bounds(&mut ui, &renderer, "Resume tail").center();
    let mut paint = |ui: &mut Ui, cursor| {
        now += std::time::Duration::from_secs(1);
        ui.update(
            &[Event::Window(window::Event::RedrawRequested(now))],
            cursor,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut vec![],
        );
        ui.draw(
            &mut renderer,
            &iced::Theme::Dark,
            &iced::advanced::renderer::Style {
                text_color: iced::Color::WHITE,
            },
            cursor,
        );
        renderer.screenshot(Size::new(600, 1000), 1.0, iced::Color::BLACK)
    };
    let rest = paint(&mut ui, mouse::Cursor::Unavailable);
    let hover = paint(&mut ui, mouse::Cursor::Available(point));
    assert!(
        rest != hover,
        "the retained native button must paint its enabled hover state"
    );
    if let Ok(path) = std::env::var("ICE_RETAINED_CAPTURE") {
        std::fs::write(path, hover).unwrap();
    }
}

#[test]
#[ignore = "requires bundled retained-fixture wasm"]
fn bundled_retained_full_ring_keeps_paused_history_on_append() {
    let mut renderer = renderer();
    let guest = guest();
    let session = guest.lock().unwrap().log_session.clone();
    for row in 0..256 {
        session.append(&format!("host row {row:03}"));
    }
    let mut now = std::time::Instant::now();
    let ui = build(&guest, user_interface::Cache::default(), &mut renderer);
    let mut ui = frames(ui, &guest, &mut renderer, &mut now);
    click(&mut ui, &mut renderer, "host row 251");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    let point = row_bounds(&mut ui, &renderer, "host row 255").center();
    ui.update(
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 4.0 },
        })],
        mouse::Cursor::Available(point),
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "RetainedFixture/following"), "false");
    let offset = value(&guest, "RetainedFixture/offset")
        .parse::<f32>()
        .unwrap();
    let label = format!("host row {:03}", (offset / 24.0).ceil() as usize);
    let row_before = row_bounds(&mut ui, &renderer, &label);
    session.append("row after ring eviction");
    ui = frames(ui, &guest, &mut renderer, &mut now);
    assert_eq!(value(&guest, "RetainedFixture/rows"), "256");
    assert_eq!(value(&guest, "RetainedFixture/selected"), "252");
    assert_eq!(
        value(&guest, "RetainedFixture/following"),
        "false",
        "bounded ring eviction must preserve paused history"
    );
    assert_eq!(value(&guest, "RetainedFixture/unread"), "1");
    let after = value(&guest, "RetainedFixture/offset")
        .parse::<f32>()
        .unwrap();
    assert!(
        (after - (offset - 24.0)).abs() < 0.5,
        "front eviction must preserve the same surviving visible rows"
    );
    let row_after = row_bounds(&mut ui, &renderer, &label);
    assert!(
        (row_after.y - row_before.y).abs() < 0.5,
        "the surviving native row must stay at its screen position: before={row_before:?}, after={row_after:?}"
    );
    drop(ui);
}

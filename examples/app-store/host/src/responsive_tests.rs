//! Actual wasm container rules, exercised by host layout and native input.
use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Operation;
use iced::{Event, Font, Pixels, Rectangle, Size, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};
use std::sync::atomic::{AtomicUsize, Ordering};
use ui_lang_runtime::view_tree;

type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;
#[derive(Default)]
struct Counts {
    live: AtomicUsize,
    built: AtomicUsize,
}
struct Lease(Arc<Counts>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.live.fetch_sub(1, Ordering::SeqCst);
    }
}
impl Lease {
    fn keep(&self) {}
}
fn renderer() -> iced::Renderer {
    iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .unwrap()
}
fn guest(counts: Arc<Counts>) -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/responsive-fixture/app_store_responsive_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle responsive fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut guest = Guest::load(&CatalogEntry {
        preferred_size: None,
        id: "responsive-fixture".into(),
        name: "Responsive fixture".into(),
        description: String::new(),
        capabilities: vec![],
        path: path.to_string_lossy().into_owned(),
        mark: "R".into(),
        hash,
    })
    .unwrap();
    guest.surfaces.insert(
        "marker".into(),
        Arc::new(move |_key, args| {
            let [wire::SurfaceValue::Str(label)] = args else {
                panic!("marker string")
            };
            counts.live.fetch_add(1, Ordering::SeqCst);
            counts.built.fetch_add(1, Ordering::SeqCst);
            let lease = Lease(counts.clone());
            let text: iced::Element<'static, (), iced::Theme, iced::Renderer> =
                iced::widget::text(format!("{label} marker")).into();
            text.map(move |()| {
                lease.keep();
                wire::SurfaceValue::Unit
            })
        }),
    );
    Arc::new(Mutex::new(guest))
}
fn build(
    guest: &Arc<Mutex<Guest>>,
    cache: user_interface::Cache,
    renderer: &mut iced::Renderer,
    width: f32,
) -> Ui {
    UserInterface::build(
        wasm_view(Surface(guest.clone()), false),
        Size::new(width, 320.0),
        cache,
        renderer,
    )
}
fn redraw(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut std::time::Instant,
    width: f32,
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
    if messages.iter().any(|m| m == "wake") {
        build(guest, ui.into_cache(), renderer, width)
    } else {
        ui
    }
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
fn bounds(ui: &mut Ui, renderer: &mut iced::Renderer, label: &str) -> Option<Rectangle> {
    let mut op = TextBounds {
        label,
        bounds: None,
    };
    ui.operate(renderer, &mut op);
    op.bounds
}
fn click(ui: &mut Ui, renderer: &mut iced::Renderer, label: &str) {
    let point = bounds(ui, renderer, label)
        .expect("visible button label")
        .center();
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
fn green_area(ui: &mut Ui, renderer: &mut iced::Renderer, width: u32) -> usize {
    ui.draw(
        renderer,
        &iced::Theme::Dark,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::WHITE,
        },
        mouse::Cursor::Unavailable,
    );
    let pixels = renderer.screenshot(Size::new(width, 320), 1.0, iced::Color::BLACK);
    if width == 640
        && pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[..3] == [0, 255, 0])
        && let Ok(path) = std::env::var("ICE_RESPONSIVE_CAPTURE")
    {
        std::fs::write(path, &pixels).unwrap();
    }
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        // Fractional layout coordinates split edge coverage across two rows.
        // Sum channel coverage instead of counting only fully opaque interiors.
        .filter(|pixel| pixel[0] == 0 && pixel[2] == 0)
        .map(|pixel| usize::from(pixel[1]))
        .sum::<usize>()
        / 255
}

#[test]
#[ignore = "requires bundled responsive-fixture wasm"]
fn bundled_responsive_layout_selects_local_rules_without_guest_ticks() {
    let mut renderer = renderer();
    let counts = Arc::new(Counts::default());
    let guest = guest(counts.clone());
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        240.0,
    );
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 240.0);
    }
    assert!(bounds(&mut ui, &mut renderer, "Narrow").is_some());
    assert!(bounds(&mut ui, &mut renderer, "Wide").is_none());
    assert_eq!(
        counts.live.load(Ordering::SeqCst),
        1,
        "hidden provider must not mount"
    );
    assert_eq!(green_area(&mut ui, &mut renderer, 240), 0);
    let ticks = guest.lock().unwrap().ticks;
    let built = counts.built.load(Ordering::SeqCst);
    ui = ui.relayout(Size::new(240.0, 320.0), &mut renderer);
    assert_eq!(
        counts.built.load(Ordering::SeqCst),
        built,
        "same-size layout must reuse content"
    );
    ui = ui.relayout(Size::new(640.0, 320.0), &mut renderer);
    assert_eq!(
        guest.lock().unwrap().ticks,
        ticks,
        "layout must not call the guest"
    );
    assert!(bounds(&mut ui, &mut renderer, "Wide").is_some());
    assert!(bounds(&mut ui, &mut renderer, "Narrow").is_none());
    assert!(
        bounds(&mut ui, &mut renderer, "Nested wide").is_some(),
        "nested rule must read both its own and its ancestor's size"
    );
    assert_eq!(
        counts.live.load(Ordering::SeqCst),
        1,
        "old native branch must unmount"
    );
    assert_eq!(
        green_area(&mut ui, &mut renderer, 640),
        900,
        "wide branch paints its 30x30 green box"
    );
    click(&mut ui, &mut renderer, "Wide");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(
        bounds(&mut ui, &mut renderer, "wide").is_some(),
        "native click must update guest state"
    );
    click(&mut ui, &mut renderer, "Raise threshold");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert!(
        bounds(&mut ui, &mut renderer, "Narrow").is_some(),
        "updated guest threshold must change the host rule"
    );
    assert!(bounds(&mut ui, &mut renderer, "Wide").is_none());
    assert!(bounds(&mut ui, &mut renderer, "Nested wide").is_none());
    assert_eq!(green_area(&mut ui, &mut renderer, 640), 0);
    drop(ui);
    assert_eq!(
        counts.live.load(Ordering::SeqCst),
        0,
        "unmount releases native view leases"
    );
}

#[test]
#[ignore = "requires bundled responsive-fixture wasm"]
fn bundled_responsive_input_survives_resize_and_instances_are_isolated() {
    use iced::keyboard;
    let mut renderer = renderer();
    let first = guest(Arc::new(Counts::default()));
    let second = guest(Arc::new(Counts::default()));
    let mut a = build(
        &first,
        user_interface::Cache::default(),
        &mut renderer,
        640.0,
    );
    let mut b = build(
        &second,
        user_interface::Cache::default(),
        &mut renderer,
        240.0,
    );
    let mut now = std::time::Instant::now();
    for _ in 0..4 {
        a = redraw(a, &first, &mut renderer, &mut now, 640.0);
        b = redraw(b, &second, &mut renderer, &mut now, 240.0);
    }
    click(&mut a, &mut renderer, "Focus draft");
    for _ in 0..3 {
        a = redraw(a, &first, &mut renderer, &mut now, 640.0);
    }
    let focused = |ui: &mut Ui, renderer: &iced::Renderer| -> bool {
        let bytes = view_tree::execute_widget_command(
            wire::WidgetCommand::Focused {
                target: "ResponsiveFixture/panel/draft".into(),
            },
            |op| ui.operate(renderer, op),
        )
        .unwrap();
        wire::decode(&bytes).unwrap()
    };
    assert!(
        focused(&mut a, &renderer),
        "host request focuses the responsive input"
    );
    assert!(!focused(&mut b, &renderer), "focus is instance-local");
    a = a.relayout(Size::new(240.0, 320.0), &mut renderer);
    assert!(
        focused(&mut a, &renderer),
        "stable input retains focus through branch change"
    );
    a.update(
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("X".into()),
            modified_key: keyboard::Key::Character("X".into()),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: Some("X".into()),
            repeat: false,
        })],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    for _ in 0..3 {
        a = redraw(a, &first, &mut renderer, &mut now, 240.0);
        b = redraw(b, &second, &mut renderer, &mut now, 240.0);
    }
    assert!(
        bounds(&mut a, &mut renderer, "X").is_some(),
        "native edit reaches guest echo"
    );
    assert!(
        bounds(&mut b, &mut renderer, "X").is_none(),
        "other instance keeps its draft"
    );
    a = a.relayout(Size::new(640.0, 320.0), &mut renderer);
    assert!(
        bounds(&mut a, &mut renderer, "X").is_some(),
        "draft survives return to wide"
    );
    assert!(bounds(&mut a, &mut renderer, "Wide").is_some());
    assert!(bounds(&mut b, &mut renderer, "Narrow").is_some());
}

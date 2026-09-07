//! Actual wasm row identity through native input.
use super::*;
use iced::advanced::renderer::Headless;
use iced::{Event, Font, Pixels, Size, keyboard, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};
use ui_lang_runtime::view_tree;

type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;
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
        .join("../target/keyed-fixture/app_store_keyed_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle keyed fixture first");
    let hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let guest = Guest::load(&CatalogEntry {
        id: "keyed-fixture".into(),
        name: "Keyed fixture".into(),
        description: String::new(),
        capabilities: vec![],
        path: path.to_string_lossy().into_owned(),
        mark: "L".into(),
        hash,
    })
    .unwrap();
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
        Size::new(width, 600.0),
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
fn activate(guest: &Arc<Mutex<Guest>>, label: &str) {
    fn route(node: &wire::Node, label: &str) -> Option<u32> {
        if let wire::Node::Button {
            content: wire::ButtonContent::Label(text),
            on_press,
            ..
        } = node
            && text == label
        {
            return *on_press;
        }
        node.children().iter().find_map(|child| route(child, label))
    }
    let mut guest = guest.lock().unwrap();
    let slot = route(guest.frame.root.as_ref().unwrap(), label).expect("rendered button route");
    guest.deliver(Output::Activate(slot));
}

fn focus(ui: &mut Ui, renderer: &iced::Renderer, key: &str) -> bool {
    view_tree::execute_widget_command(wire::WidgetCommand::Focus { target: key.into() }, |op| {
        ui.operate(renderer, op)
    })
    .expect("host focus command");
    let result = view_tree::execute_widget_command(
        wire::WidgetCommand::Focused { target: key.into() },
        |op| ui.operate(renderer, op),
    )
    .unwrap();
    wire::decode(&result).unwrap()
}
fn type_text(ui: &mut Ui, renderer: &mut iced::Renderer, text: &str) {
    ui.update(
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(text.into()),
            modified_key: keyboard::Key::Character(text.into()),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: Some(text.into()),
            repeat: false,
        })],
        mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn inputs(guest: &Arc<Mutex<Guest>>) -> Vec<(String, String)> {
    fn collect(node: &wire::Node, out: &mut Vec<(String, String)>) {
        if let wire::Node::Input { key, value, .. } = node {
            out.push((key.clone(), value.clone()));
        }
        for child in node.children() {
            collect(child, out);
        }
    }
    let mut out = vec![];
    collect(guest.lock().unwrap().frame.root.as_ref().unwrap(), &mut out);
    out
}
fn is_focused(ui: &mut Ui, renderer: &iced::Renderer, key: &str) -> bool {
    let bytes = view_tree::execute_widget_command(
        wire::WidgetCommand::Focused { target: key.into() },
        |op| ui.operate(renderer, op),
    )
    .unwrap();
    wire::decode(&bytes).unwrap()
}

#[test]
#[ignore = "requires bundled keyed-fixture wasm"]
fn bundled_keyed_rows_keep_input_and_focus_through_reordering() {
    keyed_input_and_focus(false);
}

#[test]
#[ignore = "requires bundled keyed-fixture wasm"]
fn bundled_nonvirtual_keyed_rows_keep_input_and_focus_through_reordering() {
    keyed_input_and_focus(true);
}

fn keyed_input_and_focus(ordinary: bool) {
    let mut renderer = renderer();
    let guest = guest();
    let mut now = std::time::Instant::now();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        640.0,
    );
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    if ordinary {
        activate(&guest, "Ordinary");
        for _ in 0..4 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
        }
    }
    let all = inputs(&guest);
    assert_eq!(all.len(), 3);
    let key = all
        .iter()
        .find(|(key, _)| key.contains("/key(2)/"))
        .unwrap_or_else(|| panic!("row 2 key: {all:?}"))
        .0
        .clone();
    assert!(focus(&mut ui, &renderer, &key), "row 2 accepts focus");
    type_text(&mut ui, &mut renderer, "A");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    assert_eq!(
        inputs(&guest).iter().find(|(k, _)| k == &key).unwrap().1,
        "A"
    );
    activate(&guest, "Reorder");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    assert!(
        is_focused(&mut ui, &renderer, &key),
        "focus follows row 2 through reorder"
    );
    type_text(&mut ui, &mut renderer, "B");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    assert_eq!(
        inputs(&guest).iter().find(|(k, _)| k == &key).unwrap().1,
        "AB"
    );
    activate(&guest, "Prepend");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    assert!(
        is_focused(&mut ui, &renderer, &key),
        "focus follows row 2 through prepend"
    );
    type_text(&mut ui, &mut renderer, "C");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    let all = inputs(&guest);
    assert_eq!(all.len(), 4);
    assert_eq!(all.iter().find(|(k, _)| k == &key).unwrap().1, "ABC");
    assert!(
        all.iter()
            .filter(|(k, _)| k != &key)
            .all(|(_, value)| value.is_empty()),
        "typing affects only the keyed row"
    );
    activate(&guest, "Remove");
    ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    assert!(!is_focused(&mut ui, &renderer, &key));
    assert_eq!(inputs(&guest).len(), 2);
}

#[derive(Default)]
struct MountedRows {
    inputs: usize,
    texts: Vec<String>,
}
impl iced::advanced::widget::Operation for MountedRows {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        operate(self);
    }
    fn text_input(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        _: iced::Rectangle,
        _: &mut dyn iced::advanced::widget::operation::TextInput,
    ) {
        self.inputs += 1;
    }
    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, _: iced::Rectangle, text: &str) {
        self.texts.push(text.into());
    }
}

#[test]
#[ignore = "requires bundled keyed-fixture wasm"]
fn bundled_keyed_large_list_mounts_only_a_window_and_reaches_the_tail() {
    let guest = guest();
    let mut renderer = renderer();
    let mut now = std::time::Instant::now();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        640.0,
    );
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    activate(&guest, "Many");
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    assert_eq!(
        inputs(&guest).len(),
        200,
        "all rows exist in the guest wire tree"
    );
    let mut initial = MountedRows::default();
    ui.operate(&renderer, &mut initial);
    assert!(
        (1..=32).contains(&initial.inputs),
        "only the viewport window is mounted: {}",
        initial.inputs
    );
    assert!(
        !initial.texts.iter().any(|text| text == "199"),
        "tail is initially offscreen"
    );
    fn list(node: &wire::Node) -> Option<String> {
        if let wire::Node::Scroll {
            key,
            virtual_rows: true,
            ..
        } = node
        {
            return Some(key.clone());
        }
        node.children().iter().find_map(list)
    }
    let target = list(guest.lock().unwrap().frame.root.as_ref().unwrap()).unwrap();
    view_tree::execute_widget_command(
        wire::WidgetCommand::Snap {
            target,
            x: 0.0,
            y: 1.0,
        },
        |op| ui.operate(&renderer, op),
    )
    .unwrap();
    ui = build(&guest, ui.into_cache(), &mut renderer, 640.0);
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 640.0);
    }
    let mut tail = MountedRows::default();
    ui.operate(&renderer, &mut tail);
    assert!(
        (1..=32).contains(&tail.inputs),
        "tail still mounts a bounded window: {}",
        tail.inputs
    );
    assert!(
        tail.texts.iter().any(|text| text == "199"),
        "scroll reaches the actual last row: {:?}",
        tail.texts
    );
}

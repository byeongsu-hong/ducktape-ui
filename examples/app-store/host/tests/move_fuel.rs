//! What one pointer event costs a guest, in fuel: the number quoted in the
//! README's cost section. The counter's card is a mouse area, so a move over
//! it is a tick with one `Event::Pointer`, a handler run and a view rebuild
//! — the same shape as a press, plus the position.
//!
//! Needs the bundled component, so it is ignored by default:
//!
//! ```text
//! cargo ice bundle --target wasm32-unknown-unknown \
//!     --manifest-path examples/app-store/Cargo.toml -p app-store-counter
//! cd examples/app-store && cargo test -p app-store-host --test move_fuel -- --ignored --nocapture
//! ```

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

use ui_lang_wire as wire;

wasmtime::component::bindgen!({
    path: "../../../crates/ui-lang-guest/wit/view.wit",
    world: "view",
});

struct Host;

impl ViewImports for Host {
    fn panicked(&mut self, message: String) {
        panic!("guest panicked: {message}");
    }
}

/// The same budget the store arms every tick with.
const FUEL: u64 = 100_000_000;

struct Guest {
    store: Store<Host>,
    view: View,
    root: wire::Node,
}

impl Guest {
    fn load(engine: &Engine, component: &Component) -> Self {
        let mut store = Store::new(engine, Host);
        store.set_fuel(FUEL).unwrap();
        let mut linker = Linker::new(engine);
        View::add_to_linker::<Host, wasmtime::component::HasSelf<Host>>(&mut linker, |host| host)
            .unwrap();
        linker.define_unknown_imports_as_traps(component).unwrap();
        let view = View::instantiate(&mut store, component, &linker).unwrap();
        store.set_fuel(FUEL).unwrap();
        view.call_init(&mut store, cfg!(target_os = "macos"))
            .unwrap();
        let mut guest = Self {
            store,
            view,
            root: wire::Node::empty(),
        };
        guest.tick(Vec::new());
        guest
    }

    /// One tick with `events`; the fuel it burned.
    fn tick(&mut self, events: Vec<wire::Event>) -> u64 {
        self.store.set_fuel(FUEL).unwrap();
        let bytes = self
            .view
            .call_tick(&mut self.store, &wire::encode(&events))
            .unwrap();
        let spent = FUEL - self.store.get_fuel().unwrap();
        let mut frame: wire::Frame = wire::decode(&bytes).unwrap();
        if let Some(root) = frame.root.take() {
            self.root = root;
        } else if !frame.unchanged {
            wire::apply(&mut self.root, frame.patches).unwrap();
        }
        spent
    }

    /// The first node under the root that `matches`.
    fn find(&self, matches: &dyn Fn(&wire::Node) -> bool) -> &wire::Node {
        fn walk<'a>(
            node: &'a wire::Node,
            matches: &dyn Fn(&wire::Node) -> bool,
        ) -> Option<&'a wire::Node> {
            if matches(node) {
                return Some(node);
            }
            node.children()
                .iter()
                .find_map(|child| walk(child, matches))
        }
        walk(&self.root, matches).expect("the node is in the tree")
    }

    fn press(&self, label: &str) -> Vec<wire::Event> {
        let wire::Node::Button { on_press, .. } = self.find(&|node| {
            matches!(node, wire::Node::Button { content: wire::ButtonContent::Label(text), .. } if text == label)
        }) else {
            unreachable!()
        };
        vec![wire::Event::Message(on_press.unwrap())]
    }

    fn pad(&self) -> &wire::Node {
        self.find(&|node| matches!(node, wire::Node::MouseArea { key, .. } if key == "Counter/app/content/pad"))
    }

    fn hover(&self) -> Vec<wire::Event> {
        let wire::Node::MouseArea { on_enter, .. } = self.pad() else {
            unreachable!()
        };
        vec![wire::Event::Message(on_enter.unwrap())]
    }

    fn move_to(&self, x: f32, y: f32) -> Vec<wire::Event> {
        let wire::Node::MouseArea { on_move, .. } = self.pad() else {
            unreachable!()
        };
        vec![wire::Event::Pointer {
            handler: on_move.unwrap(),
            x,
            y,
        }]
    }

    fn scroll(&self, dy: f32) -> Vec<wire::Event> {
        let wire::Node::MouseArea { on_scroll, .. } = self.pad() else {
            unreachable!()
        };
        vec![wire::Event::Scroll {
            handler: on_scroll.unwrap(),
            dx: 0.0,
            dy,
            pixels: false,
        }]
    }
}

#[test]
#[ignore = "needs the bundled counter component; see the module doc"]
fn a_pointer_move_costs_about_what_a_press_does() {
    let catalog = std::env::var("APP_STORE_CATALOG")
        .unwrap_or_else(|_| "../target/app-store-catalog".to_string());
    let path = format!("{catalog}/app_store_counter.wasm");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let component = Component::new(&engine, &bytes).unwrap();
    println!("launch  idle  press  hover  move  move(again)  scroll");
    for launch in 0..5 {
        let mut guest = Guest::load(&engine, &component);
        let idle = guest.tick(Vec::new());
        let pressed = guest.tick(guest.press("+"));
        let hovered = guest.tick(guest.hover());
        let moved = guest.tick(guest.move_to(10.0, 20.0));
        let moved_again = guest.tick(guest.move_to(11.0, 20.0));
        let scrolled = guest.tick(guest.scroll(1.0));
        println!("{launch}  {idle}  {pressed}  {hovered}  {moved}  {moved_again}  {scrolled}");
        assert!(
            moved > idle,
            "a move is a handler and a view: {moved} vs {idle}"
        );
        assert!(
            moved < 4 * pressed.max(1),
            "a move should cost about a press: {moved} vs {pressed}"
        );
    }
}

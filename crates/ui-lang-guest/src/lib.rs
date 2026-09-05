//! The guest side of `ui-lang-wire`: a generated Ice application, compiled
//! for the `tree` target, running inside wasm as a view module.
//!
//! The guest keeps state, runs handlers and their tasks, and builds a
//! [`wire::Node`] tree every tick. It never lays out or draws: the host's
//! toolkit does that, so nothing from a renderer, a font system or a
//! windowing layer is linked here. Interaction comes back as meaning —
//! "message 3", "input handler 0 now reads `abc`" — through the per-frame
//! tables in [`slots`] that the generated view fills while it builds.
//!
//! [`export_app!`] turns an app into the `ice:view` component exports;
//! `boot_native`/`tick_native` drive the same app in an ordinary test.

use std::any::Any;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Wake, Waker};

pub use ui_lang_wire as wire;
pub use wit_bindgen;

use iced_runtime::futures::BoxStream;
use iced_runtime::{Action, task};

pub mod host;
pub mod testing;

/// What `export_app!` needs from the generated application.
pub trait App: Sized + 'static {
    type Message: Clone + iced_runtime::futures::MaybeSend + 'static;
    fn boot() -> (Self, iced::Task<Self::Message>);
    fn view(&self) -> wire::Node;
    fn update(&mut self, message: Self::Message) -> iced::Task<Self::Message>;
}

/// The per-frame tables a view fills as it builds: a button's `on_press`
/// is the index its message took here, an input's `on_input` the index of
/// its `String -> Message` constructor. The host echoes an index back; the
/// driver looks the message up in the table of the frame it echoed.
///
/// The tables are untyped so the generated code can push through this
/// crate without naming the app's message type; the driver downcasts.
pub mod slots {
    use super::*;

    thread_local! {
        static MESSAGES: RefCell<Vec<Box<dyn Any>>> = const { RefCell::new(Vec::new()) };
        static HANDLERS: RefCell<Vec<Box<dyn Any>>> = const { RefCell::new(Vec::new()) };
    }

    pub fn message<M: 'static>(message: M) -> u32 {
        MESSAGES.with_borrow_mut(|table| {
            table.push(Box::new(message));
            (table.len() - 1) as u32
        })
    }

    pub fn handler<M: 'static>(handler: Box<dyn Fn(String) -> M>) -> u32 {
        HANDLERS.with_borrow_mut(|table| {
            table.push(Box::new(handler));
            (table.len() - 1) as u32
        })
    }

    pub(crate) fn reset() {
        MESSAGES.with_borrow_mut(Vec::clear);
        HANDLERS.with_borrow_mut(Vec::clear);
    }

    pub(crate) fn take_message<M: Clone + 'static>(index: u32) -> Option<M> {
        MESSAGES.with_borrow(|table| {
            table
                .get(index as usize)
                .and_then(|entry| entry.downcast_ref::<M>())
                .cloned()
        })
    }

    pub(crate) fn run_handler<M: 'static>(index: u32, text: String) -> Option<M> {
        HANDLERS.with_borrow(|table| {
            table
                .get(index as usize)
                .and_then(|entry| entry.downcast_ref::<Box<dyn Fn(String) -> M>>())
                .map(|handler| handler(text))
        })
    }
}

type Tasks<M> = Vec<BoxStream<Action<M>>>;

/// One running app: its state, its in-flight tasks, and the last tree it
/// sent so an identical one crosses as `unchanged`.
pub struct Driver<A: App> {
    app: A,
    tasks: Tasks<A::Message>,
    last_root: Option<wire::Node>,
}

impl<A: App> Default for Driver<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: App> Driver<A> {
    pub fn new() -> Self {
        let (app, boot) = A::boot();
        let mut tasks = Vec::new();
        spawn(&mut tasks, boot);
        Self {
            app,
            tasks,
            last_root: None,
        }
    }

    /// Delivers the host's events and returns the frame they produced.
    ///
    /// Events name entries in the tables the LAST view filled, so they are
    /// dispatched before the tables are reset for this view. An index the
    /// last frame did not hand out (the host raced a rebuild) is dropped.
    ///
    /// The frame always carries the tree, `unchanged` or not, so a test can
    /// read it; the component export drops an unchanged tree before it
    /// crosses to the host.
    pub fn tick(&mut self, events: Vec<wire::Event>) -> wire::Frame {
        let Self {
            app,
            tasks,
            last_root,
        } = self;
        run_tasks(app, tasks);
        for event in events {
            let message = match event {
                wire::Event::Message(index) => slots::take_message::<A::Message>(index),
                wire::Event::Input { handler, text } => {
                    slots::run_handler::<A::Message>(handler, text)
                }
                wire::Event::Response { id, result, done } => {
                    host::fulfill(id, result, done);
                    None
                }
            };
            if let Some(message) = message {
                spawn(tasks, app.update(message));
                run_tasks(app, tasks);
            }
        }
        // A response woke a task without a message of its own to run: poll
        // once more so what it produced reaches `update` before the view.
        run_tasks(app, tasks);
        slots::reset();
        let root = app.view();
        let unchanged = last_root.as_ref() == Some(&root);
        if !unchanged {
            *last_root = Some(root.clone());
        }
        wire::Frame {
            root: Some(root),
            requests: host::drain_outbox(),
            cancels: host::drain_cancels(),
            unchanged,
        }
    }
}

fn spawn<M: iced_runtime::futures::MaybeSend + 'static>(tasks: &mut Tasks<M>, task: iced::Task<M>) {
    if let Some(stream) = task::into_stream(task) {
        tasks.push(stream);
    }
}

/// Polls every task; a message it produced goes through `update`, whose own
/// task joins the pool, until a pass produces nothing. Bounded so a handler
/// that re-emits synchronously forever cannot pin the frame.
fn run_tasks<A: App>(app: &mut A, tasks: &mut Tasks<A::Message>) {
    for _ in 0..8 {
        let messages = poll_tasks(tasks);
        if messages.is_empty() {
            return;
        }
        for message in messages {
            spawn(tasks, app.update(message));
        }
    }
}

struct Woken(AtomicBool);

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn poll_tasks<M>(tasks: &mut Tasks<M>) -> Vec<M> {
    let woken = Arc::new(Woken(AtomicBool::new(false)));
    let waker = Waker::from(woken.clone());
    let mut context = Context::from_waker(&waker);
    let mut messages = Vec::new();
    tasks.retain_mut(|stream| {
        // A task that yields (every `Task::stream` starts with one) wakes
        // itself; poll it again until it is waiting on something real.
        for _ in 0..64 {
            woken.0.store(false, Ordering::SeqCst);
            match stream.as_mut().poll_next(&mut context) {
                Poll::Ready(Some(Action::Output(message))) => messages.push(message),
                // Widget operations, clipboard, window and system actions
                // belong to the host's toolkit; a guest has none.
                Poll::Ready(Some(_)) => {}
                Poll::Ready(None) => return false,
                Poll::Pending if woken.0.load(Ordering::SeqCst) => {}
                Poll::Pending => return true,
            }
        }
        true
    });
    messages
}

/// The `ice:view` world, as the exports are generated from it. The text is
/// repeated inside [`export_app!`] because a proc macro takes only a
/// literal; a test keeps the two identical.
pub const WIT: &str = include_str!("../wit/view.wit");

/// The most a panic message may carry across the `panicked` import. A host
/// shows one line of it, and every byte over that is one the host lifts out
/// of guest memory before it can refuse anything — so the message is cut
/// here, where the guest still owns it, on a char boundary.
pub const MAX_PANIC_BYTES: usize = 1024;

/// The line the panic hook hands the host: the payload and where it came
/// from, cut to [`MAX_PANIC_BYTES`].
pub fn panic_line(message: &str, at: &str) -> String {
    let mut line = format!("{message} at {at}");
    if line.len() > MAX_PANIC_BYTES {
        let cut = (0..=MAX_PANIC_BYTES)
            .rev()
            .find(|at| line.is_char_boundary(*at))
            .unwrap_or(0);
        line.truncate(cut);
    }
    line
}

pub const fn manifest_bytes<const N: usize>(text: &str) -> [u8; N] {
    let bytes = text.as_bytes();
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

/// Exports a generated Ice application as an `ice:view` component.
///
/// `$app` and `$message` are the names `include_app!` generated; `$name`
/// and `$description` are what the host lists; the capabilities are the
/// request kinds the app will make (`host.echo`, `clock.sleep`...), which
/// the host checks every request against. They land in the `ice.manifest`
/// custom section, readable without instantiating the module.
///
/// `boot_native` and `tick_native` drive the same app in an ordinary test.
#[macro_export]
macro_rules! export_app {
    ($app:ident, $message:ident, $name:expr, $description:expr, [$($capability:literal),* $(,)?]) => {
        struct __IceApp($app);

        impl $crate::App for __IceApp {
            type Message = $message;

            fn boot() -> (Self, ::iced::Task<Self::Message>) {
                let (app, boot) = <$app>::__boot();
                (Self(app), boot)
            }

            fn view(&self) -> $crate::wire::Node {
                self.0.__view()
            }

            fn update(&mut self, message: Self::Message) -> ::iced::Task<Self::Message> {
                self.0.__update(message)
            }
        }

        const __ICE_MANIFEST: &str = concat!($name, "\n", $description, "\n" $(, $capability, ",")*);

        #[unsafe(link_section = "ice.manifest")]
        #[used]
        static __ICE_MANIFEST_SECTION: [u8; __ICE_MANIFEST.len()] =
            $crate::manifest_bytes(__ICE_MANIFEST);

        thread_local! {
            static __ICE_DRIVER: ::std::cell::RefCell<Option<$crate::Driver<__IceApp>>> =
                const { ::std::cell::RefCell::new(None) };
        }

        pub fn boot_native() {
            __ICE_DRIVER.with(|driver| *driver.borrow_mut() = Some($crate::Driver::new()));
        }

        pub fn tick_native(events: Vec<$crate::wire::Event>) -> $crate::wire::Frame {
            __ICE_DRIVER.with(|driver| driver.borrow_mut().as_mut().expect("boot first").tick(events))
        }

        #[cfg(target_arch = "wasm32")]
        mod __ice_exports {
            $crate::wit_bindgen::generate!({
                inline: "package ice:view@0.1.0;

world view {
    import panicked: func(message: string);

    export init: func();
    export tick: func(events: list<u8>) -> list<u8>;
}
",
                runtime_path: "::ui_lang_guest::wit_bindgen::rt",
            });

            struct __IceComponent;

            impl Guest for __IceComponent {
                fn init() {
                    // A trapped instance can never be entered again, so the
                    // message leaves through the host's import before the
                    // abort that follows the hook.
                    ::std::panic::set_hook(::std::boxed::Box::new(|info| {
                        let payload = info.payload();
                        let message = payload
                            .downcast_ref::<&str>()
                            .copied()
                            .or_else(|| payload.downcast_ref::<::std::string::String>().map(|text| text.as_str()))
                            .unwrap_or("panicked");
                        let at = info
                            .location()
                            .map(|location| ::std::format!("{}:{}", location.file(), location.line()))
                            .unwrap_or_else(|| "unknown".into());
                        panicked(&$crate::panic_line(message, &at));
                    }));
                    super::boot_native();
                }

                fn tick(events: Vec<u8>) -> Vec<u8> {
                    let events: Vec<$crate::wire::Event> =
                        $crate::wire::decode(&events).unwrap_or_default();
                    let mut frame = super::tick_native(events);
                    // The host keeps the tree it has; only the rest crosses.
                    if frame.unchanged {
                        frame.root = None;
                    }
                    $crate::wire::encode(&frame)
                }
            }

            export!(__IceComponent);
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_macro_carries_the_wit_file_verbatim() {
        // The proc macro takes only a literal, so the world is spelled twice.
        let source = include_str!("lib.rs");
        let start = source.find("inline: \"").expect("inline wit") + "inline: \"".len();
        let end = source[start..].find("\",").expect("wit end") + start;
        let inline = &source[start..end];
        let file: String = super::WIT
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(inline.trim(), file.trim());
    }

    #[test]
    fn a_panic_message_crosses_bounded_and_on_a_char_boundary() {
        let line = super::panic_line(&"목".repeat(4096), "app.ice:1");
        assert!(line.len() <= super::MAX_PANIC_BYTES);
        assert!(line.len() > super::MAX_PANIC_BYTES - 4);
        // Truncated inside a character would not be a `String` at all.
        assert!(line.chars().all(|glyph| glyph == '목'));
    }

    #[test]
    fn a_short_panic_message_keeps_its_location() {
        assert_eq!(super::panic_line("boom", "app.ice:1"), "boom at app.ice:1");
    }
}

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

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

pub use ui_lang_wire as wire;
pub use wit_bindgen;

use iced_runtime::futures::BoxStream;
use iced_runtime::futures::futures::StreamExt;
use iced_runtime::futures::futures::channel::mpsc;
use iced_runtime::futures::subscription::{self, Tracker};
use iced_runtime::{Action, task};

#[cfg(feature = "authored-tests")]
pub mod authored;

mod clipboard;
pub mod keyboard;
mod markdown;
mod memo;
pub use markdown::Markdown;
pub use memo::memo_lazy;
pub mod host;
pub mod testing;
pub mod widget;
pub mod window;

pub use snapshot::SnapshotApp;
mod snapshot;

/// What `export_app!` needs from the generated application.
pub trait App: Sized + 'static {
    type Message: Clone + iced_runtime::futures::MaybeSend + 'static;
    fn boot() -> (Self, iced::Task<Self::Message>);
    fn view(&self) -> wire::Node;
    fn update(&mut self, message: Self::Message) -> iced::Task<Self::Message>;
    fn subscription(&self) -> iced::Subscription<Self::Message>;
}

/// The per-frame tables a view fills as it builds: a button's `on_press`
/// is the index its message took here, an input's `on_input` the index of
/// its `String -> Message` constructor, a checkbox's `on_toggle` that of a
/// `bool -> Message` one, a slider's `f32`, a pick list's `u32`, a
/// sensor's and a mouse area's `(f32, f32)` size or position, a mouse
/// area's `(f32, f32, bool)` scroll. The host
/// echoes an index back with the value; the driver looks the handler up in
/// the table of the frame it echoed and runs it.
///
/// The tables are untyped so the generated code can push through this
/// crate without naming the app's message type; the driver downcasts, by
/// argument and message type both, so an index the host sends with the
/// wrong kind of value finds nothing.
pub mod slots;

/// One running task: its stream, and the flag its waker sets. A task is
/// polled only when the flag is up — set at spawn, by a host answer
/// through [`host::fulfill`], by its own yield, or by another task in the
/// same pass — so a task waiting on the host costs a tick nothing.
struct Task<M> {
    /// Tracker runners restart from restored state; ordinary tasks must settle.
    subscription: bool,
    woken: Arc<Woken>,
    stream: BoxStream<Action<M>>,
}

/// One running app: its state, its in-flight tasks, the streams its
/// `subscribe` block keeps alive, and the last tree it sent so an identical
/// one crosses as `unchanged` and a changed one as patches against it.
pub struct Driver<A: App> {
    slots: slots::Context,
    window: iced::window::Id,
    app: A,
    tasks: Vec<Task<A::Message>>,
    /// Diffs the recipes `subscription` returns against the ones running,
    /// exactly as iced's own runtime does: a recipe that hashes the same
    /// keeps its stream, a new one is started, a missing one is dropped —
    /// and a dropped stream cancels whatever it asked the host for.
    tracker: Tracker,
    /// Where a subscription's stream puts what it produced; drained into
    /// `update` after every poll pass.
    subscribed: mpsc::Sender<A::Message>,
    produced: mpsc::Receiver<A::Message>,
    last_root: Option<wire::Node>,
    /// The last tick ran out of budget with work still ready: the frame
    /// asks the host for the next tick at once instead of waiting for an
    /// event or an answer that may never come.
    busy: bool,
}

impl<A: App> Default for Driver<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// How many `update` rounds one tick runs before it hands the rest to the
/// next: a handler that re-emits synchronously forever cannot pin the frame.
const MAX_ROUNDS: usize = 8;

/// How many times one poll pass revisits the tasks still woken.
const MAX_POLLS: usize = 64;

/// How many messages a subscription's streams may queue between two poll
/// passes before they wait for the drain.
const SUBSCRIPTION_QUEUE: usize = 100;

impl<A: App> Driver<A> {
    pub fn new() -> Self {
        Self::with_macos(cfg!(target_os = "macos"))
    }

    /// The host platform selects Command/word-jump semantics before app boot.
    pub fn with_macos(macos: bool) -> Self {
        Self::initialize(macos, || Ok(A::boot())).expect("ordinary boot is infallible")
    }

    fn initialize(
        macos: bool,
        boot: impl FnOnce() -> Result<(A, iced::Task<A::Message>), String>,
    ) -> Result<Self, String> {
        let slots = slots::Context::with_macos(macos);
        let _context = slots.enter();
        let (app, boot) = boot()?;
        let (subscribed, produced) = mpsc::channel(SUBSCRIPTION_QUEUE);
        let mut driver = Self {
            slots,
            window: iced::window::Id::unique(),
            app,
            tasks: Vec::new(),
            tracker: Tracker::new(),
            subscribed,
            produced,
            last_root: None,
            busy: false,
        };
        spawn(&mut driver.tasks, boot);
        Ok(driver)
    }

    /// Delivers the host's events and returns the frame they produced.
    ///
    /// Events name entries in the tables the LAST view filled, so they are
    /// dispatched before the tables are reset for this view. An index the
    /// last frame did not hand out (the host raced a rebuild) is dropped.
    ///
    /// The frame always carries the tree, `unchanged` or patched or not, so
    /// a test can read it; the component export drops a tree the host can
    /// rebuild before it crosses.
    pub fn tick(&mut self, events: Vec<wire::Event>) -> wire::Frame {
        let _context = self.slots.enter();
        for message in slots::take_deferred::<A::Message>() {
            spawn(&mut self.tasks, self.app.update(message));
            self.settle();
        }
        self.settle();
        for event in events {
            let message = match event {
                wire::Event::Keyboard { event, captured } => {
                    self.tracker.broadcast(subscription::Event::Interaction {
                        window: self.window,
                        event: iced::Event::Keyboard(event.into()),
                        status: if captured {
                            iced::event::Status::Captured
                        } else {
                            iced::event::Status::Ignored
                        },
                    });
                    // Drain before the next key: Tracker channels are bounded,
                    // and a handler can change which subscriptions are live.
                    self.settle();
                    None
                }
                wire::Event::Message(index) => slots::take_message::<A::Message>(index),
                wire::Event::Surface { handler, value } => {
                    slots::run_handler::<wire::SurfaceValue, A::Message>(handler, value)
                }
                wire::Event::Input { handler, text } => {
                    slots::run_handler::<String, A::Message>(handler, text)
                }
                wire::Event::Edit { handler, text } => {
                    slots::run_handler::<String, A::Message>(handler, text)
                }
                wire::Event::Toggle { handler, on } => {
                    slots::run_handler::<bool, A::Message>(handler, on)
                }
                wire::Event::Slide { handler, value } => {
                    slots::run_handler::<f32, A::Message>(handler, value)
                }
                wire::Event::Select { handler, index } => {
                    slots::run_handler::<u32, A::Message>(handler, index)
                }
                wire::Event::Size {
                    handler,
                    width,
                    height,
                } => slots::run_handler::<(f32, f32), A::Message>(handler, (width, height)),
                wire::Event::Pointer { handler, x, y } => {
                    slots::run_handler::<(f32, f32), A::Message>(handler, (x, y))
                }
                wire::Event::Scroll {
                    handler,
                    dx,
                    dy,
                    pixels,
                } => slots::run_handler::<(f32, f32, bool), A::Message>(handler, (dx, dy, pixels)),
                wire::Event::ScrollOffset {
                    handler,
                    x,
                    y,
                    relative_x,
                    relative_y,
                } => slots::run_handler::<(f32, f32, f32, f32), A::Message>(
                    handler,
                    (x, y, relative_x, relative_y),
                ),
                wire::Event::Response { id, result, done } => {
                    host::fulfill(id, result, done);
                    None
                }
                // The host dropped the tree the patches build on.
                wire::Event::Resync => {
                    self.last_root = None;
                    None
                }
            };
            if let Some(message) = message {
                spawn(&mut self.tasks, self.app.update(message));
                self.settle();
            }
        }
        // A response woke a task without a message of its own to run: poll
        // once more so what it produced reaches `update` before the view.
        self.settle();
        slots::reset();
        let mut root = self.app.view();
        // Synchronous mount pruning can cancel work after the last settle.
        // Reconcile subscriptions and request another tick to drain woken
        // tasks; updating here would publish a tree from before that update.
        self.subscribe();
        self.busy |= slots::has_deferred()
            || self
                .tasks
                .iter()
                .any(|task| task.woken.0.load(Ordering::Relaxed));
        let unchanged = self.last_root.as_ref() == Some(&root);
        let mut patches = Vec::new();
        if !unchanged {
            // Patches against the last tree, unless there is none — a first
            // frame, or one after the host asked to resync — or the patches
            // would cross bigger than the tree itself.
            if let Some(last) = &mut self.last_root {
                patches = wire::diff(last, &mut root);
                if patches.len() > wire::MAX_PATCHES
                    || wire::encoded_size(&patches) >= wire::encoded_size(&root)
                {
                    patches.clear();
                }
            }
            // Remembered without the picture bytes this frame carried: the
            // next view names those pictures by hash alone, and that is
            // the same tree — and the tree the host keeps, which drops the
            // bytes the same way once it has the pictures.
            let mut kept = root.clone();
            kept.for_each_mut(&mut |node| match node {
                wire::Node::Svg { bytes, .. } => *bytes = None,
                wire::Node::Image { data, .. } | wire::Node::ImageViewer { data, .. } => {
                    *data = None
                }
                _ => {}
            });
            self.last_root = Some(kept);
        }
        wire::Frame {
            root: Some(root),
            patches,
            requests: host::drain_outbox(),
            cancels: host::drain_cancels(),
            unchanged,
            busy: self.busy,
        }
    }

    /// Brings the subscription in line with the state, polls every woken
    /// task, and runs what they produced through `update` — whose own tasks
    /// join the pool and whose state changes move the subscription — until
    /// a round produces nothing or the round budget is spent. Spent with
    /// work still ready is what `busy` means.
    fn settle(&mut self) {
        for _ in 0..MAX_ROUNDS {
            self.subscribe();
            let (messages, cut_short) = poll_tasks(&mut self.tasks, &mut self.produced);
            self.busy = cut_short;
            if messages.is_empty() {
                return;
            }
            for message in messages {
                spawn(&mut self.tasks, self.app.update(message));
            }
        }
        self.busy = true;
    }

    fn subscribe(&mut self) {
        let recipes = subscription::into_recipes(self.app.subscription());
        for future in self
            .tracker
            .update(recipes.into_iter(), self.subscribed.clone())
        {
            // A subscription's stream feeds the channel, so as a task it
            // produces nothing itself: it is in the pool to be polled.
            self.tasks.push(Task {
                subscription: true,
                woken: Arc::new(Woken(AtomicBool::new(true))),
                stream: iced_runtime::futures::boxed_stream(
                    iced_runtime::futures::futures::stream::once(future)
                        .filter_map(|()| std::future::ready(None)),
                ),
            });
        }
    }
}

fn spawn<M: iced_runtime::futures::MaybeSend + 'static>(
    tasks: &mut Vec<Task<M>>,
    task: iced::Task<M>,
) {
    if let Some(stream) = task::into_stream(task) {
        tasks.push(Task {
            subscription: false,
            woken: Arc::new(Woken(AtomicBool::new(true))),
            stream,
        });
    }
}

struct Woken(AtomicBool);

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Polls every woken task once per pass, and passes again while something
/// was woken — by its own yield (every `Task::stream` starts with one), or
/// by a task polled earlier in the pass — until nothing is or the pass
/// budget is spent. Returns the messages the tasks output and whether the
/// budget ran out with a task still woken.
fn poll_tasks<M: iced_runtime::futures::MaybeSend + 'static>(
    tasks: &mut Vec<Task<M>>,
    produced: &mut mpsc::Receiver<M>,
) -> (Vec<M>, bool) {
    let mut messages = Vec::new();
    for _ in 0..MAX_POLLS {
        let mut polled = false;
        let mut effects = Vec::new();
        tasks.retain_mut(|task| {
            if !task.woken.0.swap(false, Ordering::SeqCst) {
                return true;
            }
            polled = true;
            let waker = Waker::from(task.woken.clone());
            let mut context = Context::from_waker(&waker);
            match task.stream.as_mut().poll_next(&mut context) {
                Poll::Ready(Some(action)) => {
                    match action {
                        Action::Output(message) => messages.push(message),
                        Action::Clipboard(action) => effects.push(clipboard::run(action)),
                        other => host::log(format!(
                            "dropped a {}: a guest has no toolkit to run it",
                            describe(&other)
                        )),
                    }
                    // Ready is not done: it is polled again next pass, and
                    // the pass budget is what bounds a stream that is
                    // always ready.
                    task.woken.0.store(true, Ordering::SeqCst);
                    true
                }
                Poll::Ready(None) => false,
                Poll::Pending => true,
            }
        });
        for effect in effects {
            spawn(tasks, effect);
        }
        // What the subscriptions' streams put in the channel during the pass.
        while let Ok(message) = produced.try_recv() {
            messages.push(message);
        }
        if !polled {
            return (messages, false);
        }
    }
    (
        messages,
        tasks.iter().any(|task| task.woken.0.load(Ordering::SeqCst)),
    )
}

/// What a task asked for that only a toolkit can do. Named for the log,
/// since the app's message type carries no `Debug`.
fn describe<M>(action: &Action<M>) -> &'static str {
    match action {
        Action::Output(_) => "message",
        Action::LoadFont { .. } => "font load",
        Action::Widget(_) => "widget operation",
        Action::Clipboard(_) => "clipboard action",
        Action::Window(_) => "window action",
        Action::System(_) => "system action",
        Action::Image(_) => "image action",
        Action::Reload => "reload",
        Action::Exit => "exit",
    }
}

/// An Ice `every` in a guest: a module has no clock, so the period is the
/// host's `clock.ticks` — which the app's manifest must declare `clock`
/// for. The route carries no instant, because a guest cannot make one.
/// A refusal is logged and ends the stream; the recipe hashes by period,
/// so a `subscribe` that keeps the same `every` keeps the same ticker.
pub fn every(period: Duration) -> iced::Subscription<()> {
    iced::Subscription::run_with(period, |period| ticks(*period))
}

/// An Ice `repeat f() every d` in a guest: `f` at once, then once per host
/// tick.
pub fn repeat<F, T>(f: fn() -> F, period: Duration) -> iced::Subscription<T>
where
    F: Future<Output = T> + iced_runtime::futures::MaybeSend + 'static,
    T: iced_runtime::futures::MaybeSend + 'static,
{
    iced::Subscription::run_with((f, period), |(f, period)| {
        let f = *f;
        iced_runtime::futures::boxed_stream(
            iced_runtime::futures::futures::stream::once(std::future::ready(()))
                .chain(ticks(*period))
                .then(move |()| f()),
        )
    })
}

fn ticks(period: Duration) -> BoxStream<()> {
    let millis = i64::try_from(period.as_millis()).unwrap_or(i64::MAX);
    iced_runtime::futures::boxed_stream(
        host::subscribe("clock.ticks", &millis.to_le_bytes()).filter_map(|answer| {
            std::future::ready(match answer {
                Ok(_) => Some(()),
                Err(message) => {
                    host::log(format!("`every` needs the host's clock: {message}"));
                    None
                }
            })
        }),
    )
}

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

/// Concatenates the versioned manifest prefix and generated window size at compile time.
pub const fn manifest_bytes<const N: usize>(text: &str, preferred_size: &str) -> [u8; N] {
    let bytes = text.as_bytes();
    let size = preferred_size.as_bytes();
    assert!(N == bytes.len() + size.len());
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = if i < bytes.len() {
            bytes[i]
        } else {
            size[i - bytes.len()]
        };
        i += 1;
    }
    out
}

/// Exports a generated Ice application as an `ice:view` component.
///
/// `$app` is the struct `include_app!` generated (its message enum is
/// reached through the `__IceMessage` alias the `tree` target emits);
/// `$name` and `$description` are what the host lists; the capabilities are the
/// request kinds the app will make (`host.echo`, `clock.sleep`...), which
/// the host checks every request against. They land in the `ice.manifest`
/// custom section, followed by the generated primary window size in the strict
/// five-line `ice.manifest.v1` format, readable without instantiating the module.
///
/// `boot_native` and `tick_native` drive the same app in an ordinary test.
#[macro_export]
macro_rules! export_app {
    ($app:ident, $name:expr, $description:expr, [$($capability:literal),* $(,)?]) => {
        $crate::__export_app!(production, $app, $name, $description, [$($capability),*]);
    };
}

/// Export an explicitly built authored-test artifact. Never install this package
/// in a production catalog; its manifest and protocol deliberately differ.
#[cfg(feature = "authored-tests")]
#[macro_export]
macro_rules! export_test_app {
    ($app:ident, $name:expr, $description:expr, [$($capability:literal),* $(,)?]) => {
        $crate::__export_app!(test, $app, $name, $description, [$($capability),*]);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __export_app {
    ($mode:ident, $app:ident, $name:expr, $description:expr, [$($capability:literal),* $(,)?]) => {
        struct __IceApp($app);
        $crate::__test_app_impl!($mode, __IceApp, $app);

        impl $crate::App for __IceApp {
            type Message = __IceMessage;

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

            fn subscription(&self) -> ::iced::Subscription<Self::Message> {
                self.0.__subscription()
            }
        }

        impl $crate::SnapshotApp for __IceApp {
            fn snapshot(&self) -> ::std::result::Result<::std::vec::Vec<u8>, ::std::string::String> { self.0.__snapshot() }
            fn restore(bytes: &[u8]) -> ::std::result::Result<Self, ::std::string::String> { <$app>::__restore(bytes).map(Self) }
        }

        const __ICE_MANIFEST: &str = concat!($crate::__manifest_header!($mode), $name, "\n", $description, "\n" $(, $capability, ",")*, "\n");

        #[unsafe(link_section = "ice.manifest")]
        #[used]
        static __ICE_MANIFEST_SECTION: [u8; __ICE_MANIFEST.len() + <$app>::__PREFERRED_WINDOW_SIZE.len()] =
            $crate::manifest_bytes(__ICE_MANIFEST, <$app>::__PREFERRED_WINDOW_SIZE);

        thread_local! {
            static __ICE_DRIVER: ::std::cell::RefCell<Option<$crate::Driver<__IceApp>>> =
                const { ::std::cell::RefCell::new(None) };
        }

        #[cfg(not(target_arch = "wasm32"))]
        pub fn run_native() -> ::std::result::Result<(), ::std::string::String> {
            $crate::__native_entry!($mode, __IceApp, __ICE_MANIFEST_SECTION)
        }

        pub fn boot_native() {
            __ICE_DRIVER.with(|driver| *driver.borrow_mut() = Some($crate::Driver::new()));
        }

        pub fn snapshot_native() -> ::std::result::Result<::std::vec::Vec<u8>, ::std::string::String> {
            __ICE_DRIVER.with(|driver| driver.borrow().as_ref().ok_or_else(|| ::std::string::String::from("initialize first"))?.snapshot())
        }

        pub fn restore_native(bytes: &[u8], macos: bool) -> ::std::result::Result<(), ::std::string::String> {
            let candidate = $crate::Driver::from_snapshot(bytes, macos)?;
            __ICE_DRIVER.with(|driver| *driver.borrow_mut() = Some(candidate));
            ::std::result::Result::Ok(())
        }

        pub fn tick_native(events: Vec<$crate::wire::Event>) -> $crate::wire::Frame {
            __ICE_DRIVER.with(|driver| driver.borrow_mut().as_mut().expect("boot first").tick(events))
        }

        #[cfg(target_arch = "wasm32")]
        mod __ice_exports {
            macro_rules! bindings {
                ($wit:literal) => {
                    $crate::wit_bindgen::generate!({
                        inline: $wit,
                        runtime_path: "::ui_lang_guest::wit_bindgen::rt",
                    });
                };
            }
            $crate::__export_bindings!($mode, bindings);

            struct __IceComponent;

            fn install_panic_hook() {
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
            }

            impl Guest for __IceComponent {
                $crate::__test_export!($mode, __ICE_DRIVER);
                fn init(macos: bool) {
                    install_panic_hook();
                    super::__ICE_DRIVER.with(|driver| *driver.borrow_mut() = Some($crate::Driver::with_macos(macos)));
                }

                fn snapshot() -> Result<Vec<u8>, String> { super::snapshot_native() }
                fn restore(state: Vec<u8>, macos: bool) -> Result<(), String> {
                    install_panic_hook();
                    super::restore_native(&state, macos)
                }

                fn tick(events: Vec<u8>) -> Vec<u8> {
                    let events: Vec<$crate::wire::Event> =
                        $crate::wire::decode(&events).unwrap_or_default();
                    let mut frame = super::tick_native(events);
                    // The host keeps the tree it has, or patches it; the
                    // whole tree crosses only when neither will do.
                    if frame.unchanged || !frame.patches.is_empty() {
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
    use super::*;

    struct ViewBoot {
        seen: std::cell::Cell<bool>,
        count: u32,
    }

    impl App for ViewBoot {
        type Message = u32;
        fn boot() -> (Self, iced::Task<u32>) {
            (
                Self {
                    seen: false.into(),
                    count: 0,
                },
                iced::Task::none(),
            )
        }
        fn view(&self) -> wire::Node {
            if !self.seen.replace(true) {
                slots::defer(vec![10u32]);
            }
            slots::message(1u32);
            wire::Node::empty()
        }
        fn update(&mut self, value: u32) -> iced::Task<u32> {
            // The external event must observe initialization already applied.
            if value == 1 {
                assert_eq!(self.count, 10);
            }
            self.count += value;
            iced::Task::none()
        }
        fn subscription(&self) -> iced::Subscription<u32> {
            iced::Subscription::none()
        }
    }

    #[test]
    fn view_boots_wake_the_host_and_stay_with_their_driver() {
        let mut first = Driver::<ViewBoot>::default();
        assert!(first.tick(vec![]).busy);
        assert_eq!(first.app.count, 0, "rendering never runs update");
        let mut other = Driver::<ViewBoot>::default();
        assert!(other.tick(vec![]).busy);
        assert!(!first.tick(vec![wire::Event::Message(0)]).busy);
        assert_eq!(first.app.count, 11);
        assert_eq!(other.app.count, 0, "another driver's queue is untouched");
        assert!(!other.tick(vec![]).busy);
        assert_eq!(other.app.count, 10);
        first.tick(vec![]);
        assert_eq!(first.app.count, 11, "boots are drained once");
    }

    /// Boots with a task that outputs `count` messages at once and remembers
    /// how many `update` saw.
    struct Burst(u32);

    thread_local! {
        static BURST: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }

    impl App for Burst {
        type Message = u32;

        fn boot() -> (Self, iced::Task<u32>) {
            let count = BURST.get();
            (
                Self(0),
                iced::Task::stream(iced_runtime::futures::futures::stream::iter(0..count)),
            )
        }

        fn view(&self) -> wire::Node {
            wire::Node::empty()
        }

        fn update(&mut self, _: u32) -> iced::Task<u32> {
            self.0 += 1;
            iced::Task::none()
        }

        fn subscription(&self) -> iced::Subscription<u32> {
            iced::Subscription::none()
        }
    }

    /// A list of labelled rows, one of them marked; the message for a row
    /// marks it.
    struct Marked(u32);

    impl App for Marked {
        type Message = u32;

        fn boot() -> (Self, iced::Task<u32>) {
            (Self(0), iced::Task::none())
        }

        fn view(&self) -> wire::Node {
            wire::Node::Linear {
                max_width: None,
                clip: false,
                wrap: None,
                key: "App/list".into(),
                axis: wire::Axis::Column,
                spacing: None,
                padding: None,
                width: None,
                height: None,
                align: None,
                background: None,
                border: None,
                children: (0..40)
                    .map(|row| wire::Node::Button {
                        checked: None,
                        expanded: None,
                        description: None,
                        key: format!("App/list/{row}"),
                        content: wire::ButtonContent::Label(match row == self.0 {
                            true => format!("row {row} *"),
                            false => format!("row {row}"),
                        }),
                        label: None,
                        on_press: Some(slots::message::<u32>(row)),
                        width: None,
                        height: None,
                        padding: None,
                        style: wire::ButtonStyle::default(),
                    })
                    .collect(),
            }
        }

        fn update(&mut self, row: u32) -> iced::Task<u32> {
            self.0 = row;
            iced::Task::none()
        }

        fn subscription(&self) -> iced::Subscription<u32> {
            iced::Subscription::none()
        }
    }

    /// The first frame is whole; a change is patches that turn the last
    /// tree into the new one and cross smaller than it; the same tree again
    /// is `unchanged`; and after the host asks to resync, the next frame is
    /// whole again.
    #[test]
    fn a_changed_tree_crosses_as_patches_until_the_host_asks_for_it_whole() {
        let mut driver = Driver::<Marked>::new();
        let first = driver.tick(Vec::new());
        assert!(first.patches.is_empty() && !first.unchanged);
        let mut held = first.root.clone().unwrap();

        let second = driver.tick(vec![wire::Event::Message(7)]);
        assert!(!second.unchanged);
        assert_eq!(second.patches.len(), 2, "{:?}", second.patches);
        assert!(
            wire::encoded_size(&second.patches) < wire::encoded_size(&second.root),
            "the patches are the smaller encoding"
        );
        wire::apply(&mut held, second.patches.clone()).unwrap();
        assert_eq!(Some(held), second.root);

        let third = driver.tick(Vec::new());
        assert!(third.unchanged && third.patches.is_empty());

        let fourth = driver.tick(vec![wire::Event::Resync]);
        assert!(!fourth.unchanged && fourth.patches.is_empty());
        assert!(fourth.root.is_some());
    }

    #[test]
    fn a_task_the_tick_budget_cuts_short_marks_the_frame_busy_until_it_is_done() {
        BURST.set(10_000);
        let mut driver = Driver::<Burst>::new();
        let first = driver.tick(Vec::new());
        assert!(first.busy, "work was still ready when the budget ran out");
        assert!(driver.app.0 < 10_000, "{}", driver.app.0);
        let mut ticks = 1;
        while driver.tick(Vec::new()).busy {
            ticks += 1;
            assert!(ticks < 100, "the burst never drains");
        }
        assert_eq!(driver.app.0, 10_000);
        assert!(ticks > 1, "the burst was not one tick's work");
    }

    #[test]
    fn a_task_that_fits_the_budget_leaves_the_frame_quiet() {
        BURST.set(3);
        let mut driver = Driver::<Burst>::new();
        let frame = driver.tick(Vec::new());
        assert!(!frame.busy);
        assert_eq!(driver.app.0, 3);
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

#[cfg(test)]
mod driver_routes_tests {
    use super::*;

    struct Next(u32);
    impl App for Next {
        type Message = u32;
        fn boot() -> (Self, iced::Task<u32>) {
            (Self(0), iced::Task::none())
        }
        fn view(&self) -> wire::Node {
            wire::Node::Button {
                key: "next".into(),
                content: wire::ButtonContent::Label(self.0.to_string()),
                label: None,
                checked: None,
                expanded: None,
                description: None,
                on_press: Some(slots::message(self.0 + 1)),
                width: None,
                height: None,
                padding: None,
                style: wire::ButtonStyle::default(),
            }
        }
        fn update(&mut self, message: u32) -> iced::Task<u32> {
            self.0 = message;
            iced::Task::none()
        }
        fn subscription(&self) -> iced::Subscription<u32> {
            iced::Subscription::none()
        }
    }

    #[test]
    fn two_drivers_keep_their_own_rendered_routes() {
        let mut first = Driver::<Next>::new();
        first.tick(vec![]);
        let first_frame = first.tick(vec![wire::Event::Message(0)]);
        assert!(testing::has_text(&first_frame, "1"));
        let mut second = Driver::<Next>::new();
        second.tick(vec![]);
        let first_frame = first.tick(vec![wire::Event::Message(0)]);
        assert!(
            testing::has_text(&first_frame, "2"),
            "the first driver's next route must survive a second driver's render"
        );
    }
}

mod combo;
pub use combo::Combo;

#[doc(hidden)]
#[macro_export]
macro_rules! __manifest_header {
    (production) => {
        "ice.manifest.v1\n"
    };
    (test) => {
        "ice.test.manifest.v1\n"
    };
}
#[doc(hidden)]
#[macro_export]
macro_rules! __native_entry {
    (production, $app:ident, $manifest:ident) => {
        $crate::native::run::<$app>(&$manifest)
    };
    (test, $app:ident, $manifest:ident) => {
        $crate::authored::run::<$app>(&$manifest)
    };
}
#[doc(hidden)]
#[macro_export]
macro_rules! __export_bindings {
    (production, $callback:ident) => {
        $crate::wire::with_view_wit!($callback);
    };
    (test, $callback:ident) => {
        $crate::wire::with_test_view_wit!($callback);
    };
}
#[doc(hidden)]
#[macro_export]
macro_rules! __test_app_impl {
    (production, $wrapper:ident, $app:ident) => {};
    (test, $wrapper:ident, $app:ident) => {
        impl $crate::authored::TestApp for $wrapper {
            const FINGERPRINT: u64 = $app::__ICE_TEST_FINGERPRINT;
            fn test_boot(
                test: u32,
            ) -> ::std::result::Result<(Self, ::iced::Task<Self::Message>), ::std::string::String>
            {
                $app::__ice_test_boot(test).map(|(app, task)| (Self(app), task))
            }
            fn test_step(
                &self,
                test: u32,
                step: u32,
            ) -> ::std::result::Result<::std::option::Option<Self::Message>, ::std::string::String>
            {
                self.0.__ice_test_step(test, step)
            }
        }
    };
}
#[doc(hidden)]
#[macro_export]
macro_rules! __test_export {
    (production, $driver:ident) => {};
    (test, $driver:ident) => {
        fn authored(
            command: ::std::vec::Vec<u8>,
        ) -> ::std::result::Result<::std::vec::Vec<u8>, ::std::string::String> {
            let request = $crate::wire::decode(&command)?;
            super::$driver
                .with(|driver| $crate::authored::respond(&mut driver.borrow_mut(), request))
        }
    };
}

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
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

pub use ui_lang_wire as wire;
pub use wit_bindgen;

use iced_runtime::futures::BoxStream;
use iced_runtime::futures::futures::StreamExt;
use iced_runtime::futures::futures::channel::mpsc;
use iced_runtime::futures::subscription::{self, Tracker};
use iced_runtime::{Action, task};

pub mod host;
pub mod testing;

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
/// `bool -> Message` one, a slider's `f32`, a pick list's `u32`. The host
/// echoes an index back with the value; the driver looks the handler up in
/// the table of the frame it echoed and runs it.
///
/// The tables are untyped so the generated code can push through this
/// crate without naming the app's message type; the driver downcasts, by
/// argument and message type both, so an index the host sends with the
/// wrong kind of value finds nothing.
pub mod slots {
    use super::*;

    thread_local! {
        static MESSAGES: RefCell<Vec<Box<dyn Any>>> = const { RefCell::new(Vec::new()) };
        static HANDLERS: RefCell<Vec<Box<dyn Any>>> = const { RefCell::new(Vec::new()) };
        /// Every picture hash a frame has carried the bytes for. Not a
        /// per-frame table: the host keeps a picture for the guest's life,
        /// so the bytes cross once and the hash stands for them after.
        static PICTURES: RefCell<std::collections::HashSet<u64>> =
            RefCell::new(std::collections::HashSet::new());
    }

    /// What a [`wire::Node::Svg`] carries for `bytes`: the picture's hash,
    /// and the picture itself the first time this guest shows it. The hash
    /// is the process's default hasher over the bytes — an opaque key the
    /// host never recomputes, consistent for as long as the guest runs.
    pub fn picture(bytes: impl AsRef<[u8]>) -> (u64, Option<Vec<u8>>) {
        use std::hash::{Hash, Hasher};
        let bytes = bytes.as_ref();
        let mut hasher = std::hash::DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        let first = PICTURES.with_borrow_mut(|sent| sent.insert(hash));
        (hash, first.then(|| bytes.to_vec()))
    }

    pub fn message<M: 'static>(message: M) -> u32 {
        MESSAGES.with_borrow_mut(|table| {
            table.push(Box::new(message));
            (table.len() - 1) as u32
        })
    }

    /// A handler returns `None` for a value it has no message for — a
    /// pick list index past its options — and the event is dropped.
    pub fn handler<A: 'static, M: 'static>(handler: Box<dyn Fn(A) -> Option<M>>) -> u32 {
        HANDLERS.with_borrow_mut(|table| {
            table.push(Box::new(handler));
            (table.len() - 1) as u32
        })
    }

    pub(crate) fn reset() {
        MESSAGES.with_borrow_mut(Vec::clear);
        HANDLERS.with_borrow_mut(Vec::clear);
    }

    /// A new driver faces a host that has seen nothing.
    pub(crate) fn forget_pictures() {
        PICTURES.with_borrow_mut(std::collections::HashSet::clear);
    }

    pub(crate) fn take_message<M: Clone + 'static>(index: u32) -> Option<M> {
        MESSAGES.with_borrow(|table| {
            table
                .get(index as usize)
                .and_then(|entry| entry.downcast_ref::<M>())
                .cloned()
        })
    }

    pub(crate) fn run_handler<A: 'static, M: 'static>(index: u32, value: A) -> Option<M> {
        HANDLERS.with_borrow(|table| {
            table
                .get(index as usize)
                .and_then(|entry| entry.downcast_ref::<Box<dyn Fn(A) -> Option<M>>>())
                .and_then(|handler| handler(value))
        })
    }
}

/// One running task: its stream, and the flag its waker sets. A task is
/// polled only when the flag is up — set at spawn, by a host answer
/// through [`host::fulfill`], by its own yield, or by another task in the
/// same pass — so a task waiting on the host costs a tick nothing.
struct Task<M> {
    woken: Arc<Woken>,
    stream: BoxStream<Action<M>>,
}

/// One running app: its state, its in-flight tasks, the streams its
/// `subscribe` block keeps alive, and the last tree it sent so an identical
/// one crosses as `unchanged` and a changed one as patches against it.
pub struct Driver<A: App> {
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
        slots::forget_pictures();
        let (app, boot) = A::boot();
        let (subscribed, produced) = mpsc::channel(SUBSCRIPTION_QUEUE);
        let mut driver = Self {
            app,
            tasks: Vec::new(),
            tracker: Tracker::new(),
            subscribed,
            produced,
            last_root: None,
            busy: false,
        };
        spawn(&mut driver.tasks, boot);
        driver
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
        self.settle();
        for event in events {
            let message = match event {
                wire::Event::Message(index) => slots::take_message::<A::Message>(index),
                wire::Event::Input { handler, text } => {
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
            kept.for_each_mut(&mut |node| {
                if let wire::Node::Svg { bytes, .. } = node {
                    *bytes = None;
                }
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
fn poll_tasks<M>(tasks: &mut Vec<Task<M>>, produced: &mut mpsc::Receiver<M>) -> (Vec<M>, bool) {
    let mut messages = Vec::new();
    for _ in 0..MAX_POLLS {
        let mut polled = false;
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
/// `$app` is the struct `include_app!` generated (its message enum is
/// reached through the `__IceMessage` alias the `tree` target emits);
/// `$name` and `$description` are what the host lists; the capabilities are the
/// request kinds the app will make (`host.echo`, `clock.sleep`...), which
/// the host checks every request against. They land in the `ice.manifest`
/// custom section, readable without instantiating the module.
///
/// `boot_native` and `tick_native` drive the same app in an ordinary test.
#[macro_export]
macro_rules! export_app {
    ($app:ident, $name:expr, $description:expr, [$($capability:literal),* $(,)?]) => {
        struct __IceApp($app);

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
                key: "App/list".into(),
                axis: wire::Axis::Column,
                spacing: None,
                padding: None,
                width: None,
                height: None,
                align: None,
                children: (0..40)
                    .map(|row| wire::Node::Button {
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

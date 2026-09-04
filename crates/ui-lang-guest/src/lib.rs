//! What an Ice app needs to run inside wasm: a headless [`Driver`] around the
//! generated application, and [`export_app!`], which turns an ordinary
//! `include_app!` into a wasm module the store can install.
//!
//! The module's ABI is four C exports — `init`, `input_ptr`, `tick`,
//! `output_ptr` — plus an `ice.manifest` custom section carrying the name,
//! the description and the capabilities the app needs, so a catalog can
//! list an app (and say what it will touch) without instantiating it.
//!
//! Two more exports exist only in wasm: `panic_ptr` and `panic_len` bound
//! the module's last panic message (`<payload> at <file>:<line>`, empty
//! until one happens). With `panic = "abort"` the trap the host sees is a
//! bare `unreachable`, so after a trap it reads that buffer for the reason.
//!
//! An app's `task`s run: the driver polls them on every tick, and anything
//! they need from outside goes through [`host::request`] / [`host::subscribe`]
//! and comes back as response events. There is no executor thread and no
//! clock inside the module — only what the host sends in.

pub use driver::Driver;
pub use ui_lang_wire as frame;

mod driver;
pub mod host;
pub mod testing;

/// The software renderer by name, not `iced::Renderer`: an app's workspace
/// enables tiny-skia alone, so there the two are one type, while this crate
/// also builds in a workspace where wgpu turns `iced::Renderer` into the
/// fallback enum.
pub type Renderer = iced_tiny_skia::Renderer;
pub type Element<'a, Message> = iced::Element<'a, Message, iced::Theme, Renderer>;

/// The generated application, seen from the driver.
pub trait WasmApp: Sized + 'static {
    type Message: Clone + iced_runtime::futures::MaybeSend + 'static;
    const NAME: &'static str;
    const DESCRIPTION: &'static str;

    fn boot() -> (Self, iced::Task<Self::Message>);
    fn view(&self) -> Element<'_, Self::Message>;
    fn update(&mut self, message: Self::Message) -> iced::Task<Self::Message>;
    fn theme(&self) -> iced::Theme;
}

/// Copies a manifest string into a fixed array at compile time, which is the
/// form a `link_section` static must take.
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

/// Implements [`WasmApp`] for an `include_app!` application and emits the
/// wasm exports and manifest section. `$message` is the generated message
/// enum (`__<App>Message`); the list names the capabilities the app's
/// requests will use (`clock`, `storage`, `bus`), which the host shows in the
/// catalog and enforces.
#[macro_export]
macro_rules! export_app {
    ($app:ident, $message:ident, $name:expr, $description:expr, [$($capability:literal),* $(,)?]) => {
        /// A private newtype, so the generated message enum — private to the
        /// crate — never appears in a public interface.
        struct __IceApp($app);

        impl $crate::WasmApp for __IceApp {
            type Message = $message;
            const NAME: &'static str = $name;
            const DESCRIPTION: &'static str = $description;

            fn boot() -> (Self, ::iced::Task<Self::Message>) {
                let (app, boot) = <$app>::__boot();
                (Self(app), boot)
            }

            fn view(&self) -> $crate::Element<'_, Self::Message> {
                self.0.__view()
            }

            fn update(&mut self, message: Self::Message) -> ::iced::Task<Self::Message> {
                self.0.__update(message)
            }

            fn theme(&self) -> ::iced::Theme {
                self.0.__theme()
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
            static __ICE_INPUT: ::std::cell::RefCell<Vec<u8>> = const { ::std::cell::RefCell::new(Vec::new()) };
            static __ICE_OUTPUT: ::std::cell::RefCell<Vec<u8>> = const { ::std::cell::RefCell::new(Vec::new()) };
        }

        /// Boots the app. Also the native entry point for tests.
        pub fn boot_native() {
            __ICE_DRIVER.with(|driver| *driver.borrow_mut() = Some($crate::Driver::new()));
        }

        /// One frame, natively: the same tick the wasm export runs.
        pub fn tick_native(events: Vec<$crate::frame::Event>) -> $crate::frame::Frame {
            __ICE_DRIVER.with(|driver| driver.borrow_mut().as_mut().expect("boot first").tick(events))
        }

        /// The last panic message, for a host holding a trapped instance.
        #[cfg(target_arch = "wasm32")]
        thread_local! {
            static __ICE_PANIC: ::std::cell::RefCell<::std::string::String> =
                const { ::std::cell::RefCell::new(::std::string::String::new()) };
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn panic_ptr() -> u32 {
            __ICE_PANIC.with(|text| text.borrow().as_ptr() as u32)
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn panic_len() -> u32 {
            __ICE_PANIC.with(|text| text.borrow().len() as u32)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn init() {
            // Only in wasm: natively this would hijack the test harness's
            // own hook, which is what reports a failing test.
            #[cfg(target_arch = "wasm32")]
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
                __ICE_PANIC.with(|text| *text.borrow_mut() = ::std::format!("{message} at {at}"));
            }));
            boot_native();
        }

        /// Reserves `len` bytes for the next event batch and returns where to write them.
        #[unsafe(no_mangle)]
        pub extern "C" fn input_ptr(len: u32) -> u32 {
            __ICE_INPUT.with(|input| {
                let mut input = input.borrow_mut();
                input.clear();
                input.resize(len as usize, 0);
                input.as_mut_ptr() as u32
            })
        }

        /// Applies the `len` bytes of events in the input buffer, draws a frame
        /// into the output buffer, and returns the frame's length.
        #[unsafe(no_mangle)]
        pub extern "C" fn tick(len: u32) -> u32 {
            // A batch that does not decode becomes no events: the host is
            // trusted to write the buffer it just asked for.
            let events: Vec<$crate::frame::Event> = __ICE_INPUT
                .with(|input| $crate::frame::decode(&input.borrow()[..len as usize]))
                .unwrap_or_default();
            let bytes = $crate::frame::encode(&tick_native(events));
            let len = bytes.len() as u32;
            __ICE_OUTPUT.with(|output| *output.borrow_mut() = bytes);
            len
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn output_ptr() -> u32 {
            __ICE_OUTPUT.with(|output| output.borrow().as_ptr() as u32)
        }
    };
}

#[cfg(not(feature = "cef"))]
const FEATURE_HINT: &str =
    "CEF support is disabled; run the example with `--features cef` or use the bundler";

#[derive(Debug, Clone)]
pub struct AttachResult {
    pub attached: bool,
    pub status: String,
}

pub fn normalize_url(value: &str) -> String {
    let value = value.trim();
    if value.contains("://") || value.starts_with("about:") || value.starts_with("data:") {
        value.to_owned()
    } else {
        format!("https://{value}")
    }
}

#[cfg(not(feature = "cef"))]
pub fn run() -> iced::Result {
    crate::CefBrowser::run()
}

#[cfg(not(feature = "cef"))]
pub fn attach(_url: String) -> iced::Task<AttachResult> {
    iced::Task::done(AttachResult {
        attached: false,
        status: FEATURE_HINT.to_owned(),
    })
}

#[cfg(not(feature = "cef"))]
pub fn pump() -> bool {
    false
}

#[cfg(not(feature = "cef"))]
pub fn load(_url: String) -> bool {
    false
}

#[cfg(not(feature = "cef"))]
pub fn go_back() -> bool {
    false
}

#[cfg(not(feature = "cef"))]
pub fn go_forward() -> bool {
    false
}

#[cfg(not(feature = "cef"))]
pub fn reload() -> bool {
    false
}

#[cfg(not(feature = "cef"))]
pub fn can_go_back() -> bool {
    false
}

#[cfg(not(feature = "cef"))]
pub fn can_go_forward() -> bool {
    false
}

#[cfg(feature = "cef")]
mod enabled {
    use cef::rc::Rc;
    use cef::{
        Browser, BrowserSettings, CefString, Client, ImplBrowser, ImplBrowserHost, ImplClient,
        ImplFrame, ImplLifeSpanHandler, LifeSpanHandler, Rect, Settings, WindowInfo, WrapClient,
        WrapLifeSpanHandler,
    };
    use iced::window::raw_window_handle::RawWindowHandle;
    use std::cell::RefCell;
    use std::io;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};

    const WINDOW_WIDTH: f32 = 1100.0;
    const WINDOW_HEIGHT: f32 = 760.0;
    const ICE_CHROME_HEIGHT: f32 = 96.0;
    const WELCOME_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
  * { box-sizing: border-box; }
  body { margin: 0; min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(circle at 20% 10%, #dbeafe, transparent 35%), #f8fafc;
    color: #172033; font: 16px system-ui, sans-serif; }
  main { width: min(680px, calc(100% - 48px)); padding: 44px; border: 1px solid #d8e0eb;
    border-radius: 20px; background: rgba(255,255,255,.9); box-shadow: 0 24px 80px #1e3a5f20; }
  small { color: #2563eb; font-weight: 700; letter-spacing: .12em; }
  h1 { margin: 12px 0; font-size: 42px; letter-spacing: -.04em; }
  p { margin: 0; color: #526078; line-height: 1.65; }
  code { color: #1d4ed8; }
</style>
</head>
<body><main><small>CHROMIUM EMBEDDED FRAMEWORK</small><h1>CEF is rendering inside Ice.</h1>
<p>The toolbar above is generated from <code>browser.ice</code>. This page is a native CEF child window attached through iced's raw window handle.</p>
</main></body></html>"#;

    struct BrowserState {
        browser: Browser,
        closed: Arc<AtomicBool>,
    }

    thread_local! {
        static BROWSER: RefCell<Option<BrowserState>> = const { RefCell::new(None) };
    }

    cef::wrap_life_span_handler! {
        struct BrowserLifeSpan {
            closed: Arc<AtomicBool>,
        }

        impl LifeSpanHandler {
            fn on_before_close(&self, _browser: Option<&mut Browser>) {
                self.closed.store(true, Ordering::Release);
            }
        }
    }

    cef::wrap_client! {
        struct BrowserClient {
            life_span: LifeSpanHandler,
        }

        impl Client {
            fn life_span_handler(&self) -> Option<LifeSpanHandler> {
                Some(self.life_span.clone())
            }
        }
    }

    pub fn run() -> iced::Result {
        #[cfg(target_os = "macos")]
        let _library = load_library();
        initialize_api();
        let args = cef::args::Args::new();
        let exit_code = cef::execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
        if exit_code >= 0 {
            std::process::exit(exit_code);
        }

        let root_cache_path = std::env::temp_dir().join("ice-cef-browser-cache");
        std::fs::create_dir_all(&root_cache_path)
            .map_err(|error| iced_error(&format!("failed to create CEF cache: {error}")))?;
        let settings = Settings {
            external_message_pump: 1,
            no_sandbox: 1,
            root_cache_path: CefString::from(root_cache_path.to_string_lossy().as_ref()),
            ..Settings::default()
        };
        if cef::initialize(
            Some(args.as_main_args()),
            Some(&settings),
            None,
            std::ptr::null_mut(),
        ) != 1
        {
            return Err(iced_error("CEF initialization failed"));
        }

        let result = crate::CefBrowser::run();
        close_browser();
        cef::shutdown();
        result
    }

    pub fn run_helper() -> i32 {
        #[cfg(target_os = "macos")]
        let _library = load_library();
        initialize_api();
        let args = cef::args::Args::new();
        cef::execute_process(Some(args.as_main_args()), None, std::ptr::null_mut())
    }

    pub fn attach(url: String) -> iced::Task<super::AttachResult> {
        iced::window::oldest().then(move |id| {
            let url = url.clone();
            match id {
                Some(id) => iced::window::scale_factor(id).then(move |scale_factor| {
                    let url = url.clone();
                    iced::window::run(id, move |window| create_browser(window, &url, scale_factor))
                }),
                None => iced::Task::done(super::AttachResult {
                    attached: false,
                    status: "the iced window is not available".to_owned(),
                }),
            }
        })
    }

    pub fn pump() -> bool {
        cef::do_message_loop_work();
        BROWSER.with_borrow(|browser| browser.is_some())
    }

    pub fn load(url: String) -> bool {
        with_browser(|browser| {
            let Some(frame) = browser.main_frame() else {
                return false;
            };
            frame.load_url(Some(&browser_url(&url)));
            true
        })
    }

    pub fn go_back() -> bool {
        with_browser(|browser| {
            if browser.can_go_back() == 0 {
                return false;
            }
            browser.go_back();
            true
        })
    }

    pub fn go_forward() -> bool {
        with_browser(|browser| {
            if browser.can_go_forward() == 0 {
                return false;
            }
            browser.go_forward();
            true
        })
    }

    pub fn reload() -> bool {
        with_browser(|browser| {
            browser.reload();
            true
        })
    }

    pub fn can_go_back() -> bool {
        with_browser(|browser| browser.can_go_back() != 0)
    }

    pub fn can_go_forward() -> bool {
        with_browser(|browser| browser.can_go_forward() != 0)
    }

    fn create_browser(
        window: &dyn iced::window::Window,
        url: &str,
        scale_factor: f32,
    ) -> super::AttachResult {
        if BROWSER.with_borrow(|browser| browser.is_some()) {
            return super::AttachResult {
                attached: true,
                status: "CEF child browser attached".to_owned(),
            };
        }

        let parent = match parent_handle(window) {
            Ok(parent) => parent,
            Err(error) => {
                return super::AttachResult {
                    attached: false,
                    status: error,
                };
            }
        };
        let pixel_scale = if cfg!(target_os = "macos") {
            1.0
        } else {
            scale_factor
        };
        let bounds = Rect {
            x: 0,
            y: (ICE_CHROME_HEIGHT * pixel_scale).round() as i32,
            width: (WINDOW_WIDTH * pixel_scale).round() as i32,
            height: ((WINDOW_HEIGHT - ICE_CHROME_HEIGHT) * pixel_scale).round() as i32,
        };
        let window_info = WindowInfo::default().set_as_child(parent, &bounds);
        let closed = Arc::new(AtomicBool::new(false));
        let life_span = BrowserLifeSpan::new(closed.clone());
        let mut client = BrowserClient::new(life_span);
        let browser_url = browser_url(url);
        let browser = cef::browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client),
            Some(&browser_url),
            Some(&BrowserSettings::default()),
            None,
            None,
        );
        let Some(browser) = browser else {
            return super::AttachResult {
                attached: false,
                status: "CEF did not create a browser".to_owned(),
            };
        };

        BROWSER.with_borrow_mut(|slot| {
            *slot = Some(BrowserState { browser, closed });
        });
        super::AttachResult {
            attached: true,
            status: "CEF child browser attached".to_owned(),
        }
    }

    #[cfg(target_os = "linux")]
    fn parent_handle(
        window: &dyn iced::window::Window,
    ) -> Result<cef::sys::cef_window_handle_t, String> {
        match window
            .window_handle()
            .map_err(|error| error.to_string())?
            .as_raw()
        {
            RawWindowHandle::Xlib(handle) => Ok(handle.window),
            other => Err(format!(
                "CEF requires iced's X11 backend on Linux, got {other:?}"
            )),
        }
    }

    #[cfg(target_os = "windows")]
    fn parent_handle(
        window: &dyn iced::window::Window,
    ) -> Result<cef::sys::cef_window_handle_t, String> {
        match window
            .window_handle()
            .map_err(|error| error.to_string())?
            .as_raw()
        {
            RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as _),
            other => Err(format!("expected a Win32 window handle, got {other:?}")),
        }
    }

    #[cfg(target_os = "macos")]
    fn parent_handle(
        window: &dyn iced::window::Window,
    ) -> Result<cef::sys::cef_window_handle_t, String> {
        match window
            .window_handle()
            .map_err(|error| error.to_string())?
            .as_raw()
        {
            RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr()),
            other => Err(format!("expected an AppKit window handle, got {other:?}")),
        }
    }

    fn with_browser(action: impl FnOnce(&Browser) -> bool) -> bool {
        BROWSER.with_borrow(|state| state.as_ref().is_some_and(|state| action(&state.browser)))
    }

    fn browser_url(value: &str) -> CefString {
        if value.trim() == "ice://welcome" {
            let encoded = CefString::from(&cef::base64_encode(Some(WELCOME_HTML.as_bytes())));
            let encoded = encoded.to_string();
            CefString::from(format!("data:text/html;base64,{encoded}").as_str())
        } else {
            CefString::from(super::normalize_url(value).as_str())
        }
    }

    fn close_browser() {
        let Some(state) = BROWSER.take() else {
            return;
        };
        if let Some(host) = state.browser.host() {
            host.close_browser(1);
        }
        drop(state.browser);

        let deadline = Instant::now() + Duration::from_secs(2);
        while !state.closed.load(Ordering::Acquire) && Instant::now() < deadline {
            cef::do_message_loop_work();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn iced_error(message: &str) -> iced::Error {
        iced::Error::WindowCreationFailed(Box::new(io::Error::other(message)))
    }

    fn initialize_api() {
        let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    }

    #[cfg(target_os = "macos")]
    fn load_library() -> cef::library_loader::LibraryLoader {
        let loader = cef::library_loader::LibraryLoader::new(
            &std::env::current_exe().expect("current executable path"),
            false,
        );
        assert!(loader.load(), "failed to load the CEF framework");
        loader
    }
}

#[cfg(feature = "cef")]
pub use enabled::*;

#[cfg(test)]
mod tests {
    use super::normalize_url;

    #[test]
    fn normalizes_host_names_without_changing_explicit_schemes() {
        assert_eq!(normalize_url(" example.com "), "https://example.com");
        assert_eq!(
            normalize_url("http://localhost:8080"),
            "http://localhost:8080"
        );
        assert_eq!(normalize_url("about:blank"), "about:blank");
    }
}

#[cfg(not(feature = "cef"))]
const FEATURE_HINT: &str =
    "CEF support is disabled; run the example with `--features cef` or use the bundler";

#[derive(Debug, Clone)]
pub struct AttachResult {
    pub attached: bool,
    pub status: String,
}

#[cfg(any(feature = "cef", test))]
const DISABLED_CREDENTIAL_PREFERENCES: &[&str] = &[
    "credentials_enable_service",
    "credentials_enable_autosignin",
    "credentials_enable_passkeys",
];

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CefProcessKind {
    Browser,
    Helper,
}

#[cfg(any(target_os = "macos", test))]
impl CefProcessKind {
    fn uses_helper_bundle_layout(self) -> bool {
        matches!(self, Self::Helper)
    }
}

#[cfg(any(feature = "cef", test))]
fn credential_switches(target_os: &str) -> Vec<(&'static str, Option<&'static str>)> {
    let mut switches = vec![("disable-blink-features", Some("WebAuth"))];
    match target_os {
        "linux" => switches.push(("password-store", Some("basic"))),
        "macos" => switches.push(("use-mock-keychain", None)),
        _ => {}
    }
    switches
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
#[cfg_attr(test, allow(dead_code))]
mod enabled {
    use cef::rc::Rc;
    use cef::wrap_app;
    use cef::wrap_browser_process_handler;
    use cef::{
        App, Browser, BrowserProcessHandler, BrowserSettings, CefString, Client, CommandLine,
        ImplApp, ImplBrowser, ImplBrowserHost, ImplBrowserProcessHandler, ImplClient,
        ImplCommandLine, ImplFrame, ImplLifeSpanHandler, ImplPreferenceManager, ImplValue,
        LifeSpanHandler, Rect, Settings, WindowInfo, WrapApp, WrapBrowserProcessHandler,
        WrapClient, WrapLifeSpanHandler,
    };
    use iced::window::raw_window_handle::RawWindowHandle;
    use std::cell::RefCell;
    use std::io;
    use std::path::{Path, PathBuf};
    #[cfg(target_os = "macos")]
    use std::sync::atomic::AtomicU64;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    const WINDOW_WIDTH: f32 = 1100.0;
    const WINDOW_HEIGHT: f32 = 760.0;
    const ICE_CHROME_HEIGHT: f32 = 68.0;
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

    struct UserProfileDir(PathBuf);

    impl UserProfileDir {
        fn create() -> io::Result<Self> {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("ice-cef-browser-{}-{unique}", std::process::id()));
            std::fs::create_dir(&path)?;
            restrict_to_current_user(&path)?;
            Ok(Self(path))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for UserProfileDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    thread_local! {
        static BROWSER: RefCell<Option<BrowserState>> = const { RefCell::new(None) };
    }

    #[cfg(target_os = "macos")]
    static MESSAGE_PUMP_RUNNING: AtomicBool = AtomicBool::new(false);
    #[cfg(target_os = "macos")]
    static MESSAGE_PUMP_GENERATION: AtomicU64 = AtomicU64::new(0);

    wrap_browser_process_handler! {
        struct BrowserProcessCallbacks;

        impl BrowserProcessHandler {
            fn on_schedule_message_pump_work(&self, delay_ms: i64) {
                schedule_message_pump_work(delay_ms);
            }
        }
    }

    wrap_app! {
        struct BrowserApp {
            browser_process_handler: BrowserProcessHandler,
        }

        impl App {
            fn on_before_command_line_processing(
                &self,
                _process_type: Option<&CefString>,
                command_line: Option<&mut CommandLine>,
            ) {
                let Some(command_line) = command_line else {
                    return;
                };
                for (name, value) in super::credential_switches(std::env::consts::OS) {
                    let name = CefString::from(name);
                    if let Some(value) = value {
                        let value = CefString::from(value);
                        command_line.append_switch_with_value(Some(&name), Some(&value));
                    } else {
                        command_line.append_switch(Some(&name));
                    }
                }
            }

            fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
                Some(self.browser_process_handler.clone())
            }
        }
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
        let _library = load_library(super::CefProcessKind::Browser);
        initialize_api();
        let args = cef::args::Args::new();
        let mut app = BrowserApp::new(BrowserProcessCallbacks::new());
        let exit_code = cef::execute_process(
            Some(args.as_main_args()),
            Some(&mut app),
            std::ptr::null_mut(),
        );
        if exit_code >= 0 {
            std::process::exit(exit_code);
        }

        let profile = UserProfileDir::create()
            .map_err(|error| iced_error(&format!("failed to create CEF profile: {error}")))?;
        let settings = Settings {
            external_message_pump: 1,
            no_sandbox: 1,
            root_cache_path: CefString::from(profile.path().to_string_lossy().as_ref()),
            ..Settings::default()
        };
        if cef::initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut app),
            std::ptr::null_mut(),
        ) != 1
        {
            return Err(iced_error("CEF initialization failed"));
        }
        if let Err(error) = disable_credential_services() {
            cef::shutdown();
            return Err(error);
        }

        #[cfg(target_os = "macos")]
        MESSAGE_PUMP_RUNNING.store(true, Ordering::Release);
        let result = crate::CefBrowser::run();
        #[cfg(target_os = "macos")]
        stop_message_pump();
        close_browser();
        cef::shutdown();
        drop(profile);
        result
    }

    pub fn run_helper() -> i32 {
        #[cfg(target_os = "macos")]
        let _library = load_library(super::CefProcessKind::Helper);
        initialize_api();
        let args = cef::args::Args::new();
        let mut app = BrowserApp::new(BrowserProcessCallbacks::new());
        cef::execute_process(
            Some(args.as_main_args()),
            Some(&mut app),
            std::ptr::null_mut(),
        )
    }

    pub fn attach(url: String) -> iced::Task<super::AttachResult> {
        #[cfg(test)]
        {
            let _ = url;
            return iced::Task::done(super::AttachResult {
                attached: false,
                status: "CEF attachment skipped in the headless test runtime".to_owned(),
            });
        }

        #[cfg(not(test))]
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
        pump_message_loop();
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

    #[cfg(target_os = "macos")]
    fn pump_message_loop() {}

    #[cfg(not(target_os = "macos"))]
    fn pump_message_loop() {
        cef::do_message_loop_work();
    }

    #[cfg(not(target_os = "macos"))]
    fn schedule_message_pump_work(_delay_ms: i64) {}

    #[cfg(target_os = "macos")]
    fn schedule_message_pump_work(delay_ms: i64) {
        // CEF may invoke this callback from any thread. A newer request
        // supersedes an older delayed request, and all actual CEF work runs on
        // AppKit's main dispatch queue beside Iced's stock event loop.
        let generation = MESSAGE_PUMP_GENERATION
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        let work = move || {
            if MESSAGE_PUMP_RUNNING.load(Ordering::Acquire)
                && MESSAGE_PUMP_GENERATION.load(Ordering::Acquire) == generation
            {
                cef::do_message_loop_work();
            }
        };
        let queue = dispatch2::DispatchQueue::main();
        if delay_ms <= 0 {
            queue.exec_async(work);
        } else {
            let delay = Duration::from_millis(delay_ms as u64);
            let when =
                dispatch2::DispatchTime::try_from(delay).unwrap_or(dispatch2::DispatchTime::NOW);
            let _ = queue.after(when, work);
        }
    }

    #[cfg(target_os = "macos")]
    fn stop_message_pump() {
        MESSAGE_PUMP_RUNNING.store(false, Ordering::Release);
        MESSAGE_PUMP_GENERATION.fetch_add(1, Ordering::AcqRel);
    }

    fn disable_credential_services() -> iced::Result {
        let context = cef::request_context_get_global_context()
            .ok_or_else(|| iced_error("CEF global request context is unavailable"))?;
        for preference in super::DISABLED_CREDENTIAL_PREFERENCES {
            let name = CefString::from(*preference);
            let mut value = cef::value_create()
                .ok_or_else(|| iced_error("CEF did not create a preference value"))?;
            if value.set_bool(0) != 1 {
                return Err(iced_error(&format!(
                    "CEF did not accept the {preference} preference value"
                )));
            }
            let mut error = CefString::from("");
            if context.set_preference(Some(&name), Some(&mut value), Some(&mut error)) != 1 {
                return Err(iced_error(&format!(
                    "failed to disable CEF preference {preference}: {error}"
                )));
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn restrict_to_current_user(path: &Path) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
    }

    #[cfg(not(unix))]
    fn restrict_to_current_user(_path: &Path) -> io::Result<()> {
        Ok(())
    }

    fn initialize_api() {
        let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    }

    #[cfg(target_os = "macos")]
    fn load_library(kind: super::CefProcessKind) -> cef::library_loader::LibraryLoader {
        let loader = cef::library_loader::LibraryLoader::new(
            &std::env::current_exe().expect("current executable path"),
            kind.uses_helper_bundle_layout(),
        );
        assert!(loader.load(), "failed to load the CEF framework");
        loader
    }
}

#[cfg(feature = "cef")]
pub use enabled::*;

#[cfg(test)]
mod tests {
    use super::{
        CefProcessKind, DISABLED_CREDENTIAL_PREFERENCES, credential_switches, normalize_url,
    };

    #[test]
    fn normalizes_host_names_without_changing_explicit_schemes() {
        assert_eq!(normalize_url(" example.com "), "https://example.com");
        assert_eq!(
            normalize_url("http://localhost:8080"),
            "http://localhost:8080"
        );
        assert_eq!(normalize_url("about:blank"), "about:blank");
    }

    #[test]
    fn credential_policy_avoids_platform_secret_stores() {
        assert_eq!(
            credential_switches("linux"),
            vec![
                ("disable-blink-features", Some("WebAuth")),
                ("password-store", Some("basic"))
            ]
        );
        assert_eq!(
            credential_switches("macos"),
            vec![
                ("disable-blink-features", Some("WebAuth")),
                ("use-mock-keychain", None)
            ]
        );
        assert_eq!(
            credential_switches("windows"),
            vec![("disable-blink-features", Some("WebAuth"))]
        );
        assert_eq!(
            DISABLED_CREDENTIAL_PREFERENCES,
            [
                "credentials_enable_service",
                "credentials_enable_autosignin",
                "credentials_enable_passkeys"
            ]
        );
    }

    #[test]
    fn helper_processes_use_the_parent_bundle_framework_layout() {
        assert!(!CefProcessKind::Browser.uses_helper_bundle_layout());
        assert!(CefProcessKind::Helper.uses_helper_bundle_layout());
    }
}

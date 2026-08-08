//! System tray support: a status item in the macOS menu bar with a reactive
//! label and click events bridged into iced subscriptions.
//!
//! The native handle is `!Send` and main-thread-only, so it lives in a
//! thread-local owned by this module; generated code only calls the free
//! functions below. On platforms without an implementation every function is
//! a no-op so the same generated program compiles everywhere.

/// Compile-time embedded tray icon plus its initial appearance.
#[derive(Clone, Copy, Debug)]
pub struct TrayConfig {
    /// Raw RGBA bytes, exactly `icon_width × icon_height × 4` of them.
    pub icon_rgba: &'static [u8],
    pub icon_width: u32,
    pub icon_height: u32,
    /// macOS template rendering: black + alpha, recolored by the system.
    pub icon_template: bool,
}

/// The status item's bounds in physical screen pixels, as reported by the
/// platform at click time.
#[derive(Clone, Debug, PartialEq)]
pub struct TrayRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Events delivered to the generated `__TrayEvent` message.
#[derive(Clone, Debug)]
pub enum TrayEvent {
    /// Primary-button press on the status item.
    LeftClick {
        /// Icon bounds in physical pixels.
        icon: TrayRect,
    },
}

/// Gap between the menu bar and an anchored popover, in logical points.
const ANCHOR_MARGIN: f32 = 4.0;

/// Whether `ICE_TRAY_DEBUG` asked for a trace of the tray's native boundary.
/// A status item that does nothing looks identical whether the platform never
/// delivered the click, the bridge dropped it, or the popover landed off the
/// screen, and none of the three is visible from inside the application.
fn tracing() -> bool {
    static TRACING: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACING.get_or_init(|| std::env::var_os("ICE_TRAY_DEBUG").is_some())
}

macro_rules! trace {
    ($($arg:tt)*) => {
        if $crate::tray::tracing() {
            eprintln!("ice tray: {}", ::std::format!($($arg)*));
        }
    };
}

/// Top-left logical position for a popover of `window_size` anchored under
/// the status item. `icon` is in physical pixels; `scale_factor` converts it
/// to the logical space used by iced window positioning.
///
/// `monitor` is a display's logical size — iced reports no origin for it, so
/// it only describes a coordinate span when the icon sits on that same
/// display. Screen coordinates are global: a display arranged right of the
/// primary starts beyond its width, and one arranged left of it runs
/// negative. Clamping against a display the icon is not on would teleport the
/// popover onto another screen, so the edge clamp applies only when the
/// icon's own centre falls inside the reported span.
#[must_use]
pub fn anchor_position(
    icon: &TrayRect,
    scale_factor: f64,
    window_size: iced::Size,
    monitor: Option<iced::Size>,
) -> iced::Point {
    let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    let center = (icon.x + icon.width / 2.0) / scale;
    let bottom = (icon.y + icon.height) / scale;
    let mut x = center as f32 - window_size.width / 2.0;
    if let Some(monitor) =
        monitor.filter(|monitor| (0.0..=f64::from(monitor.width)).contains(&center))
    {
        x = x.min(monitor.width - window_size.width).max(0.0);
    }
    let anchor = iced::Point::new(x, bottom as f32 + ANCHOR_MARGIN);
    trace!(
        "anchor: icon {icon:?} scale {scale} window {window_size:?} monitor {monitor:?} -> {anchor:?}"
    );
    anchor
}

pub use platform::{events, init, set_label, set_tooltip};

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::RefCell;
    use std::sync::Mutex;

    use super::{TrayConfig, TrayEvent, TrayRect};

    struct TrayState {
        icon: tray_icon::TrayIcon,
        label: String,
        tooltip: String,
    }

    thread_local! {
        static TRAY: RefCell<Option<TrayState>> = const { RefCell::new(None) };
    }

    static EVENTS: Mutex<Option<iced::futures::channel::mpsc::UnboundedReceiver<TrayEvent>>> =
        Mutex::new(None);

    /// Creates the status item. Must run on the main thread with the iced
    /// event loop initialized — generated boot satisfies both. A platform
    /// failure is reported to stderr and leaves every later call a no-op;
    /// the application itself keeps running.
    pub fn init(config: TrayConfig) {
        let (sender, receiver) = iced::futures::channel::mpsc::unbounded();
        *EVENTS.lock().expect("tray event receiver lock") = Some(receiver);
        tray_icon::TrayIconEvent::set_event_handler(Some(
            move |event: tray_icon::TrayIconEvent| {
                trace!("native event: {event:?}");
                if let tray_icon::TrayIconEvent::Click {
                    rect,
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Down,
                    ..
                } = event
                {
                    let delivered = sender.unbounded_send(TrayEvent::LeftClick {
                        icon: TrayRect {
                            x: rect.position.x,
                            y: rect.position.y,
                            width: f64::from(rect.size.width),
                            height: f64::from(rect.size.height),
                        },
                    });
                    trace!("forwarded left click: {delivered:?}");
                }
            },
        ));
        let icon = match tray_icon::Icon::from_rgba(
            config.icon_rgba.to_vec(),
            config.icon_width,
            config.icon_height,
        ) {
            Ok(icon) => icon,
            Err(error) => {
                eprintln!("ice tray: invalid icon RGBA: {error}");
                return;
            }
        };
        match tray_icon::TrayIconBuilder::new()
            .with_icon(icon)
            .with_icon_as_template(config.icon_template)
            .build()
        {
            Ok(icon) => TRAY.with(|slot| {
                trace!("status item created");
                *slot.borrow_mut() = Some(TrayState {
                    icon,
                    label: String::new(),
                    tooltip: String::new(),
                });
            }),
            Err(error) => eprintln!("ice tray: status item creation failed: {error}"),
        }
    }

    /// Shows `value` next to the icon; diffs against the last value so
    /// unchanged updates skip the native call.
    pub fn set_label(value: &str) {
        TRAY.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut()
                && state.label != value
            {
                state.icon.set_title(Some(value));
                value.clone_into(&mut state.label);
            }
        });
    }

    /// Updates the hover tooltip with the same diffing as [`set_label`]. A
    /// failed update is not recorded, so the next sync retries it instead of
    /// leaving the tooltip stale until the value happens to change again.
    pub fn set_tooltip(value: &str) {
        TRAY.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut()
                && state.tooltip != value
            {
                match state.icon.set_tooltip(Some(value)) {
                    Ok(()) => value.clone_into(&mut state.tooltip),
                    Err(error) => eprintln!("ice tray: tooltip update failed: {error}"),
                }
            }
        });
    }

    /// Recipe identity for the tray event stream. `Subscription::run_with`
    /// identifies a recipe by this type plus its hash, so the unit struct is
    /// the whole identity: one tray stream per program.
    #[derive(Hash)]
    struct TraySubscription;

    fn tray_stream(
        _subscription: &TraySubscription,
    ) -> iced::futures::channel::mpsc::UnboundedReceiver<TrayEvent> {
        EVENTS
            .lock()
            .expect("tray event receiver lock")
            .take()
            .inspect(|_| trace!("subscription took the live event receiver"))
            .unwrap_or_else(|| {
                trace!("subscription found no receiver — clicks cannot arrive");
                let (_sender, receiver) = iced::futures::channel::mpsc::unbounded();
                receiver
            })
    }

    /// Stream of tray events for the generated subscription batch.
    pub fn events() -> iced::Subscription<TrayEvent> {
        iced::Subscription::run_with(TraySubscription, tray_stream)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{TrayConfig, TrayEvent};

    /// No tray backend on this platform; the declaration is a no-op.
    pub fn init(_config: TrayConfig) {}

    /// No tray backend on this platform; the label is dropped.
    pub fn set_label(_value: &str) {}

    /// No tray backend on this platform; the tooltip is dropped.
    pub fn set_tooltip(_value: &str) {}

    /// Never yields events on this platform.
    pub fn events() -> iced::Subscription<TrayEvent> {
        iced::Subscription::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> TrayRect {
        // A status item on a 2x display: 60 physical ≙ 30 logical from the
        // left, 44 wide, menu bar 48 tall.
        TrayRect {
            x: 3000.0,
            y: 0.0,
            width: 44.0,
            height: 48.0,
        }
    }

    #[test]
    fn anchors_centered_under_the_icon() {
        let position = anchor_position(&rect(), 2.0, iced::Size::new(320.0, 240.0), None);
        // Icon center: (3000 + 22) / 2 = 1511 logical; window left edge
        // 1511 - 160 = 1351. Below the bar: 48 / 2 = 24, plus the margin.
        assert_eq!(position, iced::Point::new(1351.0, 24.0 + ANCHOR_MARGIN));
    }

    #[test]
    fn clamps_to_the_left_edge_of_the_icons_monitor() {
        let icon = TrayRect {
            x: 10.0,
            y: 0.0,
            width: 44.0,
            height: 48.0,
        };
        let position = anchor_position(
            &icon,
            2.0,
            iced::Size::new(320.0, 240.0),
            Some(iced::Size::new(3008.0, 1692.0)),
        );
        assert_eq!(position.x, 0.0);
    }

    #[test]
    fn clamps_inside_the_monitor_right_edge() {
        let icon = TrayRect {
            x: 5900.0,
            y: 0.0,
            width: 44.0,
            height: 48.0,
        };
        let position = anchor_position(
            &icon,
            2.0,
            iced::Size::new(320.0, 240.0),
            Some(iced::Size::new(3008.0, 1692.0)),
        );
        assert_eq!(position.x, 3008.0 - 320.0);
    }

    /// A status item on a display arranged right of the primary reports
    /// global coordinates past that display's width. The reported size
    /// belongs to whichever display the popover was created on, so clamping
    /// with it would drag the popover onto the wrong screen.
    #[test]
    fn ignores_a_monitor_the_icon_is_not_on() {
        let icon = TrayRect {
            x: 8000.0,
            y: 0.0,
            width: 44.0,
            height: 48.0,
        };
        let position = anchor_position(
            &icon,
            2.0,
            iced::Size::new(320.0, 240.0),
            Some(iced::Size::new(3008.0, 1692.0)),
        );
        // Centre 4011 logical, so the popover stays under the icon at 3851
        // instead of being pulled back to the primary display's edge.
        assert_eq!(position.x, 3851.0);
    }

    /// A display arranged left of the primary runs negative, which the
    /// left-edge floor would otherwise pull onto the primary display.
    #[test]
    fn keeps_negative_coordinates_off_the_primary_monitor() {
        let icon = TrayRect {
            x: -1800.0,
            y: 0.0,
            width: 44.0,
            height: 48.0,
        };
        let position = anchor_position(
            &icon,
            2.0,
            iced::Size::new(320.0, 240.0),
            Some(iced::Size::new(3008.0, 1692.0)),
        );
        assert_eq!(position.x, -1049.0);
    }

    #[test]
    fn treats_non_positive_scale_as_identity() {
        let icon = TrayRect {
            x: 600.0,
            y: 0.0,
            width: 22.0,
            height: 24.0,
        };
        let position = anchor_position(&icon, 0.0, iced::Size::new(100.0, 50.0), None);
        assert_eq!(position, iced::Point::new(561.0, 24.0 + ANCHOR_MARGIN));
    }
}

//! The clock's one source of time: a stream of ticks the host sends.

use app_store_sdk::host;
use iced::futures::{Stream, StreamExt};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClockError {
    pub message: String,
}

/// Host uptime in milliseconds, once per `every_ms`.
pub fn ticks(every_ms: i64) -> impl Stream<Item = Result<i64, ClockError>> + Send + 'static {
    host::subscribe("clock.ticks", &every_ms.to_le_bytes()).map(|answer| {
        let bytes = answer.map_err(|message| ClockError { message })?;
        let ms = bytes
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|_| ClockError {
                message: "a tick that is not a u64".into(),
            })?;
        Ok(ms as i64)
    })
}

/// What one `clock.now` says: the epoch millisecond the host stood at zero
/// uptime, and the uptime it was read at.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Now {
    pub at_boot: i64,
    /// The app's own uptime until its first tick lands, which the host
    /// schedules one whole period after the subscription — without it every
    /// label would show the wall clock as it was when the store started.
    pub uptime: i64,
}

/// The wall clock, asked once. Asking every tick would spend a request on
/// what arithmetic already knows: the uptime the ticks carry moves the wall
/// clock by exactly as much.
///
/// The answer is two `u64`s — the wall clock and the uptime it was read at —
/// because the app cannot know how long the host had been up when it was
/// installed, and its ticks are measured from the host's start, not its own.
pub async fn now() -> Result<Now, ClockError> {
    let bytes = host::request("clock.now", &[])
        .await
        .map_err(|message| ClockError { message })?;
    let ms = |slice: &[u8]| slice.try_into().map(u64::from_le_bytes).ok();
    let (unix, uptime) = bytes.split_at(bytes.len().min(8));
    let (Some(unix), Some(uptime)) = (ms(unix), ms(uptime)) else {
        return Err(ClockError {
            message: "a time that is not two u64s".into(),
        });
    };
    Ok(Now {
        at_boot: unix as i64 - uptime as i64,
        uptime: uptime as i64,
    })
}

/// The wall clock, from the epoch millisecond the host stood at zero uptime
/// plus the uptime since. The host offers no timezone, so the label says UTC
/// and means it.
pub fn wall_label(now_at_boot_ms: i64, uptime_ms: i64) -> String {
    let second = (now_at_boot_ms + uptime_ms)
        .div_euclid(1000)
        .rem_euclid(86_400);
    format!(
        "{:02}:{:02}:{:02} UTC",
        second / 3600,
        second / 60 % 60,
        second % 60
    )
}

pub fn uptime_label(ms: i64) -> String {
    let seconds = ms / 1000;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// Ten dots, the lit one walking with every tick.
pub fn dots_label(ticks: i64) -> String {
    let lit = (ticks.max(0) % 10) as usize;
    (0..10)
        .map(|i| if i == lit { '●' } else { '○' })
        .collect::<String>()
}

pub fn ticks_label(ticks: i64) -> String {
    format!("{ticks} ticks received")
}

/// The host's colour mode: `light` or `dark`, once on subscribing and again
/// on every change.
pub fn theme_changes() -> impl Stream<Item = Result<String, ClockError>> + Send + 'static {
    host::theme().map(|answer| {
        answer
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|message| ClockError { message })
    })
}

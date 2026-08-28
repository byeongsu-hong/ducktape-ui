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

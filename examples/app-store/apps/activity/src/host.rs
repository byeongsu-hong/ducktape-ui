//! A feed of everything on the host's bus.

use iced::futures::{Stream, StreamExt};
use ui_lang_guest::host;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BusError {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entry {
    pub from: String,
    pub topic: String,
    pub text: String,
}

/// Every message published under `topic` (`*` for all). The host delivers
/// `from\ntopic\ntext`: the publisher's app id is the host's word, not the
/// publisher's, which is what makes it worth showing.
pub fn events(topic: String) -> impl Stream<Item = Result<Entry, BusError>> + Send + 'static {
    host::subscribe("bus.subscribe", topic.as_bytes()).map(|answer| {
        let bytes = answer.map_err(|message| BusError { message })?;
        let message = String::from_utf8_lossy(&bytes);
        let mut parts = message.splitn(3, '\n');
        Ok(Entry {
            from: parts.next().unwrap_or_default().to_string(),
            topic: parts.next().unwrap_or_default().to_string(),
            text: parts.next().unwrap_or_default().to_string(),
        })
    })
}

const KEEP: usize = 50;

pub fn push_entry(mut log: Vec<Entry>, entry: Entry) -> Vec<Entry> {
    log.insert(0, entry);
    log.truncate(KEEP);
    log
}

pub fn origin_label(entry: Entry) -> String {
    format!("{} · {}", entry.from, entry.topic)
}

pub fn count_label(log: &[Entry]) -> String {
    format!("{} events", log.len())
}

/// A row is its 14 px text line inside 12 px of padding, under an 8 px gap.
const ROW_HEIGHT: f64 = 17.0 + 24.0 + 8.0;

/// How many of the log's rows the feed's measured `height` shows.
pub fn visible_label(height: f64, log: &[Entry]) -> String {
    let fit = (height / ROW_HEIGHT).floor().max(0.0) as usize;
    format!("{} rows visible", fit.min(log.len()))
}

/// The host's colour mode: `light` or `dark`, once on subscribing and again
/// on every change.
pub fn theme_changes() -> impl Stream<Item = Result<String, BusError>> + Send + 'static {
    host::theme().map(|answer| {
        answer
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|message| BusError { message })
    })
}

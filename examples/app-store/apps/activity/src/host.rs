//! A feed of everything on the host's bus.

use app_store_sdk::host;
use iced::futures::{Stream, StreamExt};

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

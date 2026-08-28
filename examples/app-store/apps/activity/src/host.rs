//! A feed of everything on the host's bus.

use app_store_sdk::host;
use iced::futures::{Stream, StreamExt};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BusError {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entry {
    pub topic: String,
    pub text: String,
}

/// Every message published under `topic` (`*` for all), as `topic\ntext`.
pub fn events(topic: String) -> impl Stream<Item = Result<Entry, BusError>> + Send + 'static {
    host::subscribe("bus.subscribe", topic.as_bytes()).map(|answer| {
        let bytes = answer.map_err(|message| BusError { message })?;
        let text = String::from_utf8_lossy(&bytes);
        let (topic, text) = text.split_once('\n').unwrap_or((&text, ""));
        Ok(Entry {
            topic: topic.to_string(),
            text: text.to_string(),
        })
    })
}

const KEEP: usize = 50;

pub fn push_entry(mut log: Vec<Entry>, entry: Entry) -> Vec<Entry> {
    log.insert(0, entry);
    log.truncate(KEEP);
    log
}

pub fn count_label(log: Vec<Entry>) -> String {
    format!("{} events", log.len())
}

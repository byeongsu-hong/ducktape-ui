//! What the host lets an app do, one module per capability. Every function
//! here is reached through [`crate::store::Guest::answer`], which has already
//! checked that the app's manifest declares the capability and that the
//! payload is within [`crate::store::MAX_PAYLOAD_BYTES`].

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use app_store_frame as wire;

use crate::store::MAX_INBOX;

/// A guest's bus deliveries, filled by other guests' publishes and drained
/// into its event batch on its next redraw.
pub type Inbox = Arc<Mutex<Mailbox>>;

#[derive(Default)]
pub struct Mailbox {
    events: VecDeque<wire::Event>,
    /// Deliveries this guest never got because it was not draining — faulted,
    /// or slower than whoever publishes. Shown in its status line.
    pub dropped: u64,
}

impl Mailbox {
    /// Keeps the newest [`MAX_INBOX`] deliveries: a full inbox loses its
    /// oldest message, never the host's memory.
    fn deliver(&mut self, event: wire::Event) {
        if self.events.len() >= MAX_INBOX {
            self.events.pop_front();
            self.dropped += 1;
        }
        self.events.push_back(event);
    }

    pub fn take(&mut self) -> Vec<wire::Event> {
        self.events.drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

pub mod host {
    //! The two things every app may ask for, capability or not.

    use crate::store::MAX_RANDOM_BYTES;

    /// A module has no stdout — `println!` inside one goes nowhere — so its
    /// lines come out of the store's stderr, tagged with who wrote them.
    pub fn log(app: &str, message: &[u8]) {
        eprintln!("[{app}] {}", String::from_utf8_lossy(message));
    }

    /// A `u32` count of bytes, little-endian; the only entropy in a module,
    /// which cannot link `getrandom` without JS glue.
    pub fn random(payload: &[u8]) -> Result<Vec<u8>, String> {
        let count = payload.try_into().map(u32::from_le_bytes).unwrap_or(0) as usize;
        if count > MAX_RANDOM_BYTES {
            return Err(format!("more than {MAX_RANDOM_BYTES} random bytes at once"));
        }
        let mut bytes = vec![0; count];
        getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
        Ok(bytes)
    }
}

pub mod storage {
    //! One directory per app, one file per key. Nothing an app writes can
    //! name a path outside its own directory.

    use std::path::{Path, PathBuf};

    use crate::store::{MAX_APP_STORAGE, MAX_VALUE_BYTES};

    const DEFAULT_DIR: &str = "target/app-store-data";

    /// Where a value lands before it is renamed over the key it replaces.
    /// A leading dot is not a legal key, so it can never be one.
    const TEMP_NAME: &str = ".tmp";

    /// Everything the host keeps between runs: one directory per app, plus
    /// the store's own `installed` list.
    pub fn data_dir() -> PathBuf {
        PathBuf::from(std::env::var("APP_STORE_DATA").unwrap_or_else(|_| DEFAULT_DIR.to_string()))
    }

    fn path(app: &str, key: &[u8]) -> Result<PathBuf, String> {
        let key = std::str::from_utf8(key).map_err(|_| "a key that is not UTF-8".to_string())?;
        let plain = !key.is_empty()
            && !key.starts_with('.')
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
        if !plain {
            return Err(format!("`{key}` is not a storage key"));
        }
        Ok(data_dir().join(app).join(key))
    }

    /// The value, or nothing if the key was never written.
    pub fn get(app: &str, key: &[u8]) -> Result<Vec<u8>, String> {
        match std::fs::read(path(app, key)?) {
            Ok(bytes) => Ok(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error.to_string()),
        }
    }

    /// `key\nvalue`.
    pub fn set(app: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
        let split = payload
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| "a set without a `key\\n` prefix".to_string())?;
        let path = path(app, &payload[..split])?;
        let value = &payload[split + 1..];
        if value.len() as u64 > MAX_VALUE_BYTES {
            return Err(format!("a value larger than {MAX_VALUE_BYTES} bytes"));
        }
        let dir = path.parent().expect("a key path has a directory");
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        if used_bytes(dir, &path) + value.len() as u64 > MAX_APP_STORAGE {
            return Err(format!("more than {MAX_APP_STORAGE} bytes of storage"));
        }
        // Rename is the atomic write: a crash mid-`write` would otherwise
        // leave the key holding neither the old value nor the new one.
        let temp = dir.join(TEMP_NAME);
        std::fs::write(&temp, value).map_err(|error| error.to_string())?;
        std::fs::rename(&temp, &path).map_err(|error| error.to_string())?;
        Ok(Vec::new())
    }

    pub fn delete(app: &str, key: &[u8]) -> Result<Vec<u8>, String> {
        match std::fs::remove_file(path(app, key)?) {
            Ok(()) => Ok(Vec::new()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Every key the app has written, newline-separated. Sorted, because the
    /// order a directory lists in is the filesystem's business, not the app's.
    pub fn list(app: &str) -> Result<Vec<u8>, String> {
        let mut keys: Vec<String> = read_dir(&data_dir().join(app))
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| !name.starts_with('.'))
            .collect();
        keys.sort();
        Ok(keys.join("\n").into_bytes())
    }

    /// What the app's directory already holds, ignoring the key about to be
    /// overwritten. One `stat` per key on every `set`: a quota that is always
    /// right beats a counter that drifts from the disk.
    fn used_bytes(dir: &Path, replacing: &Path) -> u64 {
        read_dir(dir)
            .filter(|entry| entry.path() != replacing)
            .filter_map(|entry| entry.metadata().ok())
            .filter(std::fs::Metadata::is_file)
            .map(|metadata| metadata.len())
            .sum()
    }

    /// A directory that is missing or unreadable simply has no entries: an
    /// app that never wrote anything has no directory yet.
    fn read_dir(dir: &Path) -> impl Iterator<Item = std::fs::DirEntry> {
        std::fs::read_dir(dir).into_iter().flatten().flatten()
    }
}

pub mod bus {
    //! Publish/subscribe between apps, through the host. A message is
    //! `topic\ntext` going in and `from\ntopic\ntext` coming out; a
    //! subscription names a topic or `*`.

    use super::*;

    struct Subscriber {
        topic: String,
        id: u64,
        /// The subscriber's instance, still running. A faulted or dropped
        /// guest clears it, which is what prunes this entry — reading a flag
        /// takes no lock, and the publisher may be its own subscriber.
        alive: Arc<AtomicBool>,
        inbox: Weak<Mutex<Mailbox>>,
    }

    static SUBSCRIBERS: Mutex<Vec<Subscriber>> = Mutex::new(Vec::new());

    pub fn subscribe(topic: &[u8], id: u64, inbox: &Inbox, alive: &Arc<AtomicBool>) {
        SUBSCRIBERS.lock().expect("bus").push(Subscriber {
            topic: String::from_utf8_lossy(topic).into_owned(),
            id,
            alive: alive.clone(),
            inbox: Arc::downgrade(inbox),
        });
    }

    /// Drops one subscription. Request ids are per guest — every module
    /// counts its own from zero — so the inbox is the other half of the key.
    pub fn cancel(id: u64, inbox: &Inbox) {
        SUBSCRIBERS.lock().expect("bus").retain(|subscriber| {
            subscriber.id != id || !std::ptr::eq(subscriber.inbox.as_ptr(), Arc::as_ptr(inbox))
        });
    }

    /// Delivers `from\n` + the message to every live subscriber of the topic;
    /// returns how many heard it. Locks inboxes only: the publisher is a
    /// guest, may be its own subscriber, and must not wait on itself.
    pub fn publish(from: &str, payload: &[u8]) -> usize {
        let text = String::from_utf8_lossy(payload);
        let topic = text
            .split_once('\n')
            .map_or(text.as_ref(), |(topic, _)| topic);
        let mut message = Vec::with_capacity(from.len() + 1 + payload.len());
        message.extend_from_slice(from.as_bytes());
        message.push(b'\n');
        message.extend_from_slice(payload);
        let mut subscribers = SUBSCRIBERS.lock().expect("bus");
        subscribers.retain(|subscriber| subscriber.alive.load(Ordering::Relaxed));
        let mut delivered = 0;
        for subscriber in subscribers.iter() {
            let listening = subscriber.topic == "*" || subscriber.topic == topic;
            let Some(inbox) = listening.then(|| subscriber.inbox.upgrade()).flatten() else {
                continue;
            };
            inbox.lock().expect("inbox").deliver(wire::Event::Response {
                id: subscriber.id,
                result: Ok(message.clone()),
                done: false,
            });
            delivered += 1;
        }
        delivered
    }
}

pub mod clock {
    //! The guest has no clock; these read the host's.

    use std::sync::OnceLock;
    use std::time::{SystemTime, UNIX_EPOCH};

    use iced::time::Instant;

    /// Milliseconds since the store started.
    pub fn uptime_ms(now: Instant) -> u64 {
        static START: OnceLock<Instant> = OnceLock::new();
        let start = *START.get_or_init(|| now);
        now.saturating_duration_since(start).as_millis() as u64
    }

    /// Milliseconds since the unix epoch — the only wall clock a guest can
    /// get, `SystemTime::now()` being an abort inside the module.
    pub fn unix_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis() as u64)
    }

    /// A little-endian `i64` of milliseconds, clamped to something sane.
    pub fn millis(payload: &[u8]) -> u64 {
        payload
            .try_into()
            .map(i64::from_le_bytes)
            .unwrap_or(0)
            .clamp(16, 60 * 60 * 1000) as u64
    }
}

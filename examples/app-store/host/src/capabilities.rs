//! What the host lets an app do, one module per capability. Every function
//! here is reached through [`crate::store::Guest::answer`], which has already
//! checked that the app's manifest declares the capability and that the
//! payload is within [`crate::store::MAX_PAYLOAD_BYTES`].

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use app_store_frame as wire;

use crate::store::{MAX_INBOX, MAX_INBOX_BYTES};

/// A guest's bus deliveries, filled by other guests' publishes and drained
/// into its event batch on its next redraw.
pub type Inbox = Arc<Mutex<Mailbox>>;

#[derive(Default)]
pub struct Mailbox {
    events: VecDeque<wire::Event>,
    /// What those events carry. Counting them is not enough: a thousand
    /// deliveries of a megabyte is a gigabyte of the host's memory, and the
    /// whole inbox crosses into the subscriber's own on its next tick.
    bytes: usize,
    /// Deliveries this guest never got because it was not draining — faulted,
    /// or slower than whoever publishes. Shown in its status line.
    pub dropped: u64,
}

impl Mailbox {
    /// Keeps the newest deliveries that fit in [`MAX_INBOX`] events and
    /// [`MAX_INBOX_BYTES`]: a full inbox loses its oldest message, never the
    /// host's memory.
    fn deliver(&mut self, event: wire::Event) {
        let bytes = payload_len(&event);
        while self.events.len() >= MAX_INBOX || self.bytes + bytes > MAX_INBOX_BYTES {
            let Some(dropped) = self.events.pop_front() else {
                break;
            };
            self.bytes -= payload_len(&dropped);
            self.dropped += 1;
        }
        self.bytes += bytes;
        self.events.push_back(event);
    }

    pub fn take(&mut self) -> Vec<wire::Event> {
        self.bytes = 0;
        self.events.drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// What one delivery costs the host to hold and the guest to decode.
fn payload_len(event: &wire::Event) -> usize {
    match event {
        wire::Event::Response {
            result: Ok(bytes), ..
        } => bytes.len(),
        wire::Event::Response {
            result: Err(message),
            ..
        } => message.len(),
        _ => 0,
    }
}

pub mod host {
    //! The two things every app may ask for, capability or not.

    use std::io::Write;

    use crate::store::{MAX_LOG_BYTES, MAX_RANDOM_BYTES};

    /// A module has no stdout — `println!` inside one goes nowhere — so its
    /// lines come out of the store's stderr, tagged with who wrote them.
    ///
    /// Not `eprintln!`: that panics when the write fails, which would let a
    /// guest end the whole host by logging into a closed stderr. The line is
    /// the guest's, so it is truncated and escaped as well — otherwise it
    /// could forge another app's tag or drive the operator's terminal.
    pub fn log(app: &str, message: &[u8]) {
        let end = message.len().min(MAX_LOG_BYTES);
        let text: String = String::from_utf8_lossy(&message[..end])
            .escape_debug()
            .collect();
        let _ = writeln!(std::io::stderr().lock(), "[{app}] {text}");
    }

    /// A `u32` count of bytes, little-endian; the only entropy in a module,
    /// which cannot link `getrandom` without JS glue. A payload of any other
    /// width is refused rather than read as zero: an app that sent a `u64`
    /// would otherwise seed itself from an empty `Ok`.
    pub fn random(payload: &[u8]) -> Result<Vec<u8>, String> {
        let count = payload
            .try_into()
            .map(u32::from_le_bytes)
            .map_err(|_| "a count that is not a `u32`".to_string())? as usize;
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

    use crate::store::{MAX_APP_KEYS, MAX_APP_STORAGE, MAX_VALUE_BYTES};

    /// What one key costs even when its value is empty: a directory entry and
    /// a block. Without it a quota counted in bytes never moves.
    const BLOCK_BYTES: u64 = 4096;

    const DEFAULT_DIR: &str = "target/app-store-data";

    /// Where a value lands before it is renamed over the key it replaces.
    /// A leading dot is not a legal key, so it can never be one.
    const TEMP_NAME: &str = ".tmp";

    /// Everything the host keeps between runs: one directory per app, plus
    /// the store's own `installed` list.
    pub fn data_dir() -> PathBuf {
        PathBuf::from(std::env::var("APP_STORE_DATA").unwrap_or_else(|_| DEFAULT_DIR.to_string()))
    }

    /// Names Windows resolves to a device in every directory, with or without
    /// an extension and in any case. A key is a file name, so the host must
    /// refuse them here rather than discover them on the one platform where
    /// `storage.set "nul\n…"` writes to the null device.
    const DEVICES: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    fn path(app: &str, key: &[u8]) -> Result<PathBuf, String> {
        let key = std::str::from_utf8(key).map_err(|_| "a key that is not UTF-8".to_string())?;
        let stem = key
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let plain = !key.is_empty()
            && !key.starts_with('.')
            // Win32 strips a trailing dot or space, so `todo.` would land in
            // `todo`; a space never passes the character set below anyway.
            && !key.ends_with('.')
            && !key.ends_with(' ')
            && !DEVICES.contains(&stem.as_str())
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
        if !plain {
            return Err(format!("`{key}` is not a storage key"));
        }
        Ok(data_dir().join(app).join(key))
    }

    /// A crash mid-write must leave a file holding either the old bytes or the
    /// new ones, never half of each, so every write lands in a sibling temp
    /// file and is renamed over its name. Not fsync'd: a power cut can still
    /// lose the last write.
    pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let dir = path
            .parent()
            .ok_or(std::io::ErrorKind::InvalidInput)?
            .to_path_buf();
        let temp = dir.join(TEMP_NAME);
        std::fs::write(&temp, bytes)?;
        std::fs::rename(temp, path)
    }

    /// The value, or nothing if the key was never written.
    pub fn get(app: &str, key: &[u8]) -> Result<Vec<u8>, String> {
        match std::fs::read(path(app, key)?) {
            Ok(bytes) => Ok(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error.to_string()),
        }
    }

    /// `key\nvalue`. `held` is what the app's directory was last known to
    /// hold, scanned on first use and kept from there: the host is its only
    /// writer, and a walk per write is what makes a tick of 256 sets a
    /// quarter of a million `stat`s.
    pub fn set(
        app: &str,
        payload: &[u8],
        held: &mut Option<(u64, usize)>,
    ) -> Result<Vec<u8>, String> {
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
        let (used, keys) = *held.get_or_insert_with(|| scan(dir));
        // What this key holds today, which the write replaces: one `stat`,
        // not the directory again. Both totals are then what the app would
        // hold afterwards whether the key is new or replaced.
        let (replaced, replacing) = std::fs::metadata(&path)
            .ok()
            .filter(std::fs::Metadata::is_file)
            .map_or((0, 0), |metadata| (metadata.len().max(BLOCK_BYTES), 1));
        let (used, keys) = (
            used.saturating_sub(replaced),
            keys.saturating_sub(replacing),
        );
        if keys + 1 > MAX_APP_KEYS {
            return Err(format!("more than {MAX_APP_KEYS} storage keys"));
        }
        let cost = (value.len() as u64).max(BLOCK_BYTES);
        if used + cost > MAX_APP_STORAGE {
            return Err(format!("more than {MAX_APP_STORAGE} bytes of storage"));
        }
        write_atomic(&path, value).map_err(|error| error.to_string())?;
        *held = Some((used + cost, keys + 1));
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

    /// What the app's directory holds and how many keys that is. One `stat`
    /// per key, so `MAX_APP_KEYS` is what keeps it short — and the caller
    /// keeps the answer rather than asking again on the next write.
    fn scan(dir: &Path) -> (u64, usize) {
        read_dir(dir)
            .filter_map(|entry| entry.metadata().ok())
            .filter(std::fs::Metadata::is_file)
            .fold((0, 0), |(bytes, keys), metadata| {
                (bytes + metadata.len().max(BLOCK_BYTES), keys + 1)
            })
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

    /// Drops one subscription and says whether there was one. Request ids are
    /// per guest — every module counts its own from zero — so the inbox is the
    /// other half of the key.
    pub fn cancel(id: u64, inbox: &Inbox) -> bool {
        let mut subscribers = SUBSCRIBERS.lock().expect("bus");
        let before = subscribers.len();
        subscribers.retain(|subscriber| {
            subscriber.id != id || !std::ptr::eq(subscriber.inbox.as_ptr(), Arc::as_ptr(inbox))
        });
        before != subscribers.len()
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

    static START: OnceLock<Instant> = OnceLock::new();

    /// Starts the clock the guests read. Called before the window opens: the
    /// origin is otherwise whenever the first guest happened to tick, so
    /// "host uptime" would mean "since the first app was installed".
    pub fn start() {
        let _ = START.set(Instant::now());
    }

    /// Milliseconds since the store started.
    pub fn uptime_ms(now: Instant) -> u64 {
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

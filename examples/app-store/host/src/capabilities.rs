//! What the host lets an app do, one module per capability. Every function
//! here is reached through [`crate::store::Guest::answer`], which has already
//! checked that the app's manifest declares the capability.

use std::sync::{Arc, Mutex, Weak};

use app_store_frame as wire;

/// A guest's bus deliveries, filled by other guests' publishes and drained
/// into its event batch on its next redraw.
pub type Inbox = Arc<Mutex<Vec<wire::Event>>>;

pub mod storage {
    //! One directory per app, one file per key. Nothing an app writes can
    //! name a path outside its own directory.

    use std::path::PathBuf;

    const DEFAULT_DIR: &str = "target/app-store-data";

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
        let dir = std::env::var("APP_STORE_DATA").unwrap_or_else(|_| DEFAULT_DIR.to_string());
        Ok(PathBuf::from(dir).join(app).join(key))
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
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        }
        std::fs::write(path, &payload[split + 1..]).map_err(|error| error.to_string())?;
        Ok(Vec::new())
    }
}

pub mod bus {
    //! Publish/subscribe between apps, through the host. A message is
    //! `topic\ntext`; a subscription names a topic or `*`.

    use super::*;

    struct Subscriber {
        topic: String,
        id: u64,
        inbox: Weak<Mutex<Vec<wire::Event>>>,
    }

    static SUBSCRIBERS: Mutex<Vec<Subscriber>> = Mutex::new(Vec::new());

    // ponytail: a subscription lives until its guest is dropped; an app that
    // drops its stream keeps receiving into the void until then. Add a
    // `Frame.cancels` list when an app needs to unsubscribe while alive.
    pub fn subscribe(topic: &[u8], id: u64, inbox: &Inbox) {
        SUBSCRIBERS.lock().expect("bus").push(Subscriber {
            topic: String::from_utf8_lossy(topic).into_owned(),
            id,
            inbox: Arc::downgrade(inbox),
        });
    }

    /// Delivers to every live subscriber of the topic; returns how many.
    pub fn publish(payload: &[u8]) -> usize {
        let text = String::from_utf8_lossy(payload);
        let topic = text
            .split_once('\n')
            .map_or(text.as_ref(), |(topic, _)| topic);
        let mut subscribers = SUBSCRIBERS.lock().expect("bus");
        subscribers.retain(|subscriber| subscriber.inbox.strong_count() > 0);
        let mut delivered = 0;
        for subscriber in subscribers.iter() {
            let listening = subscriber.topic == "*" || subscriber.topic == topic;
            let Some(inbox) = listening.then(|| subscriber.inbox.upgrade()).flatten() else {
                continue;
            };
            inbox.lock().expect("inbox").push(wire::Event::Response {
                id: subscriber.id,
                result: Ok(payload.to_vec()),
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

    use iced::time::Instant;

    /// Milliseconds since the store started.
    pub fn uptime_ms(now: Instant) -> u64 {
        static START: OnceLock<Instant> = OnceLock::new();
        let start = *START.get_or_init(|| now);
        now.saturating_duration_since(start).as_millis() as u64
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

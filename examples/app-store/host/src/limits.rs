//! What one guest may spend, and what it may make the host spend.

use std::time::Duration;

/// What one tick may burn before the host ends the app. Roughly one fuel
/// per wasm instruction; a busy frame of a list app is a few million.
pub(crate) const FUEL_PER_TICK: u64 = 200_000_000;

/// The most linear memory an app may grow to.
pub(crate) const MEMORY_LIMIT: usize = 64 << 20;

// ---------- what a hostile guest may not do ----------
//
// Fuel and memory bound what a module does to itself. These bound what it can
// make the host do: everything below is copied out of the guest, kept in the
// host's memory, or turned into a host wake-up.

/// The frame is copied out of guest memory every tick; past this the guest is
/// ended rather than followed into an allocation it chose.
pub(crate) const MAX_FRAME_BYTES: usize = 8 << 20;

/// Every request is answered before the next tick, so a module that asks in a
/// loop would otherwise queue answers faster than it drains them.
pub(crate) const MAX_REQUESTS_PER_TICK: usize = 256;

/// A payload crosses into the host and, on the bus, into other guests.
pub(crate) const MAX_PAYLOAD_BYTES: usize = 1 << 20;

/// Every ticker is a host wake-up; sixteen timers is already a busy app.
pub(crate) const MAX_TICKERS: usize = 16;

/// A guest that stops draining must not grow the host's memory; the oldest
/// deliveries go and the count of them is shown in the guest's status line.
pub(crate) const MAX_INBOX: usize = 1024;

/// One value. Bigger than this belongs in a file the app names, not in a
/// key/value store the host copies through wasm memory twice.
pub(crate) const MAX_VALUE_BYTES: u64 = 1 << 20;

/// Everything one app may store, summed over its directory on every write.
pub(crate) const MAX_APP_STORAGE: u64 = 64 << 20;

/// One `host.random` answer. Entropy is cheap; a 4 GB request is not.
pub(crate) const MAX_RANDOM_BYTES: usize = 4096;

/// One bus message. Smaller than a payload the host only reads, because this
/// one is copied into every subscriber's inbox and decoded inside every
/// subscriber's memory limit.
pub(crate) const MAX_BUS_BYTES: usize = 64 << 10;

/// What one guest's undrained inbox may hold, in bytes as well as events: a
/// thousand deliveries is only a bound if a delivery is bounded too.
pub(crate) const MAX_INBOX_BYTES: usize = 1 << 20;

/// Everything the host is still holding for one guest — mostly sleeps it was
/// asked to wake up for. Walked and re-partitioned on every redraw.
pub(crate) const MAX_DUE: usize = 1024;

/// Bus subscriptions per guest: every publish by anyone walks all of them.
pub(crate) const MAX_SUBSCRIPTIONS: usize = 64;

/// Theme subscriptions per guest: each one is an answer per mode change,
/// and an app needs one.
pub(crate) const MAX_THEME_SUBSCRIPTIONS: usize = 16;

/// One subscription's topic. Held for as long as the guest runs and compared
/// against on every publish by anyone, so the payload cap alone would let
/// sixty-four subscriptions pin sixty-four megabytes of the host's memory for
/// nothing.
pub(crate) const MAX_TOPIC_BYTES: usize = 256;

/// A cancel is cheap to send and not cheap to serve — each one walks the due
/// list, the tickers and the process-wide subscriber list. There is nothing
/// left to cancel past everything the host holds for one guest.
pub(crate) const MAX_CANCELS: usize =
    MAX_DUE + MAX_TICKERS + MAX_SUBSCRIPTIONS + MAX_REQUESTS_PER_TICK;

/// What one tick may carry in either direction: the answers, the payloads the
/// requests came with, and what a publish copied into every subscriber's
/// inbox. [`MAX_DUE`] counts answers, not their size, and an operation that
/// answers nothing — a `storage.set`, a publish — is not free for having said
/// nothing back.
pub(crate) const MAX_REPLY_BYTES_PER_TICK: usize = 4 << 20;

/// One `host.log` line, on the store's own stderr.
pub(crate) const MAX_LOG_BYTES: usize = 1024;

/// The panic message read out of a faulted guest and shaped in its window on
/// every frame, so bounded like the log line rather than by the memory limit.
pub(crate) const MAX_FAULT_BYTES: usize = 1024;

/// Keys per app, and how long the one directory scan behind a guest's
/// `storage_used` takes.
pub(crate) const MAX_APP_KEYS: usize = 1024;

/// How often at most a guest's publishes wake the other windows. The
/// messages are in the subscribers' inboxes the moment they are published;
/// the wake is what makes the other windows redraw to take them, and one
/// per frame from a guest that publishes every frame would repaint every
/// window at its frame rate. The last publish of a burst is never lost:
/// its wake comes when the interval is up.
pub(crate) const BUS_WAKE_INTERVAL: Duration = Duration::from_millis(50);

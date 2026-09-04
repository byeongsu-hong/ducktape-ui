//! Five ways an app can misbehave. Two never return — the host's fuel budget
//! and memory limit end them, and the store shows why. One panics, which is
//! the same end with a message the module wrote itself. The last two ask the
//! host for things it will not give: a capability the manifest never
//! declared, and more requests than one tick allows.

use ui_lang_guest::host;
use iced::futures::{Stream, StreamExt};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostError {
    pub message: String,
}

/// Never terminates on its own: the fuel budget for one tick runs out first.
pub fn spin() -> i64 {
    let mut x: u64 = 1;
    loop {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        if x == 0 {
            return 0;
        }
    }
}

/// Asks for more memory than the host allows: the grow traps. `black_box`
/// keeps the optimizer from noticing nobody reads the bytes and skipping
/// the allocation altogether — which it does.
pub fn hog() -> i64 {
    let bytes: Vec<u8> = std::hint::black_box(vec![1; 1 << 30]);
    bytes.len() as i64
}

/// Ends the instance with a message of the module's own: with
/// `panic = "abort"` the trap itself says nothing, so what the store shows
/// is what the sdk's panic hook parked for it.
pub fn boom() -> i64 {
    panic!("chaos: on purpose")
}

/// How many asks one tick of `flood` makes — comfortably past the host's
/// `MAX_REQUESTS_PER_TICK`.
pub const FLOOD: usize = 1_000;

/// A thousand asks before the host can answer one. `notify` keeps no slot,
/// so the answers to the first `FLOOD - 1` are dropped; the last is an
/// ordinary request, and the refusal it comes back with is the cap.
pub async fn flood() -> Result<bool, HostError> {
    for nth in 0..FLOOD - 1 {
        host::notify("host.echo", format!("flood {nth}").as_bytes());
    }
    host::request("host.echo", b"the last of a thousand")
        .await
        .map(|_| true)
        .map_err(|message| HostError { message })
}

/// `clock` is not in this app's manifest; the answer is the refusal.
pub async fn borrow_clock() -> Result<bool, HostError> {
    host::request("clock.sleep", &10_i64.to_le_bytes())
        .await
        .map(|_| true)
        .map_err(|message| HostError { message })
}

pub fn result_label(result: i64) -> String {
    format!("result: {result} (you should never read this)")
}

/// The host's colour mode: `light` or `dark`, once on subscribing and again
/// on every change.
pub fn theme_changes() -> impl Stream<Item = Result<String, HostError>> + Send + 'static {
    host::theme().map(|answer| {
        answer
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|message| HostError { message })
    })
}

//! Three ways an app can misbehave. Two never return — the host's fuel
//! budget and memory limit end them, and the store shows why. The third
//! asks for a capability the manifest never declared, and is refused.

use app_store_sdk::host;

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

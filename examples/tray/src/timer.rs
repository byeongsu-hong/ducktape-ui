//! The arithmetic a focus timer needs, kept out of the view.

const SESSION: i64 = 1500;

/// `m:ss`, wide enough that the menu bar label does not jitter as it counts.
pub fn clock(seconds: i64) -> String {
    let seconds = seconds.max(0);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The run's state as the menu reads it back, in one word.
pub fn phase(running: bool, remaining: i64) -> String {
    if remaining <= 0 {
        "DONE".to_owned()
    } else if running {
        "RUNNING".to_owned()
    } else if remaining == SESSION {
        "READY".to_owned()
    } else {
        "PAUSED".to_owned()
    }
}

/// What pressing the toggle row will do, which is also what the row says.
pub fn start_label(running: bool) -> String {
    if running { "Pause" } else { "Start" }.to_owned()
}

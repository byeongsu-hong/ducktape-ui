//! The arithmetic a focus timer needs, kept out of the view.

const SESSION: i64 = 1500;

/// `m:ss`, wide enough that the menu bar label does not jitter as it counts.
pub fn clock(seconds: i64) -> String {
    let seconds = seconds.max(0);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The run already spent, as a width across `width` points.
pub fn elapsed_width(remaining: i64, width: f64) -> f64 {
    let spent = (SESSION - remaining.clamp(0, SESSION)) as f64;
    width * spent / SESSION as f64
}

/// The rest of the rail, so the two halves always sum to `width`.
pub fn remaining_width(remaining: i64, width: f64) -> f64 {
    width - elapsed_width(remaining, width)
}

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

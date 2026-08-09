//! The arithmetic a focus timer needs, kept out of the view.

/// `m:ss`, wide enough that the menu bar label does not jitter as it counts.
pub fn clock(seconds: i64) -> String {
    let seconds = seconds.max(0);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The run's state as the menu reads it back, in one word.
pub fn phase(running: bool, remaining: i64, session: i64) -> String {
    if remaining <= 0 {
        "DONE".to_owned()
    } else if running {
        "RUNNING".to_owned()
    } else if remaining == session {
        "READY".to_owned()
    } else {
        "PAUSED".to_owned()
    }
}

/// One row of the session-length submenu, marked when it is the length in use.
///
/// The mark is the row's own text rather than a native checkmark: a submenu
/// row is a `str` expression like every other row, so what a menu bar draws as
/// a tick is spelled here as the string the row says.
pub fn length_label(session: i64, choice: i64) -> String {
    let mark = if session == choice { '•' } else { ' ' };
    format!("{mark} {} minutes", choice / 60)
}

/// What pressing the toggle row will do, which is also what the row says.
pub fn start_label(running: bool) -> String {
    if running { "Pause" } else { "Start" }.to_owned()
}

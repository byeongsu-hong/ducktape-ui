//! The chats this window has had, kept by this window.
//!
//! The CLI's own rollouts were read here before, and they are the wrong record
//! to keep: what they preserve is what the API was sent and what the CLI drew,
//! and the two most interesting parts of a turn survive neither. Reasoning is
//! encrypted in the raw stream and empty in the drawn one — 0 of 14,319 items
//! on the machine this was measured on carried any text — so a chat read back
//! showed questions, answers and tools, and nothing of how the answer was
//! arrived at.
//!
//! This window already holds that. A turn is a list of `Entry` rows in the
//! order they happened, reasoning included, and the input the next turn would
//! resend sits beside it. Writing both out is the whole of this file, and what
//! comes back is the turn as it was drawn rather than a reconstruction of it.
//!
//! One file per chat, JSONL, rewritten whole when a turn ends:
//!
//! ```text
//! {"title":"Which iced is current?","when":"2026-08-10","model":"gpt-5.6-sol","turns":2}
//! {"row":{"id":1,"kind":"prompt",…}}
//! {"input":{"type":"message","role":"user",…}}
//! ```
//!
//! Meta first so that listing a chat reads one line rather than a whole file.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use smol::channel::{Receiver, Sender};

use crate::codex::{CodexError, Entry, Session};

/// How many chats to offer. They are the most recent ones.
const CHATS: usize = 200;
/// How many rows one chat may put on screen. A long chat can outrun what
/// anyone will scroll, and a transcript nobody reaches the end of is not one.
const ROWS: usize = 500;

/// One chat this window had.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chat {
    /// The file it is kept in, which is what opening one is by.
    pub path: String,
    /// The first thing asked in it, which is the only name it has.
    pub title: String,
    /// The day it was last written to.
    pub when: String,
    /// Which model answered in it.
    pub model: String,
}

/// Where chats are kept. `AI_CHAT_HOME` moves the lot.
fn sessions_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("AI_CHAT_HOME") {
        return PathBuf::from(home).join("sessions");
    }
    // The suite drives whole turns through the real handlers, and those write.
    // Under test the store is somewhere disposable, so running the tests never
    // touches the chats of whoever ran them — and it is per run, so a chat one
    // run left behind is never a chat the next run finds.
    if cfg!(test) {
        return std::env::temp_dir()
            .join(format!("ai-chat-suite-{}", std::process::id()))
            .join("sessions");
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    base.join("ducktape-ai-chat").join("sessions")
}

/// Where a chat starting now will be kept.
///
/// The name is only ever an identity — what the list shows comes out of the
/// file — so the clock at the moment of asking is enough to tell two apart.
pub fn new_file() -> PathBuf {
    named_in(&sessions_dir())
}

fn named_in(dir: &Path) -> PathBuf {
    dir.join(format!("{}.jsonl", nanos()))
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

/// Write a chat out.
///
/// Through a temporary file and a rename, because the alternative — truncating
/// the real one and failing part way — turns a chat into half a chat. A failure
/// here is reported rather than swallowed: the caller draws it as a row, since
/// a window that quietly stops recording is worse than one that says it has.
pub fn save(file: &Path, rows: &[Entry], input: &[Value], model: &str) -> Result<(), CodexError> {
    let kept = |what: &str| CodexError::new(format!("This chat was not saved: {what}"));

    let Some(parent) = file.parent() else {
        return Err(kept("it has nowhere to go"));
    };
    std::fs::create_dir_all(parent).map_err(|error| kept(&error.to_string()))?;

    let mut text = json!({
        "title": title_of(rows),
        "when": today(),
        "model": model,
        "turns": rows.iter().map(|row| row.turn).max().unwrap_or(0),
    })
    .to_string();
    text.push('\n');
    for row in rows {
        let row = serde_json::to_value(row).map_err(|error| kept(&error.to_string()))?;
        text.push_str(&json!({ "row": row }).to_string());
        text.push('\n');
    }
    for item in input {
        text.push_str(&json!({ "input": item }).to_string());
        text.push('\n');
    }

    let staged = file.with_extension("writing");
    std::fs::write(&staged, text).map_err(|error| kept(&error.to_string()))?;
    std::fs::rename(&staged, file).map_err(|error| kept(&error.to_string()))
}

/// What the chat is called: the first thing asked in it.
fn title_of(rows: &[Entry]) -> String {
    rows.iter()
        .find(|row| row.kind == "prompt")
        .map(|row| first_line(&row.body))
        .unwrap_or_default()
}

/// Every chat in a directory, newest first.
fn files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    found.sort_by_key(|path| {
        std::cmp::Reverse(
            path.metadata()
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        )
    });
    found
}

/// A chat's name and date, from its first line alone.
fn chat_at(path: &Path) -> Option<Chat> {
    let mut first = String::new();
    BufReader::new(std::fs::File::open(path).ok()?)
        .read_line(&mut first)
        .ok()?;
    let meta: Value = serde_json::from_str(&first).ok()?;

    // A chat nobody asked anything in has no name and nothing to read. It can
    // exist — a turn that failed before the question went out writes one — and
    // listing it would offer an empty transcript under a blank line.
    let title = meta["title"].as_str().unwrap_or_default();
    if title.is_empty() {
        return None;
    }
    Some(Chat {
        path: path.to_string_lossy().into_owned(),
        title: title.to_owned(),
        when: meta["when"].as_str().unwrap_or_default().to_owned(),
        model: meta["model"].as_str().unwrap_or_default().to_owned(),
    })
}

/// How many chats to read before saying so. Small enough that the list fills
/// visibly, large enough not to wake the screen for every file.
const BATCH: usize = 24;

/// How far the scan has got, and what it has found so far.
#[derive(Clone, Debug, PartialEq)]
pub struct Scan {
    pub chats: Vec<Chat>,
    pub ratio: f64,
    pub found: i64,
    pub total: i64,
}

/// The chats on offer, read straight through.
///
/// This is what a turn ending uses: the chat that has just been had should be
/// in the sidebar, and by then it is one more line to read among the ones
/// already read. The scan below is for the other case — a window opening onto
/// a store it has not looked at yet.
pub fn recent_chats() -> Vec<Chat> {
    chats_in(&sessions_dir())
}

/// Every chat one directory offers, newest first.
///
/// The directory is a parameter so a test can be handed one of its own. The
/// alternative — pointing the whole store somewhere through the environment —
/// is a mutation two tests running at once would fight over.
fn chats_in(dir: &Path) -> Vec<Chat> {
    files(dir)
        .iter()
        .filter_map(|path| chat_at(path))
        .take(CHATS)
        .collect()
}

/// The chats on offer, handed over as they are found.
///
/// One line per file is a fast read and this store holds only what this window
/// wrote, so the list usually arrives at once. It is still published as it
/// fills rather than when it is done, because the cost is a directory that has
/// grown, and a list that only appears when complete looks broken while it does.
pub fn scan_chats() -> Receiver<Scan> {
    scan_in(sessions_dir())
}

fn scan_in(dir: PathBuf) -> Receiver<Scan> {
    let (sender, receiver) = scan_channel();
    std::thread::spawn(move || {
        let files = files(&dir);
        let total = files.len() as i64;
        let mut found: Vec<Chat> = Vec::new();
        for (index, path) in files.iter().enumerate() {
            if let Some(chat) = chat_at(path) {
                found.push(chat);
            }
            let read = index + 1;
            let full = found.len() >= CHATS;
            if read % BATCH == 0 || full || read == files.len() {
                let scan = Scan {
                    chats: found.clone(),
                    ratio: if total == 0 {
                        1.0
                    } else {
                        read as f64 / total as f64
                    },
                    found: found.len() as i64,
                    total,
                };
                // A closed channel is the window having moved on. Each
                // complete list supersedes the progress snapshot before it.
                if !publish_scan(&sender, scan) {
                    return;
                }
            }
            if full {
                return;
            }
        }
        // An empty store still has to say it finished, or the bar never leaves.
        if files.is_empty() {
            let _ = publish_scan(
                &sender,
                Scan {
                    chats: Vec::new(),
                    ratio: 1.0,
                    found: 0,
                    total: 0,
                },
            );
        }
    });
    receiver
}

fn scan_channel() -> (Sender<Scan>, Receiver<Scan>) {
    smol::channel::bounded(1)
}

fn publish_scan(sender: &Sender<Scan>, scan: Scan) -> bool {
    sender.force_send(scan).is_ok()
}

/// Open a chat, off the frame loop.
pub async fn open_recent(session: Session, path: String) -> Result<Vec<Entry>, CodexError> {
    smol::unblock(move || open_chat(session, path)).await
}

/// Open a chat: draw what it said, and take on what it would resend.
///
/// The session is replaced rather than added to, because this is that chat now
/// — carrying on from it sends its own history, and saving writes its file.
pub fn open_chat(session: Session, path: String) -> Result<Vec<Entry>, CodexError> {
    let file = std::fs::File::open(&path)
        .map_err(|error| CodexError::new(format!("That chat could not be opened: {error}")))?;

    let mut rows: Vec<Entry> = Vec::new();
    let mut input: Vec<Value> = Vec::new();
    let mut unreadable = 0usize;
    for line in BufReader::new(file).lines().map_while(Result::ok).skip(1) {
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            unreadable += 1;
            continue;
        };
        if let Some(row) = record.get("row") {
            match serde_json::from_value::<Entry>(row.clone()) {
                Ok(entry) => rows.push(entry),
                Err(_) => unreadable += 1,
            }
        } else if let Some(item) = record.get("input") {
            input.push(item.clone());
        }
    }

    let turn = rows.iter().map(|row| row.turn).max().unwrap_or(0);
    let dropped = rows.len().saturating_sub(ROWS);
    if dropped > 0 {
        rows = rows.split_off(dropped);
        // The cut takes the oldest rows, and a turn's folded working-out sits
        // behind the summary that unfolds it. A cut landing between the two
        // would leave rows nothing on screen could ever reveal.
        let unfoldable: std::collections::HashSet<i64> = rows
            .iter()
            .filter(|row| row.kind == "work")
            .map(|row| row.turn)
            .collect();
        rows.retain(|row| !row.hidden || unfoldable.contains(&row.turn));
    }

    // Said rather than swallowed. A chat that comes back short should say so,
    // or a mapping that has broken looks like a chat that was always this
    // length.
    let missing = match (dropped, unreadable) {
        (0, 0) => None,
        (0, bad) => Some(format!("{bad} rows could not be read")),
        (cut, 0) => Some(format!("{cut} earlier rows not shown")),
        (cut, bad) => Some(format!(
            "{cut} earlier rows not shown, {bad} could not be read"
        )),
    };
    if let Some(said) = missing {
        let mut note = Entry::of("note", said);
        note.turn = rows.first().map_or(0, |row| row.turn);
        rows.insert(0, note);
    }

    Ok(crate::codex::adopt(
        session,
        rows,
        input,
        turn,
        PathBuf::from(path),
    ))
}

/// A list with no directory behind it, so a capture and a test draw the panel
/// without a machine's real chats — or their titles — in it.
pub fn sample_chats() -> Vec<Chat> {
    let chat = |title: &str, when: &str, model: &str, id: u64| Chat {
        path: format!("/sessions/{id}.jsonl"),
        title: title.to_owned(),
        when: when.to_owned(),
        model: model.to_owned(),
    };
    vec![
        chat(
            "Which version of iced is current?",
            "2026-08-10",
            "gpt-5.6-sol",
            4,
        ),
        chat(
            "Explain how the parser handles indentation",
            "2026-08-09",
            "gpt-5.6-sol",
            3,
        ),
        chat(
            "Write a test for the SSE reader",
            "2026-08-08",
            "gpt-5.5",
            2,
        ),
        chat(
            "Why is this allocation showing up in the profile?",
            "2026-08-06",
            "gpt-5.4-mini",
            1,
        ),
    ]
}

/// A transcript of `rows` rows, for measuring what drawing one costs.
pub fn sample_transcript(rows: i64) -> Vec<Entry> {
    let one = crate::codex::sample_entries(false);
    (0..rows)
        .map(|index| {
            let mut row = one[index as usize % one.len()].clone();
            row.id = -(index + 1000);
            row.turn = index / 4 + 1;
            row
        })
        .collect()
}

fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    if line.chars().count() <= 90 {
        return line.to_owned();
    }
    format!("{}…", line.chars().take(90).collect::<String>())
}

/// Today, as `2026-08-10`.
///
/// The civil-from-days conversion rather than a date crate: one date is not
/// worth a dependency, and this is the standard algorithm for it.
fn today() -> String {
    day_of(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
    )
}

fn day_of(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of this test's own, so tests running at once never read or
    /// write each other's chats — nor the ones belonging to whoever ran them.
    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ai-chat-{name}-{}", nanos()))
    }

    fn row(kind: &str, title: &str, body: &str) -> Entry {
        let mut row = Entry::of(kind, title);
        row.body = body.to_owned();
        row.turn = 1;
        row
    }

    #[test]
    fn scan_buffer_keeps_latest_progress_and_stops_without_a_receiver() {
        let step = |found: i64| Scan {
            chats: Vec::new(),
            ratio: found as f64 / 3.0,
            found,
            total: 3,
        };
        let (sender, receiver) = scan_channel();
        assert_eq!(receiver.capacity(), Some(1));
        assert!(publish_scan(&sender, step(1)));
        assert!(publish_scan(&sender, step(2)));
        assert_eq!(receiver.len(), 1);
        assert_eq!(receiver.recv_blocking().expect("latest scan").found, 2);

        drop(receiver);
        assert!(
            !publish_scan(&sender, step(3)),
            "the scan must stop when the window stops listening"
        );
    }

    /// One turn as this window records it: what was asked, what the model was
    /// doing, and what it said.
    fn chat(asked: &str, answered: &str) -> Vec<Entry> {
        vec![
            row("prompt", "", asked),
            row("reasoning", "Checking the crate", "It could have moved."),
            row("answer", "", answered),
        ]
    }

    /// The whole point of keeping our own record: a turn goes out and comes
    /// back as the turn, reasoning included. That is the part the CLI's
    /// rollouts do not keep, and the reason this file exists.
    #[test]
    fn a_saved_chat_comes_back_as_the_turn_that_was_had() {
        let rows = chat("Which iced is current?", "0.14.0.");
        let input = vec![json!({"type": "message", "role": "user"})];
        let file = named_in(&scratch("roundtrip"));
        save(&file, &rows, &input, "gpt-5.6-sol").expect("it saves");

        let session = crate::codex::codex_session();
        let back = open_chat(session, file.to_string_lossy().into_owned()).expect("it opens");
        let kinds: Vec<&str> = back.iter().map(|row| row.kind.as_str()).collect();
        assert_eq!(kinds, ["prompt", "reasoning", "answer"]);
        assert_eq!(back[0].body, "Which iced is current?");
        assert_eq!(
            back[1].title, "Checking the crate",
            "the reasoning is what a rollout could not give back"
        );
        assert_eq!(back[1].body, "It could have moved.");
        assert_eq!(back[2].body, "0.14.0.");
    }

    /// What the sidebar draws comes off one line of each file, and it has to
    /// be the right line for the right chat.
    #[test]
    fn the_listing_names_each_chat_by_what_was_asked_in_it() {
        let dir = scratch("listing");
        for (asked, model) in [
            ("Which iced is current?", "gpt-5.6-sol"),
            ("Why is this slow?", "gpt-5.5"),
        ] {
            save(&named_in(&dir), &chat(asked, "…"), &[], model).expect("it saves");
        }

        let mut listed = Vec::new();
        let scan = scan_in(dir.clone());
        while let Ok(step) = scan.recv_blocking() {
            listed = step.chats;
        }
        assert_eq!(
            listed.len(),
            chats_in(&dir).len(),
            "reading straight through and filling as it goes must offer the same chats"
        );
        let mut named: Vec<(String, String)> = listed
            .into_iter()
            .map(|found| (found.title, found.model))
            .collect();
        named.sort();
        assert_eq!(
            named,
            [
                (
                    "Which iced is current?".to_owned(),
                    "gpt-5.6-sol".to_owned()
                ),
                ("Why is this slow?".to_owned(), "gpt-5.5".to_owned()),
            ]
        );
    }

    /// A turn that failed before the question went out leaves a file with
    /// nothing in it. Offering it would be a blank line that opens onto
    /// nothing.
    #[test]
    fn a_chat_with_nothing_asked_in_it_is_not_offered() {
        let dir = scratch("empty");
        save(&named_in(&dir), &[], &[], "gpt-5.6-sol").expect("it saves");
        save(
            &named_in(&dir),
            &chat("A real question", "…"),
            &[],
            "gpt-5.6-sol",
        )
        .expect("it saves");
        let listed: Vec<String> = chats_in(&dir)
            .into_iter()
            .map(|found| found.title)
            .collect();
        assert_eq!(listed, ["A real question"]);
    }

    /// Truncating the real file and failing part way through the write turns a
    /// chat into half a chat, so the write lands somewhere else and is moved
    /// into place. What that must not leave behind is a listing entry for the
    /// half-written copy.
    #[test]
    fn a_chat_is_never_left_half_written() {
        let dir = scratch("atomic");
        let file = named_in(&dir);
        save(&file, &chat("First question", "…"), &[], "gpt-5.6-sol").expect("it saves");
        save(&file, &chat("Second question", "…"), &[], "gpt-5.6-sol").expect("it saves again");

        let staged: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("the directory")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext != "jsonl"))
            .collect();
        assert!(staged.is_empty(), "left behind: {staged:?}");
        assert_eq!(
            chat_at(&file).expect("a chat").title,
            "Second question",
            "the rewrite replaced the chat rather than appending to it"
        );
    }

    /// A chat that comes back short has to say so. Silently returning fewer
    /// rows than were written is a broken mapping wearing the face of a chat
    /// that was always this length.
    #[test]
    fn a_chat_that_comes_back_short_says_so() {
        let dir = scratch("short");
        let file = named_in(&dir);
        save(
            &file,
            &chat("Which iced is newest?", "0.14.0."),
            &[],
            "gpt-5.6-sol",
        )
        .expect("it saves");

        let whole = std::fs::read_to_string(&file).expect("the file");
        let mut lines: Vec<String> = whole.lines().map(str::to_owned).collect();
        lines.insert(2, "{\"row\":{\"kind\":\"from a later format\"}}".to_owned());
        std::fs::write(&file, lines.join("\n")).expect("it rewrites");

        let back = open_chat(
            crate::codex::codex_session(),
            file.to_string_lossy().into_owned(),
        )
        .expect("it opens");
        assert_eq!(back.first().map(|row| row.kind.as_str()), Some("note"));
        assert_eq!(back[0].title, "1 rows could not be read");
        assert!(
            back.iter().any(|row| row.body == "0.14.0."),
            "and the rows that did read are still there"
        );
    }

    /// The date the sidebar shows is computed here rather than fetched, so it
    /// is worth knowing it is the right one.
    #[test]
    fn a_timestamp_becomes_the_day_it_falls_on() {
        assert_eq!(day_of(0), "1970-01-01");
        assert_eq!(day_of(1_770_000_000), "2026-02-02");
        // A leap day, which is where a hand-written conversion goes wrong.
        assert_eq!(day_of(1_709_164_800), "2024-02-29");
    }
}

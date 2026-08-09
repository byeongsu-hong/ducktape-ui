//! Chats the Codex CLI has already had, read back off disk.
//!
//! The CLI writes one JSONL rollout per session under
//! `~/.codex/sessions/YYYY/MM/DD/`. Two streams run through it: the raw
//! `response_item` records, which are what was resent to the API, and the
//! `event_msg/item_completed` records, which are what the CLI drew. This reads
//! the second, because it is already the list this window draws — reasoning
//! with its summary in the clear, commands with their output, file changes,
//! the answer — and mapping it needs no interpretation.
//!
//! The raw stream is read too, so a chat opened here can be carried on rather
//! than only looked at: it is exactly the `input` the next turn has to resend.
//!
//! These files are large — a median of 2MB and a long tail past 90MB — so the
//! listing reads only each file's head, and loading one streams it a line at a
//! time and keeps a bounded number of rows.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::codex::{CodexError, Entry, Session};

/// How much of a file to read when all that is wanted is its name and date.
///
/// Measured across 120 sampled rollouts, the first question someone actually
/// typed — as opposed to the context the CLI injects ahead of it — sits a
/// median of 71KB in, three quarters within 99KB and nine tenths within 825KB.
/// Reading stops the moment it is found, so this bound is only ever paid by a
/// rollout that buries its question or has none at all.
const HEAD_BYTES: u64 = 1024 * 1024;
/// How many chats to offer. They are the most recent ones.
const CHATS: usize = 120;
/// How many rows one chat may put on screen. A rollout can hold thousands, and
/// a transcript nobody can scroll to the end of is not a transcript.
const ROWS: usize = 500;
/// A tool's output, cut to what a row can hold.
const OUTPUT: usize = 1_200;

/// One chat that has already happened.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chat {
    /// The rollout's path, which is what opening one is by.
    pub path: String,
    /// The first thing asked in it, which is the only name it has.
    pub title: String,
    /// When it started, as the file records it.
    pub when: String,
    /// Where it was had.
    pub cwd: String,
}

/// Text the CLI puts in front of a session in the shape of a user message.
///
/// It arrives as a `response_item` with a user role, indistinguishable by
/// shape from something typed, so it is told apart by what it opens with. Left
/// unfiltered it becomes the chat's name — one rollout here was titled
/// `# AGENTS.md instructions for /home/…` — and its first turn.
const INJECTED: [&str; 5] = [
    "<environment_context",
    "<recommended_plugins",
    "<codex_internal_context",
    "<user_instructions",
    "# AGENTS.md instructions for",
];

fn is_injected(text: &str) -> bool {
    let text = text.trim_start();
    INJECTED.iter().any(|marker| text.starts_with(marker))
}

fn sessions_root() -> PathBuf {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        return PathBuf::from(home).join("sessions");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join(".codex")
        .join("sessions")
}

/// Every rollout on disk, newest first.
fn rollouts() -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&sessions_root(), &mut found, 0);
    found.sort_by_key(|path| {
        std::cmp::Reverse(
            path.metadata()
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        )
    });
    found
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>, depth: usize) {
    // The CLI files these under year/month/day, so there is no reason to
    // descend past that and every reason not to.
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found, depth + 1);
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            found.push(path);
        }
    }
}

/// A rollout's name and date, from its head alone.
fn chat_at(path: &Path) -> Option<Chat> {
    let file = std::fs::File::open(path).ok()?;
    let head = BufReader::new(file.take(HEAD_BYTES));

    let (mut when, mut cwd, mut title) = (String::new(), String::new(), String::new());
    for line in head.lines().map_while(Result::ok) {
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let payload = &record["payload"];
        match record["type"].as_str().unwrap_or_default() {
            "session_meta" => {
                when = payload["timestamp"].as_str().unwrap_or_default().to_owned();
                cwd = payload["cwd"].as_str().unwrap_or_default().to_owned();
            }
            "event_msg" if payload["type"] == "user_message" && title.is_empty() => {
                title = first_line(payload["message"].as_str().unwrap_or_default());
            }
            // Most rollouts record the question only here, so a head that
            // looked for the event alone found a name for almost none of them.
            "response_item"
                if payload["type"] == "message"
                    && payload["role"] == "user"
                    && title.is_empty() =>
            {
                let asked = text_of(&payload["content"]);
                if !is_injected(&asked) {
                    title = first_line(&asked);
                }
            }
            _ => {}
        }
        if !when.is_empty() && !title.is_empty() {
            break;
        }
    }

    // A rollout nobody asked anything in is not a chat to reopen: an exec run,
    // a sub-agent's own session, a start that went nowhere. Listing them under
    // a placeholder name fills the sidebar with things that open onto machine
    // output and no question. The name is the test because it is the same
    // thing: what a person typed, as opposed to what the CLI injected.
    if when.is_empty() || title.is_empty() {
        return None;
    }
    Some(Chat {
        path: path.to_string_lossy().into_owned(),
        title,
        when: day_of(&when),
        cwd: last_component(&cwd),
    })
}

/// A list with no directory behind it, so a capture and a test draw the panel
/// without a machine's real chats — or their titles — in it.
pub fn sample_chats() -> Vec<Chat> {
    let chat = |title: &str, when: &str, cwd: &str| Chat {
        path: format!("/sessions/{when}-{cwd}.jsonl"),
        title: title.to_owned(),
        when: when.to_owned(),
        cwd: cwd.to_owned(),
    };
    vec![
        chat(
            "Which version of iced is current?",
            "2026-08-10",
            "ducktape-ui",
        ),
        chat(
            "Explain how the parser handles indentation",
            "2026-08-09",
            "ducktape-ui",
        ),
        chat("Write a test for the SSE reader", "2026-08-08", "ai-chat"),
        chat(
            "Why is this allocation showing up in the profile?",
            "2026-08-06",
            "trading",
        ),
    ]
}

/// How many rollouts to read before saying so. Small enough that the list
/// fills visibly, large enough not to wake the screen for every file.
const BATCH: usize = 24;

/// How far the scan has got, and what it has found so far.
#[derive(Clone, Debug, PartialEq)]
pub struct Scan {
    pub chats: Vec<Chat>,
    pub ratio: f64,
    pub found: i64,
    pub total: i64,
}

/// The chats on offer, handed over as they are found.
///
/// A thousand rollouts take a fifth of a second to read the heads of, which is
/// long enough to look broken if nothing happens meanwhile. So the list is
/// published as it fills rather than when it is done, and carries how far the
/// scan has got with it.
pub fn scan_chats() -> smol::channel::Receiver<Scan> {
    let (sender, receiver) = smol::channel::unbounded();
    std::thread::spawn(move || {
        let files = rollouts();
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
                // A closed channel is the window having moved on.
                if sender.send_blocking(scan).is_err() {
                    return;
                }
            }
            if full {
                return;
            }
        }
    });
    receiver
}

/// Open a chat, off the frame loop. A rollout runs to tens of megabytes.
pub async fn open_recent(session: Session, path: String) -> Result<Vec<Entry>, CodexError> {
    smol::unblock(move || open_chat(session, path)).await
}

/// Open a chat: draw what it said, and take on what it would resend.
///
/// The session is replaced rather than added to, because this is that chat now
/// — carrying on from it sends its own history, not this window's.
pub fn open_chat(session: Session, path: String) -> Result<Vec<Entry>, CodexError> {
    let file = std::fs::File::open(&path)
        .map_err(|error| CodexError::new(format!("That chat could not be opened: {error}")))?;

    let mut rows: Vec<Entry> = Vec::new();
    let mut input: Vec<Value> = Vec::new();
    let mut turn = 0i64;
    let mut usage = String::new();
    // What was asked can arrive three ways depending on how the session was
    // run, and some rollouts carry only the third. Collapsing by text is what
    // keeps a chat that carries two of them from saying everything twice.
    let mut said: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let payload = &record["payload"];
        match record["type"].as_str().unwrap_or_default() {
            // What the API was given, kept so the chat can be carried on —
            // and, for the sessions that recorded nothing else, the only place
            // the question survives.
            "response_item" => {
                if payload["type"] == "message" && payload["role"] == "user" {
                    let asked = text_of(&payload["content"]);
                    // Still resent — the model needs it — but not drawn as
                    // though someone typed it.
                    if !asked.trim().is_empty()
                        && !is_injected(&asked)
                        && said.insert(asked.clone())
                    {
                        turn += 1;
                        let mut row = Entry::of("prompt", "").with_body(asked);
                        row.turn = turn;
                        rows.push(row);
                    }
                }
                input.push(payload.clone());
            }
            "event_msg" => match payload["type"].as_str().unwrap_or_default() {
                "item_completed" => {
                    if let Some(mut row) = row_for(&payload["item"]) {
                        let repeat = matches!(row.kind.as_str(), "prompt" | "answer")
                            && !said.insert(row.body.clone());
                        if !repeat {
                            if row.kind == "prompt" {
                                turn += 1;
                            }
                            row.turn = turn;
                            rows.push(row);
                        }
                    }
                }
                // What was asked arrives as its own event in most rollouts and
                // as a drawn item in some. Both are taken, and a drawn one
                // repeating what the event already said is dropped.
                "user_message" => {
                    let asked = payload["message"].as_str().unwrap_or_default();
                    if !asked.trim().is_empty() && said.insert(asked.to_owned()) {
                        turn += 1;
                        let mut row = Entry::of("prompt", "").with_body(asked);
                        row.turn = turn;
                        rows.push(row);
                    }
                }
                // Most rollouts record the answer as its own event; the ones
                // that draw items record it there too, and the repeat is
                // dropped the same way a repeated prompt is.
                "agent_message" => {
                    let answered = payload["message"].as_str().unwrap_or_default();
                    if !answered.trim().is_empty() && said.insert(answered.to_owned()) {
                        let mut row = Entry::of("answer", "").with_body(answered);
                        row.turn = turn;
                        rows.push(row);
                    }
                }
                "token_count" => {
                    usage = tokens_of(&payload["info"]["total_token_usage"]);
                }
                _ => {}
            },
            _ => {}
        }
    }

    if !usage.is_empty() {
        let mut row = Entry::of("usage", "");
        row.detail = usage;
        row.turn = turn;
        rows.push(row);
    }

    let dropped = rows.len().saturating_sub(ROWS);
    if dropped > 0 {
        rows = rows.split_off(dropped);
        let mut note = Entry::of("note", format!("{dropped} earlier rows not shown"));
        note.turn = rows.first().map_or(0, |row| row.turn);
        rows.insert(0, note);
    }

    Ok(crate::codex::adopt(session, rows, input, turn))
}

/// One drawn item, as the row this window draws.
///
/// Everything the CLI drew has a place here, including the things this window
/// cannot itself produce — it runs no shell and writes no files — because a
/// chat is being read back, not re-run.
fn row_for(item: &Value) -> Option<Entry> {
    let kind = item["type"].as_str().unwrap_or_default();
    match kind {
        "UserMessage" => Some(Entry::of("prompt", "").with_body(text_of(&item["content"]))),
        "AgentMessage" => Some(Entry::of("answer", "").with_body(text_of(&item["content"]))),
        // Recorded, but empty in every rollout written so far: `summary_text`
        // and `raw_content` are both `[]` in all 14,319 reasoning items across
        // the 1,037 on this machine, and the raw record is encrypted. Read
        // either shape the field takes, and expect nothing.
        "Reasoning" => {
            let summary = text_of(&item["summary_text"]);
            let summary = if summary.trim().is_empty() {
                text_of(&item["raw_content"])
            } else {
                summary
            };
            if summary.trim().is_empty() {
                return None;
            }
            let (title, body) = crate::codex::headed(&summary);
            Some(Entry::of("reasoning", title).with_body(body))
        }
        "CommandExecution" => {
            let failed = item["exit_code"].as_i64().unwrap_or(0) != 0;
            let mut row = Entry::of("tool", "Ran a command");
            row.detail = clipped(item["command"].as_str().unwrap_or_default(), OUTPUT);
            row.body = clipped(
                item["aggregated_output"]
                    .as_str()
                    .or_else(|| item["formatted_output"].as_str())
                    .unwrap_or_default(),
                OUTPUT,
            );
            row.status = if failed { "failed" } else { "done" }.to_owned();
            Some(row)
        }
        "FileChange" => {
            let files: Vec<&str> = item["changes"]
                .as_object()
                .map(|changes| changes.keys().map(String::as_str).collect())
                .unwrap_or_default();
            let mut row = Entry::of("tool", "Changed files");
            row.detail = clipped(&files.join("\n"), OUTPUT);
            row.status = "done".to_owned();
            Some(row)
        }
        // A search the model ran. Dropping it made an answer that came from
        // the web read as though the model simply knew.
        "Extension" => {
            let mut row = Entry::of("tool", item["kind"].as_str().unwrap_or("Extension"));
            row.detail = clipped(
                item["query"]
                    .as_str()
                    .or_else(|| item["action"].as_str())
                    .unwrap_or_default(),
                OUTPUT,
            );
            row.body = clipped(&item["results"].to_string(), OUTPUT);
            row.status = "done".to_owned();
            Some(row)
        }
        // Where the CLI summarised older turns away, which is the only
        // explanation a reader gets for a model forgetting something.
        "ContextCompaction" => {
            let mut row = Entry::of("tool", "Compacted the context");
            row.status = "done".to_owned();
            Some(row)
        }
        "ImageView" => {
            let mut row = Entry::of("tool", "Viewed an image");
            row.detail = clipped(item["path"].as_str().unwrap_or_default(), OUTPUT);
            row.status = "done".to_owned();
            Some(row)
        }
        // Handing off to a sub-agent is worth a line; the polling in between
        // is not. `interacted` is 621 of 660 of these, and a transcript that
        // drew them all would be mostly them.
        "SubAgentActivity" => {
            let kind = item["kind"].as_str().unwrap_or_default();
            if !matches!(kind, "started" | "interrupted") {
                return None;
            }
            let mut row = Entry::of("tool", format!("Sub-agent {kind}"));
            row.detail = clipped(item["agent_path"].as_str().unwrap_or_default(), OUTPUT);
            row.status = if kind == "interrupted" {
                "failed"
            } else {
                "done"
            }
            .to_owned();
            Some(row)
        }
        // Deliberately not a row: every one of the 411 in this machine's
        // rollouts is `tool: "wait"`, the orchestrator waiting on a sub-agent.
        // Drawing them would add only noise, and denser than any other type.
        "CollabAgentToolCall" => None,
        "McpToolCall" => {
            let mut row = Entry::of(
                "tool",
                format!(
                    "{} · {}",
                    item["server"].as_str().unwrap_or("tool"),
                    item["tool"].as_str().unwrap_or_default()
                ),
            );
            row.detail = clipped(&item["arguments"].to_string(), OUTPUT);
            row.status = "done".to_owned();
            Some(row)
        }
        _ => None,
    }
}

/// Content that may be a string, or the API's list of parts.
fn text_of(content: &Value) -> String {
    if let Some(text) = content.as_str() {
        return text.to_owned();
    }
    content
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn tokens_of(usage: &Value) -> String {
    let count = |key: &str| usage[key].as_i64().unwrap_or(0);
    format!(
        "{} in · {} out · {} reasoning",
        crate::codex::grouped(count("input_tokens")),
        crate::codex::grouped(count("output_tokens")),
        crate::codex::grouped(count("reasoning_output_tokens")),
    )
}

fn first_line(text: &str) -> String {
    clipped(
        text.lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or(""),
        90,
    )
}

/// `2026-08-10T00:10:12.019Z` is a timestamp; `2026-08-10` is a date.
fn day_of(timestamp: &str) -> String {
    timestamp.split('T').next().unwrap_or(timestamp).to_owned()
}

fn last_component(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_owned()
}

fn clipped(text: &str, limit: usize) -> String {
    let text = text.trim();
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut cut = limit;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}…", &text[..cut])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(raw: &str) -> Value {
        serde_json::from_str(raw).expect("item")
    }

    /// The whole reason this reads the drawn stream rather than the raw one:
    /// a rollout's `response_item/reasoning` carries only encrypted content,
    /// while the drawn item carries the summary in the clear.
    #[test]
    fn a_past_reasoning_summary_comes_back_readable() {
        let row = row_for(&item(
            r#"{"type":"Reasoning","summary_text":"**Checking the crate**\n\nIt could have moved."}"#,
        ))
        .expect("a row");
        assert_eq!(row.kind, "reasoning");
        assert_eq!(row.title, "Checking the crate");
        assert_eq!(row.body, "It could have moved.");
    }

    /// A command that failed and one that did not must not look the same.
    #[test]
    fn a_failed_command_says_so() {
        let ran = |code: i64| {
            row_for(&item(&format!(
                r#"{{"type":"CommandExecution","command":"cargo test","exit_code":{code},
                    "aggregated_output":"ok"}}"#
            )))
            .expect("a row")
        };
        assert_eq!(ran(0).status, "done");
        assert_eq!(ran(101).status, "failed");
        assert_eq!(ran(0).detail, "cargo test");
        assert_eq!(ran(0).body, "ok");
    }

    /// This window runs no shell and writes no files, so these rows can only
    /// ever come from a chat read back — which is exactly why they are kept.
    #[test]
    fn what_this_window_cannot_do_is_still_drawn() {
        let files = row_for(&item(
            r#"{"type":"FileChange","changes":{"src/main.rs":{},"README.md":{}}}"#,
        ))
        .expect("a row");
        assert_eq!(files.kind, "tool");
        assert!(files.detail.contains("src/main.rs"));

        let mcp = row_for(&item(
            r#"{"type":"McpToolCall","server":"github","tool":"list_issues","arguments":{}}"#,
        ))
        .expect("a row");
        assert_eq!(mcp.title, "github · list_issues");
    }

    /// The CLI puts its own text in front of a session in the shape of a user
    /// message. Read as one it becomes the chat's name and its first turn —
    /// one rollout here was titled `# AGENTS.md instructions for /home/…`,
    /// which is neither a question nor anyone's business.
    #[test]
    fn what_the_cli_injected_is_not_what_anyone_asked() {
        for injected in [
            "<environment_context>\n  <cwd>/home/eddy</cwd>",
            "# AGENTS.md instructions for /home/eddy/dev/ducktape",
            "<recommended_plugins>Here is a list of plugins",
            "<codex_internal_context source=\"goal\">",
        ] {
            assert!(is_injected(injected), "not caught: {injected}");
        }
        for asked in [
            "Which version of iced is current?",
            "  # Why is this slow?",
            "Explain <environment_context> to me",
        ] {
            assert!(!is_injected(asked), "wrongly caught: {asked}");
        }
    }

    /// A search the model ran, a hand-off, and a compaction are all things a
    /// reader needs to see; the orchestrator waiting is not.
    #[test]
    fn the_drawn_items_worth_reading_become_rows_and_the_rest_do_not() {
        let row = |raw: &str| row_for(&item(raw));
        assert_eq!(
            row(r#"{"type":"Extension","kind":"web.search","query":"iced 0.14"}"#)
                .map(|row| (row.title, row.detail)),
            Some(("web.search".to_owned(), "iced 0.14".to_owned()))
        );
        assert_eq!(
            row(r#"{"type":"SubAgentActivity","kind":"started","agent_path":"reviewer"}"#)
                .map(|row| row.title),
            Some("Sub-agent started".to_owned())
        );
        assert!(
            row(r#"{"type":"SubAgentActivity","kind":"interacted"}"#).is_none(),
            "polling is not a step anyone reads"
        );
        assert!(
            row(r#"{"type":"CollabAgentToolCall","tool":"wait"}"#).is_none(),
            "nor is waiting"
        );
        assert!(row(r#"{"type":"ContextCompaction"}"#).is_some());
    }

    /// An item this build does not model is not a row, rather than an empty one.
    #[test]
    fn an_unmodelled_item_is_no_row_at_all() {
        assert!(row_for(&item(r#"{"type":"ThreadGoalUpdated"}"#)).is_none());
        assert!(row_for(&item(r#"{"type":"Reasoning","summary_text":"   "}"#)).is_none());
    }

    /// The one thing a unit test cannot check: that the mapping holds against
    /// what the CLI actually writes, at the size it actually writes it.
    #[test]
    #[ignore = "reads the CLI's own session directory, which a machine may not have"]
    fn a_real_rollout_comes_back_as_a_transcript() {
        let scan = scan_chats();
        let mut listed = Vec::new();
        while let Ok(step) = scan.recv_blocking() {
            listed = step.chats;
        }
        let Some(chat) = listed.into_iter().next() else {
            eprintln!("no rollouts on this machine");
            return;
        };
        let size = std::fs::metadata(&chat.path).map(|m| m.len()).unwrap_or(0);
        let started = std::time::Instant::now();
        let rows =
            open_chat(crate::codex::codex_session(), chat.path.clone()).expect("the chat opens");
        eprintln!(
            "{}KB in {:?} -> {} rows",
            size / 1024,
            started.elapsed(),
            rows.len()
        );

        let mut kinds = std::collections::BTreeMap::new();
        for row in &rows {
            *kinds.entry(row.kind.clone()).or_insert(0) += 1;
        }
        eprintln!("kinds: {kinds:?}");
        assert!(!rows.is_empty(), "a recorded chat has rows");
        assert!(
            rows.iter().any(|row| row.kind == "prompt"),
            "and something was asked in it"
        );
    }

    /// Reading every rollout through to list them would be minutes of IO for a
    /// list of names. The head of each file is all a name needs.
    #[test]
    #[ignore = "reads the CLI's own session directory, which a machine may not have"]
    fn listing_reads_only_what_a_name_needs() {
        let started = std::time::Instant::now();
        let mut chats = Vec::new();
        let scan = scan_chats();
        while let Ok(step) = scan.recv_blocking() {
            chats = step.chats;
        }
        let took = started.elapsed();
        eprintln!("{} chats in {took:?}", chats.len());
        for chat in chats.iter().take(3) {
            eprintln!("  {} · {} · {}", chat.when, chat.cwd, chat.title);
        }
        assert!(!chats.is_empty(), "there are rollouts on this machine");
        assert!(took < std::time::Duration::from_secs(10), "took {took:?}");
    }
}

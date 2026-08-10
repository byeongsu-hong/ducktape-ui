//! An audit of the chats already on this machine.
//!
//! The mapping in `history` has been wrong twice against real data — once
//! losing every prompt, once every answer — and both times a unit test on a
//! hand-written fixture said it was fine. This reads the rollouts themselves
//! and reports what comes out badly, so the next hole is found the same way.
//!
//! It never opens a network connection and never starts a turn.

#[cfg(test)]
mod audit {
    use crate::codex::codex_session;
    use crate::history::{Chat, open_chat, scan_chats};
    use std::collections::BTreeMap;

    /// What is wrong with one chat, in the words a fix would be written from.
    fn faults(chat: &Chat, rows: &[crate::codex::Entry], raw: &str) -> Vec<String> {
        let mut found = Vec::new();
        let count = |kind: &str| rows.iter().filter(|row| row.kind == kind).count();
        let (prompts, answers, tools, reasoning) = (
            count("prompt"),
            count("answer"),
            count("tool"),
            count("reasoning"),
        );

        if rows.is_empty() {
            found.push("no rows at all".into());
        }
        if answers > 0 && prompts == 0 {
            found.push(format!("{answers} answers but no question"));
        }
        if prompts > 0 && answers == 0 && tools == 0 {
            found.push(format!("{prompts} questions and nothing back"));
        }
        if chat.title == "Untitled chat" {
            found.push("no name".into());
        }
        if rows.iter().any(|row| {
            matches!(row.kind.as_str(), "answer" | "prompt") && row.body.trim().is_empty()
        }) {
            found.push("a message with no text".into());
        }
        if rows
            .iter()
            .any(|row| row.kind == "tool" && row.title.trim().is_empty())
        {
            found.push("a tool call with no name".into());
        }
        // Only a reasoning record that actually carries text and still does
        // not come back is a mapping hole. An empty one is the format.
        if reasoning == 0 && carries_reasoning_text(raw) {
            found.push("recorded reasoning that did not come back".into());
        }
        found
    }

    /// Whether any drawn reasoning item in this file has text in it at all.
    fn carries_reasoning_text(raw: &str) -> bool {
        raw.lines().any(|line| {
            let Ok(record) = serde_json::from_str::<serde_json::Value>(line) else {
                return false;
            };
            let item = &record["payload"]["item"];
            item["type"] == "Reasoning"
                && [&item["summary_text"], &item["raw_content"]]
                    .iter()
                    .any(|field| {
                        field.as_str().is_some_and(|text| !text.trim().is_empty())
                            || field.as_array().is_some_and(|parts| !parts.is_empty())
                    })
        })
    }

    /// Item types the drawn stream carries that `history` does not model.
    fn unmapped(raw: &str) -> Vec<String> {
        // Everything the parser has an arm for, including the one it answers
        // for by deliberately drawing nothing.
        let known = [
            "UserMessage",
            "AgentMessage",
            "Reasoning",
            "CommandExecution",
            "FileChange",
            "McpToolCall",
            "Extension",
            "ContextCompaction",
            "ImageView",
            "SubAgentActivity",
            "CollabAgentToolCall",
        ];
        let mut seen = Vec::new();
        for line in raw.lines() {
            let Ok(record) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let item = &record["payload"]["item"];
            if let Some(kind) = item["type"].as_str()
                && !known.contains(&kind)
                && !seen.contains(&kind.to_owned())
            {
                seen.push(kind.to_owned());
            }
        }
        seen
    }

    /// Read a slice of the machine's rollouts and say what came out badly.
    ///
    /// `AI_CHAT_QA_FROM` and `AI_CHAT_QA_TO` bound the slice, newest first, so
    /// the sweep can be split across several runs.
    #[test]
    #[ignore = "reads the CLI's own session directory"]
    fn audit_the_chats_on_this_machine() {
        let from: usize = std::env::var("AI_CHAT_QA_FROM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let to: usize = std::env::var("AI_CHAT_QA_TO")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(40);

        let scan = scan_chats();
        let mut chats = Vec::new();
        while let Ok(step) = scan.recv_blocking() {
            chats = step.chats;
        }
        let slice: Vec<Chat> = chats.into_iter().skip(from).take(to - from).collect();
        eprintln!("auditing {} chats [{from}..{to})\n", slice.len());

        let mut tally: BTreeMap<String, usize> = BTreeMap::new();
        let mut holes: BTreeMap<String, usize> = BTreeMap::new();
        let (mut clean, mut total_rows) = (0usize, 0usize);

        for chat in &slice {
            let raw = std::fs::read_to_string(&chat.path).unwrap_or_default();
            let started = std::time::Instant::now();
            let rows = match open_chat(codex_session(), chat.path.clone()) {
                Ok(rows) => rows,
                Err(error) => {
                    eprintln!("FAILED {} :: {}", short(&chat.path), error.message);
                    *tally.entry("would not open".into()).or_default() += 1;
                    continue;
                }
            };
            let took = started.elapsed();
            total_rows += rows.len();

            for kind in unmapped(&raw) {
                *holes.entry(kind).or_default() += 1;
            }
            let faults = faults(chat, &rows, &raw);
            if faults.is_empty() {
                clean += 1;
            } else {
                eprintln!(
                    "BAD  {:<28} {:>4} rows {:>7.0?}  {}",
                    short(&chat.path),
                    rows.len(),
                    took,
                    faults.join("; ")
                );
                for fault in faults {
                    *tally.entry(fault_kind(&fault)).or_default() += 1;
                }
            }
        }

        eprintln!(
            "\n{clean}/{} came out clean, {total_rows} rows total",
            slice.len()
        );
        eprintln!("faults:");
        for (fault, count) in &tally {
            eprintln!("  {count:>4}  {fault}");
        }
        eprintln!("unmapped drawn items:");
        for (kind, count) in &holes {
            eprintln!("  {count:>4}  {kind}");
        }
    }

    /// Group a fault by its shape, not its numbers.
    fn fault_kind(fault: &str) -> String {
        for shape in [
            "answers but no question",
            "questions and nothing back",
            "no rows at all",
            "no name",
            "a message with no text",
            "a tool call with no name",
            "recorded reasoning that did not come back",
        ] {
            if fault.contains(shape) {
                return shape.to_owned();
            }
        }
        fault.to_owned()
    }

    fn short(path: &str) -> String {
        path.rsplit('/')
            .next()
            .unwrap_or(path)
            .chars()
            .take(28)
            .collect()
    }
}

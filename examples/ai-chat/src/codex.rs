//! The Codex CLI's ChatGPT session, opened with the tokens its own headless
//! login already wrote, and a turn broken into the things a screen draws.
//!
//! `codex login` — browser or `--device-auth` — finishes by writing OAuth
//! tokens to `~/.codex/auth.json`. This app reads that file instead of running
//! a second login, and posts to the same ChatGPT backend the CLI posts to. No
//! token is copied anywhere else, and none of them cross into Ice.
//!
//! A turn is not one answer. It is an ordered run of reasoning summaries, tool
//! calls, answer text and a token bill, and the screen draws all of it — so
//! that is the shape handed over: a flat, ordered [`Entry`] list that only ever
//! grows. Text that arrives token by token is handed over as pieces instead,
//! through [`Chunk`], because appending to a parsed document is the difference
//! between a flat and a quadratic cost per token.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};
use smol::channel::{Receiver, Sender};

const RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
/// Used only when `~/.codex/config.toml` names no model.
const DEFAULT_MODEL: &str = "gpt-5.6-sol";
const INSTRUCTIONS: &str = "You are Codex, answering inside a desktop chat window. \
     Reply in Markdown. Search the web when a fact could have changed since training.";
/// A tool result this app cannot model is still drawn, cut to a length a card
/// can hold rather than pasted whole.
const DETAIL_LIMIT: usize = 400;

/// Ids are unique for the life of the process, not the life of a chat, because
/// the parsed-Markdown cache in `render` is keyed on them and a chat cleared
/// mid-session must not hand a new answer an old answer's layout.
static NEXT_ID: AtomicI64 = AtomicI64::new(1);

fn next_id() -> i64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Debug)]
pub struct CodexError {
    pub message: String,
}

impl CodexError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// One row of the transcript, in the order it happened.
///
/// Every row of the transcript is one of these — the prompt included — so the
/// screen renders a single flat list and a turn's interleaving survives
/// exactly as the model produced it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: i64,
    /// `prompt` · `reasoning` · `tool` · `answer` · `usage`
    pub kind: String,
    /// The row's heading — a tool's name, or what it did.
    pub title: String,
    /// One line under the heading: a query, a URL, a token count.
    pub detail: String,
    /// Markdown, for the rows that carry prose.
    pub body: String,
    /// `running` while a tool is still working, `done` once it is not.
    pub status: String,
    /// Which turn produced this row. Everything a turn did folds away together
    /// under one summary once it is finished.
    pub turn: i64,
    /// Whether a folded row is showing its body.
    pub open: bool,
    /// Folded away inside a closed summary. Hidden rows are filtered out of
    /// what the screen is handed, so a folded turn costs no widgets at all.
    pub hidden: bool,
    /// Which palette this row was handed over for.
    ///
    /// A row carries it because `lazy` rebuilds a row only when the row
    /// changes, and the palette decides how the row is drawn. Stamping it here
    /// is what makes a theme switch reach rows that are otherwise settled.
    pub dark: bool,
}

/// What a `lazy` boundary keys a row's redraw on.
///
/// Hashing is deliberately not derived. `lazy` hashes its dependency on every
/// frame, and a derived hash would walk every row's full answer text — which
/// costs about as much as rebuilding the row and defeats the boundary
/// entirely (measured in `main.rs`'s `perf` module).
///
/// Hashing the identity instead is sound because the rest of a row is
/// immutable: rows are only ever appended, a row's prose never changes after
/// it settles, and the two fields that do change — whether it is folded and
/// which palette it was stamped for — are both here.
impl std::hash::Hash for Entry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.open.hash(state);
        self.dark.hash(state);
        self.hidden.hash(state);
        // A running tool becomes a finished one in place, under one id.
        self.status.hash(state);
    }
}

impl Entry {
    fn new(kind: &str, title: impl Into<String>) -> Self {
        Self {
            id: next_id(),
            kind: kind.to_owned(),
            title: title.into(),
            detail: String::new(),
            body: String::new(),
            status: String::new(),
            turn: 0,
            open: false,
            hidden: false,
            dark: false,
        }
    }

    fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = clipped(detail.into());
        self
    }

    fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    fn status(mut self, status: &str) -> Self {
        self.status = status.to_owned();
        self
    }
}

/// Text arriving token by token, addressed to the surface that draws it.
///
/// There is a field per live surface rather than a kind to switch on, because
/// Ice handlers do not branch: the screen appends both unconditionally and an
/// empty one is a no-op. `status` is what the composer says at that moment,
/// carried by every chunk so progress is never inferred from what arrived.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub answer: String,
    pub thinking: String,
    pub status: String,
}

impl Chunk {
    fn status(status: &str) -> Self {
        Self {
            answer: String::new(),
            thinking: String::new(),
            status: status.to_owned(),
        }
    }

    fn answer(text: impl Into<String>) -> Self {
        Self {
            answer: text.into(),
            ..Self::status("Responding")
        }
    }

    fn thinking(text: impl Into<String>) -> Self {
        Self {
            thinking: text.into(),
            ..Self::status("Thinking")
        }
    }
}

/// The conversation, held once and shared by the screen and the worker thread.
///
/// Two views of the same chat live here: `input` is what the API is resent on
/// every turn — this app does not `store` — and `entries` is what the screen
/// draws. They are kept together because they must not disagree: an answer
/// drawn but not resent would vanish from the model's next reply.
#[derive(Clone)]
pub struct Session {
    state: Arc<Mutex<Transcript>>,
}

#[derive(Default)]
struct Transcript {
    input: Vec<Value>,
    entries: Vec<Entry>,
    /// Which turn is being recorded, so its rows can be gathered afterwards.
    turn: i64,
    /// When the turn in progress started, for what its summary says.
    started: Option<std::time::Instant>,
    dark: bool,
    /// Which model answers. Held here rather than read per turn, so choosing
    /// one applies to this chat and not to whatever the CLI is configured for.
    model: String,
    /// How hard it thinks first. Not every model offers the same levels, so
    /// this is reconciled against the model whenever the model changes.
    effort: String,
    /// Where the screen is listening while a turn runs, if it is.
    watcher: Option<Sender<Vec<Entry>>>,
}

impl Transcript {
    /// The list as the screen should draw it now.
    ///
    /// Rows folded inside a closed summary are left out rather than drawn and
    /// hidden, so a folded turn costs nothing to have on screen.
    fn snapshot(&self) -> Vec<Entry> {
        self.entries
            .iter()
            .filter(|row| !row.hidden)
            .map(|row| Entry {
                dark: self.dark,
                ..row.clone()
            })
            .collect()
    }

    fn push(&mut self, entry: Entry) {
        let turn = self.turn;
        self.entries.push(Entry { turn, ..entry });
    }

    /// Gather everything this turn did under one summary and fold it away.
    ///
    /// A finished turn's working-out is context, not the answer. It stays open
    /// while it is happening — that is the only time it is worth watching — and
    /// closes once there is an answer to read instead.
    fn close_work(&mut self) {
        let turn = self.turn;
        let is_work = |row: &Entry| {
            row.turn == turn && matches!(row.kind.as_str(), "reasoning" | "tool") && !row.hidden
        };
        let Some(first) = self.entries.iter().position(is_work) else {
            return;
        };
        let counted = self.entries.iter().filter(|row| is_work(row)).count();
        let seconds = self.started.map_or(0, |at| at.elapsed().as_secs() as i64);

        for row in self.entries.iter_mut().filter(|row| is_work(row)) {
            row.hidden = true;
        }
        let summary = Entry::new("work", worked_for(seconds, counted));
        self.entries.insert(first, Entry { turn, ..summary });
    }

    /// Show or hide one summary's rows.
    fn fold_work(&mut self, id: i64) {
        let Some(summary) = self.entries.iter_mut().find(|row| row.id == id) else {
            return;
        };
        summary.open = !summary.open;
        let (turn, open) = (summary.turn, summary.open);
        for row in self.entries.iter_mut() {
            if row.turn == turn && matches!(row.kind.as_str(), "reasoning" | "tool") {
                row.hidden = !open;
            }
        }
    }

    /// Hand the list to the screen, once, because it changed.
    fn publish(&mut self) {
        let Some(watcher) = self.watcher.clone() else {
            return;
        };
        if watcher.send_blocking(self.snapshot()).is_err() {
            self.watcher = None;
        }
    }
}

/// What a finished turn's summary says it did.
fn worked_for(seconds: i64, steps: usize) -> String {
    let steps = if steps == 1 {
        "1 step".to_owned()
    } else {
        format!("{steps} steps")
    };
    if seconds < 60 {
        return format!("Worked for {seconds}s · {steps}");
    }
    format!("Worked for {}m {}s · {steps}", seconds / 60, seconds % 60)
}

impl Session {
    fn lock(&self) -> MutexGuard<'_, Transcript> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Session")
    }
}

pub fn codex_session() -> Session {
    Session {
        state: Arc::new(Mutex::new(Transcript {
            model: codex_model(),
            effort: codex_effort(),
            ..Transcript::default()
        })),
    }
}

/// The models Codex itself knows this account can use.
///
/// Read from the catalogue the CLI keeps, so the list is the one `codex` would
/// offer rather than one this app invented. The model in force is always in it,
/// even when the catalogue has not been written yet — a picker that cannot show
/// the current selection is worse than no picker.
pub fn codex_models() -> Vec<String> {
    let mut models: Vec<String> = catalogue().iter().filter_map(slug_of).collect();
    let current = codex_model();
    if !models.contains(&current) {
        models.insert(0, current);
    }
    models
}

/// The whole model catalogue the CLI keeps.
fn catalogue() -> Vec<Value> {
    std::fs::read_to_string(codex_home().join("models_cache.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|cache| Some(cache["models"].as_array()?.clone()))
        .unwrap_or_default()
}

/// The catalogue names a model by `slug`; `id` is accepted in case it ever
/// does otherwise.
fn slug_of(model: &Value) -> Option<String> {
    model["slug"]
        .as_str()
        .or_else(|| model["id"].as_str())
        .map(str::to_owned)
}

fn entry_for(model: &str) -> Option<Value> {
    catalogue()
        .into_iter()
        .find(|item| slug_of(item).as_deref() == Some(model))
}

/// The levels this model offers, as the catalogue declares them.
///
/// They differ by model, which is why the picker is rebuilt whenever the model
/// changes rather than offering one fixed list.
pub fn codex_efforts(model: String) -> Vec<String> {
    let declared: Vec<String> = entry_for(&model)
        .and_then(|item| {
            Some(
                item["supported_reasoning_levels"]
                    .as_array()?
                    .iter()
                    .filter_map(|level| level["effort"].as_str().map(str::to_owned))
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap_or_default();
    if declared.is_empty() {
        return vec!["low".into(), "medium".into(), "high".into()];
    }
    declared
}

/// How hard the CLI is configured to think, or what this model defaults to.
pub fn codex_effort() -> String {
    let configured = config_value("model_reasoning_effort");
    let model = codex_model();
    let offered = codex_efforts(model.clone());
    if offered.contains(&configured) {
        return configured;
    }
    entry_for(&model)
        .and_then(|item| item["default_reasoning_level"].as_str().map(str::to_owned))
        .filter(|level| offered.contains(level))
        .or_else(|| offered.first().cloned())
        .unwrap_or_else(|| "medium".to_owned())
}

/// Answer as this model from the next turn on.
///
/// The effort comes along: a level the new model does not offer would be
/// rejected by the backend, so it falls back to that model's own default.
pub fn set_model(session: Session, model: String) -> String {
    let offered = codex_efforts(model.clone());
    let mut state = session.lock();
    state.model = model.clone();
    if !offered.contains(&state.effort) {
        state.effort = entry_for(&model)
            .and_then(|item| item["default_reasoning_level"].as_str().map(str::to_owned))
            .filter(|level| offered.contains(level))
            .or_else(|| offered.first().cloned())
            .unwrap_or_else(|| "medium".to_owned());
    }
    model
}

/// A fresh chat that keeps the model and effort the last one was using.
///
/// Starting over should clear what was said, not undo a choice about how to
/// answer.
pub fn new_chat(session: Session) -> Session {
    let (model, effort) = {
        let state = session.lock();
        (state.model.clone(), state.effort.clone())
    };
    let next = codex_session();
    {
        let mut state = next.lock();
        state.model = model;
        state.effort = effort;
    }
    next
}

/// Think this hard from the next turn on.
pub fn set_effort(session: Session, effort: String) -> String {
    session.lock().effort = effort.clone();
    effort
}

/// What the session settled on, after a model change reconciled it.
pub fn session_effort(session: Session) -> String {
    session.lock().effort.clone()
}

/// Draw the prompt the moment it is typed, before the socket is opened.
pub fn push_user(session: Session, text: String) -> Vec<Entry> {
    let mut state = session.lock();
    state.input.push(json!({
        "type": "message",
        "role": "user",
        "content": [{"type": "input_text", "text": text}],
    }));
    state.turn += 1;
    state.started = Some(std::time::Instant::now());
    state.push(Entry::new("prompt", "").body(text));
    state.snapshot()
}

/// Fold or unfold one row, and hand back the transcript.
///
/// The open flag lives on the row for the same reason the palette does: a
/// settled row sits behind `lazy`, which redraws it only when the row changes.
pub fn toggle_row(session: Session, id: i64) -> Vec<Entry> {
    let mut state = session.lock();
    let summary = state
        .entries
        .iter()
        .any(|row| row.id == id && row.kind == "work");
    if summary {
        state.fold_work(id);
    } else if let Some(row) = state.entries.iter_mut().find(|row| row.id == id) {
        row.open = !row.open;
    }
    state.snapshot()
}

/// Switch palettes, and hand back the transcript stamped for the new one.
///
/// The stamp is what makes settled rows redraw: they sit behind `lazy`, which
/// rebuilds a row only when the row itself changes.
pub fn set_palette(session: Session, dark: bool) -> Vec<Entry> {
    let mut state = session.lock();
    state.dark = dark;
    state.snapshot()
}

/// One turn's streamed text, and what the composer should say while it runs.
///
/// The request is blocking, so it runs on its own thread and reaches the async
/// side through a channel — a slow first token never touches the frame loop.
pub fn codex_turn(session: Session) -> impl iced::task::Straw<Vec<Entry>, Chunk, CodexError> {
    iced::task::sipper(move |mut sender| async move {
        let (chunks, incoming) = smol::channel::unbounded();
        let (outcome, settled) = smol::channel::bounded(1);
        std::thread::spawn(move || {
            let result = pump(&session, &chunks);
            drop(chunks);
            let _ = outcome.send_blocking(result);
        });

        while let Ok(chunk) = incoming.recv().await {
            sender.send(chunk).await;
        }
        settled
            .recv()
            .await
            .unwrap_or_else(|_| Err(CodexError::new("Codex stopped without answering.")))
    })
}

/// The same turn's row list, published only when it actually changes.
///
/// This is a second channel on purpose. Tool calls and settled blocks are rare
/// — a handful per turn — while text deltas are constant, and putting the list
/// on every delta would copy the whole transcript once per token.
///
/// The list published last in a turn is the one the turn settles with, so the
/// two channels cannot disagree about the transcript whichever order their
/// final messages arrive in.
///
/// The channel closes when the turn ends, so the stream reading it completes
/// rather than idling until the next turn replaces it.
pub fn codex_entries(session: Session) -> Receiver<Vec<Entry>> {
    let (sender, receiver) = smol::channel::unbounded();
    session.lock().watcher = Some(sender);
    receiver
}

/// Post the conversation and read the answer back event by event.
fn pump(session: &Session, chunks: &Sender<Chunk>) -> Result<Vec<Entry>, CodexError> {
    #[cfg(test)]
    if !wire_is_open() {
        return offline(session, chunks);
    }
    let auth = read_auth()?;
    let (input, model, effort) = {
        let state = session.lock();
        (
            state.input.clone(),
            state.model.clone(),
            state.effort.clone(),
        )
    };
    let body = json!({
        "model": model,
        "instructions": INSTRUCTIONS,
        "input": input,
        // Hosted, so the backend runs the search itself. Codex's own shell and
        // patch tools would need this window to execute them, which is a very
        // different program; a served tool still puts a real tool call on
        // screen, which is what a chat client has to be able to draw.
        "tools": [{"type": "web_search"}],
        "tool_choice": "auto",
        "parallel_tool_calls": false,
        "store": false,
        "stream": true,
        // The summary is asked for whatever the model defaults to, because
        // drawing the reasoning is half of what this window is for.
        "reasoning": {"effort": effort, "summary": "detailed"},
        "include": ["reasoning.encrypted_content"],
    });

    // One retry, and only for a login this app owns: a refresh rotates the
    // token it is given, and rotating the CLI's would break `codex` for a
    // login this window only borrowed.
    let mut auth = auth;
    let mut refreshed = false;
    let response = loop {
        let attempt = ureq::post(RESPONSES_URL)
            .config()
            // The rejection body carries the only useful part of a refusal —
            // which model is refused, or that the login expired — and
            // status-as-error would throw it away.
            .http_status_as_error(false)
            .build()
            .header("Authorization", &format!("Bearer {}", auth.access_token))
            .header("chatgpt-account-id", &auth.account_id)
            .header("OpenAI-Beta", "responses=experimental")
            .header("Accept", "text/event-stream")
            .header("originator", "codex_cli_rs")
            .send_json(&body)
            .map_err(|error| CodexError::new(format!("Could not reach Codex: {error}")))?;

        if attempt.status().as_u16() != 401 || refreshed {
            break attempt;
        }
        refreshed = true;
        let Some(token) = auth
            .refresh_token
            .clone()
            .filter(|_| auth.whose == crate::auth::Login::Ours)
        else {
            return Err(CodexError::new(
                "The Codex login has expired. Run `codex login` to renew it.",
            ));
        };
        crate::auth::refresh(&token)?;
        auth = read_auth()?;
    };

    let status = response.status();
    if !status.is_success() {
        let detail = response.into_body().read_to_string().unwrap_or_default();
        return Err(CodexError::new(format!(
            "Codex refused the request ({}): {}",
            status.as_u16(),
            reason(&detail)
        )));
    }

    let mut answer = String::new();
    let reader = BufReader::new(response.into_body().into_reader());
    for line in reader.lines() {
        let line =
            line.map_err(|error| CodexError::new(format!("Codex stream broke off: {error}")))?;
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        if let Some(failure) = failure(&event) {
            return Err(CodexError::new(failure));
        }
        for chunk in stream_text(&event, &mut answer) {
            // A closed channel is the screen having moved on, so there is
            // nothing left to draw for.
            if chunks.send_blocking(chunk).is_err() {
                return Err(CodexError::new("Turn cancelled."));
            }
        }
        record(session, &event);
    }

    Ok(settle(session, answer))
}

/// Close the turn: keep the answer for the next request, hand the finished
/// transcript over, and close the row channel so its stream completes.
fn settle(session: &Session, answer: String) -> Vec<Entry> {
    let mut state = session.lock();
    state.input.push(json!({
        "type": "message",
        "role": "assistant",
        "content": [{"type": "output_text", "text": answer}],
    }));
    state.publish();
    state.watcher = None;
    state.snapshot()
}

/// Whether this build may reach the ChatGPT backend.
///
/// Closed under test unless the run asks for it, so the ordinary suite never
/// depends on a login, a network, or tokens spent. `AI_CHAT_LIVE=1` opens it
/// for a run whose subject is the live API.
#[cfg(test)]
fn wire_is_open() -> bool {
    std::env::var_os("AI_CHAT_LIVE").is_some_and(|value| value == "1")
}

/// A turn with the same shape as a real one, played from fixed events.
///
/// It goes through the same parser and the same channels the wire does, so a
/// test that drives the screen exercises the real path rather than a stand-in
/// for it.
#[cfg(test)]
fn offline(session: &Session, chunks: &Sender<Chunk>) -> Result<Vec<Entry>, CodexError> {
    // The shape of a real turn, taken from one: the model reasons, searches,
    // reasons again, opens a page, then answers with a citation. Anything less
    // interleaved would not exercise what the transcript exists to draw.
    const TURN: &[&str] = &[
        r#"{"type":"response.created"}"#,
        r#"{"type":"response.reasoning_summary_text.delta","delta":"**Planning a source search**"}"#,
        r#"{"type":"response.output_item.done","item":{"type":"reasoning","summary":[
            {"type":"summary_text","text":"**Planning a source search**\n\nThe version could have moved since training, so it is worth a look."}]}}"#,
        r#"{"type":"response.output_item.added","item":{"type":"web_search_call"}}"#,
        r#"{"type":"response.output_item.done","item":{"type":"web_search_call","status":"completed",
            "action":{"type":"search","queries":["site:crates.io/crates/iced latest version","iced-rs releases"]}}}"#,
        r#"{"type":"response.output_item.done","item":{"type":"reasoning","summary":[
            {"type":"summary_text","text":"**Reading the changelog**"}]}}"#,
        r#"{"type":"response.output_item.added","item":{"type":"web_search_call"}}"#,
        r#"{"type":"response.output_item.done","item":{"type":"web_search_call","status":"completed",
            "action":{"type":"open_page","url":"https://raw.githubusercontent.com/iced-rs/iced/master/CHANGELOG.md"}}}"#,
        r#"{"type":"response.output_text.delta","delta":"The newest released `iced` is **0.14.0**. "}"#,
        r#"{"type":"response.output_text.delta","delta":"One addition is [reactive rendering](https://docs.rs/iced), which skips redraws nothing asked for."}"#,
        r#"{"type":"response.output_item.done","item":{"type":"message","content":[{"type":"output_text",
            "text":"The newest released `iced` is **0.14.0**. One addition is [reactive rendering](https://docs.rs/iced), which skips redraws nothing asked for."}]}}"#,
        r#"{"type":"response.completed","response":{"usage":{"input_tokens":22875,"output_tokens":321,
            "output_tokens_details":{"reasoning_tokens":257}}}}"#,
    ];
    let mut answer = String::new();
    for raw in TURN {
        let event: Value = serde_json::from_str(raw).expect("fixture events parse");
        for chunk in stream_text(&event, &mut answer) {
            if chunks.send_blocking(chunk).is_err() {
                return Err(CodexError::new("Turn cancelled."));
            }
        }
        record(session, &event);
    }
    Ok(settle(session, answer))
}

/// The two things that arrive in pieces, handed over as pieces.
fn stream_text(event: &Value, answer: &mut String) -> Vec<Chunk> {
    match event["type"].as_str().unwrap_or_default() {
        "response.created" | "response.in_progress" => vec![Chunk::status("Thinking")],
        "response.reasoning_summary_text.delta" => {
            vec![Chunk::thinking(event["delta"].as_str().unwrap_or_default())]
        }
        "response.reasoning_summary_part.done" => vec![Chunk::thinking("\n\n")],
        "response.output_text.delta" => {
            let delta = event["delta"].as_str().unwrap_or_default();
            answer.push_str(delta);
            vec![Chunk::answer(delta)]
        }
        "response.web_search_call.in_progress" => vec![Chunk::status("Searching")],
        "response.completed" => vec![Chunk::status("")],
        _ => Vec::new(),
    }
}

/// The rows of the transcript, appended as each item of the turn settles.
fn record(session: &Session, event: &Value) {
    let kind = event["type"].as_str().unwrap_or_default();
    if !matches!(
        kind,
        "response.output_item.added" | "response.output_item.done" | "response.completed"
    ) {
        return;
    }
    let item = &event["item"];
    let item_kind = item["type"].as_str().unwrap_or_default();
    let mut state = session.lock();

    match (kind, item_kind) {
        // A tool appears the moment it starts, so the screen can say so while
        // it is still working.
        // A call that is still working stays open; one that has finished
        // collapses to its own title. Mid-turn, that leaves exactly one step
        // expanded — the one actually happening.
        ("response.output_item.added", "web_search_call") => {
            let mut running = Entry::new("tool", "Searching the web").status("running");
            running.open = true;
            state.push(running);
        }
        ("response.output_item.done", "web_search_call") => {
            let (title, detail) = search_action(&item["action"]);
            let failed = item["status"].as_str() == Some("failed");
            if let Some(open) = state
                .entries
                .iter_mut()
                .rev()
                .find(|row| row.kind == "tool" && row.status == "running")
            {
                open.title = title;
                open.detail = clipped(detail);
                open.status = if failed { "failed" } else { "done" }.to_owned();
            }
        }
        ("response.output_item.done", "reasoning") => {
            let summary = summary_text(&item["summary"]);
            if !summary.trim().is_empty() {
                let (title, body) = headed(&summary);
                state.push(Entry::new("reasoning", title).body(body));
            }
        }
        ("response.output_item.done", "message") => {
            let text = message_text(&item["content"]);
            if !text.trim().is_empty() {
                state.push(Entry::new("answer", "").body(text));
            }
        }
        // Anything this build does not model is still a row, because a chat
        // window that silently drops part of a turn is misreporting it.
        ("response.output_item.done", other) if !other.is_empty() => {
            state.push(
                Entry::new("tool", other)
                    .detail(item["status"].as_str().unwrap_or("done"))
                    .body(format!(
                        "```json\n{}\n```",
                        serde_json::to_string_pretty(item).unwrap_or_default()
                    ))
                    .status("done"),
            );
        }
        ("response.completed", _) => {
            state.close_work();
            state.push(Entry::new("usage", "").detail(tokens(&event["response"]["usage"])));
        }
        _ => return,
    }
    state.publish();
}

/// What a web search actually did, said the way a person would say it.
fn search_action(action: &Value) -> (String, String) {
    match action["type"].as_str().unwrap_or_default() {
        "open_page" => (
            "Opened a page".to_owned(),
            action["url"].as_str().unwrap_or_default().to_owned(),
        ),
        "find_in_page" => (
            "Searched within a page".to_owned(),
            action["pattern"].as_str().unwrap_or_default().to_owned(),
        ),
        _ => {
            let queries: Vec<&str> = action["queries"]
                .as_array()
                .map(|list| list.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let detail = if queries.is_empty() {
                action["query"].as_str().unwrap_or_default().to_owned()
            } else {
                queries.join("  ·  ")
            };
            ("Searched the web".to_owned(), detail)
        }
    }
}

/// A reasoning summary states its own subject on a bold first line. That line
/// is the row's heading, so the fold shows what was being thought about rather
/// than a generic label, and the body underneath is left as plain prose.
fn headed(summary: &str) -> (String, String) {
    let (first, rest) = summary.split_once('\n').unwrap_or((summary, ""));
    let heading = first.trim().trim_matches('*').trim();
    if heading.is_empty() || !first.trim().starts_with("**") {
        return ("Thought process".to_owned(), summary.trim().to_owned());
    }
    (heading.to_owned(), rest.trim().to_owned())
}

fn summary_text(summary: &Value) -> String {
    summary
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}

fn message_text(content: &Value) -> String {
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

/// The stream's own way of failing, as opposed to the socket's.
fn failure(event: &Value) -> Option<String> {
    match event["type"].as_str().unwrap_or_default() {
        "error" => Some(reason(&event.to_string())),
        "response.failed" => Some(reason(&event["response"]["error"].to_string())),
        "response.incomplete" => Some(format!(
            "Codex stopped early: {}",
            event["response"]["incomplete_details"]["reason"]
                .as_str()
                .unwrap_or("no reason given")
        )),
        _ => None,
    }
}

fn tokens(usage: &Value) -> String {
    let count = |key: &str| grouped(usage[key].as_i64().unwrap_or(0));
    let reasoning = grouped(
        usage["output_tokens_details"]["reasoning_tokens"]
            .as_i64()
            .unwrap_or(0),
    );
    format!(
        "{} in · {} out · {reasoning} reasoning",
        count("input_tokens"),
        count("output_tokens"),
    )
}

/// Thousands grouped, because a turn's input runs to five figures and an
/// ungrouped one has to be counted rather than read.
fn grouped(count: i64) -> String {
    let digits = count.abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    if count < 0 {
        out.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A one-line field, kept to one line's worth of text.
fn clipped(mut text: String) -> String {
    if text.len() > DETAIL_LIMIT {
        let mut cut = DETAIL_LIMIT;
        // Back off to a char boundary, so a multi-byte glyph is never halved.
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
        text.push('…');
    }
    text
}

/// The sentence inside an error body, or the body when it has no shape.
fn reason(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return body.trim().to_owned();
    };
    for path in [&["detail"][..], &["error", "message"], &["message"]] {
        let mut found = &value;
        for key in path {
            found = &found[*key];
        }
        if let Some(text) = found.as_str() {
            return text.to_owned();
        }
    }
    body.trim().to_owned()
}

struct Auth {
    access_token: String,
    account_id: String,
    refresh_token: Option<String>,
    whose: crate::auth::Login,
}

fn codex_home() -> PathBuf {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        return PathBuf::from(home);
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".codex")
}

fn read_auth() -> Result<Auth, CodexError> {
    let Some((file, whose)) = crate::auth::stored() else {
        return Err(CodexError::new("Not signed in."));
    };
    let tokens = &file["tokens"];
    let (Some(access_token), Some(account_id)) = (
        tokens["access_token"].as_str(),
        tokens["account_id"].as_str(),
    ) else {
        return Err(CodexError::new("The stored login is missing its account."));
    };
    Ok(Auth {
        access_token: access_token.to_owned(),
        account_id: account_id.to_owned(),
        refresh_token: tokens["refresh_token"].as_str().map(str::to_owned),
        whose,
    })
}

/// Whether anyone is signed in at all.
pub fn signed_in() -> bool {
    crate::auth::stored().is_some()
}

/// Who is signed in, for the header strip.
pub fn codex_account() -> String {
    crate::auth::stored()
        .and_then(|(file, _)| crate::auth::email_of(&file["tokens"]))
        .unwrap_or_default()
}

/// Forget this app's login, and say whether anyone is still signed in.
///
/// The CLI's file is left alone, so signing out here and remaining signed in
/// through `codex login` is the expected outcome, not a bug.
pub fn sign_out() -> bool {
    crate::auth::sign_out();
    signed_in()
}

/// The model the CLI itself is set to, so this window answers as it would.
///
/// One scan for a top-level `model` key rather than a TOML parse: a model named
/// only inside a `[profiles.*]` table is missed and the default stands.
pub fn codex_model() -> String {
    let model = config_value("model");
    if model.is_empty() {
        return DEFAULT_MODEL.to_owned();
    }
    model
}

/// One top-level key out of the CLI's config, or empty.
///
/// A scan rather than a TOML parse: a key nested inside a `[profiles.*]` table
/// is missed and the default stands.
fn config_value(key: &str) -> String {
    std::fs::read_to_string(codex_home().join("config.toml"))
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                let (found, value) = line.split_once('=')?;
                (found.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
            })
        })
        .unwrap_or_default()
}

/// A settled turn with no network behind it.
///
/// Presets and captures draw from this instead of a live account, which keeps a
/// real conversation — and the address of whoever is signed in — out of every
/// screenshot and test artifact this repository stores.
/// A session already holding the sample turn.
///
/// A preset seeds this as well as the drawn list, because the two are the same
/// conversation and every handler that changes a row goes through the session.
/// Seeding only the drawn half would let a preset exercise a path production
/// never takes.
pub fn sample_session(dark: bool) -> Session {
    let session = codex_session();
    let mut state = session.lock();
    state.dark = dark;
    state.entries = sample_rows(dark);
    state.turn = 1;
    drop(state);
    session
}

pub fn sample_entries(dark: bool) -> Vec<Entry> {
    sample_rows(dark)
        .into_iter()
        .filter(|row| !row.hidden)
        .collect()
}

/// A turn caught in the middle: one step done and closed, one still running.
pub fn sample_running(dark: bool) -> Vec<Entry> {
    let row = |id: i64, open: bool, entry: Entry| Entry {
        id,
        turn: 1,
        open,
        dark,
        ..entry
    };
    vec![
        row(
            -21,
            false,
            Entry::new("prompt", "").body("Which version of iced is current?"),
        ),
        row(
            -22,
            false,
            Entry::new("reasoning", "Checking the crate before answering")
                .body("The version could have moved since training."),
        ),
        row(
            -23,
            false,
            Entry::new("tool", "Searched the web")
                .detail("site:crates.io/crates/iced latest version")
                .status("done"),
        ),
        row(
            -24,
            true,
            Entry::new("tool", "Searching the web").status("running"),
        ),
    ]
}

/// The whole turn, folded as a finished one is: the working-out is present but
/// tucked under its summary, which is the state a transcript is read in.
fn sample_rows(dark: bool) -> Vec<Entry> {
    // Fixed, negative ids. Live rows count up from one, so a sample row can
    // never collide with a real one — in the Markdown cache or in a widget
    // path — and a test can name a row by hand.
    let row = |id: i64, hidden: bool, entry: Entry| Entry {
        id,
        turn: 1,
        hidden,
        dark,
        ..entry
    };
    vec![
        row(
            -1,
            false,
            Entry::new("prompt", "").body(
                "Which version of iced is current, and how do I stream a reply into a \
                 Markdown view?",
            ),
        ),
        row(-2, false, Entry::new("work", worked_for(12, 4))),
        row(
            -3,
            true,
            Entry::new("reasoning", "Checking the crate before answering").body(
                "The version could have moved since training, so it is worth a look \
                 rather than a guess.",
            ),
        ),
        row(
            -4,
            true,
            Entry::new("tool", "Searched the web")
                .detail("site:crates.io/crates/iced latest version  ·  iced-rs releases")
                .status("done"),
        ),
        row(
            -5,
            true,
            Entry::new("tool", "Opened a page")
                .detail("https://crates.io/crates/iced")
                .status("done"),
        ),
        row(
            -6,
            false,
            Entry::new("answer", "").body(
                "**iced 0.14** is current.\n\nFor the Markdown view, append to the parsed \
                 document instead of rebuilding it:\n\n\
                 ```rust\n\
                 content.push_str(&delta);\n\
                 ```\n\n\
                 Only the tail is reparsed, so the cost of a token stays flat as the \
                 answer grows.\n\n\
                 - The document is parsed once, then extended\n\
                 - Earlier blocks keep their layout\n\
                 - The view rebuilds one row, not the transcript",
            ),
        ),
        row(
            -7,
            false,
            Entry::new("usage", "").detail("2,914 in · 268 out · 192 reasoning"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(raw: &str) -> Value {
        serde_json::from_str(raw).expect("event")
    }

    fn empty() -> Session {
        codex_session()
    }

    fn rows(session: &Session) -> Vec<Entry> {
        session.lock().snapshot()
    }

    /// Streamed text must arrive as pieces and accumulate, because the screen
    /// appends them and the settled row is built from the total.
    #[test]
    fn streamed_answer_accumulates_and_is_handed_over_in_pieces() {
        let mut answer = String::new();
        let drawn: Vec<Chunk> = [
            r#"{"type":"response.output_text.delta","delta":"Hel"}"#,
            r#"{"type":"response.output_text.delta","delta":"lo"}"#,
        ]
        .iter()
        .flat_map(|raw| stream_text(&event(raw), &mut answer))
        .collect();

        assert_eq!(answer, "Hello", "the settled row needs the whole answer");
        assert_eq!(
            drawn.iter().map(|c| c.answer.as_str()).collect::<Vec<_>>(),
            ["Hel", "lo"],
            "pieces, not prefixes: a repeated prefix would double the text"
        );
    }

    /// A tool has to be on screen while it is still running, then say what it
    /// did — one row that changes, not two rows.
    #[test]
    fn a_tool_call_is_drawn_when_it_starts_and_updated_when_it_finishes() {
        let session = empty();
        record(
            &session,
            &event(r#"{"type":"response.output_item.added","item":{"type":"web_search_call"}}"#),
        );
        let running = rows(&session);
        assert_eq!(running.len(), 1, "the tool appears immediately");
        assert_eq!(running[0].status, "running");
        assert!(
            running[0].open,
            "the step happening now is the one to watch"
        );

        record(
            &session,
            &event(
                r#"{"type":"response.output_item.done","item":{"type":"web_search_call",
                    "status":"completed","action":{"type":"search","queries":["iced 0.14"]}}}"#,
            ),
        );
        let settled = rows(&session);
        assert_eq!(
            settled.len(),
            1,
            "the same row settles; it is not duplicated"
        );
        assert_eq!(settled[0].status, "done");
        assert_eq!(settled[0].title, "Searched the web");
        assert_eq!(settled[0].detail, "iced 0.14");
    }

    /// Opening a page and running a query are different acts, and a transcript
    /// that calls both "searched" is not reporting what happened.
    #[test]
    fn each_search_action_is_named_for_what_it_did() {
        assert_eq!(
            search_action(&event(r#"{"type":"open_page","url":"https://crates.io"}"#)),
            ("Opened a page".to_owned(), "https://crates.io".to_owned())
        );
        let (title, detail) = search_action(&event(r#"{"type":"search","queries":["one","two"]}"#));
        assert_eq!(title, "Searched the web");
        assert_eq!(detail, "one  ·  two");
    }

    /// An item this build never modelled is still part of the turn.
    #[test]
    fn an_unmodelled_item_becomes_a_row_rather_than_being_dropped() {
        let session = empty();
        record(
            &session,
            &event(
                r#"{"type":"response.output_item.done","item":{"type":"custom_tool_call","status":"completed"}}"#,
            ),
        );
        let entries = rows(&session);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "custom_tool_call");
        assert!(entries[0].body.contains("```json"), "drawn as itself");
    }

    /// Reasoning and the answer arrive twice — streamed, then as a completed
    /// item. Recording the completed item is what settles the row; recording
    /// an empty one would leave a blank row behind.
    #[test]
    fn empty_reasoning_and_message_items_leave_no_row() {
        let session = empty();
        for raw in [
            r#"{"type":"response.output_item.done","item":{"type":"reasoning","summary":[]}}"#,
            r#"{"type":"response.output_item.done","item":{"type":"message","content":[]}}"#,
        ] {
            record(&session, &event(raw));
        }
        assert!(rows(&session).is_empty());
    }

    /// Every row is keyed by its id, so two rows must never share one.
    #[test]
    fn rows_never_share_an_id() {
        let session = empty();
        let entries = push_user(session.clone(), "first".to_owned());
        assert_eq!(entries.len(), 1);
        record(
            &session,
            &event(r#"{"type":"response.completed","response":{"usage":{}}}"#),
        );
        let entries = rows(&session);
        assert_ne!(
            entries[0].id, entries[1].id,
            "a shared id collapses two rows"
        );
    }

    /// One real turn against the signed-in account, end to end: the request is
    /// accepted, text arrives in pieces, and the pieces settle into rows.
    ///
    /// Ignored by default because it needs `codex login` and spends tokens.
    /// Run it with:
    /// `cargo test -p ai-chat-example -- --ignored --nocapture a_live_turn`
    #[test]
    #[ignore = "reaches the ChatGPT backend; needs `codex login`"]
    fn a_live_turn_streams_text_and_settles_into_rows() {
        let session = codex_session();
        let question =
            std::env::var("AI_CHAT_ASK").unwrap_or_else(|_| "Reply with exactly: pong".to_owned());
        eprintln!("\n>>> {question}");
        push_user(session.clone(), question);

        let (chunks, incoming) = smol::channel::unbounded();
        let worker = session.clone();
        let turn = std::thread::spawn(move || pump(&worker, &chunks));

        let mut streamed = String::new();
        let mut statuses = Vec::new();
        while let Ok(chunk) = incoming.recv_blocking() {
            streamed.push_str(&chunk.answer);
            if !chunk.status.is_empty() && statuses.last() != Some(&chunk.status) {
                statuses.push(chunk.status.clone());
            }
        }
        let rows = turn
            .join()
            .expect("worker thread")
            .expect("a finished turn");

        eprintln!("\nstatuses seen: {statuses:?}\n");
        for row in &rows {
            eprintln!("--- {} ---", row.kind);
            if !row.title.is_empty() {
                eprintln!("{}", row.title);
            }
            if !row.detail.is_empty() {
                eprintln!("{}", row.detail);
            }
            if !row.body.is_empty() {
                eprintln!("{}", row.body);
            }
        }
        assert!(
            !streamed.trim().is_empty(),
            "the answer must arrive as streamed pieces"
        );
        assert_eq!(rows.first().map(|row| row.kind.as_str()), Some("prompt"));
        assert!(
            rows.iter()
                .any(|row| row.kind == "answer" && row.body == streamed),
            "the settled answer row must equal what was streamed"
        );
        assert!(
            rows.last()
                .is_some_and(|row| row.kind == "usage" && !row.detail.is_empty()),
            "a turn ends with what it cost"
        );
    }

    /// A summary states its subject on a bold first line, and often says
    /// nothing else. The heading becomes the row's title either way, and a
    /// heading-only summary must leave no body — the screen offers a fold only
    /// when there is something folded away.
    #[test]
    fn a_reasoning_summary_gives_up_its_heading() {
        assert_eq!(
            headed("**Checking the crate**\n\nIt could have moved."),
            (
                "Checking the crate".to_owned(),
                "It could have moved.".to_owned()
            )
        );
        assert_eq!(
            headed("**Verifying the spelling**"),
            ("Verifying the spelling".to_owned(), String::new()),
            "a heading-only summary has no body to fold"
        );
        assert_eq!(
            headed("No heading here.\n\nJust prose."),
            (
                "Thought process".to_owned(),
                "No heading here.\n\nJust prose.".to_owned()
            ),
            "an unheaded summary keeps all of itself"
        );
    }

    /// A picker that cannot show what is currently selected is worse than no
    /// picker, so the model in force is in the list even when the CLI has
    /// written no catalogue for it to come from.
    #[test]
    fn the_model_in_force_is_always_offered() {
        let models = codex_models();
        assert!(
            models.contains(&codex_model()),
            "the current model must be selectable: {models:?}"
        );
    }

    /// Choosing a model has to change what the next turn asks for, and it
    /// belongs to this chat rather than to whatever the CLI is configured for.
    #[test]
    fn choosing_a_model_changes_what_the_next_turn_asks_for() {
        let session = codex_session();
        assert_eq!(
            session.lock().model,
            codex_model(),
            "a new chat starts on the CLI's model"
        );

        set_model(session.clone(), "gpt-5.4-mini".to_owned());
        assert_eq!(session.lock().model, "gpt-5.4-mini");
        assert_eq!(
            codex_model(),
            codex_model(),
            "and the CLI's own configuration is untouched"
        );
    }

    /// The catalogue is only useful if it is actually read. It names models by
    /// `slug`, and reading the wrong key leaves a picker with one entry in it
    /// — which looks like a working picker and is not one.
    #[test]
    #[ignore = "reads the CLI's own catalogue, which a machine may not have"]
    fn the_catalogue_yields_more_than_the_current_model() {
        let models = codex_models();
        eprintln!("models: {models:?}");
        eprintln!(
            "efforts for {}: {:?}",
            codex_model(),
            codex_efforts(codex_model())
        );
        eprintln!("effort in force: {}", codex_effort());
        assert!(models.len() > 1, "the catalogue was not read: {models:?}");
    }

    /// Levels differ by model, and the backend rejects one the model does not
    /// offer. Changing model must therefore carry the effort with it rather
    /// than leave a setting that will fail on the next turn.
    #[test]
    fn an_effort_the_new_model_does_not_offer_is_replaced() {
        let session = codex_session();
        set_effort(session.clone(), "ultra".to_owned());

        // A model whose catalogue entry is unknown offers the plain three.
        set_model(session.clone(), "a-model-no-catalogue-knows".to_owned());
        let settled = session_effort(session.clone());
        assert!(
            codex_efforts("a-model-no-catalogue-knows".to_owned()).contains(&settled),
            "the effort must be one the model offers, got {settled:?}"
        );
        assert_ne!(settled, "ultra", "the unsupported level must not survive");
    }

    /// Starting over clears what was said, not a choice about how to answer.
    #[test]
    fn a_new_chat_keeps_the_model_and_effort() {
        let session = codex_session();
        set_model(session.clone(), "gpt-5.4-mini".to_owned());
        set_effort(session.clone(), "low".to_owned());

        let next = new_chat(session);
        assert_eq!(next.lock().model, "gpt-5.4-mini");
        assert_eq!(session_effort(next.clone()), "low");
        assert!(next.lock().entries.is_empty(), "but the transcript is gone");
    }

    /// A refusal is the only place a cause is stated, so it has to survive the
    /// shapes the backend states it in.
    #[test]
    fn a_refusal_keeps_its_sentence() {
        assert_eq!(
            reason(r#"{"detail":"model is not supported"}"#),
            "model is not supported"
        );
        assert_eq!(
            reason(r#"{"error":{"message":"token expired"}}"#),
            "token expired"
        );
        assert_eq!(reason("upstream is down"), "upstream is down");
    }

    /// The token bill is drawn under every answer; a missing detail must read
    /// as zero rather than take the line down with it.
    #[test]
    fn a_partial_usage_report_still_reads() {
        assert_eq!(
            tokens(&event(r#"{"input_tokens":27,"output_tokens":84}"#)),
            "27 in · 84 out · 0 reasoning"
        );
    }

    /// A turn's input runs to five figures, and the count is read at a glance
    /// or not at all.
    #[test]
    fn a_token_count_is_grouped_where_it_needs_to_be() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(321), "321");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(22_875), "22,875");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }
}

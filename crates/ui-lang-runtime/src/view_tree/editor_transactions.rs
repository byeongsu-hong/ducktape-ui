//! The ordered host input lane for one resolved editor binding.
use std::collections::VecDeque;

const MAX_INPUTS: usize = 128;
const DEADLINE_MS: u64 = 5_000;
const MAX_ATTEMPTS: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fault {
    Overflow,
    Timeout,
    Conflicts,
    Identity,
    Limit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Ready,
    Decision { since_ms: u64, attempt: u32 },
    AwaitingGuest,
    Faulted(Fault),
}

#[derive(Debug, Clone)]
pub(super) struct Queued<T> {
    pub sequence: u64,
    pub input: T,
    bytes: usize,
}

/// Stored by logical binding within one guest instance, never by equal text.
/// Inputs stay owned here through the commit's guest acknowledgment.
#[derive(Clone, Debug)]
pub(super) struct Lane<T> {
    queue: VecDeque<Queued<T>>,
    bytes: usize,
    phase: Phase,
}

impl<T> Default for Lane<T> {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            bytes: 0,
            phase: Phase::Ready,
        }
    }
}

impl<T> Lane<T> {
    pub fn admit(&mut self, sequence: u64, bytes: usize, input: T) -> Result<(), (Fault, T)> {
        if let Phase::Faulted(fault) = self.phase {
            return Err((fault, input));
        }
        if self
            .queue
            .back()
            .is_some_and(|back| sequence <= back.sequence)
        {
            return Err((Fault::Identity, input));
        }
        let Some(total) = self
            .bytes
            .checked_add(bytes)
            .filter(|n| *n <= ui_lang_wire::editor_transaction::MAX_EDITOR_INPUT_BYTES)
        else {
            self.phase = Phase::Faulted(Fault::Overflow);
            return Err((Fault::Overflow, input));
        };
        if self.queue.len() == MAX_INPUTS {
            self.phase = Phase::Faulted(Fault::Overflow);
            return Err((Fault::Overflow, input));
        }
        self.bytes = total;
        self.queue.push_back(Queued {
            sequence,
            bytes,
            input,
        });
        Ok(())
    }
    pub fn front(&self) -> Option<&Queued<T>> {
        self.queue.front()
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn fail(&mut self, reason: Fault) {
        self.phase = Phase::Faulted(reason);
    }
    pub fn request(&mut self, now_ms: u64) {
        if self.phase == Phase::Ready && !self.queue.is_empty() {
            self.phase = Phase::Decision {
                since_ms: now_ms,
                attempt: 1,
            };
        }
    }
    pub fn commit(&mut self, sequence: u64) -> Result<(), Fault> {
        if self
            .queue
            .front()
            .is_none_or(|front| front.sequence != sequence)
            || !matches!(self.phase, Phase::Ready | Phase::Decision { .. })
        {
            return Err(Fault::Identity);
        }
        self.phase = Phase::AwaitingGuest;
        Ok(())
    }
    pub fn acknowledge(&mut self, sequence: u64) -> Result<T, Fault> {
        if self.phase != Phase::AwaitingGuest
            || self
                .queue
                .front()
                .is_none_or(|front| front.sequence != sequence)
        {
            return Err(Fault::Identity);
        }
        let front = self.queue.pop_front().expect("matching front checked");
        self.bytes -= front.bytes;
        self.phase = Phase::Ready;
        Ok(front.input)
    }
    pub fn conflict(&mut self, now_ms: u64) -> Result<(), Fault> {
        self.check_deadline(now_ms);
        let Phase::Decision { since_ms, attempt } = self.phase else {
            return Err(match self.phase {
                Phase::Faulted(fault) => fault,
                _ => Fault::Identity,
            });
        };
        if attempt == MAX_ATTEMPTS {
            self.phase = Phase::Faulted(Fault::Conflicts);
            return Err(Fault::Conflicts);
        }
        self.phase = Phase::Decision {
            since_ms,
            attempt: attempt + 1,
        };
        Ok(())
    }
    pub fn check_deadline(&mut self, now_ms: u64) {
        if let Phase::Decision { since_ms, .. } = self.phase
            && now_ms.saturating_sub(since_ms) >= DEADLINE_MS
        {
            self.phase = Phase::Faulted(Fault::Timeout);
        }
    }
    pub fn remove_inputs(&mut self, mut remove: impl FnMut(&T) -> bool) -> Vec<Queued<T>> {
        let mut removed = Vec::new();
        let mut kept = VecDeque::new();
        let first = self.queue.front().map(|q| q.sequence);
        while let Some(input) = self.queue.pop_front() {
            if remove(&input.input) {
                self.bytes -= input.bytes;
                removed.push(input);
            } else {
                kept.push_back(input);
            }
        }
        self.queue = kept;
        if removed.iter().any(|q| Some(q.sequence) == first) {
            self.phase = Phase::Ready;
        }
        removed
    }
    pub fn cancel(self) -> impl Iterator<Item = Queued<T>> {
        self.queue.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_typing_cannot_pass_a_decision_or_its_commit_ack() {
        let mut lane = Lane::default();
        lane.admit(1, 3, "Tab").unwrap();
        lane.request(10);
        lane.admit(2, 1, "x").unwrap();
        assert_eq!(lane.front().unwrap().input, "Tab");
        lane.commit(1).unwrap();
        assert_eq!(lane.phase(), Phase::AwaitingGuest);
        assert_eq!(lane.front().unwrap().sequence, 1);
        assert_eq!(lane.acknowledge(1), Ok("Tab"));
        assert_eq!(lane.front().unwrap().input, "x");
        assert_eq!(lane.phase(), Phase::Ready);
    }
    #[test]
    fn overflow_retains_accepted_inputs_and_returns_rejected_input() {
        let mut lane = Lane::default();
        for i in 0..MAX_INPUTS {
            lane.admit(i as u64, 1, i).unwrap();
        }
        assert_eq!(lane.admit(128, 1, 999), Err((Fault::Overflow, 999)));
        assert_eq!(lane.phase(), Phase::Faulted(Fault::Overflow));
        assert_eq!(
            lane.cancel().map(|q| q.input).collect::<Vec<_>>(),
            (0..MAX_INPUTS).collect::<Vec<_>>()
        );
    }
    #[test]
    fn conflict_retries_same_identity_and_timeout_does_not_default() {
        let mut lane = Lane::default();
        lane.admit(10, 1, "Enter").unwrap();
        lane.request(1);
        lane.conflict(2).unwrap();
        assert_eq!(lane.front().unwrap().sequence, 10);
        assert_eq!(
            lane.phase(),
            Phase::Decision {
                since_ms: 1,
                attempt: 2
            }
        );
        lane.check_deadline(5_001);
        assert_eq!(lane.phase(), Phase::Faulted(Fault::Timeout));
        assert_eq!(lane.front().unwrap().input, "Enter");
    }
    #[test]
    fn stale_ack_cannot_remove_a_pending_input() {
        let mut lane = Lane::default();
        lane.admit(2, 1, "x").unwrap();
        assert_eq!(lane.acknowledge(1), Err(Fault::Identity));
        assert_eq!(lane.front().unwrap().sequence, 2);
    }
}

use iced::time::Instant;
use iced::widget::text_editor;
use iced::{Event, mouse};
use std::sync::{Arc, Mutex};
use ui_lang_wire as wire;

#[derive(Clone, Debug)]
pub(super) enum NativeWork {
    Event(Event),
    Actions(Vec<text_editor::Action>),
    Caret(wire::EditorCursor),
}

#[derive(Clone, Debug)]
pub(super) struct NativeInput {
    pub key: String,
    pub work: NativeWork,
    pub cursor: mouse::Cursor,
    pub clipboard: Option<String>,
    pub time_ms: u64,
}
#[derive(Clone, Debug)]
pub(super) struct Control {
    pub lane: Lane<NativeInput>,
    pub reset: u64,
    pub text_revision: u64,
    pub last_text: String,
    pub loaded: bool,
    pub available: bool,
    pub revision: u64,
    pub cursor: wire::EditorCursor,
    pub next_sequence: u64,
    pub sequences: Arc<std::sync::atomic::AtomicU64>,
    pub pending: Option<wire::EditorKeyRequest>,
    pub bypass_claim: bool,
    pub committed: Option<u64>,
    pub composing: bool,
    pub dragging: bool,
    pub fault_reported: bool,
    pub fault_key: Option<String>,
    pub started: Instant,
}
pub(super) type Shared = Arc<Mutex<Control>>;
impl Control {
    pub fn new(reset: u64, sequences: Arc<std::sync::atomic::AtomicU64>) -> Self {
        Self {
            lane: Lane::default(),
            reset,
            text_revision: 0,
            last_text: String::new(),
            loaded: false,
            available: false,
            revision: 0,
            cursor: wire::EditorCursor::default(),
            next_sequence: 0,
            sequences,
            pending: None,
            bypass_claim: false,
            committed: None,
            composing: false,
            dragging: false,
            fault_reported: false,
            fault_key: None,
            started: Instant::now(),
        }
    }
    pub fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    pub document: String,
    pub key: String,
    pub sequence: u64,
    pub reset: u64,
    pub actions: Vec<text_editor::Action>,
    pub request: Option<(wire::keyboard::KeyState, bool)>,
}

fn reference(
    document: &str,
    state: &wire::EditorState,
    text_revision: u64,
) -> wire::editor_document::EditorDocumentRef {
    wire::editor_document::EditorDocumentRef {
        document: document.into(),
        reset: state.reset,
        text_revision,
        revision: state.revision,
        cursor: state.cursor,
        byte_len: state.text.len() as u32,
    }
}

pub(super) fn state(field: &super::EditorField) -> wire::EditorState {
    let content = super::lock(&field.content);
    wire::EditorState {
        text: content.text(),
        cursor: super::editor_cursor(&content),
        reset: field.reset,
        revision: field.revision,
    }
}

/// Translate a validated byte endpoint into Content's native logical position.
fn position(text: &str, byte: usize) -> text_editor::Position {
    let mut offset = 0;
    for (line, source) in wire::editor_lines(text).enumerate() {
        if byte <= offset + source.len() {
            return text_editor::Position {
                line,
                column: byte - offset,
            };
        }
        offset += source.len();
        let end = &text[offset..];
        offset += if end.starts_with("\r\n") || end.starts_with("\n\r") {
            2
        } else {
            1
        };
    }
    unreachable!("validated editor endpoint")
}

pub(super) fn apply_patches(
    content: &mut text_editor::Content,
    patches: &[wire::EditorPatch],
    cursor: wire::EditorCursor,
) -> Result<(), wire::EditorPatchError> {
    let text = content.text();
    let old_cursor = content.cursor();
    let expected = wire::patched_editor_text(&text, patches, cursor)?;
    for patch in patches.iter().rev() {
        let start = position(&text, patch.start_byte as usize);
        let end = position(&text, patch.end_byte as usize);
        // Native move_to(None) leaves old selection behind; clear it explicitly.
        content.perform(text_editor::Action::Move(text_editor::Motion::Left));
        content.move_to(text_editor::Cursor {
            position: end,
            selection: (start != end).then_some(start),
        });
        if patch.replacement.is_empty() {
            if start != end {
                content.perform(text_editor::Action::Edit(text_editor::Edit::Backspace));
            }
        } else {
            content.perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                Arc::new(patch.replacement.clone()),
            )));
        }
    }
    content.perform(text_editor::Action::Move(text_editor::Motion::Left));
    content.move_to(text_editor::Cursor {
        position: text_editor::Position {
            line: cursor.position.line as usize,
            column: cursor.position.column as usize,
        },
        selection: cursor.selection.map(|p| text_editor::Position {
            line: p.line as usize,
            column: p.column as usize,
        }),
    });
    if content.text() != expected {
        *content = text_editor::Content::with_text(&text);
        content.move_to(old_cursor);
        return Err(wire::EditorPatchError::Range);
    }
    Ok(())
}

pub(super) fn kind(actions: &[text_editor::Action]) -> wire::EditorEditKind {
    use wire::EditorEditKind as K;
    actions
        .iter()
        .find_map(|action| match action {
            text_editor::Action::Edit(edit) => Some(match edit {
                text_editor::Edit::Insert(_) => K::Insert,
                text_editor::Edit::Paste(_) => K::Paste,
                text_editor::Edit::Enter => K::Enter,
                text_editor::Edit::Backspace => K::Backspace,
                text_editor::Edit::Delete => K::Delete,
                text_editor::Edit::Indent => K::Indent,
                text_editor::Edit::Unindent => K::Unindent,
            }),
            _ => None,
        })
        .unwrap_or(K::Cursor)
}

pub(super) fn adopt(inputs: &mut super::Inputs, root: &wire::Node) {
    let mut bindings = std::collections::HashMap::new();
    let mut reported = std::collections::HashMap::new();
    fn visit(
        node: &wire::Node,
        bindings: &mut std::collections::HashMap<String, (String, wire::EditorBinding)>,
        reported: &mut std::collections::HashMap<String, (u64, u64)>,
    ) {
        if let wire::Node::Editor {
            key,
            options,
            document,
            ..
        } = node
            && let Some(binding) = &options.binding
        {
            bindings.insert(
                key.clone(),
                (document.document.clone(), (**binding).clone()),
            );
            reported.insert(key.clone(), (document.reset, document.revision));
        }
        for child in node.children() {
            visit(child, bindings, reported);
        }
    }
    visit(root, &mut bindings, &mut reported);
    // A second rendering of one state may omit key claims. It still joins the
    // document lane and reports native commits to the document's history route.
    fn siblings(
        node: &wire::Node,
        bindings: &mut std::collections::HashMap<String, (String, wire::EditorBinding)>,
        reported: &mut std::collections::HashMap<String, (u64, u64)>,
    ) {
        if let wire::Node::Editor {
            key,
            options,
            document,
            ..
        } = node
            && options.binding.is_none()
            && let Some((binding_document, binding)) = bindings
                .values()
                .find(|(doc, _)| doc == &document.document)
                .cloned()
        {
            bindings.insert(key.clone(), (binding_document, binding));
            reported.insert(key.clone(), (document.reset, document.revision));
        }
        for child in node.children() {
            siblings(child, bindings, reported);
        }
    }
    siblings(root, &mut bindings, &mut reported);
    for (document, shared) in &inputs.editor_transactions {
        let mut control = super::lock(shared);
        let new_reset = bindings
            .iter()
            .find(|(_, (doc, _))| doc == document)
            .map(|(key, _)| reported[key].0);
        let owner_removed = control.lane.front().is_some_and(|front| {
            bindings
                .get(&front.input.key)
                .is_none_or(|(doc, _)| doc != document)
        });
        if new_reset != Some(control.reset) || owner_removed {
            if let Some(request) = control.pending.clone()
                && let Some(front) = control.lane.front()
                && let Some((doc, binding)) = inputs.editor_bindings.get(&front.input.key)
                && doc == document
            {
                inputs
                    .editor_notifications
                    .push(wire::Event::EditorTransaction {
                        handler: binding.on_event,
                        event: wire::EditorTransactionEvent::Cancelled {
                            id: request.id,
                            state: request.state,
                        },
                    });
            }
            if new_reset != Some(control.reset) {
                // Explicit document cancellation owns the old document's accepted FIFO.
                for _input in std::mem::take(&mut control.lane).cancel() {}
            } else {
                control.lane.remove_inputs(|input| {
                    bindings
                        .get(&input.key)
                        .is_none_or(|(doc, _)| doc != document)
                });
            }
            control.pending = None;
            control.committed = None;
            control.bypass_claim = false;
            control.fault_reported = false;
            control.fault_key = None;
            control.composing = false;
            control.dragging = false;
        } else {
            control.lane.remove_inputs(|input| {
                bindings
                    .get(&input.key)
                    .is_none_or(|(doc, _)| doc != document)
            });
        }
    }
    for (key, (document, _)) in &bindings {
        let reset = reported[key].0;
        let control = inputs
            .editor_transactions
            .entry(document.clone())
            .or_insert_with(|| {
                Arc::new(Mutex::new(Control::new(
                    reset,
                    inputs.editor_sequence.clone(),
                )))
            });
        let mut control = super::lock(control);
        control.available = control.loaded && control.reset == reset;
    }
    inputs.editor_transactions.retain(|document, _| {
        inputs
            .editor_references
            .values()
            .any(|reference| &reference.document.document == document)
    });
    inputs.editor_bindings = bindings;
    inputs.editor_reported = reported;
}

impl super::Inputs {
    pub fn editor_transactions_pending(&self) -> bool {
        self.editor_transfer.is_some()
            || self.editor_outgoing.is_some()
            || self
                .editor_transactions
                .values()
                .any(|shared| super::lock(shared).lane.front().is_some())
    }

    pub fn editor_wants_redraw(&self) -> bool {
        !self.editor_notifications.is_empty()
            || self.editor_transfer.is_some()
            || self.editor_outgoing.is_some()
            || self.editor_transactions.values().any(|shared| {
                let control = super::lock(shared);
                control.lane.phase() == Phase::Ready && control.lane.front().is_some()
            })
    }
    pub(super) fn editor_fault(&mut self, document: &str, pending: &mut Vec<wire::Event>) {
        let Some(shared) = self.editor_transactions.get(document) else {
            return;
        };
        let mut control = super::lock(shared);
        if control.fault_reported {
            return;
        }
        let Some(key) = control
            .lane
            .front()
            .map(|front| &front.input.key)
            .or(control.fault_key.as_ref())
        else {
            return;
        };
        let Some((doc, binding)) = self.editor_bindings.get(key) else {
            return;
        };
        if doc != document {
            return;
        };
        let Some(field) = self.editors.get(key) else {
            return;
        };
        if field.document != document {
            return;
        }
        let current = super::editor_documents::current_reference(document, &control);
        let id = control
            .pending
            .as_ref()
            .map(|request| request.id.clone())
            .unwrap_or(wire::EditorTransactionId {
                instance: self.instance,
                document: document.into(),
                reset: control.reset,
                sequence: control
                    .lane
                    .front()
                    .map_or(control.next_sequence, |front| front.sequence),
                attempt: 0,
                text_revision: control.text_revision,
                revision: current.revision,
            });
        let reason = match control.lane.phase() {
            Phase::Faulted(Fault::Timeout) => wire::EditorFault::Timeout,
            Phase::Faulted(Fault::Conflicts) => wire::EditorFault::Conflicts,
            Phase::Faulted(Fault::Identity) => wire::EditorFault::InvalidResponse,
            Phase::Faulted(Fault::Limit) => wire::EditorFault::Limit,
            _ => wire::EditorFault::Overflow,
        };
        control.fault_reported = true;
        pending.push(wire::Event::EditorTransaction {
            handler: binding.on_event,
            event: wire::EditorTransactionEvent::Fault {
                id,
                state: current,
                reason,
            },
        });
    }

    pub(super) fn apply_editor_batch(&mut self, batch: Batch, pending: &mut Vec<wire::Event>) {
        let Some(shared) = self.editor_transactions.get(&batch.document).cloned() else {
            return;
        };
        let mut control = super::lock(&shared);
        let Some(front) = control.lane.front() else {
            return;
        };
        if front.sequence != batch.sequence || control.reset != batch.reset {
            return;
        }
        let Some(field) = self.editors.get(&batch.key) else {
            return;
        };
        let before_revision = field.revision;
        let Some((owner, binding)) = self.editor_bindings.get(&batch.key).cloned() else {
            return;
        };
        if owner != batch.document {
            return;
        }
        let input_time_ms = front.input.time_ms;
        if let Some((key, repeat)) = batch.request {
            let now = control.now_ms();
            control.lane.request(now);
            let attempt = match control.lane.phase() {
                Phase::Decision { attempt, .. } => attempt,
                _ => return,
            };
            let id = wire::EditorTransactionId {
                instance: self.instance,
                document: batch.document,
                reset: batch.reset,
                sequence: batch.sequence,
                attempt,
                text_revision: control.text_revision,
                revision: before_revision,
            };
            let request = wire::EditorKeyRequest {
                state: super::editor_documents::current_reference(&id.document, &control),
                id,
                key,
                repeat,
                input_time_ms,
            };
            control.pending = Some(request.clone());
            pending.push(wire::Event::EditorKeyRequest {
                handler: binding.on_request,
                request,
            });
            return;
        }
        drop(control);
        self.commit_editor(
            &batch.document,
            &batch.key,
            batch.actions,
            None,
            wire::EditorHistoryEffect::Native,
            pending,
        );
    }

    pub(super) fn retry_editor_after_mirror(
        &mut self,
        id: &wire::editor_document::EditorTransferId,
        target: &wire::editor_document::EditorDocumentRef,
        pending: &mut Vec<wire::Event>,
    ) {
        let Some(shared) = self.editor_transactions.get(&id.document).cloned() else {
            return;
        };
        let mut control = super::lock(&shared);
        let Some(mut request) = control.pending.clone() else {
            return;
        };
        if request.id.instance != id.instance
            || request.id.sequence != id.serial
            || request.id.attempt != id.attempt
            || request.state != *target
        {
            return;
        }
        let Some(front) = control.lane.front() else {
            return;
        };
        let key = front.input.key.clone();
        let Some(field) = self.editors.get(&key) else {
            return;
        };
        if field.document != id.document {
            return;
        }
        let now = control.now_ms();
        if control.lane.conflict(now).is_err() {
            drop(control);
            self.editor_fault(&id.document, pending);
            return;
        }
        let Phase::Decision { attempt, .. } = control.lane.phase() else {
            return;
        };
        request.id.attempt = attempt;
        request.id.revision = control.revision;
        request.id.text_revision = control.text_revision;
        request.state = super::editor_documents::current_reference(&id.document, &control);
        control.pending = Some(request.clone());
        if let Some((document, binding)) = self.editor_bindings.get(&key)
            && document == &id.document
        {
            pending.push(wire::Event::EditorKeyRequest {
                handler: binding.on_request,
                request,
            });
        }
    }

    pub(super) fn admit_editor_work(
        &mut self,
        key: &str,
        reset: u64,
        work: NativeWork,
        pending: &mut Vec<wire::Event>,
    ) {
        if self
            .editor_references
            .get(key)
            .is_none_or(|reference| !reference.editable)
        {
            return;
        }
        let Some(field) = self.editors.get(key) else {
            return;
        };
        let document = field.document.clone();
        let Some(shared) = self.editor_transactions.get(&document).cloned() else {
            return;
        };
        let mut control = super::lock(&shared);
        if field.reset != reset || !control.available {
            return;
        }
        let bytes = match &work {
            NativeWork::Actions(actions) => actions
                .iter()
                .map(|action| match action {
                    text_editor::Action::Edit(text_editor::Edit::Paste(text)) => text.len(),
                    _ => 0,
                })
                .sum(),
            _ => 0,
        };
        let sequence = control.sequences.fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |next| next.checked_add(1),
        );
        let admitted = if let Ok(previous) = sequence {
            let sequence = previous + 1;
            let input = NativeInput {
                key: key.into(),
                work,
                cursor: mouse::Cursor::Unavailable,
                clipboard: None,
                time_ms: control.now_ms(),
            };
            control.next_sequence = sequence;
            control.lane.admit(sequence, bytes, input).is_ok()
        } else {
            control.lane.fail(Fault::Limit);
            false
        };
        if !admitted {
            control.fault_key = Some(key.into());
            drop(control);
            self.editor_fault(&document, pending);
        }
    }

    fn commit_editor(
        &mut self,
        document: &str,
        key: &str,
        actions: Vec<text_editor::Action>,
        patches: Option<(Vec<wire::EditorPatch>, wire::EditorCursor)>,
        history: wire::EditorHistoryEffect,
        pending: &mut Vec<wire::Event>,
    ) {
        let document_limit = super::editor_documents::edit_byte_limit(self, document).unwrap_or(0);
        let Some(shared) = self.editor_transactions.get(document).cloned() else {
            return;
        };
        let mut control = super::lock(&shared);
        let Some(front) = control.lane.front() else {
            return;
        };
        if !matches!(control.lane.phase(), Phase::Ready | Phase::Decision { .. }) {
            return;
        }
        let sequence = front.sequence;
        let input_time_ms = front.input.time_ms;
        let input_work = front.input.work.clone();
        let Some((owner, binding)) = self.editor_bindings.get(key).cloned() else {
            return;
        };
        if owner != document {
            return;
        }
        let Some(field) = self.editors.get_mut(key) else {
            return;
        };
        let before = state(field);
        let before_text_revision = control.text_revision;
        let Some(revision) = self.editor_revision.checked_add(1) else {
            control.lane.phase = Phase::Faulted(Fault::Limit);
            drop(control);
            self.editor_fault(document, pending);
            return;
        };
        let mut content = super::lock(&field.content);
        let is_patch = patches.is_some();
        let edit_kind = match history {
            wire::EditorHistoryEffect::Undo => wire::EditorEditKind::Undo,
            wire::EditorHistoryEffect::Redo => wire::EditorEditKind::Redo,
            _ if is_patch => wire::EditorEditKind::GuestPatch,
            _ if matches!(
                input_work,
                NativeWork::Event(Event::InputMethod(
                    iced::advanced::input_method::Event::Commit(_)
                ))
            ) =>
            {
                wire::EditorEditKind::ImeCommit
            }
            _ if matches!(&input_work, NativeWork::Event(Event::Keyboard(iced::keyboard::Event::KeyPressed { key: iced::keyboard::Key::Character(key), modifiers, .. })) if key.eq_ignore_ascii_case("x") && modifiers.command()) => {
                wire::EditorEditKind::Cut
            }
            _ => kind(&actions),
        };
        if let Some((patches, cursor)) = patches {
            if history == wire::EditorHistoryEffect::Native
                || apply_patches(&mut content, &patches, cursor).is_err()
            {
                control.lane.phase = Phase::Faulted(Fault::Identity);
                drop(content);
                drop(control);
                self.editor_fault(document, pending);
                return;
            }
        } else {
            for action in actions {
                content.perform(action);
            }
            if let NativeWork::Caret(mut cursor) = input_work {
                cursor.clamp(&content.text());
                content.move_to(text_editor::Cursor {
                    position: text_editor::Position {
                        line: cursor.position.line as usize,
                        column: cursor.position.column as usize,
                    },
                    selection: None,
                });
            }
        }
        if content.text().len() > document_limit {
            *content = super::editor_content(&before);
            control.lane.phase = Phase::Faulted(Fault::Overflow);
            drop(content);
            drop(control);
            self.editor_fault(document, pending);
            return;
        }
        if content.text() != before.text {
            let Some(revision) = control.text_revision.checked_add(1) else {
                *content = super::editor_content(&before);
                control.lane.phase = Phase::Faulted(Fault::Limit);
                drop(content);
                drop(control);
                self.editor_fault(document, pending);
                return;
            };
            control.text_revision = revision;
        }
        control.last_text = content.text();
        control.cursor = super::editor_cursor(&content);
        control.revision = revision;
        field.revision = revision;
        self.editor_revision = revision;
        let after = wire::EditorState {
            text: content.text(),
            cursor: super::editor_cursor(&content),
            reset: field.reset,
            revision,
        };
        drop(content);
        let id = control
            .pending
            .as_ref()
            .map(|request| request.id.clone())
            .unwrap_or(wire::EditorTransactionId {
                instance: self.instance,
                document: document.into(),
                reset: field.reset,
                sequence,
                attempt: 0,
                text_revision: before_text_revision,
                revision: before.revision,
            });
        if control.lane.commit(sequence).is_err() {
            return;
        }
        control.committed = Some(revision);
        control.bypass_claim = false;
        pending.push(wire::Event::EditorTransaction {
            handler: binding.on_event,
            event: wire::EditorTransactionEvent::Commit {
                id,
                before: reference(document, &before, before_text_revision),
                after: reference(document, &after, control.text_revision),
                patches: wire::editor_document::editor_changed_span(&before.text, &after.text)
                    .expect("validated native editor span"),
                kind: edit_kind,
                history,
                input_time_ms,
            },
        });
    }

    /// Call after merging/adopting the complete frame, even if its tree is unchanged.
    pub fn editor_frame(&mut self, frame: &wire::Frame, pending: &mut Vec<wire::Event>) -> bool {
        let previous_revision = self.editor_revision;
        let documents_changed = super::editor_documents::frame(self, frame, pending);
        pending.append(&mut self.editor_notifications);
        if !frame.busy {
            for shared in self.editor_transactions.values() {
                let mut control = super::lock(shared);
                let Some(front) = control.lane.front() else {
                    continue;
                };
                let sequence = front.sequence;
                let observed = self.editor_reported.get(&front.input.key).copied();
                if let (Some(committed), Some((reset, revision))) = (control.committed, observed)
                    && reset == control.reset
                    && revision >= committed
                {
                    let _ = control.lane.acknowledge(sequence);
                    control.pending = None;
                    control.committed = None;
                }
            }
        }
        for response in &frame.editor_decisions {
            let Some(shared) = self.editor_transactions.get(&response.id.document).cloned() else {
                continue;
            };
            let mut control = super::lock(&shared);
            let Some(request) = control.pending.clone() else {
                continue;
            };
            if response.id != request.id
                || response.id.instance != self.instance
                || response.id.reset != control.reset
            {
                continue;
            }
            let Some(front) = control.lane.front() else {
                continue;
            };
            let key = front.input.key.clone();
            let Some(field) = self.editors.get(&key) else {
                continue;
            };
            if field.revision != response.id.revision
                || control.text_revision != response.id.text_revision
            {
                let now = control.now_ms();
                if control.lane.conflict(now).is_ok() {
                    let attempt = match control.lane.phase() {
                        Phase::Decision { attempt, .. } => attempt,
                        _ => continue,
                    };
                    let mut retry = request;
                    retry.state =
                        super::editor_documents::current_reference(&retry.id.document, &control);
                    retry.id.revision = field.revision;
                    retry.id.attempt = attempt;
                    retry.id.text_revision = control.text_revision;
                    control.pending = Some(retry.clone());
                    if let Some((_, binding)) = self.editor_bindings.get(&key) {
                        pending.push(wire::Event::EditorKeyRequest {
                            handler: binding.on_request,
                            request: retry,
                        });
                    }
                }
                continue;
            }
            match control.lane.phase() {
                Phase::Faulted(Fault::Timeout) => {
                    let attempt = request.id.attempt;
                    control.lane.phase = Phase::Decision {
                        since_ms: control.now_ms(),
                        attempt,
                    };
                    control.fault_reported = false;
                }
                Phase::Faulted(_) | Phase::AwaitingGuest => continue,
                _ => {}
            }
            match &response.decision {
                wire::EditorDecision::DefaultEditorAction => {
                    control.bypass_claim = true;
                    control.lane.phase = Phase::Ready;
                }
                wire::EditorDecision::Noop => {
                    drop(control);
                    self.commit_editor(
                        &response.id.document,
                        &key,
                        Vec::new(),
                        None,
                        wire::EditorHistoryEffect::Native,
                        pending,
                    );
                }
                wire::EditorDecision::Apply {
                    patches,
                    cursor,
                    history,
                } => {
                    drop(control);
                    self.commit_editor(
                        &response.id.document,
                        &key,
                        Vec::new(),
                        Some((patches.clone(), *cursor)),
                        *history,
                        pending,
                    );
                }
            }
        }
        let faults: Vec<_> = self
            .editor_transactions
            .iter()
            .filter_map(|(document, shared)| {
                let control = super::lock(shared);
                (matches!(control.lane.phase(), Phase::Faulted(_)) && !control.fault_reported)
                    .then(|| document.clone())
            })
            .collect();
        for document in faults {
            self.editor_fault(&document, pending);
        }
        documents_changed || previous_revision != self.editor_revision
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    #[test]
    fn native_content_applies_multiple_patches_and_clears_selection() {
        let mut content = text_editor::Content::with_text("1. 한글\n2. next");
        content.perform(text_editor::Action::SelectAll);
        let cursor = wire::EditorCursor {
            position: wire::EditorPosition { line: 0, column: 7 },
            selection: None,
        };
        apply_patches(
            &mut content,
            &[
                wire::EditorPatch {
                    start_byte: 0,
                    end_byte: 1,
                    replacement: "10".into(),
                },
                wire::EditorPatch {
                    start_byte: 10,
                    end_byte: 11,
                    replacement: "11".into(),
                },
            ],
            cursor,
        )
        .unwrap();
        assert_eq!(content.text(), "10. 한글\n11. next");
        assert_eq!(super::super::editor_cursor(&content), cursor);
        let before = content.text();
        assert!(
            apply_patches(
                &mut content,
                &[wire::EditorPatch {
                    start_byte: 5,
                    end_byte: 7,
                    replacement: "bad".into()
                }],
                cursor
            )
            .is_err()
        );
        assert_eq!(content.text(), before);
    }
    #[test]
    fn rebinding_a_widget_key_cancels_its_old_document_lane() {
        let editor = |key: &str, document: &str| wire::Node::Editor {
            key: key.into(),
            placeholder: String::new(),
            document: wire::editor_document::EditorDocumentRef {
                document: document.into(),
                reset: 0,
                text_revision: 0,
                revision: 0,
                cursor: wire::EditorCursor::default(),
                byte_len: 2,
            },
            options: Box::new(wire::EditorOptions {
                binding: Some(Box::new(wire::EditorBinding {
                    claims: vec![],
                    on_request: 1,
                    on_event: if key == "owner" { 22 } else { 11 },
                })),
                ..Default::default()
            }),
            on_document: 3,
            editable: true,
            width: None,
            height: None,
            min_height: None,
            max_height: None,
        };
        let tree = |owner| wire::Node::Linear {
            key: "root".into(),
            axis: wire::Axis::Column,
            children: vec![editor("owner", owner), editor("base", "A")],
            spacing: None,
            padding: None,
            width: None,
            height: None,
            max_width: None,
            clip: false,
            wrap: None,
            align: None,
            background: None,
            border: None,
        };
        let mut inputs = super::super::Inputs::default();
        super::super::editor_documents::test_support::assign(
            &mut inputs,
            &tree("A"),
            &[("A", "ab")],
        );
        let shared = inputs.editor_transactions["A"].clone();
        {
            let mut control = super::super::lock(&shared);
            control.fault_key = Some("owner".into());
            control.lane.fail(Fault::Overflow);
        }
        let mut first_fault = vec![];
        inputs.editor_fault("A", &mut first_fault);
        assert!(
            matches!(
                first_fault.as_slice(),
                [wire::Event::EditorTransaction {
                    handler: 22,
                    event: wire::EditorTransactionEvent::Fault { .. }
                }]
            ),
            "rejected first input must retain its widget route"
        );
        {
            let mut control = super::super::lock(&shared);
            // Only the lane is rewound; the assigned document stays as its
            // transfer left it, since no test may stand a document up itself.
            control.lane = Lane::default();
            control.pending = None;
            control.fault_reported = false;
            control.fault_key = None;
            for (sequence, key) in [(1, "owner"), (2, "base")] {
                control
                    .lane
                    .admit(
                        sequence,
                        1,
                        NativeInput {
                            key: key.into(),
                            work: NativeWork::Event(Event::InputMethod(
                                iced::advanced::input_method::Event::Commit("x".into()),
                            )),
                            cursor: iced::mouse::Cursor::Unavailable,
                            clipboard: None,
                            time_ms: 0,
                        },
                    )
                    .unwrap();
            }
            control.lane.request(0);
            let state = super::super::editor_documents::current_reference("A", &control);
            control.pending = Some(wire::EditorKeyRequest {
                id: wire::EditorTransactionId {
                    instance: inputs.instance,
                    document: "A".into(),
                    reset: 0,
                    sequence: 1,
                    attempt: 1,
                    text_revision: 0,
                    revision: 0,
                },
                state,
                key: wire::keyboard::KeyState {
                    key: wire::keyboard::Key::Named(wire::keyboard::Named::Tab),
                    modified_key: wire::keyboard::Key::Named(wire::keyboard::Named::Tab),
                    physical_key: wire::keyboard::Physical::Unidentified(
                        wire::keyboard::NativeCode::Unidentified,
                    ),
                    modifiers: Default::default(),
                    location: wire::keyboard::Location::Standard,
                },
                repeat: false,
                input_time_ms: 0,
            });
        }
        let mut faults = vec![];
        super::super::lock(&shared).lane.fail(Fault::Limit);
        inputs.editor_fault("A", &mut faults);
        assert!(
            matches!(
                faults.as_slice(),
                [wire::Event::EditorTransaction {
                    handler: 22,
                    event: wire::EditorTransactionEvent::Fault { .. }
                }]
            ),
            "fault must return to the originating widget"
        );
        let rebound = super::super::editor_documents::test_support::assign(
            &mut inputs,
            &tree("B"),
            &[("A", "ab"), ("B", "ab")],
        );
        {
            let control = super::super::lock(&shared);
            assert_eq!(control.lane.front().unwrap().input.key, "base");
            assert_eq!(control.lane.phase(), Phase::Ready);
            assert!(control.pending.is_none());
        }
        let transactions: Vec<_> = rebound
            .iter()
            .filter(|event| matches!(event, wire::Event::EditorTransaction { .. }))
            .collect();
        assert!(
            matches!(
                transactions.as_slice(),
                [wire::Event::EditorTransaction {
                    handler: 22,
                    event: wire::EditorTransactionEvent::Cancelled { .. },
                }]
            ),
            "{transactions:?}"
        );
        let mut events = vec![];
        inputs.commit_editor(
            "A",
            "owner",
            vec![text_editor::Action::Edit(text_editor::Edit::Insert('!'))],
            None,
            wire::EditorHistoryEffect::Native,
            &mut events,
        );
        assert!(
            events.is_empty(),
            "old document cannot commit through rebound key"
        );
        assert_eq!(
            super::super::editor_documents::test_support::text(&inputs, "base"),
            "ab",
            "the old document keeps the text its transfer delivered"
        );
        // An oversized first event has no accepted front, but still has an
        // originating widget and must not pick a sibling's callback.
        let shared = inputs.editor_transactions["B"].clone();
        {
            let mut control = super::super::lock(&shared);
            control.fault_key = Some("owner".into());
            control.lane.fail(Fault::Overflow);
        }
        inputs.editor_fault("B", &mut events);
        assert!(matches!(
            events.as_slice(),
            [wire::Event::EditorTransaction {
                handler: 22,
                event: wire::EditorTransactionEvent::Fault { .. }
            }]
        ));
    }

    #[test]
    fn removing_overlay_owner_preserves_later_base_input() {
        let mut lane = Lane::default();
        lane.admit(1, 7, "overlay").unwrap();
        lane.request(10);
        lane.admit(2, 4, "base").unwrap();
        let cancelled = lane.remove_inputs(|owner| *owner == "overlay");
        assert_eq!(cancelled[0].sequence, 1);
        assert_eq!(lane.phase(), Phase::Ready);
        assert_eq!(lane.front().unwrap().input, "base");
    }
    #[test]
    fn excessive_conflicts_retain_the_key_and_freeze_the_lane() {
        let mut lane = Lane::default();
        lane.admit(9, 1, 'x').unwrap();
        lane.request(0);
        for now in 1..4 {
            lane.conflict(now).unwrap();
        }
        assert_eq!(lane.conflict(4), Err(Fault::Conflicts));
        assert_eq!(lane.phase(), Phase::Faulted(Fault::Conflicts));
        assert_eq!(lane.front().unwrap().sequence, 9);
    }
}

#[cfg(test)]
mod document_budget_tests {
    use super::super::{Inputs, editor_documents::test_support as session};
    use super::*;

    fn tree(specs: &[(&str, usize, usize)]) -> wire::Node {
        let children = specs
            .iter()
            .flat_map(|(name, bytes, count)| {
                (0..*count).map(move |index| wire::Node::Editor {
                    key: format!("{name}/{index}"),
                    placeholder: String::new(),
                    document: wire::editor_document::EditorDocumentRef {
                        document: (*name).into(),
                        reset: 0,
                        text_revision: 0,
                        revision: 0,
                        cursor: wire::EditorCursor::default(),
                        byte_len: *bytes as u32,
                    },
                    options: Box::new(wire::EditorOptions {
                        binding: Some(Box::new(wire::EditorBinding {
                            claims: vec![],
                            on_request: 2,
                            on_event: 3,
                        })),
                        ..Default::default()
                    }),
                    on_document: 4,
                    editable: true,
                    width: None,
                    height: None,
                    min_height: None,
                    max_height: None,
                })
            })
            .collect();
        wire::Node::Linear {
            key: "root".into(),
            axis: wire::Axis::Column,
            children,
            spacing: None,
            padding: None,
            width: None,
            height: None,
            max_width: None,
            clip: false,
            wrap: None,
            align: None,
            background: None,
            border: None,
        }
    }
    fn assigned(specs: &[(&str, usize, usize)]) -> Inputs {
        let sources: Vec<_> = specs
            .iter()
            .map(|(name, bytes, _)| (*name, "x".repeat(*bytes)))
            .collect();
        let borrowed: Vec<_> = sources
            .iter()
            .map(|(name, text)| (*name, text.as_str()))
            .collect();
        let mut inputs = Inputs::default();
        session::assign(&mut inputs, &tree(specs), &borrowed);
        inputs
    }

    #[test]
    fn native_and_guest_edits_reserve_all_canonical_and_projection_bytes() {
        for (specs, growth) in [
            (
                vec![
                    ("a", 500_000, 1),
                    ("b", 900_000, 1),
                    ("c", 900_000, 1),
                    ("d", 900_000, 1),
                    ("e", 900_000, 1),
                ],
                100_000,
            ),
            (vec![("a", 900_000, 9)], 50_000),
        ] {
            for guest_patch in [false, true] {
                let mut inputs = assigned(&specs);
                let before = session::reference(&inputs, "a/0");
                let old_text = session::text(&inputs, "a/0");
                let replacement = "y".repeat(growth);
                let action = text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(
                    replacement.clone(),
                )));
                let mut events = vec![];
                inputs.admit_editor_work(
                    "a/0",
                    0,
                    NativeWork::Actions(vec![action.clone()]),
                    &mut events,
                );
                if guest_patch {
                    inputs.commit_editor(
                        "a",
                        "a/0",
                        vec![],
                        Some((
                            vec![wire::EditorPatch {
                                start_byte: 0,
                                end_byte: 0,
                                replacement,
                            }],
                            wire::EditorCursor::default(),
                        )),
                        wire::EditorHistoryEffect::NewGroup,
                        &mut events,
                    );
                } else {
                    inputs.commit_editor(
                        "a",
                        "a/0",
                        vec![action],
                        None,
                        wire::EditorHistoryEffect::Native,
                        &mut events,
                    );
                }
                assert!(
                    events.iter().any(|event| matches!(
                        event,
                        wire::Event::EditorTransaction {
                            event: wire::EditorTransactionEvent::Fault {
                                reason: wire::EditorFault::Overflow,
                                ..
                            },
                            ..
                        }
                    )),
                    "an individually valid edit exceeding the shared budget must explicitly fault"
                );
                assert!(
                    !events.iter().any(|event| matches!(
                        event,
                        wire::Event::EditorTransaction {
                            event: wire::EditorTransactionEvent::Commit { .. },
                            ..
                        }
                    )),
                    "a refused edit cannot enter guest history"
                );
                assert_eq!(session::reference(&inputs, "a/0"), before);
                assert!(
                    session::text(&inputs, "a/0") == old_text,
                    "canonical text survives rejection"
                );
                assert!(
                    super::super::lock(&inputs.editors["a/0"].content).text() == old_text,
                    "native Content rolls back atomically"
                );
            }
        }
    }

    #[test]
    fn stale_echo_cannot_hide_the_cost_of_new_native_projections() {
        let mut inputs = assigned(&[("a", 512_000, 4), ("b", 512_000, 4)]);
        let events = session::edit(
            &mut inputs,
            "a/0",
            0,
            text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new("y".repeat(300_000)))),
        );
        assert!(
            events.iter().any(|event| matches!(
                event,
                wire::Event::EditorTransaction {
                    event: wire::EditorTransactionEvent::Commit { .. },
                    ..
                }
            )),
            "the first growth is within all budgets"
        );
        let candidate = tree(&[("a", 512_000, 9), ("b", 512_000, 4)]);
        assert!(
            inputs.validate_editor_documents(&candidate).is_err(),
            "stale smaller refs must not hide actual projection bytes"
        );
        inputs.adopt(&candidate);
        assert_eq!(
            inputs.editors.len(),
            8,
            "rejection retains the previous projections"
        );
        assert_eq!(session::text(&inputs, "a/0").len(), 812_000);
    }
}

//! Transfer readiness is a prerequisite of the existing logical editor lane.
use super::{EditorField, Inputs, editor_transactions, lock};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::Ordering};
use ui_lang_wire as wire;
use wire::editor_document::{
    EditorDocumentMessage as Message, EditorDocumentRef, EditorTransferId, EditorTransferReceiver,
};

#[derive(Clone, Debug)]
pub(super) struct Reference {
    pub document: EditorDocumentRef,
    pub handler: u32,
    pub editable: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Incoming {
    id: EditorTransferId,
    target: EditorDocumentRef,
    handler: u32,
    receiver: Arc<Mutex<EditorTransferReceiver>>,
}

#[derive(Clone, Debug)]
pub(super) struct Outgoing {
    id: EditorTransferId,
    target: EditorDocumentRef,
    handler: u32,
    sender: Arc<Mutex<wire::editor_document::EditorTransferSender>>,
}
pub(super) fn current_reference(
    document: &str,
    control: &editor_transactions::Control,
) -> EditorDocumentRef {
    EditorDocumentRef {
        document: document.into(),
        reset: control.reset,
        text_revision: control.text_revision,
        revision: control.revision,
        cursor: control.cursor,
        byte_len: control.last_text.len() as u32,
    }
}
fn request_mirror(
    inputs: &mut Inputs,
    id: &EditorTransferId,
    target: &EditorDocumentRef,
    pending: &mut Vec<wire::Event>,
) {
    if inputs.editor_transfer.is_some() || inputs.editor_outgoing.is_some() {
        return;
    }
    let Some(shared) = inputs.editor_transactions.get(&id.document) else {
        return;
    };
    let control = lock(shared);
    let Some(request) = &control.pending else {
        return;
    };
    if id.instance != inputs.instance
        || id.serial != request.id.sequence
        || id.attempt != request.id.attempt
        || id.reset != control.reset
        || request.state != *target
    {
        return;
    }
    let Some(front) = control.lane.front() else {
        return;
    };
    let Some(reference) = inputs.editor_references.get(&front.input.key) else {
        return;
    };
    if reference.document.document != id.document {
        return;
    }
    if current_reference(&id.document, &control) != *target {
        pending.push(wire::Event::EditorDocument {
            handler: reference.handler,
            message: Message::Failed {
                id: id.clone(),
                reason: wire::editor_document::EditorTransferError::Identity,
            },
        });
        return;
    }
    let Ok(sender) = wire::editor_document::EditorTransferSender::new(id.clone(), target.clone())
    else {
        return;
    };
    inputs.editor_outgoing = Some(Outgoing {
        id: id.clone(),
        target: target.clone(),
        handler: reference.handler,
        sender: Arc::new(Mutex::new(sender)),
    });
}
fn send_mirror(inputs: &mut Inputs, pending: &mut Vec<wire::Event>) {
    let Some(outgoing) = inputs.editor_outgoing.take() else {
        return;
    };
    let active = inputs
        .editor_transactions
        .get(&outgoing.id.document)
        .is_some_and(|shared| {
            let control = lock(shared);
            control.reset == outgoing.id.reset
                && matches!(
                    control.lane.phase(),
                    editor_transactions::Phase::Decision { .. }
                )
                && control.pending.as_ref().is_some_and(|request| {
                    request.id.instance == outgoing.id.instance
                        && request.id.sequence == outgoing.id.serial
                        && request.id.attempt == outgoing.id.attempt
                })
        });
    if !active {
        pending.push(wire::Event::EditorDocument {
            handler: outgoing.handler,
            message: Message::Failed {
                id: outgoing.id,
                reason: wire::editor_document::EditorTransferError::Aborted,
            },
        });
        return;
    }
    let control = lock(&inputs.editor_transactions[&outgoing.id.document]);
    let reference = current_reference(&outgoing.id.document, &control);
    let next = lock(&outgoing.sender).next_frame(&reference, &control.last_text);
    let mut retain = true;
    match next {
        Ok(Some(transfer)) => {
            retain = !matches!(
                transfer,
                wire::editor_document::EditorTransfer::Abort { .. }
            );
            pending.push(wire::Event::EditorDocument {
                handler: outgoing.handler,
                message: Message::Transfer(transfer),
            });
        }
        Ok(None) => {}
        Err(reason) => {
            retain = false;
            pending.push(wire::Event::EditorDocument {
                handler: outgoing.handler,
                message: Message::Failed {
                    id: outgoing.id.clone(),
                    reason,
                },
            });
        }
    }
    drop(control);
    if retain {
        inputs.editor_outgoing = Some(outgoing);
    }
}

// Reserve each logical document once and each native projection separately.
// Echoed refs can lag native commits, so live canonical lengths take precedence.
fn reservations(
    inputs: &Inputs,
    references: &HashMap<String, Reference>,
) -> HashMap<String, (usize, usize)> {
    let mut sizes = HashMap::new();
    for reference in references.values() {
        let target = &reference.document;
        let size = sizes.entry(target.document.clone()).or_insert_with(|| {
            let bytes = inputs
                .editor_transactions
                .get(&target.document)
                .map(|shared| lock(shared))
                .filter(|control| control.loaded && control.reset == target.reset)
                .map_or(target.byte_len as usize, |control| control.last_text.len());
            (bytes, 0)
        });
        size.1 += 1;
    }
    sizes
}

/// The candidate edit replaces one reservation; all other current documents and
/// incomplete assignments remain reserved. No document text is cloned here.
pub(super) fn edit_byte_limit(inputs: &Inputs, document: &str) -> Option<usize> {
    use wire::editor_document::{
        MAX_EDITOR_DOCUMENT_BYTES, MAX_EDITOR_LIVE_BYTES, MAX_EDITOR_PROJECTION_BYTES,
    };
    let sizes = reservations(inputs, &inputs.editor_references);
    let (_, projections) = sizes.get(document)?;
    let mut canonical = MAX_EDITOR_LIVE_BYTES;
    let mut projected = MAX_EDITOR_PROJECTION_BYTES;
    for (name, (bytes, count)) in &sizes {
        if name == document {
            continue;
        }
        canonical = canonical.checked_sub(*bytes)?;
        projected = projected.checked_sub(bytes.checked_mul(*count)?)?;
    }
    Some(
        MAX_EDITOR_DOCUMENT_BYTES
            .min(canonical)
            .min(projected.checked_div(*projections)?),
    )
}

pub(super) fn validate(
    inputs: &Inputs,
    references: &HashMap<String, Reference>,
) -> Result<(), (EditorDocumentRef, &'static str)> {
    if wire::editor_document::validate_editor_document_refs(
        references.values().map(|reference| &reference.document),
    )
    .is_err()
        && let Some(reference) = references.values().next()
    {
        return Err((
            reference.document.clone(),
            "invalid editor document references or budget",
        ));
    }
    let sizes = reservations(inputs, references);
    let canonical = sizes
        .values()
        .try_fold(0usize, |total, (bytes, _)| total.checked_add(*bytes));
    let projected = sizes.values().try_fold(0usize, |total, (bytes, count)| {
        total.checked_add(bytes.checked_mul(*count)?)
    });
    if (canonical.is_none_or(|bytes| bytes > wire::editor_document::MAX_EDITOR_LIVE_BYTES)
        || projected.is_none_or(|bytes| bytes > wire::editor_document::MAX_EDITOR_PROJECTION_BYTES))
        && let Some(reference) = references.values().next()
    {
        return Err((
            reference.document.clone(),
            "live editor document or projection budget",
        ));
    }
    for reference in references.values() {
        let target = &reference.document;
        if let Some(control) = inputs.editor_transactions.get(&target.document) {
            let control = lock(control);
            if control.loaded && control.reset == target.reset {
                if target.text_revision > control.text_revision {
                    return Err((
                        target.clone(),
                        "editor reference advanced without an assignment",
                    ));
                }
                if target.text_revision == control.text_revision
                    && target.validate_text(&control.last_text).is_err()
                {
                    return Err((
                        target.clone(),
                        "editor reference does not describe its document",
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn adopt(inputs: &mut Inputs) {
    if inputs
        .editor_document_fault
        .as_ref()
        .is_some_and(|(failed, _)| {
            !inputs
                .editor_references
                .values()
                .any(|reference| &reference.document == failed)
        })
    {
        inputs.editor_document_fault = None;
    }
    if inputs.editor_transfer.as_ref().is_some_and(|incoming| {
        !inputs
            .editor_references
            .values()
            .any(|reference| reference.document == incoming.target)
    }) {
        let incoming = inputs.editor_transfer.take().unwrap();
        let _ = lock(&incoming.receiver).receive(&wire::editor_document::EditorTransfer::Abort {
            id: incoming.id.clone(),
        });
        inputs
            .editor_notifications
            .push(wire::Event::EditorDocument {
                handler: incoming.handler,
                message: Message::Failed {
                    id: incoming.id,
                    reason: wire::editor_document::EditorTransferError::Aborted,
                },
            });
    }
    for reference in inputs.editor_references.values() {
        let target = &reference.document;
        let control = inputs
            .editor_transactions
            .entry(target.document.clone())
            .or_insert_with(|| {
                Arc::new(Mutex::new(editor_transactions::Control::new(
                    target.reset,
                    inputs.editor_sequence.clone(),
                )))
            });
        let mut control = lock(control);
        control.available = control.loaded && control.reset == target.reset;
    }
    // New projections and sibling observations reuse the canonical document;
    // an unchanged frame never rebuilds Content or retransfers text.
    for (key, reference) in &inputs.editor_references {
        let Some(shared) = inputs.editor_transactions.get(&reference.document.document) else {
            continue;
        };
        let control = lock(shared);
        if !control.available {
            continue;
        }
        match inputs.editors.get_mut(key) {
            Some(field)
                if field.document == reference.document.document
                    && field.reset == control.reset
                    && field.revision == control.revision => {}
            Some(field) => {
                *lock(&field.content) =
                    super::editor_content_parts(&control.last_text, control.cursor);
                field.document = reference.document.document.clone();
                field.reset = control.reset;
                field.revision = control.revision;
            }
            None => {
                inputs.editors.insert(
                    key.clone(),
                    EditorField {
                        document: reference.document.document.clone(),
                        content: Arc::new(Mutex::new(super::editor_content_parts(
                            &control.last_text,
                            control.cursor,
                        ))),
                        reset: control.reset,
                        revision: control.revision,
                    },
                );
            }
        }
    }
    schedule(inputs);
}

fn schedule(inputs: &mut Inputs) {
    if inputs.editor_transfer.is_some()
        || inputs.editor_outgoing.is_some()
        || inputs.editor_document_fault.is_some()
    {
        return;
    }
    let mut references: Vec<_> = inputs.editor_references.iter().collect();
    references.sort_by_key(|(key, _)| *key);
    let next = references
        .into_iter()
        .find(|(_, reference)| {
            inputs
                .editor_transactions
                .get(&reference.document.document)
                .is_none_or(|control| !lock(control).available)
        })
        .map(|(_, reference)| reference.clone());
    let Some(reference) = next else {
        return;
    };
    let Ok(previous) =
        inputs
            .editor_sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
    else {
        inputs.editor_document_fault = Some((reference.document, "editor transfer sequence limit"));
        return;
    };
    let serial = previous + 1;
    let id = EditorTransferId {
        instance: inputs.instance,
        document: reference.document.document.clone(),
        reset: reference.document.reset,
        serial,
        attempt: 0,
    };
    let receiver = EditorTransferReceiver::new(id.clone(), reference.document.clone())
        .expect("validated document reference");
    inputs
        .editor_notifications
        .push(wire::Event::EditorDocument {
            handler: reference.handler,
            message: Message::Request {
                id: id.clone(),
                target: reference.document.clone(),
            },
        });
    inputs.editor_transfer = Some(Incoming {
        id,
        target: reference.document,
        handler: reference.handler,
        receiver: Arc::new(Mutex::new(receiver)),
    });
}

pub(super) fn frame(
    inputs: &mut Inputs,
    frame: &wire::Frame,
    pending: &mut Vec<wire::Event>,
) -> bool {
    let mut changed = false;
    for message in &frame.editor_documents {
        if let Message::Request { id, target } = message {
            request_mirror(inputs, id, target, pending);
            continue;
        }
        if let Message::Failed { id, .. } = message
            && inputs
                .editor_outgoing
                .as_ref()
                .is_some_and(|outgoing| &outgoing.id == id)
        {
            inputs.editor_outgoing = None;
            if let Some(shared) = inputs.editor_transactions.get(&id.document) {
                lock(shared).lane.fail(editor_transactions::Fault::Identity);
            }
            inputs.editor_fault(&id.document, pending);
            continue;
        }
        if let Message::Acknowledged { id } = message {
            if inputs
                .editor_outgoing
                .as_ref()
                .is_some_and(|outgoing| &outgoing.id == id)
            {
                let outgoing = inputs.editor_outgoing.take().unwrap();
                inputs.retry_editor_after_mirror(&outgoing.id, &outgoing.target, pending);
            }
            continue;
        }
        let Some(incoming) = &mut inputs.editor_transfer else {
            continue;
        };
        if message.id() != &incoming.id {
            continue;
        }
        let result = match message {
            Message::Transfer(transfer) => lock(&incoming.receiver).receive(transfer),
            Message::Failed { reason, .. } => Err(*reason),
            _ => continue,
        };
        match result {
            Ok(None) => {}
            Ok(Some(text)) => {
                let incoming = inputs.editor_transfer.take().unwrap();
                let target = incoming.target;
                let Some(shared) = inputs.editor_transactions.get(&target.document).cloned() else {
                    continue;
                };
                let mut control = lock(&shared);
                control.last_text = text;
                control.reset = target.reset;
                control.text_revision = target.text_revision;
                control.revision = target.revision;
                control.cursor = target.cursor;
                control.loaded = true;
                control.available = true;
                for (key, reference) in &inputs.editor_references {
                    if reference.document != target {
                        continue;
                    }
                    let content = super::editor_content_parts(&control.last_text, target.cursor);
                    inputs.editors.insert(
                        key.clone(),
                        EditorField {
                            document: target.document.clone(),
                            content: Arc::new(Mutex::new(content)),
                            reset: target.reset,
                            revision: target.revision,
                        },
                    );
                }
                inputs.editor_revision = inputs.editor_revision.max(target.revision);
                pending.push(wire::Event::EditorDocument {
                    handler: incoming.handler,
                    message: Message::Acknowledged { id: incoming.id },
                });
                changed = true;
            }
            Err(reason) => {
                let incoming = inputs.editor_transfer.take().unwrap();
                let _ = lock(&incoming.receiver).receive(
                    &wire::editor_document::EditorTransfer::Abort {
                        id: incoming.id.clone(),
                    },
                );
                inputs.editor_document_fault =
                    Some((incoming.target, "editor document transfer failed"));
                pending.push(wire::Event::EditorDocument {
                    handler: incoming.handler,
                    message: Message::Failed {
                        id: incoming.id,
                        reason,
                    },
                });
            }
        }
    }
    send_mirror(inputs, pending);
    schedule(inputs);
    changed
}

/// A scoped read of the canonical document. Reading metadata never clones text.
pub struct EditorDocument<'a> {
    document: &'a str,
    control: std::sync::MutexGuard<'a, editor_transactions::Control>,
}
impl EditorDocument<'_> {
    pub fn text(&self) -> &str {
        &self.control.last_text
    }
    pub fn reference(&self) -> EditorDocumentRef {
        current_reference(self.document, &self.control)
    }
}

impl Inputs {
    /// Borrow an available document by projection key; incomplete assignments
    /// expose no prefix. Drop the read before delivering further editor input.
    pub fn editor_document(&self, key: &str) -> Option<EditorDocument<'_>> {
        let reference = self.editor_references.get(key)?;
        let shared = self.editor_transactions.get(&reference.document.document)?;
        let control = lock(shared);
        if !control.available {
            return None;
        }
        Some(EditorDocument {
            document: &reference.document.document,
            control,
        })
    }

    /// Validate an entire candidate projection before replacing the accepted tree.
    pub fn validate_editor_documents(&self, root: &wire::Node) -> Result<(), &'static str> {
        let mut fields = HashMap::new();
        let mut references = HashMap::new();
        super::collect_inputs(root, &mut fields, &mut references);
        validate(self, &references).map_err(|(_, reason)| reason)
    }

    /// Keep only native projections proven identical to a fully assembled new
    /// session. Pending inputs, decision routes and transfer buffers never move.
    pub fn retain_restored_projections(
        &mut self,
        previous: &Self,
        root: &wire::Node,
    ) -> Result<(), &'static str> {
        self.validate_editor_documents(root)?;
        if !self.editor_documents_status()? {
            return Err("replacement editor documents are incomplete");
        }
        for (key, field) in &mut self.editors {
            let Some(old) = previous.editors.get(key) else {
                continue;
            };
            if old.document != field.document
                || old.reset != field.reset
                || old.revision != field.revision
            {
                continue;
            }
            let Some(shared) = self.editor_transactions.get(&field.document) else {
                continue;
            };
            let control = lock(shared);
            let old_content = lock(&old.content);
            if old_content.text() == control.last_text
                && super::editor_cursor(&old_content) == control.cursor
            {
                field.content = old.content.clone();
            }
        }
        self.fields = previous.fields.clone();
        self.combos = previous.combos.clone();
        super::combo::retain_restored(root, &mut self.combos);
        self.adopt(root);
        Ok(())
    }

    /// Candidate first-frame validation must wait for all referenced documents.
    pub fn editor_documents_status(&self) -> Result<bool, &'static str> {
        if let Some((_, reason)) = &self.editor_document_fault {
            return Err(reason);
        }
        Ok(self.editor_transfer.is_none()
            && self.editor_references.values().all(|reference| {
                self.editor_transactions
                    .get(&reference.document.document)
                    .is_some_and(|control| lock(control).available)
            }))
    }
}

/// Real document sessions for editor tests. A projection whose text never
/// crossed a transfer proves nothing about the lane that owns it, so no test
/// writes `last_text`, `loaded` or a `Content` to stand a document up.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::view_tree::Output;
    use crate::view_tree::editor_transactions::Batch;
    use iced::widget::text_editor;

    fn documents(messages: Vec<Message>) -> wire::Frame {
        wire::Frame {
            editor_documents: messages,
            ..Default::default()
        }
    }

    /// One host tick: adopt the tree, then deliver the frame that drains the
    /// notifications it queued. This is also the guest echo that acknowledges
    /// a commit, when `root` carries the reference the guest just heard.
    pub(crate) fn settle(inputs: &mut Inputs, root: &wire::Node) -> Vec<wire::Event> {
        inputs.adopt(root);
        let mut events = Vec::new();
        inputs.editor_frame(&documents(Vec::new()), &mut events);
        events
    }

    /// The projection keys a document is bound to, so the no-prefix rule can
    /// be checked where a reader would actually look.
    fn projections(inputs: &Inputs, document: &str) -> Vec<String> {
        inputs
            .editor_references
            .iter()
            .filter(|(_, reference)| reference.document.document == document)
            .map(|(key, _)| key.clone())
            .collect()
    }

    /// Answer every document request the host makes with a real bounded
    /// transfer of the named source, one wire frame per message, and check the
    /// two rules the transfer owes its reader: no prefix is visible before
    /// `Complete`, and `Complete` is acknowledged by the exact requested id.
    pub(crate) fn assign(
        inputs: &mut Inputs,
        root: &wire::Node,
        sources: &[(&str, &str)],
    ) -> Vec<wire::Event> {
        let mut events = settle(inputs, root);
        let mut answered: Vec<EditorTransferId> = Vec::new();
        loop {
            let next = events.iter().find_map(|event| match event {
                wire::Event::EditorDocument {
                    message: Message::Request { id, target },
                    ..
                } if !answered.contains(id) => Some((id.clone(), target.clone())),
                _ => None,
            });
            let Some((id, target)) = next else {
                break;
            };
            answered.push(id.clone());
            let text = sources
                .iter()
                .find(|(document, _)| *document == id.document)
                .unwrap_or_else(|| panic!("no source text for document {}", id.document))
                .1;
            let keys = projections(inputs, &id.document);
            let fresh = keys.iter().all(|key| inputs.editor_document(key).is_none());
            let mut sender =
                wire::editor_document::EditorTransferSender::new(id.clone(), target.clone())
                    .expect("the host requested a valid reference");
            while let Some(transfer) = sender
                .next_frame(&target, text)
                .expect("a bounded source document")
            {
                let complete = matches!(
                    transfer,
                    wire::editor_document::EditorTransfer::Complete { .. }
                );
                let before = events.len();
                inputs.editor_frame(&documents(vec![Message::Transfer(transfer)]), &mut events);
                if complete {
                    assert!(
                        events[before..].iter().any(|event| matches!(
                            event,
                            wire::Event::EditorDocument {
                                message: Message::Acknowledged { id: acknowledged },
                                ..
                            } if acknowledged == &id
                        )),
                        "a complete transfer is acknowledged by its exact identity"
                    );
                } else if fresh {
                    for key in &keys {
                        assert!(
                            inputs.editor_document(key).is_none(),
                            "an incomplete transfer exposed a prefix of {key}"
                        );
                    }
                }
            }
        }
        events
    }

    /// The canonical document a reader sees, by projection key.
    pub(crate) fn text(inputs: &Inputs, key: &str) -> String {
        inputs
            .editor_document(key)
            .expect("an assigned document")
            .text()
            .to_owned()
    }

    /// The reference a guest echoes after hearing this document's commit.
    pub(crate) fn reference(inputs: &Inputs, key: &str) -> EditorDocumentRef {
        inputs
            .editor_document(key)
            .expect("an assigned document")
            .reference()
    }

    /// Admission alone no longer commits. Replay the admitted front exactly as
    /// the native widget does, so ordinary and claimed input share one lane.
    fn replay(
        inputs: &mut Inputs,
        key: &str,
        reset: u64,
        admit: Output,
        actions: Vec<text_editor::Action>,
    ) -> Vec<wire::Event> {
        let mut events = Vec::new();
        inputs.apply(admit, &mut events);
        let Some(document) = inputs
            .editor_references
            .get(key)
            .map(|reference| reference.document.document.clone())
        else {
            return events;
        };
        let admitted = inputs
            .editor_transactions
            .get(&document)
            .and_then(|shared| {
                let control = lock(shared);
                control
                    .lane
                    .front()
                    .filter(|front| front.input.key == key)
                    .map(|front| front.sequence)
            });
        let Some(sequence) = admitted else {
            return events;
        };
        inputs.apply(
            Output::EditorBatch(Batch {
                document,
                key: key.into(),
                sequence,
                reset,
                actions,
                request: None,
            }),
            &mut events,
        );
        events
    }

    pub(crate) fn edit(
        inputs: &mut Inputs,
        key: &str,
        reset: u64,
        action: text_editor::Action,
    ) -> Vec<wire::Event> {
        replay(
            inputs,
            key,
            reset,
            Output::EditorAction {
                reset,
                key: key.into(),
                action: action.clone(),
            },
            vec![action],
        )
    }
}

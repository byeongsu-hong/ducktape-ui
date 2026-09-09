use ui_lang_guest::{
    App, Driver, Editor, EditorBinding, EditorDocumentUpdate, EditorTransaction, SnapshotApp, wire,
};

#[derive(Clone, Debug)]
enum Update {
    Document(EditorDocumentUpdate),
    Transaction(EditorTransaction<Update>),
    Committed(String),
}
struct DocumentApp<const LARGE: bool = false> {
    editor: Editor,
    commits: usize,
}
impl<const LARGE: bool> App for DocumentApp<LARGE> {
    type Message = Update;
    fn boot() -> (Self, iced::Task<Update>) {
        (
            Self {
                editor: Editor::new(if LARGE {
                    "x".repeat(wire::editor_document::MAX_EDITOR_DOCUMENT_BYTES)
                } else {
                    "original".into()
                }),
                commits: 0,
            },
            iced::Task::none(),
        )
    }
    fn view(&self) -> wire::Node {
        let (document, on_document) = self.editor.document("app:draft".into(), Update::Document);
        let binding = EditorBinding::new(
            vec![],
            |_| wire::EditorDecision::Noop,
            |event| match event {
                ui_lang_guest::EditorTransactionEvent::Commit { before, after, .. } => {
                    assert_eq!(before.text, "original");
                    Some(after.text.to_owned())
                }
                _ => None,
            },
        )
        .register(Update::Committed, Update::Transaction);
        wire::Node::Editor {
            options: Box::new(wire::EditorOptions {
                binding: Some(Box::new(binding)),
                ..Default::default()
            }),
            key: "draft".into(),
            placeholder: format!("commits={}", self.commits),
            document,
            on_document,
            editable: true,
            width: None,
            height: None,
            min_height: None,
            max_height: None,
        }
    }
    fn subscription(&self) -> iced::Subscription<Update> {
        iced::Subscription::none()
    }
    fn update(&mut self, update: Update) -> iced::Task<Update> {
        match update {
            Update::Document(update) => update.apply(&mut self.editor),
            Update::Transaction(update) => {
                return update
                    .apply(&mut self.editor)
                    .map_or_else(iced::Task::none, iced::Task::done);
            }
            Update::Committed(text) => {
                assert_eq!(
                    self.editor.text(),
                    text,
                    "mirror is accepted before the authored history route"
                );
                self.commits += 1;
            }
        }
        iced::Task::none()
    }
}
impl<const LARGE: bool> SnapshotApp for DocumentApp<LARGE> {
    fn snapshot(&self) -> Result<Vec<u8>, String> {
        Ok(self.editor.snapshot())
    }
    fn restore(bytes: &[u8]) -> Result<Self, String> {
        Ok(Self {
            editor: Editor::restore(bytes).ok_or("invalid snapshot")?,
            commits: 0,
        })
    }
}

#[test]
fn metadata_commit_updates_the_mirror_before_borrowed_history_and_rejects_duplicate() {
    let mut driver = Driver::<DocumentApp>::new();
    let first = driver.tick(vec![]);
    let events = ui_lang_guest::testing::edit(&first, "draft", "original", "changed");
    let encoded = wire::encode(&events);
    assert!(
        !encoded.windows(8).any(|bytes| bytes == b"original"),
        "commit transports a patch, not the previous full text"
    );
    driver.tick(events.clone());
    let once = driver.snapshot().unwrap();
    driver.tick(events);
    assert_eq!(
        driver.snapshot().unwrap(),
        once,
        "duplicate commit cannot repeat history or mutate the mirror"
    );
    driver.tick(vec![wire::Event::Resync]);
    let frame = driver.tick(vec![wire::Event::Resync]);
    let wire::Node::Editor {
        placeholder,
        document,
        ..
    } = frame.root.unwrap()
    else {
        panic!("editor");
    };
    assert_eq!(placeholder, "commits=1");
    assert_eq!(document.byte_len, 7);
}

#[test]
fn large_document_construction_preserves_the_entire_mirror() {
    let text = "x".repeat(wire::editor_document::MAX_EDITOR_DOCUMENT_BYTES);
    let editor = std::panic::catch_unwind(|| Editor::new(text.clone()));
    assert!(
        editor.is_ok(),
        "the agreed one-MiB document must be accepted without a display-budget ceiling"
    );
    let editor = editor.unwrap();
    assert_eq!(editor.text(), text);
    assert_eq!(Editor::restore(&editor.snapshot()), Some(editor));
}

#[test]
fn one_mib_transfers_in_sixteen_bounded_chunks_and_unchanged_views_remain_small() {
    use wire::editor_document::{
        EditorDocumentMessage as Message, EditorTransferId, EditorTransferReceiver,
    };
    let mut driver = Driver::<DocumentApp<true>>::new();
    let initial = driver.tick(vec![]);
    assert!(
        wire::encode(&initial).len() < 2_048,
        "editor projection contains only metadata"
    );
    let wire::Node::Editor {
        document,
        on_document,
        ..
    } = initial.root.unwrap()
    else {
        panic!("editor");
    };
    assert_eq!(
        document.byte_len as usize,
        wire::editor_document::MAX_EDITOR_DOCUMENT_BYTES
    );
    let id = EditorTransferId {
        instance: 7,
        document: document.document.clone(),
        reset: document.reset,
        serial: 3,
        attempt: 0,
    };
    let mut receiver = EditorTransferReceiver::new(id.clone(), document.clone()).unwrap();
    let mut events = vec![wire::Event::EditorDocument {
        handler: on_document,
        message: Message::Request {
            id: id.clone(),
            target: document,
        },
    }];
    let mut complete = None;
    let mut chunks = 0;
    for _ in 0..18 {
        let frame = driver.tick(std::mem::take(&mut events));
        assert!(
            wire::encode(&frame).len() < 70_000,
            "one bounded payload per tick"
        );
        assert!(
            driver.snapshot().is_err(),
            "sender stays pending until receiver acknowledgment"
        );
        let [Message::Transfer(transfer)] = frame.editor_documents.as_slice() else {
            panic!("one progressing transfer message");
        };
        if matches!(
            transfer,
            wire::editor_document::EditorTransfer::Chunk { .. }
        ) {
            chunks += 1;
        }
        if let Some(text) = receiver.receive(transfer).unwrap() {
            complete = Some(text);
        }
    }
    assert_eq!(chunks, 16);
    assert_eq!(
        complete.unwrap(),
        "x".repeat(wire::editor_document::MAX_EDITOR_DOCUMENT_BYTES)
    );
    let settled = driver.tick(vec![wire::Event::EditorDocument {
        handler: on_document,
        message: Message::Acknowledged { id },
    }]);
    assert!(settled.editor_documents.is_empty());
    assert!(wire::encode(&settled).len() < 2_048);
    let snapshot = driver.snapshot().unwrap();
    let mut restored = Driver::<DocumentApp<true>>::from_snapshot(&snapshot, false).unwrap();
    let root = restored.tick(vec![]).root.unwrap();
    let wire::Node::Editor { document, .. } = root else {
        panic!("restored editor");
    };
    assert_eq!(
        document.byte_len as usize,
        wire::editor_document::MAX_EDITOR_DOCUMENT_BYTES
    );
}

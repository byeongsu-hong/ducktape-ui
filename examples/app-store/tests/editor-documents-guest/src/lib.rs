ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    Documents,
    "Large editor document fixture",
    "Revisioned one-MiB document and guest-owned undo",
    []
);

mod fixture {
    use ui_lang_guest::wire::{
        self, EditorDecision, EditorHistoryEffect, EditorKeyClaim, EditorState,
    };
    use ui_lang_guest::{Editor, EditorBinding, EditorTransactionEvent};
    use wire::keyboard::{Key, Modifiers};

    pub fn initial_document() -> Editor {
        // Exact one MiB; bounded short lines also exercise line/caret metadata.
        Editor::new(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde\n".repeat(16_384),
        )
    }
    pub fn remember(previous: Vec<u8>, event: Vec<u8>) -> Vec<u8> {
        if event.is_empty() { previous } else { event }
    }
    pub fn keys(previous: Vec<u8>) -> EditorBinding<Vec<u8>> {
        EditorBinding::new(
            vec![EditorKeyClaim {
                key: Key::Character("z".into()),
                modifiers: Modifiers::default(),
                command: true,
            }],
            move |request| {
                let Ok(state) = wire::decode::<EditorState>(&previous) else {
                    return EditorDecision::Noop;
                };
                EditorDecision::Apply {
                    patches: wire::editor_document::editor_changed_span(
                        request.state.text,
                        &state.text,
                    )
                    .expect("valid fixture undo span"),
                    cursor: state.cursor,
                    history: EditorHistoryEffect::Undo,
                }
            },
            |event| match event {
                EditorTransactionEvent::Commit {
                    before,
                    after,
                    history,
                    ..
                } if before.text_revision != after.text_revision
                    && history != EditorHistoryEffect::Undo =>
                {
                    Some(wire::encode(&EditorState {
                        text: before.text.into(),
                        cursor: before.cursor,
                        reset: before.reset,
                        revision: before.revision,
                    }))
                }
                _ => None,
            },
        )
    }
}

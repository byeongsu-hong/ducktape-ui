ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    Transactions,
    "Editor transactions fixture",
    "Ordered editor commits and guest history",
    []
);

mod fixture {
    use ui_lang_guest::wire::{
        self, EditorCursor, EditorDecision, EditorHistoryEffect, EditorKeyClaim, EditorPatch,
        EditorPosition, EditorState, EditorTransactionEvent,
    };
    use ui_lang_guest::{Editor, EditorBinding};
    use wire::keyboard::{Key, Modifiers, Named};

    #[derive(Clone, Debug, PartialEq)]
    pub struct History {
        pub undo: Vec<Vec<u8>>,
        pub redo: Vec<Vec<u8>>,
        pub commits: i64,
        pub ordered: bool,
        pub accepted: bool,
        pub last_sequence: i64,
        pub last_kind: String,
    }
    #[derive(Clone, Debug, PartialEq)]
    pub struct Outcome {
        pub before: Vec<u8>,
        pub after: Vec<u8>,
        pub sequence: i64,
        pub kind: String,
        pub history: String,
    }
    pub fn initial_history() -> History {
        History {
            undo: vec![],
            redo: vec![],
            commits: 0,
            ordered: true,
            accepted: true,
            last_sequence: -1,
            last_kind: String::new(),
        }
    }
    pub fn record(mut history: History, outcome: Outcome, document: Editor) -> History {
        let before: EditorState = wire::decode(&outcome.before).expect("fixture before state");
        let after: EditorState = wire::decode(&outcome.after).expect("fixture after state");
        history.accepted &= document.snapshot() == outcome.after;
        history.ordered &= outcome.sequence > history.last_sequence;
        history.last_sequence = outcome.sequence;
        history.commits += 1;
        if outcome.history == "Undo" {
            history.undo.pop().expect("undo decision had a snapshot");
            history.redo.push(outcome.before);
        } else if outcome.history == "Redo" {
            history.redo.pop().expect("redo decision had a snapshot");
            history.undo.push(outcome.before);
        } else if before.text != after.text {
            // One representative typing group: the structural prefix and its
            // immediately following insertions undo together. Paste starts a group.
            if outcome.kind != "Insert"
                || !matches!(history.last_kind.as_str(), "Insert" | "GuestPatch")
            {
                history.undo.push(outcome.before);
            }
            history.redo.clear();
        }
        if before.text != after.text {
            history.last_kind = outcome.kind;
        }
        history
    }
    pub fn keys(history: History) -> EditorBinding<Outcome> {
        let bare = Modifiers::default();
        let mut claims = [Named::Tab, Named::Enter, Named::Backspace]
            .into_iter()
            .map(|key| EditorKeyClaim {
                key: Key::Named(key),
                modifiers: bare,
                command: false,
            })
            .collect::<Vec<_>>();
        for shift in [false, true] {
            claims.push(EditorKeyClaim {
                key: Key::Character("z".into()),
                modifiers: Modifiers { shift, ..bare },
                command: true,
            });
        }
        EditorBinding::new(
            claims,
            move |request| match request.key.key {
                Key::Named(Named::Tab) => {
                    let position = request.state.cursor.position;
                    let cursor = EditorCursor {
                        position: EditorPosition {
                            line: position.line,
                            column: position.column + if position.line == 0 { 2 } else { 0 },
                        },
                        selection: None,
                    };
                    EditorDecision::Apply {
                        patches: vec![EditorPatch {
                            start_byte: 0,
                            end_byte: 0,
                            replacement: "  ".into(),
                        }],
                        cursor,
                        history: EditorHistoryEffect::NewGroup,
                    }
                }
                Key::Named(Named::Enter) => EditorDecision::DefaultEditorAction,
                Key::Named(Named::Backspace) => EditorDecision::Noop,
                Key::Character(ref key) if key == "z" => {
                    let redo = request.key.modifiers.shift;
                    let saved = if redo {
                        history.redo.last()
                    } else {
                        history.undo.last()
                    };
                    saved.map_or(EditorDecision::Noop, |bytes| {
                        let state: EditorState =
                            wire::decode(bytes).expect("fixture history state");
                        EditorDecision::Apply {
                            patches: vec![EditorPatch {
                                start_byte: 0,
                                end_byte: request.state.text.len() as u32,
                                replacement: state.text,
                            }],
                            cursor: state.cursor,
                            history: if redo {
                                EditorHistoryEffect::Redo
                            } else {
                                EditorHistoryEffect::Undo
                            },
                        }
                    })
                }
                _ => EditorDecision::DefaultEditorAction,
            },
            |event| match event {
                EditorTransactionEvent::Commit {
                    id,
                    before,
                    after,
                    kind,
                    history,
                    ..
                } => Some(Outcome {
                    before: wire::encode(&before),
                    after: wire::encode(&after),
                    sequence: id.sequence as i64,
                    kind: format!("{kind:?}"),
                    history: format!("{history:?}"),
                }),
                EditorTransactionEvent::Fault { reason, .. } => {
                    panic!("fixture transaction fault: {reason:?}")
                }
                EditorTransactionEvent::Cancelled { .. } => None,
            },
        )
    }
}

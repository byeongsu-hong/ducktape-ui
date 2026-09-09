ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    Presentation,
    "Editor presentation fixture",
    "Guest paint, caret menu and atomic interaction history",
    []
);

mod fixture {
    use ui_lang_guest::wire::{
        self, EditorDecision, EditorHistoryEffect, EditorKeyClaim, EditorState,
    };
    use ui_lang_guest::{Editor, EditorBinding, EditorTransactionEvent};
    use wire::keyboard::{Key, Modifiers};

    pub fn initial_document() -> Editor {
        Editor::new("Title\n한글 link [ ]")
    }
    pub fn paint(
        state: ui_lang_guest::EditorStateView<'_>,
    ) -> wire::editor_presentation::EditorPresentation {
        use wire::editor_presentation::*;
        EditorPresentation {
            padding: Some(wire::Edges {
                top: 8.0,
                right: 32.0,
                bottom: 8.0,
                left: 48.0,
            }),
            formats: vec![EditorFormat {
                line_background: Some(wire::Rgba([0.0, 1.0, 0.0, 1.0])),
                ..Default::default()
            }],
            spans: vec![EditorSpan {
                line: 0,
                start: 0,
                end: state.text.lines().next().unwrap().len() as u32,
                format: 0,
            }],
            affordances: EditorAffordances {
                menu: state.text.starts_with("Title").then(|| EditorMenu {
                    anchor: EditorMenuAnchor::Caret,
                    items: vec![EditorMenuItem {
                        tag: "choose".into(),
                        label: "Choose heading".into(),
                    }],
                    selected: 0,
                }),
                hits: vec![
                    EditorHit {
                        line: 1,
                        start: 7,
                        end: 11,
                        tag: 2,
                    },
                    EditorHit {
                        line: 1,
                        start: 12,
                        end: 15,
                        tag: 1,
                    },
                ],
                gutters: vec![EditorGutter {
                    line: 1,
                    plus: true,
                    handle: true,
                }],
                drop_boundaries: vec![1, 2],
                margins: vec![EditorMargin { line: 1, count: 1 }],
                margin_label: "Open comments".into(),
            },
        }
    }
    pub fn remember(previous: Vec<u8>, event: Vec<u8>) -> Vec<u8> {
        if event.is_empty() || event.starts_with(b"notice:") {
            previous
        } else {
            event
        }
    }
    pub fn notification(event: Vec<u8>) -> String {
        event
            .strip_prefix(b"notice:")
            .map(|text| String::from_utf8(text.to_vec()).unwrap())
            .unwrap_or_default()
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
                EditorTransactionEvent::Interaction { action, .. } => {
                    use wire::editor_presentation::EditorInteraction;
                    match action {
                        EditorInteraction::LinePress { tag: 2, .. } => {
                            Some(b"notice:link".to_vec())
                        }
                        EditorInteraction::Margin { .. } => Some(b"notice:comments".to_vec()),
                        _ => None,
                    }
                }
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
        .on_interaction(|request| {
            use wire::editor_presentation::EditorInteraction;
            let patch = match request.action {
                EditorInteraction::MenuPick { tag } if tag == "choose" => wire::EditorPatch {
                    start_byte: 0,
                    end_byte: 5,
                    replacement: "Chosen".into(),
                },
                EditorInteraction::LinePress { tag: 1, .. } => {
                    let start = request.state.text.find("[ ]").expect("checkbox") as u32;
                    wire::EditorPatch {
                        start_byte: start + 1,
                        end_byte: start + 2,
                        replacement: "x".into(),
                    }
                }
                _ => return EditorDecision::Noop,
            };
            EditorDecision::Apply {
                patches: vec![patch],
                cursor: request.state.cursor,
                history: EditorHistoryEffect::NewGroup,
            }
        })
    }
}

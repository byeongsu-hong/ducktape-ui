ui_lang::include_app!("src/ui/app.ice");

mod document;
mod editor;
mod frame_probe;
mod library;

fn main() -> iced::Result {
    MarkdownEditor::run()
}

#[cfg(test)]
mod tests {
    use super::{__MarkdownEditorMessage, MarkdownEditor};
    use crate::editor::RichEditorAction;
    use iced::widget::text_editor::{Action, Content, Cursor, Edit, Position};

    #[test]
    fn large_document_edits_stay_in_the_native_buffer() {
        let (mut app, _) = MarkdownEditor::__boot();
        let source = "A native editor line.\n".repeat(10_000);
        app.document = Content::with_text(&source);
        app.document.move_to(Cursor {
            position: Position {
                line: 9_999,
                column: 1,
            },
            selection: None,
        });
        let _ = app.__update(__MarkdownEditorMessage::EditDocument(
            RichEditorAction::Edit(Action::Edit(Edit::Insert('x'))),
        ));

        assert_eq!(app.document.text().len(), source.len() + 1);
        assert_eq!(app.document.line_count(), 10_001);
        assert_eq!(
            app.document.line(9_999).unwrap().text,
            "Ax native editor line."
        );
    }

    #[test]
    fn app_undo_and_redo_apply_grouped_typing() {
        let (mut app, _) = MarkdownEditor::__boot();
        app.document = crate::editor::reset_document("hello".into());
        app.document.move_to(Cursor {
            position: Position { line: 0, column: 5 },
            selection: None,
        });

        for character in ['!', '?'] {
            let _ = app.__update(__MarkdownEditorMessage::EditDocument(
                RichEditorAction::Edit(Action::Edit(Edit::Insert(character))),
            ));
        }
        assert_eq!(app.document.text(), "hello!?");

        let _ = app.__update(__MarkdownEditorMessage::Undo);
        assert_eq!(app.document.text(), "hello");

        let _ = app.__update(__MarkdownEditorMessage::Redo);
        assert_eq!(app.document.text(), "hello!?");
    }

    #[test]
    fn save_completion_marks_only_the_revision_written_to_disk() {
        let (mut app, _) = MarkdownEditor::__boot();
        app.document = crate::editor::reset_document("hello".into());

        let edit = |character| {
            __MarkdownEditorMessage::EditDocument(RichEditorAction::Edit(Action::Edit(
                Edit::Insert(character),
            )))
        };
        let saved = |revision| {
            __MarkdownEditorMessage::Saved(crate::library::Saved {
                path: "/tmp/notes.md".into(),
                saved_revision: revision,
                notes: Vec::new(),
            })
        };

        let _ = app.__update(edit('!'));
        assert!(app.history.dirty);
        let written = app.history.revision;
        let _ = app.__update(saved(written));
        assert!(!app.history.dirty);

        let _ = app.__update(edit('?'));
        assert!(app.history.dirty);
        let _ = app.__update(saved(written));
        assert!(
            app.history.dirty,
            "a completion for an older snapshot must not mark the newer edit as saved"
        );
    }

    fn request_output(
        task: iced::Task<__MarkdownEditorMessage>,
    ) -> Option<__MarkdownEditorMessage> {
        use iced::futures::StreamExt;
        let mut stream = iced_runtime::task::into_stream(task)?;
        iced::futures::executor::block_on(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return Some(message);
                }
            }
            None
        })
    }

    fn open_scratch_note() -> MarkdownEditor {
        let (mut app, _) = MarkdownEditor::__boot();
        let library =
            iced::futures::executor::block_on(crate::library::open_library(String::new()))
                .expect("scratch library opens");
        let _ = app.__update(__MarkdownEditorMessage::LibraryOpened(library));
        app
    }

    fn insert(app: &mut MarkdownEditor, character: char) {
        let _ = app.__update(__MarkdownEditorMessage::EditDocument(
            RichEditorAction::Edit(Action::Edit(Edit::Insert(character))),
        ));
    }

    #[test]
    fn pending_save_preserves_new_edits_and_rejects_repeat_submit() {
        let mut app = open_scratch_note();
        insert(&mut app, 'A');
        let submitted = app.document.text();
        let completion = request_output(app.__update(__MarkdownEditorMessage::SaveNow))
            .expect("save returns its actual completion");
        assert!(app.saving);
        assert!(
            request_output(app.__update(__MarkdownEditorMessage::SaveNow)).is_none(),
            "a second submit must not launch another write while saving"
        );
        insert(&mut app, 'B');
        let edited = app.document.text();
        assert_ne!(edited, submitted);
        let _ = app.__update(completion);
        assert!(!app.saving);
        assert_eq!(app.document.text(), edited);
        assert!(
            app.history.dirty,
            "pending edits were incorrectly marked saved"
        );
        assert_eq!(std::fs::read_to_string(&app.path).unwrap(), submitted);
        let next = request_output(app.__update(__MarkdownEditorMessage::SaveNow)).unwrap();
        let _ = app.__update(next);
        assert!(!app.history.dirty);
        assert_eq!(std::fs::read_to_string(&app.path).unwrap(), edited);
        std::fs::remove_dir_all(&app.home).unwrap();
    }

    #[test]
    fn failed_save_retains_the_draft_and_successful_retry_clears_error() {
        let mut app = open_scratch_note();
        let original = app.document.text();
        let original_path = app.path.clone();
        insert(&mut app, 'A');
        let edited = app.document.text();
        // A removed parent directory produces a real filesystem error, even
        // when the tests run as root; no permission-bit or mocked failure.
        std::fs::remove_dir_all(&app.home).unwrap();
        let failure = request_output(app.__update(__MarkdownEditorMessage::SaveNow)).unwrap();
        let _ = app.__update(failure);
        assert!(!app.saving);
        assert!(app.history.dirty);
        assert_eq!(app.document.text(), edited);
        assert!(
            !app.error.is_empty(),
            "failed save must explain why it did not persist"
        );
        std::fs::create_dir_all(&app.home).unwrap();
        std::fs::write(original_path, original).unwrap();
        let retry = request_output(app.__update(__MarkdownEditorMessage::SaveNow)).unwrap();
        assert!(app.error.is_empty(), "retry must clear the previous error");
        let _ = app.__update(retry);
        assert!(!app.saving);
        assert!(!app.history.dirty);
        assert!(app.error.is_empty());
        assert_eq!(std::fs::read_to_string(&app.path).unwrap(), edited);
        std::fs::remove_dir_all(&app.home).unwrap();
    }

    #[test]
    fn clicking_a_shell_action_clears_editor_selection() {
        let (mut app, _) = MarkdownEditor::__boot();
        app.loading = false;
        app.document = crate::editor::reset_document("hello".into());
        app.document.move_to(Cursor {
            position: Position { line: 0, column: 5 },
            selection: Some(Position { line: 0, column: 0 }),
        });

        let _ = app.__update(__MarkdownEditorMessage::ToggleTheme);

        assert_eq!(app.document.selection(), None);
    }
}

ui_lang::include_app!("src/ui/app.ice");

mod document;
mod editor;

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
        let _lock = crate::editor::test_history_lock();
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
}

ui_lang::include_app!("src/ui/app.ice");

mod editor;

fn main() -> iced::Result {
    MarkdownEditor::run()
}

#[cfg(test)]
mod tests {
    use super::{__MarkdownEditorMessage, MarkdownEditor};
    use iced::widget::text_editor::{Action, Content, Edit};

    #[test]
    fn large_document_edits_stay_in_the_native_buffer() {
        let (mut app, _) = MarkdownEditor::__boot();
        let source = "A native editor line.\n".repeat(10_000);
        app.document = Content::with_text(&source);
        let _ = app.__update(__MarkdownEditorMessage::__EditDocument(Action::Edit(
            Edit::Insert('x'),
        )));

        assert_eq!(app.document.text().len(), source.len() + 1);
        assert_eq!(app.document.line_count(), 10_001);
    }
}

ui_lang::include_app!("src/ui/app.ice");

mod editor;

fn main() -> iced::Result {
    MarkdownEditor::run()
}

#[cfg(test)]
mod tests {
    use super::{__MarkdownEditorMessage, EditorMode, MarkdownEditor};
    use iced::widget::text_editor::{Action, Content, Edit};

    #[test]
    fn large_document_edits_do_not_reparse_preview() {
        let (mut app, _) = MarkdownEditor::__boot();
        let source = "A native editor line.\n".repeat(10_000);
        app.document = Content::with_text(&source);
        let preview_before = format!("{:?}", app.rendered.items());

        let _ = app.__update(__MarkdownEditorMessage::__EditDocument(Action::Edit(
            Edit::Insert('x'),
        )));

        assert_eq!(app.document.text().len(), source.len() + 1);
        assert_eq!(format!("{:?}", app.rendered.items()), preview_before);

        let _ = app.__update(__MarkdownEditorMessage::ShowPreview);
        assert_eq!(app.mode, EditorMode::Preview);
        assert_ne!(format!("{:?}", app.rendered.items()), preview_before);
    }
}

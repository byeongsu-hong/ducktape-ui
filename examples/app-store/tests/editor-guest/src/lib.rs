ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    Box,
    "Editor fixture",
    "Native editor presentation and edits",
    []
);

mod fixture {
    pub fn place(mut document: ui_lang_guest::Editor) -> ui_lang_guest::Editor {
        document.move_to(ui_lang_guest::wire::EditorCursor {
            position: ui_lang_guest::wire::EditorPosition { line: 1, column: 6 },
            selection: None,
        });
        document
    }
}

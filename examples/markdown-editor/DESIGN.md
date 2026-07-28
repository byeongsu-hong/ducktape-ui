# Markdown editor example

This example is a native, single-document Markdown editor. Markdown formatting
is presented inline in the same writing surface instead of a separate preview,
custom rich-text widget, or web view.

## Project structure

```text
src/
├── main.rs                  Rust entry point and native-buffer check
├── document.rs              file dialogs, persistence, links, shortcuts
├── editor.rs                highlighting and bounded edit history
└── ui/
    ├── app.ice              app settings, imports, and root composition
    ├── theme.ice            semantic color tokens
    ├── recipes.ice          shared control recipes
    ├── state.ice            document state and derived cursor state
    ├── components/          editor chrome and writing surface
    ├── extern/              typed Rust boundaries
    ├── handlers/            file, edit, find, and close flows
    └── tests/               app behavior contract
```

The editor keeps one native `text_editor::Content` in application state and
passes it by Ice `bind`; ordinary keystrokes mutate that buffer directly and
do not copy the full document. A bounded native delta history observes edits
through `editor-action`, while cursor built-ins feed the incremental
`editor-highlighter`. Iced continues to own selection, clipboard, wrapping,
input methods, and visible-line highlighting.

Geist is the body face and the bundled Geist Mono is used for inline and fenced
code at the same 16 px editor size. Both come from the
[Geist project](https://github.com/vercel/geist-font) under the included SIL
Open Font License.

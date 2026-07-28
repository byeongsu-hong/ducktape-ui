# Markdown editor example

This example is a native, single-document Markdown editor. Markdown formatting
is presented inline in the same writing surface instead of a separate preview,
custom rich-text widget, or web view.

## Project structure

```text
src/
├── main.rs                  Rust entry point; includes ui/app.ice
├── editor.rs                incremental inline Markdown highlighter
└── ui/
    ├── app.ice              one app root, imports, and one view
    ├── theme.ice            semantic color tokens
    ├── state.ice            editor state and deterministic test preset
    ├── components/editor.ice
    ├── extern/editor.ice    typed native editor boundary
    └── tests/app.ice        app behavior contract
```

The editor keeps one native `text_editor::Content` in application state and
passes it by Ice `bind`; ordinary keystrokes mutate that buffer directly and
never copy or parse the full document. An Ice v2 `editor-highlighter` extern
styles headings, emphasis, code, links, quotes, lists, and fenced code blocks.
Its line-state snapshots let Iced restart highlighting at the changed line
while the native editor continues to own selection, clipboard, wrapping, and
input-method behavior.

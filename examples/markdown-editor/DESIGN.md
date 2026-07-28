# Markdown editor example

This example is a native, single-document Markdown editor. It deliberately
starts with Iced's text editor and a same-pane rendered preview instead of a
custom rich-text widget or web view.

## Project structure

```text
src/
├── main.rs                  Rust entry point; includes ui/app.ice
├── editor.rs                native key-binding adapter
└── ui/
    ├── app.ice              one app root, imports, and one view
    ├── theme.ice            semantic color tokens
    ├── state.ice            editor state and deterministic test preset
    ├── components/editor.ice
    ├── extern/editor.ice    typed native editor boundary
    ├── handlers/app.ice     preview state transitions
    └── tests/app.ice        app behavior contract
```

The editor keeps one native `text_editor::Content` in application state and
passes it by Ice `bind`; ordinary keystrokes never copy the full document into
an application message. Markdown is parsed only when entering preview. The
custom key binding intercepts Command/Ctrl+Shift+P and delegates every other
key to Iced's native binding implementation.

The next editor milestone is inline Markdown presentation while preserving
native selection, clipboard, undo, and input-method behavior.

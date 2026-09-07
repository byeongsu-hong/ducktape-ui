# Rich editor surface boundary

Implement inline, batching code before verification as requested. Ducktape stays read-only.

Actual consumers require the runtime RichTextEditor, not the stock multiline editor: live Enter/Shift+Enter classification, focused formatting chords, selection and IME. Page history currently uses thread-local stacks in ducktape; that cannot serve independent mounted module views. The host adapter must own its history per view. Pages additionally need semantic todo/link/menu/gutter/comment operations; the composer slice below does not claim page parity.

Use the existing named surface/value/route boundary and Guest-owned registry. Bind one native document per scoped mounted key using weak registry entries; the widget owns its lease. Guest arguments carry text, an explicit reset generation, placeholder and disabled state. Guest echoes preserve the native document/caret/history, while an explicit reset replaces it. Native keyboard and pointer actions mutate the host document, then publish a typed record containing text, selection/caret and a submit flag. Guest handlers retain draft/submission policy. No native Action or Content crosses wasm.

- [x] Implement the actual RichTextEditor adapter, per-view bounded history and focused shortcuts; forward native clipboard/IME/redraw/capture behavior.
- [x] Add a wasm composer fixture with separate instances, redraw/reset/disable/mount controls and visible semantic notices.
- [x] Drive native typing, Enter/Shift+Enter, selection, undo/redo, formatting, IME, reset, replacement and cleanup through the bundled guest. Verify assertion-level mutations after implementation.
- [x] Run relevant runtime/host tests, real bundle, workspace checks, Clippy and formatting together; fix failures and obtain independent review.
- [ ] Update phase/evidence docs with exact coverage and remaining page/terminal requirements. Commit, push, PR, green CI, merge and clean up.

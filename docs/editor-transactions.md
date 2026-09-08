# Tree editor transactions

An existing `editor-binding keys(args...) -> Payload` declaration on the Tree
backend calls `fn(args...) -> ui_lang_guest::EditorBinding<Payload>` when building
the view. `EditorBinding::new(claims, decide, on_event)` retains both Rust closures
in the guest. The native language backend keeps its existing
`fn(KeyPress, args...) -> Option<iced::widget::text_editor::Binding<Payload>>`.
The `.ice` route receives `Payload` on both targets.

`decide(EditorKeyRequest)` sees the full bounded document, active caret,
selection anchor, original key and admission time, and versioned transaction id.
It returns `DefaultEditorAction`, `Noop`, or `Apply` with atomic byte patches,
final cursor and history effect. Patch endpoints must be native-representable
extended-grapheme boundaries, including intact paired line endings. The host
validates the entire batch before editing retained native `Content`.

`on_event(EditorTransactionEvent) -> Option<Payload>` runs after the generated
update accepts the committed editor state. Both native edits and guest patches
report before/after state, input kind and admission time. The guest alone owns
undo history. Native commits use `Native`; patches choose `NewGroup`,
`ExtendPrevious`, `Undo` or `Redo`. No host undo stack or timing policy is added.

A resolved state binding identifies a logical document within one guest instance.
Two renderings of one state share an ordered lane; independent component states
remain independent even when their contents and revisions are equal. Reset is an
authoritative guest replacement epoch; observation revision includes caret
changes; text revision changes only with content. Responses must match the whole
pending identity. A same-document revision conflict reissues the same admitted
key against current state instead of silently replaying native behavior.

Only focused declared keys are claimed. Later typing, paste, caret changes and
IME lifecycle events wait in order. Composition preedit is never intercepted by
the decision callback. Clipboard bytes are captured at paste admission. A native
fallback replays the original event once through Iced. There is no OS rebubbling.
Same-editor scrolling waits behind pending input so a deferred pointer edit sees
its original viewport; rendering and unrelated widgets remain responsive.

The lane retains at most 128 inputs and 64 KiB of input payload. A patch batch has
at most 256 patches and 64 KiB of replacements/result. Claims are limited to 32.
Undecided requests fault after five seconds or four revision conflicts. Faults
never silently become native fallback. Explicit replacement/unmount cancels the
pending identity; late responses cannot install into another epoch or instance.

Host integrations merge and validate the guest tree before calling
`Inputs::editor_frame`. Its return value invalidates the rendered view after a
commit; `editor_wants_redraw` schedules queue progress. A hot-reload host must
reject replacement while `editor_transactions_pending` is true. Guest snapshots
also reject outstanding decision/commit acknowledgments. Rebuild host and guest
together when adopting this wire protocol.

A commit with unchanged text (including Noop and caret-only changes) still advances
observation order. History reducers must compare `before.text == after.text` and
preserve undo/redo groups in that case. The admission timestamp remains identical
from Request through Commit; grouping policy is entirely application-owned.

Oversized initial editor state fails explicitly instead of opening a prefix.
Oversized host edits roll back and emit Fault. Wire sanitization is fallible if
its aggregate text/tree budget would shorten any editable document, and the host
must retain its previous accepted tree and Content on that error. There is no
safe native-only bypass under the current full-state projection. Large documents
need persistent document mirrors, incremental patches and bounded chunked initial
assignment; Pages must keep its existing host surface until that followup lands.
Display-only text truncation diagnostics remain a separate host followup.

The current host widget is stock Iced TextEditor. Ordered IME lifecycle and commit
are tested here; RichTextEditor's custom macOS Korean release-boundary recovery
is not claimed by this implementation. Key releases and modifier transitions are
ordered with other editor events so future native integration does not lose them.

Large-document product completion is tracked in [#1014](https://github.com/byeongsu-hong/ducktape-ui/issues/1014); this transaction lane alone does not make Pages replacement ready.

Fault and cancellation notifications return to the widget that admitted the input, even when several widgets share a document. Cancellation carries the retired identity for mirror cleanup; its callback runs after an explicit document reset without applying the retired snapshot to the new Editor. Unmounted callbacks may be absent, but transport cancellation still releases their pending decision.

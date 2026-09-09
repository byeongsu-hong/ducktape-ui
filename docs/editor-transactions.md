# Tree editor transactions

An existing `editor-binding keys(args...) -> Payload` declaration on the Tree
backend calls `fn(args...) -> ui_lang_guest::EditorBinding<Payload>` when building
the view. `EditorBinding::new(claims, decide, on_event)` retains both Rust closures
in the guest. The native language backend keeps its existing
`fn(KeyPress, args...) -> Option<iced::widget::text_editor::Binding<Payload>>`.
The `.ice` route receives `Payload` on both targets.

`decide(EditorKeyRequest<'_>)` borrows the canonical document, active caret,
selection anchor, original key and admission time, and versioned transaction id.
It returns `DefaultEditorAction`, `Noop`, or `Apply` with atomic byte patches,
final cursor and history effect. Patch endpoints must be native-representable
extended-grapheme boundaries, including intact paired line endings. The host
validates the entire batch before editing retained native `Content`.

`on_event(EditorTransactionEvent<'_>) -> Option<Payload>` runs after the generated
update accepts the committed editor state. Both native edits and guest patches
borrow before/after state for the duration of the callback and report input kind
and admission time. Retain only the history data the application needs; neither
callback owns a second persistent document. The guest alone owns
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

The lane retains at most 128 inputs and one MiB of captured input payload. A
patch batch has at most 256 patches and one MiB of replacements/result. The
decoder also bounds aggregate replacement bytes across the whole frame to one
MiB, independently of the 64 KiB display budget. Claims are limited to 32.
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
observation order. History adapters should first compare `before.text_revision`
with `after.text_revision` and preserve undo/redo groups when equal. This verified
text-change marker avoids comparing a one-MiB document for caret-only commits;
string equality is an equivalent but more expensive policy. The admission timestamp remains identical
from Request through Commit; grouping policy is entirely application-owned.

Editor projections carry document identity, reset, text/observation revisions,
byte length and cursor, rather than copying text into every tree. Initial or
reset assignments transfer at most sixteen 64 KiB chunks into a one-MiB document.
No prefix becomes editable: only a complete, validated assignment is installed,
and its exact acknowledgment releases the guest snapshot barrier. Shared
projections reuse the canonical document and do not consume display text budget.
An unchanged frame sends metadata only. Transaction requests also carry metadata;
a stale guest mirror is repaired by the same bounded transfer before the same
admitted key is decided again. The existing ordered lane remains the only queue.

Hosts preflight references before replacing the accepted tree. Transfer failure,
invalid metadata or a rejected edit must retain the prior document. A reload
candidate finishes its own transfer handshake after no-init restore before it
can replace the old instance. Only projections whose identity, reset, revision,
text and cursor agree may reuse old native Content; old pending inputs never
move to the new session. Display truncation reports are independent of document
transfers.

The current host widget is stock Iced TextEditor. Ordered IME lifecycle and commit
are tested here; RichTextEditor's custom macOS Korean release-boundary recovery
is not claimed by this implementation. Key releases and modifier transitions are
ordered with other editor events so future native integration does not lose them.

Large-document product completion is tracked in [#1014](https://github.com/byeongsu-hong/ducktape-ui/issues/1014); this transaction lane alone does not make Pages replacement ready.

Fault and cancellation notifications return to the widget that admitted the input, even when several widgets share a document. Cancellation carries the retired identity for mirror cleanup; its callback runs after an explicit document reset without applying the retired snapshot to the new Editor. Unmounted callbacks may be absent, but transport cancellation still releases their pending decision.

Replacement retires the old native widget Tree as part of instance isolation.
A native focus operation transfers only the eligible focused identity into the
new Tree; no old widget state is reused. The actual native/Wasm one-MiB fixture
asserts restored text/caret before redraw, then performs Undo without refocusing.
Native commits from plain renderings of a shared document use its authored
history route; explicitly authored bindings retain their own routes.

Independent editor responses share a one-MiB replacement budget per frame. The
guest drains complete responses in order and remains busy while later responses
wait for the next frame; outstanding identities remain fenced until commit or
cancellation. An individually invalid response still reaches strict host
validation, rather than becoming a silent fallback or blocking queue progress.

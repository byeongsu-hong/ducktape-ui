# Tree Editor Transactions Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans after the root review and #1 typed EditorState API checkpoint. This document authorizes no implementation yet.

**Goal:** Let a guest decide structural editor keys before native edits, preserving input order, native composition, and one application-owned undo history.

**Architecture:** Extend #1's typed editor state with a per-editor host transaction lane and typed decision/commit events. The host owns native Content, layout, focus and composition; the guest owns document rules and history. Claims capture focused keys synchronously; a guest response either invokes the saved native binding, consumes the key, or commits validated patches and a cursor atomically.

**Tech Stack:** Rust, Ice Tree codegen, existing Iced TextEditor/Content, wire and guest runtime. No new dependencies.

**Spec:** The accepted parent/Claude contract recorded in this document; syntax and exact generated type names must align with the #1 checkpoint before code is written.

## Global constraints

- No change to the live Ducktape canary or its UI pin.
- `EditorState.reset: u64` is explicit guest replacement identity, not an edit counter. #1 host event acceptance by reset remains intact.
- No OS event rebubbling after an asynchronous decision. Default means the captured native editor action only.
- Ordinary editing and caret changes cannot pass a pending claimed key. Other editors, drawing and scrolling remain responsive.
- IME preedit is host-owned and never claimed. IME commit remains one native editing transaction.
- No second undo stack in the host; Pages/application history owns native and guest changes together.
- Queue limits and revision conflicts must produce explicit outcomes; no silent dropping, stale fallback or indefinite retry.

## Verified baseline and actual use

Design branch `agent/tree-editor-transactions`, UI base `046b1d48bf95f3750bcd508f99d55d357c3b2967`. Ducktape read-only source: `origin/dev = 824f11074`, fetched without changing its primary checkout. #1979 (`501dd6c0e`) changes only `app/src/pages/surface.rs`: Content cloning lost cursor and selection, causing wrong current-line painting and slash menu visibility. `copy` now restores the cursor; Pages mod/menu/history are unchanged by that PR.

`app/src/pages/mod.rs` handles Enter continuation/fence closure, Backspace marker deletion, Tab/ShiftTab two-space indentation and ordered-list renumbering. `menu.rs` gives slash/block palette navigation and acceptance priority over editing. `history.rs` owns before-text/cursor snapshots, a 750ms group deadline, 200 steps and a 16MiB budget; it currently groups without an edit-kind discriminator. `surface.rs` still paints the native RichTextEditor, queues PageEvent and forwards an edited intent. This plan supplies a reusable primitive for Claude's subsequent guest migration; it does not move Pages code itself.

Iced 0.14 `text_editor::Edit` has Insert, Paste, Enter, Indent, Unindent, Backspace and Delete, and no Undo/Redo operation. Content has no application history stack. Native `Content::move_to` does not clear an existing selection when the requested cursor has none; use the existing selection-clear-then-move technique. Cursor columns are UTF-8 byte offsets (the graphics editor's cosmic cursor index), not scalar/grapheme counts. Native motion/deletion retains its own grapheme behavior.

`rich_text_editor/keyboard.rs` already implements native platform bindings. RichTextEditor processes composition before menu/key handling, including macOS Hangul trailing-preedit and release-only punctuation deduplication. Preserve that precedence; do not intercept raw window keys before the editor sees IME state.

## Proposed protocol

Names below specify semantics, not an independent substitute for #1's public types.

```rust
struct Identity { reset: u64, sequence: u64, attempt: u32 }
struct Version { text_revision: u64, revision: u64 }
// text_revision increases only for changed text; #1 revision orders accepted observations.
struct DecisionRequest {
    identity: Identity,
    version: Version,
    key: ClaimedKey, // logical/physical key, modifiers and repeat; native platform recorded
    cursor: Cursor,
    input_time_ms: u64, // host monotonic input timestamp, epoch relative
}
enum Decision {
    DefaultEditorAction,
    Noop,
    Apply { patches: Vec<Patch>, cursor: Cursor, history: HistoryEffect },
}
struct Patch { start_byte: u32, end_byte: u32, replacement: String }
enum HistoryEffect { Record, Undo, Redo }
enum EditKind {
    Insert, Paste, ImeCommit, Enter, Backspace, Delete, Indent, Unindent,
    Cut, Cursor, GuestPatch, Undo, Redo,
}
```

#1 checkpoint update: reuse its observation counter and Inputs sequencer rather than introducing a separate state-revision counter. EditorState, Node and Event carry observation; guest rejects stale reset/observation, and same-reset host echoes adopt only observation >= local edit. Reload seeds the sequencer from the maximum restored observation. Transaction sequence identifies an admitted input (including pending/no-op inputs), while observation identifies accepted state; reserve sequence identities from the same monotonic allocator when feasible, without treating pending identities as accepted observations. text_revision still tracks actual buffer changes. Logical document identity is an explicit Node editor binding key emitted from Core resolved StateBinding: application state uses its resolved application scope and state name, component state uses its instantiated component scope and state name. Base and overlay references to the same cell emit the same key; distinct cells emit different keys even when reset, observation and text are identical. The key is namespaced inside the host guest-instance identity, never global. #4 owns adding this Node field if #1 does not need it. Do not infer identity from text, reset or sequence. Editors bound to the same state need one shared logical lane: a sibling editor must not edit past the pending claim. Each queued input retains its widget identity/native selection context; committed state synchronizes siblings through #1 observation. Test base+overlay bindings and sibling input while pending explicitly.

A response echoes Identity and Version. The host saves the native binding/action sequence locally; it is not serialized or reconstructed by guest platform cfg. One native binding Sequence is one logical transaction. Clipboard paste bytes are captured once when the input is admitted, so waiting does not change the paste. Copy remains native; cut is an ordered editing transaction.

A committed event contains reset, sequence, before/after versions, before/after cursor, EditKind, input timestamp, history effect and the accepted #1 EditorState after-image. The guest already has the previous accepted state; if it does not, it must receive a resync before another decision. Changed text increments text_revision once per batch. Cursor-only outcomes advance #1 observation; no-op still acknowledges the sequence. Exact duplicate responses acknowledge the existing result without editing twice.

`reset` supplies document replacement cancellation; key reuse/unmount additionally gets a host instance token so an old response cannot land in a new editor with the same key. All identities remain outstanding until acknowledged, explicitly epoch-cancelled, or explicitly faulted. Same-document changed state causes a resync then redecision of the same sequence with a new attempt/version, never DefaultEditorAction by accident. Cursor-only conflicts are included: matching text alone cannot justify deleting at an old caret.

## Confirmed target-specific editor-binding callback

Reuse the existing `.ice` `editor-binding` extern kind, arguments and route; add no alternate syntax or separate claims function. For Target::Tree (both native execution and Wasm), the Rust function is `fn(authored_args...) -> ui_lang_guest::EditorBinding<Payload>`. It constructs explicit bounded key claims and a retained `Fn(EditorKeyRequest) -> EditorDecision<Payload>`. The factory executes while constructing the guest view. Only copied claims cross to the host; requests invoke the retained guest callback. No Rust closure or generated Message is serialized. Target::Native keeps the existing `fn(KeyPress, authored_args...) -> Option<iced::text_editor::Binding<Payload>>` contract. Tree previously refused this feature, so there is no prior Tree callback compatibility path to preserve.

The binding's authored route receives a typed commit containing the optional decision payload, EditKind and before/after EditorState, only after host acknowledgment. Calling the route during decision evaluation would advance document/history ahead of a host rejection; do not do that. Payload remains retained guest-side under pending transaction identity until the matching commit or explicit cancellation. All native edits on a bound editor also produce a commit, with no decision payload, so the same reducer owns typing and guest patches. The exact generated callback/commit-route integration is the next Core checkpoint with root.

## Lane and claim semantics

At most one request is undecided per editor. Other editor inputs enter its FIFO behind it. Drain one logical transaction at a time; deliver its commit and allow the guest reducer/new declarations to settle before deciding the next claimed key. This is necessary for `/`, filter text, Enter arriving in one event burst: Enter must use the newly opened palette, not declarations from the previous frame. Claims cover exact modifiers and focus; bare Backspace must not swallow platform word deletion.

Intercept claimed keys at the native widget binding seam after composition handling, synchronously capturing them. Preserve the native action for DefaultEditorAction. Buffer-affecting native outputs, caret clicks/drags and selection operations enter the same lane. Where event interpretation depends on cursor/layout, replay the queued editor input against the current Content/layout when its turn arrives instead of saving a stale computed absolute cursor. Do not replay clipboard reads or IME commits twice. Pointer selection retains native drag state; tests must cover press/move/release spanning a pending key.

Preedit can continue painting while the lane waits; committed text is queued once. Structural claims do not run while native composition is active. A reset cancels the old document's lane and composition explicitly, with no edits leaking into the replacement document.

## Atomic patches and unified history

Patch coordinates refer to the same pre-transaction UTF-8 document. Require ascending, nonoverlapping ranges, valid UTF-8 boundaries, bounded total replacement bytes, and a final cursor/selection valid in the complete resulting text. Build and validate the result before mutating Content. Apply patches from the end using native selection + Paste/Backspace, then set the final cursor (explicitly clear an unwanted selection). Keep Content and widget state; ordinary patches must not replace Content wholesale. Hold the editor lock through the application and emit one commit after completion; no intermediate tree or history event is exposed.

Every native fallback and guest batch generates exactly one history-facing commit. Claude's guest history reducer records the previous guest state and before cursor before accepting the after-state; it can retain the present 750ms snapshot algorithm. Arrival timestamps prevent asynchronous response latency from splitting a typing group. EditKind lets it later choose meaningful boundaries without inferring intent from text diff. Undo/Redo are guest decisions returning patches/cursor tagged Undo/Redo, so the reducer moves its own stacks instead of recording a new user edit. Guest transactions must not also call a second `history.record` path. No host snapshot history, native Ctrl-Z stack, or forced history policy is added by this feature.

For a native fallback involving selection followed by deletion, expose one before/after pair. For renumbering multiple lines, expose one batch. For IME composition, record only the commit, never transient preedit.

## Explicit bounded fault behavior

Initial concrete limits: 128 queued logical inputs, 1MiB retained input payload, 256 patches per response and existing wire document/frame byte limits; always apply any tighter existing wire bound; limit changes require focused boundary tests. Four conflicting attempts for one sequence fault the lane. A monotonic 5s undecided deadline produces `DecisionTimeout`; it does not invoke default editing.

On overflow, timeout, malformed response or retry exhaustion, keep the last committed Content/cursor and freeze further buffer changes for this editor. Emit one typed fault containing reset, sequence range of accepted-but-uncommitted inputs, reason and the current state/version; report the triggering unadmitted event's kind/byte count as rejected. Do not silently clear or replay the FIFO. Preserve the bounded accepted FIFO until explicit recovery or epoch cancellation. A guest can acknowledge cancellation and replace state with a new reset, or explicitly resume a timeout with a valid decision for the still-current pending identity. Overflow/malformed protocol faults require new-reset recovery. This deliberately exposes a failed edit session rather than pretending rejected typing succeeded. Host presentation must surface the fault through the existing error/report channel; it must not leave a silently inert editor.

## Owned files and integration boundary

Codex #4, after Kuhn #1 merges/checkpoints:

- `crates/ui-lang-wire/src/editor.rs` (new if #1 has not created it): transaction payloads, limits and validation; re-export from `src/lib.rs`.
- `crates/ui-lang-runtime/src/view_tree/editor_transactions.rs` (new): FIFO, identities, validation/commit/resync/fault reducer.
- `crates/ui-lang-runtime/src/view_tree/editor.rs`: native binding interception and queued event replay; retain native Content/IME state.
- `crates/ui-lang-runtime/src/view_tree.rs`: store lane beside #1 editor field; route decisions and committed outcomes.
- `crates/ui-lang-runtime/src/rich_text_editor/keyboard.rs`: extract/reuse only the native binding helper actually shared; preserve current RichTextEditor behavior.
- `crates/ui-lang-guest/src/lib.rs` and its existing editor support module: typed decision/commit dispatch, pending identity and settled acknowledgments.
- `crates/ui-lang-core/src/codegen/view/editor.rs`, `view/tree.rs`, the existing editor option checker and `codegen/tests/tree.rs`: authored claims/routes using the #1 chosen syntax. Reuse the existing checked editor-binding representation for the public decision route where feasible. Settle the exact generated callback type against Kuhn’s checkpoint through root; do not add an alternate callback DSL. Emit the resolved logical binding key here, including component-instance scope.
- `examples/app-store/apps/editor-transactions/` (new fixture following #1's fixture layout) plus host integration test: native and real Wasm input/painting proof.
- `SPEC.md`, `COVERAGE.md`, `examples/app-store/README.md`, `PARITY.md`: describe exact supported contract and remaining RichTextEditor presentation gaps.

Claude owns Ducktape guest document reducer, Pages history/menu/Markdown migration and its app integration tests. Codex does not edit Ducktape in this scope. Full rich Markdown painting, menu UI and margin controls are separate from this transaction primitive; supporting key decisions alone must not claim full Pages parity.

## Implementation and test checkpoints

### 1. Freeze #1 integration and wire state machine

- [ ] Confirm #1 exact EditorState/Cursor/event/route names with root; map the semantic types above onto them, without overloading reset.
- [ ] Add identity tests: same cell rendered in base+overlay shares one lane; two distinct empty cells at reset=0/observation=0 do not; repeated component instances use distinct scopes. Add unit tests with a held request: insert/caret/paste enqueue, matching response commits once, duplicate ignored, same-document conflict resyncs/redecides same sequence, epoch replacement cancels, keyed remount rejects old response.
- [ ] Add boundary tests at 128/129 inputs, 1MiB payload, patch count, invalid UTF-8 ranges, selection boundaries and deadline/retry exhaustion. Assert last committed text and explicit fault, not merely an error log.
- [ ] Implement wire validation and the lane. Tests should exercise reducer inputs, not a mirrored implementation.

### 2. Native binding/IME and atomic application

- [ ] Drive actual widget events through pending Tab → typing → click → paste; release the decision and assert text, cursor, sequence and one history record per transaction.
- [ ] Preserve platform default modifier bindings and native selection/clipboard semantics. Test DefaultEditorAction, Noop, patch, and a multi-action native binding separately.
- [ ] Apply reverse-ordered native patches to one retained Content; assert focus/widget identity and selection survive, and malformed last patch leaves the entire document unchanged.
- [ ] Exercise native preedit → structural key → commit, Hangul trailing-empty-preedit, punctuation press/release dedup, and reset during composition. Preedit must produce no guest key claim/history entry.

### 3. Real native/Wasm user behavior and history

- [ ] Fixture contains minimal representative guest reducers with one history stack, no async/network dependencies; it does not duplicate Pages Markdown implementation. Framework tests cover representative list Enter, atomic renumber-like patches, menu-priority Noop and native fallback. Claude runs the full actual Pages cases listed below downstream. Drive actual keyboard/pointer input on both native and bundled Wasm; use text/cursor assertions plus pixel checks for caret/selection/focus.
- [ ] Cases: nonempty bullet Enter; checked task Enter resets checkbox; empty item Enter removes marker; Backspace at marker start removes marker without merging; Backspace elsewhere uses native behavior; Tab two-space indent; ShiftTab outdent and no-op boundary; title/first-body Tab consumed; excessive-depth Tab rejected; Enter opens/closes fence only when applicable; ordered list renumbers after Enter/Backspace/Paste; multi-digit renumber preserves caret; slash+filter+Enter burst accepts palette; Tab accepts palette before indentation; Unicode selection replacement/caret positioning; unified Undo/Redo across typing, native fallback, structural multi-patch and IME commit. These are concrete baseline cases, to be cross-checked against the parent's accepted user list before claiming complete coverage.
- [ ] Red evidence: temporarily allow a native insert to bypass the pending lane (ordering assertion fails); ignore the revision check (stale caret assertion fails); skip final cursor application (selection/caret assertion fails); record guest batch twice (undo assertion fails). Restore each mutation and rerun the exact test.
- [ ] Run affected Core/wire/guest/runtime tests, workspace check, relevant strict clippy/format, actual native + Wasm fixtures and bundle. No shared build target before parent releases the lease.
- [ ] Root reviews the full implementation and authored evidence before push/PR/merge decision.

## Review checkpoint

This document is a design artifact only. No runtime/codegen/wire changes or builds have been made. Root accepted the architecture and initial fault/recovery caps. The next dependency is Kuhn #1’s concrete API checkpoint; no additional human approval is required. Bounded queues cannot promise unlimited no-loss typing during a stuck guest, hence the explicit observable failure contract.

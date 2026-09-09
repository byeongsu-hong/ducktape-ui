# Guest-authored editor presentation

Tree integration uses the #1026 document lane and wire epoch 4.

The guest owns syntax and document actions. The host owns text shaping, caret
geometry and native input. Tree editors reuse `RichTextEditor`; they do not run
the app's Markdown parser or construct another document/history model.

## Author boundary

Reuse `editor-highlighter` and `highlighter=`. Its Tree specialization receives
`EditorStateView` and declared arguments and returns `EditorPresentation`.
The compiler supplies the editor's logical document identity. Native-target
highlighter wrappers retain their native typed contract.

Presentation contains sparse logical lines and UTF-8 byte ranges, with formats
referencing a shared table. Formats use the existing wire color, named font,
line height, border and edge types. They cover native `Format`: span and full
line backgrounds, font/size, line padding, horizontal rule, strike and span
padding. Hidden source syntax retains the native 0.01-pixel metric convention;
the actual source bytes and caret offsets remain unchanged.

The envelope carries the exact logical document, reset, text revision and
observation revision used to compute it. The host accepts it only for that
canonical document state. It checks byte boundaries against its resident text
before formatting. Invalid or stale formatting cannot hide unrelated text:
the editor paints its base format until matching presentation arrives.

Sparse lines feed a native data-backed `Highlighter`. No new layout engine,
viewport protocol, callback style or document-text echo is introduced. Numeric
and collection limits are enforced during wire decoding and validation;
presentation bytes have their own aggregate display allowance, independent of
the 1 MiB document transport and 8 MiB snapshot allowances.

## Interactions

Read-only documents retain declared link and comment notifications while refusing
editing decisions. Accepted `Commit.origin` exposes the original request input
across regenerated factories and revision retries; native commits have no origin.

Reuse native `EditorMenu` / `MenuAnchor::Caret`, gutter buttons/drop boundaries,
margin marks and line press geometry. Menu and gutter metadata are declarative
presentation data. `EditorBinding::on_interaction` optionally decides an
`EditorInteractionRequest` against borrowed canonical state. `Apply` uses the
same atomic patch and Commit/history lane as a claimed key. `Noop` sends a
separate `on_event(Interaction)` notification without a history Commit.
`DefaultEditorAction` is invalid for interactions; there is no synthetic key or
authoritative reset. Existing key callbacks remain `EditorKeyRequest`.

Per editor the limits are 256 formats, 32,768 spans, 32,768 aggregate gutter,
drop-boundary, margin and hit records, and 64 menu items. Tags and labels are
bounded to 1,024 UTF-8 bytes each; menu strings also share the normal string
limit. Decoder allocations share the frame-wide allocation limit. The guest
factory output is validated before publication; over-budget or malformed
metadata faults instead of silently removing interactive regions. Native
geometry values are sanitized, but tags and hit ranges are never truncated.

The host sends the interaction plus its instance/document/reset/revision fence.
The guest validates the fence before exposing a borrowed current
`EditorStateView` to the callback. The Pages reducer then decides todo/link,
menu, gutter or comment actions using its canonical document. No document copy
is carried in the interaction. Removed/replaced/stale controls cannot invoke a
callback against the replacement editor.

## Delivery and evidence

1. Add the native sparse data highlighter and tests for actual ranges, UTF-8
   boundaries, missing lines and presentation replacement.
2. Integrate wire metadata, the existing highlighter
   factory lowering and `RichTextEditor` in the document transaction lane.
3. Connect caret menu, gutter/drop, margin and line interactions through the
   existing binding route. Preserve disabled and IME behavior.
4. Run a real Pages-style native and Wasm fixture: hidden markers outside the
   caret line, visible source on the caret line, line background, caret menu,
   routed gutter action and stale interaction rejection. Mutation evidence
   must fail the intended rendered/routed assertion.

Do not remove Ducktape's `page_document` Surface until its guest consumer also
preserves save/history, todo/link, comments, menu and gutter behavior. The
native adapter alone is an intermediate implementation, not a completed port.

### Initial adapter evidence

With the isolated `tree-reload-focus/target` cache and `-j4`:

- `cargo test -p ui-lang-runtime rich_text_editor::presentation::tests --lib`:
  three pass. Returning no spans failed the two expected-range assertions;
  bypassing the UTF-8 guard failed `malformed_ranges_never_hide_unrelated_source`
  with one accepted range instead of zero. Exact source restored, three pass.
- `cargo test -p ui-lang-wire editor_presentation::tests --lib`: four pass.
  Before validation and bounded decoding, all four failed their intended
  assertions (invalid ranges/order, display limits and oversized collections).
- `cargo test -p ui-lang-runtime editor_presentation::tests --lib`: copied
  marker/line formatting passes. The empty conversion failed the native line
  count assertion before implementation.

The actual native/Wasm fixture also verifies line paint, a real caret-menu
interaction, atomic edit and Undo across reload without refocusing. Suppressing
the routed interaction fails its expected text assertion; restoration passes
both backends. This is runtime support, not a claim that the application Pages
port is complete.

Native host rendering requires `tiny-skia` or `wgpu`, as the native rich editor
uses a graphics paragraph. Null-renderer guest builds keep the shared native
Edit/MoveTo action type without enabling a graphics backend. They construct
wire data and never render presentation.

`ui_lang_runtime::editor_format::Format` is the graphics-independent native format
type; `rich_text_editor::Format` reexports it. Pure highlighters can share formatting
with guests without enabling a rendering backend.

### Integrated route evidence

The actual native/Wasm fixture also clicks a read-only link after a multibyte
prefix and its comment margin. Both notifications preserve document, revision,
caret and prior Undo history. Restoring the old editable-only affordance gate
fails `read-only link click must reach its guest notification`; the corrected
route passes both backends. Focus is seated before replacement, with no focus
operation after reload.

The focused transaction test checks that a same-document revision retry retains
the original interaction in accepted `Commit.origin`, native edits use `None`,
and a read-only interaction cannot Apply patches.

Setting accepted `Commit.origin` to `None` failed the retry-origin assertion;
exact source restoration passed. Final focused checks: wire presentation 7 pass,
retry/read-only transaction 1 pass, actual native/Wasm route test 1 pass with both
backends exercised and no skipped backend.

# Guest-authored editor presentation

Implementation plan; Tree integration is pending the #1026 document lane.

The guest owns syntax and document actions. The host owns text shaping, caret
geometry and native input. Tree editors reuse `RichTextEditor`; they do not run
the app's Markdown parser or construct another document/history model.

## Author boundary

Reuse `editor-highlighter` and `highlighter=`. Its Tree specialization receives
a borrowed guest editor and declared arguments and returns presentation data.
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

Reuse native `EditorMenu` / `MenuAnchor::Caret`, gutter buttons/drop boundaries,
margin marks and line press geometry. Menu and gutter metadata are declarative
presentation data. Routes use the existing `EditorBinding::on_event` callback
through a distinct interaction variant, never `Commit` or a history effect.

The host sends the interaction plus its instance/document/reset/revision fence.
The guest validates the fence before exposing a borrowed current
`EditorStateView` to the callback. The Pages reducer then decides todo/link,
menu, gutter or comment actions using its canonical document. No document copy
is carried in the interaction. Removed/replaced/stale controls cannot invoke a
callback against the replacement editor.

## Delivery and evidence

1. Add the native sparse data highlighter and tests for actual ranges, UTF-8
   boundaries, missing lines and presentation replacement.
2. After #1026 settles, integrate wire metadata, the existing highlighter
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

These establish the adapters only. The unattached Tree conversion still has
unused-function warnings until the document-lane integration; no native/Wasm
end-to-end or full Pages support claim is made by this checkpoint.

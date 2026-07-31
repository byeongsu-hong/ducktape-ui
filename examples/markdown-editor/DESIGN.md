# Markdown editor example

This example is a native, single-document Markdown editor. Markdown formatting
is presented inline by a purpose-built rich-text editor widget instead of a
separate preview or web view.

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

The editor keeps one native `text_editor::Content` in application state. The
runtime `RichTextEditor` shapes each logical line into an independently cached
rich paragraph. A line's text and composed rich formats form its cache
signature; unchanged paragraphs survive source edits and IME updates while
line offsets are recomputed from their cached heights. Painting, hit testing,
selection, caret movement, scrolling, and IME placement all use those same
paragraphs, and only visible text and inline decorations are painted. The
example's bounded history supplies a document ID and text revision through
`ContentVersion`, so caret and selection layouts reuse the cached source
without materializing the complete native buffer.

The editor history also passes an `EditorChange` to
`RichTextEditor::change_hint`. The hint binds one logical-line replacement span
to exact `from` and `to` content versions: an edit within a line is `1 -> 1`,
splitting a line is `1 -> 2`, and joining two lines is `2 -> 1`. The fast path
requires the versions to match the widget's cached and current layout, the
document identity to stay equal, and the line-count equation and bounds to be
valid. A skipped frame therefore rejects the latest single-edit transition and
uses exact diffing instead of trusting a stale prefix. Ordinary edits,
selection replacement, IME commit, undo, and redo all use the same history
delta; multi-replacement batches and full snapshots deliberately omit a hint.

During composition, the widget splices the preedit into a lightweight
display-line view without constructing or shaping a second native `Content`.
Highlighting resumes at the earliest affected line and only lines whose text or
rich format actually changed are reshaped. The composition paragraph is reused
when the identical text is committed, so wrapping does not change at the
boundary. Iced's fallback preedit overlay is disabled, leaving one copy of each
glyph in the document font and baseline. Ordinary edits still mutate the owned
native buffer through the update function, and a bounded delta history records
them. The implementation uses Iced's public `Content`,
keyboard/clipboard/input-method abstractions, advanced text renderer, and
widget API; it does not patch or vendor Iced.

IBM Plex Sans KR is the body face and Monoplex KR is used for inline and fenced
code. Monoplex combines IBM Plex Mono Latin glyphs with IBM Plex Sans KR Hangul,
so prose and code have coordinated Latin forms, both cover Hangul, and code
keeps a 2:1 fixed width. Headings use per-span metrics and each fenced code
block uses one continuous, coalesced visual-line background in the same editor
layout. The bundled
files come from the pinned [IBM Plex](https://github.com/IBM/plex/tree/2f9ba1b25957d958db71a849e85d72e3ecfb845a/packages/plex-sans-kr)
and [Monoplex KR](https://github.com/y-kim/monoplex/tree/ccd78918fdaf00f1ae52282b0446d66ec0c06fea)
sources under their included SIL Open Font Licenses.

## Baseline interaction requirements

The example is not considered a usable editor unless all of these contracts
hold:

| Area | Required behavior | Regression evidence |
| --- | --- | --- |
| Unicode and IME | Every Hangul assembly stage appears in its own event cycle, uses the document font and baseline, participates in live wrapping, replaces a selection, and commits without moving the visual line | `hangul_ime_stages_relayout_before_the_next_key` drives `ㅇ → 으 → 응 → empty preedit → commit` as distinct IME events; `bundled_body_font_keeps_korean_ime_stages_on_one_baseline`, `preedit_uses_the_same_wrapped_layout_as_committed_text`, and `line_paragraphs_preserve_whole_document_caret_geometry` verify the font and geometry |
| IME boundaries | A printable ASCII key that ends built-in macOS Korean composition is inserted on the first stroke, while an IME commit that already includes the key is never duplicated | `macos_ime_boundary_survives_the_trailing_empty_preedit` and `ime_close_preserves_release_only_punctuation` follow empty-preedit, IME-close, characterless-event, and release-only comma/period streams; `macos_ime_boundary_deduplicates_committed_keys_and_recovers_ascii` covers digits, shifted symbols, Space, shortcuts, and a new composition |
| Incremental layout | Caret and selection events must not snapshot unchanged text; a composition stage or ordinary single-line edit must not shape every paragraph in a long document | Four explicit `performance_contract_100k_*` tests separately budget 1,000 caret layouts plus one-character insertion, 1,000 real pointer drag events, `ㅇ → 으 → 응`, and viewport resize while recording deterministic source, parsed-line, styled-text, line-slot, mapping, highlighting, and shaping costs; accepted hints perform zero mapping-discovery comparisons while signature comparisons remain visible; `stale_batched_and_cross_document_hints_fall_back_before_reusing_a_prefix` covers exact transition identity; `change_hint_maps_replacements_insertions_undo_and_redo_without_line_diffing` covers shifted suffix reuse; `invalid_change_hints_fall_back_to_exact_diffing` covers structural validation; `change_hint_restarts_stateful_highlighting_at_the_changed_line` covers downstream syntax-state propagation; `lightweight_composition_parser_matches_iced_line_boundaries` covers every supported line ending |
| Inline whitespace | Parser-uncovered spaces and tabs retain body metrics; only actual Markdown syntax markers may collapse | `keeps_unparsed_whitespace_at_body_metrics` covers Korean and Latin trailing spaces, a trailing tab, and a real paired strong delimiter |
| Text editing | Enter, Backspace, Delete, Tab, Shift+Tab, word deletion, and macOS line-boundary deletion edit the native buffer; Markdown lists and fences keep their atomic behaviors | runtime key-binding tests and the list, indentation, and fence tests in `editor.rs` |
| Navigation and selection | Mouse hit testing, padding clicks, in/out-of-bounds drags, post-drag clicks, double/triple selection, arrows, word/line/page movement, and Select All use the same wrapped rich geometry shown on screen | shared paragraph geometry tests plus `clicks_in_editor_padding_focus_and_clear_selection` and `a_selection_drag_does_not_turn_the_next_click_into_a_double_click` in `rich_text_editor.rs` |
| Clipboard | Command/Ctrl+C, X, and V copy, cut, and paste through Iced's native clipboard without stealing application shortcuts | `application_command_shortcuts_are_not_inserted_as_text` and the runtime binding adapter |
| Undo and redo | Command/Ctrl+Z and Command/Ctrl+Shift+Z (plus Ctrl+Y) reach the application under non-Latin input sources; adjacent typing is one bounded event; a new edit clears redo | `undo_and_redo_shortcuts_survive_a_non_latin_input_source`, `app_undo_and_redo_apply_grouped_typing`, and `undo_redo_tracks_deltas_and_saved_state` |
| Document lifecycle | New, Open, Save, Save As, dirty-close confirmation, UTF-8 errors, and cancelled dialogs preserve the current document until the user makes an explicit choice | typed handlers in `ui/handlers/app.ice` and delta-based saved-revision tracking |
| Find, formatting, and links | Find next/previous, bold, italic, inline code, link insertion, and safe HTTP(S) link opening work from toolbar or platform shortcuts | formatting/history tests, `resolves_only_the_link_under_the_cursor`, and Ice handler contracts |
| Rich presentation | Every fenced code block is one continuous surface with real 16 px inner layout padding; inline-code backgrounds reserve horizontal space and cannot bleed into neighboring lines | `consecutive_line_highlights_share_one_surface`, `line_padding_changes_wrapping_caret_and_hit_geometry`, `inline_highlight_padding_cannot_bleed_into_adjacent_lines`, and `hidden_inline_code_delimiters_reserve_the_highlight_margin` |
| Viewport and scale | The editor fills the window, wraps within the 800 px page, scrolls and reveals the rich caret, and edits a 10,000-line native buffer through the app update path | `inline_editor_fills_the_window` and `large_document_edits_stay_in_the_native_buffer` |

## Behavioral references

Concrete editor behavior is translated from these pinned JavaScript sources:

| Contract | Reference | Native implementation |
| --- | --- | --- |
| 16 px body, 1.6 line height, 800 px page, 50 px gutter | [MarkText/Muya block CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L1-L24) | Ice editor layout |
| H1–H6 scales `1.875, 1.5, 1.375, 1.25, 1.125, 1` and 1.4 line height | [MarkText/Muya heading CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L37-L82) | `RichTextEditor` span metrics and shared caret geometry |
| 90% monospace fenced code, 1.6 line height, surface, 1 px border, 3 px radius | [MarkText/Muya code-block CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L196-L235) | One coalesced native visual-line decoration; hidden fence rows provide vertical inset |
| Body-size monospace inline code with distinct spacing and a shared code surface | [MarkText/Muya inline padding and shared code-block background](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/inlineSyntax.css#L59-L69), [MarkText One Dark code surfaces](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/renderer/src/assets/themes/one-dark.theme.css#L15-L35), and [Typora GitHub inline code](https://github.com/typora/typora-default-themes/blob/cf4f2cb7e81a73050456367cfdfdb80b5a14a7b2/themes/github.css#L265-L275) | Transparent source delimiters reserve `0.4em` on both sides, the 1.6 body line height supplies vertical space, and line clipping prevents highlight paint from entering adjacent rows |
| Dark appearance follows the OS at startup and while running, with a direct toolbar override | [MarkText system-theme startup and update handling](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/main/app/index.ts#L274-L355) and [One Dark palette](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/renderer/src/assets/themes/one-dark.theme.css#L15-L43) | Ice system-theme task/subscription selects a typed light or dark palette; the native syntax highlighter changes theme with it |
| Enter on ```` ```lang ```` creates an empty fenced block and puts the caret in its code body | [MarkText/Muya paragraph conversion](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L42-L64) and [Enter handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L256-L315) | One native delta inserts the blank body and matching closing fence; one undo removes all of it |
| Fenced code uses language-aware token colors and emphasis | [MarkText/Muya Prism light theme](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/prismjs/light.theme.css#L28-L99) | Existing native `iced_highlighter` parses the info-string language incrementally |
| Syntax markers disappear unless the cursor intersects their token | [MarkText/Muya renderer](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/inlineRenderer/renderer/index.ts#L111-L136) and [marker CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/inlineSyntax.css#L1-L30) | Caret-aware highlighter collapses parser-confirmed delimiter ranges; transparent inline-code delimiters retain only the surface margin, while uncovered whitespace remains body text |
| Strong, emphasis, and deletion render as semantic inline elements with paired caret-local markers | [MarkText/Muya inline renderer](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/inlineRenderer/renderer/delEmStrongFactory.ts#L12-L65) | Composable native weight, italic, and strikethrough spans; hidden markers retain the containing font face so the first shaped word does not inherit the marker face |
| Applying bold inside an existing strong run removes the strong markers | [MarkText/Muya format toggle regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/base/__tests__/formatToggle.spec.ts#L88-L102) | Bold, italic, and inline-code commands toggle the containing run instead of nesting duplicate markers |
| Platform deletion keys: Ctrl/Option+Backspace deletes a group; macOS Cmd+Backspace deletes to the line boundary | [CodeMirror 6 standard keymap](https://github.com/codemirror/commands/blob/5b9bac974f2c4af3e20b045adef949667872ecad/src/commands.ts#L1019-L1060) | Shared native `text_editor` bindings |
| Enter continues bullet, ordered, and unchecked task markers; an empty item exits one list level; following ordered siblings are renumbered | [CodeMirror Markdown list command](https://github.com/codemirror/lang-markdown/blob/8f73fd5013a1aaa3d3319f9165fff3100d159124/src/commands.ts#L96-L185) | Caret-local Markdown edits recorded as one bounded undo event |
| Backspace at the start of list content removes the inferred list item | [CodeMirror Markdown markup deletion](https://github.com/codemirror/lang-markdown/blob/8f73fd5013a1aaa3d3319f9165fff3100d159124/src/commands.ts#L229-L275) and [MarkText list-to-paragraph behavior](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L636-L675) | The source marker is removed while nested indentation is retained |
| Tab inserts four-space code indentation; on a list it nests under a previous sibling, and Shift+Tab lifts one level | [MarkText/Muya Tab handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L922-L960) and [list-indent regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/__tests__/insertTabAndIndent.spec.ts#L180-L214) | The Markdown action adapter inserts four spaces; list nesting keeps the referenced two-space indent |
| Adjacent edits form bounded undo events | [CodeMirror 6 history grouping](https://github.com/codemirror/commands/blob/5b9bac974f2c4af3e20b045adef949667872ecad/src/history.ts#L23-L82) | In-place delta history with explicit event and byte limits |
| External URLs open through the OS shell | [MarkText desktop shell handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/main/ipc/shell.ts#L5-L17) | `open` crate default-browser dispatch |
| Only an actual rendered link is a mouse target | [MarkText/Muya link wrapper hit-testing](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/editor/linkMouseEvents.ts#L39-L57) and [hover regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/editor/__tests__/linkMouseEvents.spec.ts#L50-L74) | Native glyph-bounds hit testing returns a pointer only inside a safe Markdown link range; blank-space, selection-drag, and mismatched press/release events cannot reach the outer link handler |
| IME preedit remains a distinct composition and must not fight the committed-text caret | [CodeMirror composition state](https://github.com/codemirror/view/blob/fbff59ba004d80d8c914f64c42586387b08706ac/src/input.ts#L816-L846), [composition changed-range reuse](https://github.com/codemirror/view/blob/fbff59ba004d80d8c914f64c42586387b08706ac/src/docview.ts#L71-L135), and [composition range tracking](https://github.com/codemirror/view/blob/fbff59ba004d80d8c914f64c42586387b08706ac/src/docview.ts#L606-L615) | A transient display-line view suppresses the committed caret, reuses unchanged paragraphs, and reports the visual composition caret to the OS without enabling a second runtime overlay |
| Every preedit stage follows the normal replacement and layout path | [VS Code replaces the previous composition text through its regular type event](https://github.com/microsoft/vscode/blob/be52ea55d41df45764ea0bbe1f739f072a75e301/src/vs/editor/browser/controller/editContext/textArea/textAreaEditContextInput.ts#L81-L99), [Zed replaces marked ranges through `handle_input`](https://github.com/zed-industries/zed/blob/5f180e06dc4e776d016ee7a52a39f0b00731af1a/crates/editor/src/input.rs#L2892-L3029), and [egui replaces the preedit range and lays out immediately](https://github.com/emilk/egui/blob/c69834e65a0681d4fa40c30545b006ce39527034/crates/egui/src/widgets/text_edit/builder.rs#L1281-L1345) | Each IME update replaces the same transient source range and is shaped in the event cycle; commit uses the same text geometry instead of a special paint-only path |
| Text and syntax caches invalidate at line granularity | [Iced's highlighter resumes from the changed line](https://github.com/iced-rs/iced/blob/3997291f318a8bc06fa522f5579836fb3feb94df/core/src/text/highlighter.rs#L6-L42) and [cosmic-text resets shape/layout only when a buffer line changes](https://github.com/pop-os/cosmic-text/blob/c82ee1c5b5b8032e91eaff1cb34294b538727a7d/src/buffer_line.rs#L69-L89) | `DocumentLayout` retains paragraph objects for equal styled-line signatures, resumes the stateful highlighter at the earliest changed line, and only recomputes document top offsets |
| macOS Korean IME boundaries preserve exactly one printable key | [AppKit routes `keyDown` through `interpretKeyEvents`](https://developer.apple.com/documentation/appkit/nsresponder/interpretkeyevents%28_%3A%29), [winit 0.30.13 queues IME callbacks before deciding whether to emit keyboard input](https://github.com/rust-windowing/winit/blob/e9809ef54b18499bb4f2cac945719ecc2a61061b/src/platform_impl/macos/view.rs#L392-L520), and captured winit streams show both [commit → empty preedit → release-only punctuation](https://github.com/alacritty/alacritty/issues/6942#issuecomment-2768968472) and [commit → empty preedit → duplicate Space press](https://github.com/alacritty/alacritty/issues/8079#issuecomment-2768910856) | A pending commit survives trailing empty preedit, IME close, and characterless keyboard events; the next printable event is recovered or suppressed exactly once, while non-empty preedit and command modifiers cancel the path |

The references define observable behavior, not architecture: the app keeps
Iced's native buffer and public renderer/input integrations while the custom
widget owns the rich layout geometry instead of embedding a DOM editor.

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
runtime `RichTextEditor` shapes one rich paragraph and uses that exact layout
for painting, hit testing, selection, caret movement, scrolling, and IME
placement. Its actions move the owned buffer through the update function, so
ordinary keystrokes and explicit commands do not clone the full document. A
bounded delta history records the resulting edits. The implementation uses
Iced's public `Content`, keyboard/clipboard/input-method abstractions, advanced
text renderer, and widget API; it does not patch or vendor Iced.

IBM Plex Sans KR is the body face and Monoplex KR is used for inline and fenced
code. Monoplex combines IBM Plex Mono Latin glyphs with IBM Plex Sans KR Hangul,
so prose and code have coordinated Latin forms, both cover Hangul, and code
keeps a 2:1 fixed width. Headings use per-span metrics and fenced code uses
visual-line backgrounds in the same editor layout. The bundled
files come from the pinned [IBM Plex](https://github.com/IBM/plex/tree/2f9ba1b25957d958db71a849e85d72e3ecfb845a/packages/plex-sans-kr)
and [Monoplex KR](https://github.com/y-kim/monoplex/tree/ccd78918fdaf00f1ae52282b0446d66ec0c06fea)
sources under their included SIL Open Font Licenses.

## Behavioral references

Concrete editor behavior is translated from these pinned JavaScript sources:

| Contract | Reference | Native implementation |
| --- | --- | --- |
| 16 px body, 1.6 line height, 800 px page, 50 px gutter | [MarkText/Muya block CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L1-L24) | Ice editor layout |
| H1–H6 scales `1.875, 1.5, 1.375, 1.25, 1.125, 1` and 1.4 line height | [MarkText/Muya heading CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L37-L82) | `RichTextEditor` span metrics and shared caret geometry |
| 90% monospace fenced code, 1.6 line height, surface, 1 px border, 3 px radius | [MarkText/Muya code-block CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L196-L235) | One coalesced native visual-line decoration; hidden fence rows provide vertical inset |
| Body-size monospace inline code with `0.2em 0.4em` padding and a distinct shared code surface | [MarkText/Muya inline padding and shared code-block background](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/inlineSyntax.css#L59-L69), [MarkText One Dark code surfaces](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/renderer/src/assets/themes/one-dark.theme.css#L15-L35), and [Typora GitHub inline code](https://github.com/typora/typora-default-themes/blob/cf4f2cb7e81a73050456367cfdfdb80b5a14a7b2/themes/github.css#L265-L275) | An 8% foreground tint makes inline code visibly darker while native body metrics keep an inline-code-only line from collapsing |
| Dark appearance follows the OS at startup and while running, with a direct toolbar override | [MarkText system-theme startup and update handling](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/main/app/index.ts#L274-L355) and [One Dark palette](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/renderer/src/assets/themes/one-dark.theme.css#L15-L43) | Ice system-theme task/subscription selects a typed light or dark palette; the native syntax highlighter changes theme with it |
| Enter on ```` ```lang ```` creates an empty fenced block and puts the caret in its code body | [MarkText/Muya paragraph conversion](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L42-L64) and [Enter handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L256-L315) | One native delta inserts the blank body and matching closing fence; one undo removes all of it |
| Fenced code uses language-aware token colors and emphasis | [MarkText/Muya Prism light theme](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/prismjs/light.theme.css#L28-L99) | Existing native `iced_highlighter` parses the info-string language incrementally |
| Syntax markers are zero-size unless the cursor intersects their token | [MarkText/Muya renderer](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/inlineRenderer/renderer/index.ts#L111-L136) and [marker CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/inlineSyntax.css#L1-L30) | Caret-aware highlighter with zero-size marker spans in the shared rich layout |
| Strong, emphasis, and deletion render as semantic inline elements with paired caret-local markers | [MarkText/Muya inline renderer](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/inlineRenderer/renderer/delEmStrongFactory.ts#L12-L65) | Composable native weight, italic, and strikethrough spans; hidden markers retain the containing font face so the first shaped word does not inherit the marker face |
| Applying bold inside an existing strong run removes the strong markers | [MarkText/Muya format toggle regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/base/__tests__/formatToggle.spec.ts#L88-L102) | Bold, italic, and inline-code commands toggle the containing run instead of nesting duplicate markers |
| Platform deletion keys: Ctrl/Option+Backspace deletes a group; macOS Cmd+Backspace deletes to the line boundary | [CodeMirror 6 standard keymap](https://github.com/codemirror/commands/blob/5b9bac974f2c4af3e20b045adef949667872ecad/src/commands.ts#L1019-L1060) | Shared native `text_editor` bindings |
| Enter continues bullet, ordered, and unchecked task markers; an empty item exits one list level; following ordered siblings are renumbered | [CodeMirror Markdown list command](https://github.com/codemirror/lang-markdown/blob/8f73fd5013a1aaa3d3319f9165fff3100d159124/src/commands.ts#L96-L185) | Caret-local Markdown edits recorded as one bounded undo event |
| Backspace at the start of list content removes the inferred list item | [CodeMirror Markdown markup deletion](https://github.com/codemirror/lang-markdown/blob/8f73fd5013a1aaa3d3319f9165fff3100d159124/src/commands.ts#L229-L275) and [MarkText list-to-paragraph behavior](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L636-L675) | The source marker is removed while nested indentation is retained |
| Tab inserts four-space code indentation; on a list it nests under a previous sibling, and Shift+Tab lifts one level | [MarkText/Muya Tab handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L922-L960) and [list-indent regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/__tests__/insertTabAndIndent.spec.ts#L180-L214) | The Markdown action adapter inserts four spaces; list nesting keeps the referenced two-space indent |
| Adjacent edits form bounded undo events | [CodeMirror 6 history grouping](https://github.com/codemirror/commands/blob/5b9bac974f2c4af3e20b045adef949667872ecad/src/history.ts#L23-L82) | In-place delta history with explicit event and byte limits |
| External URLs open through the OS shell | [MarkText desktop shell handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/main/ipc/shell.ts#L5-L17) | `open` crate default-browser dispatch |
| Only an actual rendered link is a mouse target | [MarkText/Muya link wrapper hit-testing](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/editor/linkMouseEvents.ts#L39-L57) and [hover regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/editor/__tests__/linkMouseEvents.spec.ts#L50-L74) | Native glyph hit-testing returns a pointer cursor only inside a safe Markdown link range |
| IME preedit remains a distinct composition and must not fight the committed-text caret | [CodeMirror composition state](https://github.com/codemirror/view/blob/fbff59ba004d80d8c914f64c42586387b08706ac/src/input.ts#L816-L846) and [cursor suppression during composition](https://github.com/codemirror/view/blob/fbff59ba004d80d8c914f64c42586387b08706ac/src/docview.ts#L250-L261) | The native preedit overlay uses the font size and visual line height at the editor caret, and suppresses the committed caret until Hangul composition is committed |

The references define observable behavior, not architecture: the app keeps
Iced's native buffer and public renderer/input integrations while the custom
widget owns the rich layout geometry instead of embedding a DOM editor.

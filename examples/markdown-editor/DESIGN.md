# Markdown editor example

This example is a native, single-document Markdown editor. Markdown formatting
is presented inline in the same writing surface instead of a separate preview,
custom rich-text widget, or web view.

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

The editor keeps one native `text_editor::Content` in application state and
passes it by Ice `bind`; ordinary keystrokes mutate that buffer directly and
do not copy the full document. A bounded native delta history observes edits
through `editor-action`, while cursor built-ins feed the incremental
`editor-highlighter`. Iced continues to own selection, clipboard, wrapping,
input methods, mixed-metric caret geometry, and visible-line highlighting.

Geist is the body face and the bundled Geist Mono is used for inline and fenced
code. Headings use native per-span metrics and fenced code uses native
visual-line backgrounds in the same editor layout. Both fonts come from the
[Geist project](https://github.com/vercel/geist-font) under the included SIL
Open Font License.

## Behavioral references

Concrete editor behavior is translated from these pinned JavaScript sources:

| Contract | Reference | Native implementation |
| --- | --- | --- |
| 16 px body, 1.6 line height, 800 px page, 50 px gutter | [MarkText/Muya block CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L1-L24) | Ice editor layout |
| H1–H6 scales `1.875, 1.5, 1.375, 1.25, 1.125, 1` and 1.4 line height | [MarkText/Muya heading CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L37-L82) | Iced span metrics and caret geometry |
| 90% monospace fenced code, 1.6 line height, surface, 1 px border, 3 px radius | [MarkText/Muya code-block CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/blockSyntax.css#L196-L235) | One coalesced native visual-line decoration; hidden fence rows provide vertical inset |
| Body-size monospace inline code with `0.2em 0.4em` padding | [MarkText/Muya inline padding](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/inlineSyntax.css#L59-L69) and [CodeMirror decoration styles](https://github.com/codemirror/language/blob/8e9700018446d46f23267f6e31da56628d5117c0/src/highlight.ts#L200-L238) | Native span background keeps the body font size and line height so an inline-code-only line does not collapse |
| Enter on ```` ```lang ```` creates an empty fenced block and puts the caret in its code body | [MarkText/Muya paragraph conversion](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L42-L64) and [Enter handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L256-L315) | One native delta inserts the blank body and matching closing fence; one undo removes all of it |
| Fenced code uses language-aware token colors and emphasis | [MarkText/Muya Prism light theme](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/prismjs/light.theme.css#L28-L99) | Existing native `iced_highlighter` parses the info-string language incrementally |
| Syntax markers are zero-size unless the cursor intersects their token | [MarkText/Muya renderer](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/inlineRenderer/renderer/index.ts#L111-L136) and [marker CSS](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/assets/styles/inlineSyntax.css#L1-L30) | Incremental highlighter with caret-local zero-size marker spans |
| Strong, emphasis, and deletion render as semantic inline elements with paired caret-local markers | [MarkText/Muya inline renderer](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/inlineRenderer/renderer/delEmStrongFactory.ts#L12-L65) | Composable native weight, italic, and strikethrough spans; hidden markers retain the containing font face so the first shaped word does not inherit the marker face |
| Applying bold inside an existing strong run removes the strong markers | [MarkText/Muya format toggle regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/base/__tests__/formatToggle.spec.ts#L88-L102) | Bold, italic, and inline-code commands toggle the containing run instead of nesting duplicate markers |
| Platform deletion keys: Ctrl/Option+Backspace deletes a group; macOS Cmd+Backspace deletes to the line boundary | [CodeMirror 6 standard keymap](https://github.com/codemirror/commands/blob/5b9bac974f2c4af3e20b045adef949667872ecad/src/commands.ts#L1019-L1060) | Shared native `text_editor` bindings |
| Enter continues bullet, ordered, and unchecked task markers; an empty item exits one list level; following ordered siblings are renumbered | [CodeMirror Markdown list command](https://github.com/codemirror/lang-markdown/blob/8f73fd5013a1aaa3d3319f9165fff3100d159124/src/commands.ts#L96-L185) | Caret-local Markdown edits recorded as one bounded undo event |
| Backspace at the start of list content removes the inferred list item | [CodeMirror Markdown markup deletion](https://github.com/codemirror/lang-markdown/blob/8f73fd5013a1aaa3d3319f9165fff3100d159124/src/commands.ts#L229-L275) and [MarkText list-to-paragraph behavior](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L636-L675) | The source marker is removed while nested indentation is retained |
| Tab inserts native code indentation; on a list it nests under a previous sibling, and Shift+Tab lifts one level | [MarkText/Muya Tab handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/index.ts#L922-L960) and [list-indent regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/block/content/paragraphContent/__tests__/insertTabAndIndent.spec.ts#L180-L214) | Shared `text_editor` Tab bindings use a four-column native tab stop; list nesting uses the referenced two-space Markdown indent |
| Adjacent edits form bounded undo events | [CodeMirror 6 history grouping](https://github.com/codemirror/commands/blob/5b9bac974f2c4af3e20b045adef949667872ecad/src/history.ts#L23-L82) | In-place delta history with explicit event and byte limits |
| External URLs open through the OS shell | [MarkText desktop shell handler](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/desktop/src/main/ipc/shell.ts#L5-L17) | `open` crate default-browser dispatch |
| Only an actual rendered link is a mouse target | [MarkText/Muya link wrapper hit-testing](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/editor/linkMouseEvents.ts#L39-L57) and [hover regression](https://github.com/marktext/marktext/blob/e52106fd1cdcbd33c1258b7b0cdc7013c4c5d86c/packages/muya/src/editor/__tests__/linkMouseEvents.spec.ts#L50-L74) | Native glyph hit-testing returns a pointer cursor only inside a safe Markdown link range |
| IME preedit remains a distinct composition and must not fight the committed-text caret | [CodeMirror composition state](https://github.com/codemirror/view/blob/fbff59ba004d80d8c914f64c42586387b08706ac/src/input.ts#L816-L846) and [cursor suppression during composition](https://github.com/codemirror/view/blob/fbff59ba004d80d8c914f64c42586387b08706ac/src/docview.ts#L250-L261) | The native preedit overlay uses the editor caret line height and suppresses the committed caret until Hangul composition is committed |

The references define observable behavior, not architecture: the app keeps
Iced's native buffer, selection, IME, clipboard, and rendering path instead of
embedding their DOM editor implementations.

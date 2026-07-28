app MarkdownEditor
  title "Markdown Editor"
  id "dev.ducktape.ice.markdown-editor"
  text-size 16
  antialiasing true
  window
    size 1200 800
    min-size 640 480
    position centered

use "theme.ice"
use "extern/editor.ice"
use "state.ice"
use "components/editor.ice"
use "tests/app.ice"

view
  box #app
    with
      w=fill
      h=fill
      @bg-bg
    EditorSurface #editor-surface document<->document

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
use "handlers/app.ice"
use "tests/app.ice"

view
  col #app
    with
      w=fill
      h=fill
      @bg-bg
    EditorToolbar #toolbar previewing=previewing
      events
        show_preview -> show_preview
        show_editor -> show_editor
    if !previewing
      EditorSurface #editor-surface document<->document
        events
          preview_shortcut -> preview_shortcut _
    if previewing
      box #preview-surface
        with
          w=fill
          h=fill
          p=48.0
          bg=surface
          align-x=center
        scroll w=fill h=fill
          markdown rendered text-size=17.0 gap=16.0 -> link_opened _
            style inline-code-bg=hover inline-code-fg=fg link=primary inline-code-p=3.0 inline-code-border=border inline-code-border-w=1.0 inline-code-r=4.0

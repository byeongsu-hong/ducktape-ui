app MarkdownEditor
  title "Markdown Editor"
  id "dev.ducktape.ice.markdown-editor"
  font "../../assets/fonts/Geist-Regular.ttf"
  font "../../assets/fonts/Geist-Bold.ttf"
  font "../../assets/fonts/Geist-Italic.ttf"
  font "../../assets/fonts/Geist-SemiBold.ttf"
  font "../../assets/fonts/GeistMono-Regular.ttf"
  text-size 14
  antialiasing true
  window
    size 1200 800
    min-size 720 520
    position centered
    exit-on-close false

use "theme.ice"
use "recipes.ice"
use "extern/editor.ice"
use "extern/document.ice"
use "state.ice"
use "components/editor.ice"
use "components/toolbar.ice"
use "components/find_bar.ice"
use "components/status_bar.ice"
use "components/confirm_dialog.ice"
use "handlers/app.ice"
use "tests/app.ice"

font geist family="Geist" default=true
font geist_mono family="Geist Mono"

view
  overlay
    with
      when=confirming
      dismiss=cancel_pending
      backdrop=black/24
      p=24.0
      align-x=center
      align-y=center
    content
      box #app
        with
          w=fill
          h=fill
          bg=bg
        col w=fill h=fill
          Toolbar #toolbar
            with
              name=name
              dirty=is_dirty()
              busy=busy
              undo_available=can_undo()
              redo_available=can_redo()
            events
              new_document -> request_new
              open_document -> request_open
              save_document -> request_save
              save_document_as -> request_save_as
              undo -> undo
              redo -> redo
              bold -> bold
              italic -> italic
              inline_code -> inline_code
              link -> link
              find -> toggle_find
          if find_open
            FindBar #find_bar query<->find_query
              events
                previous -> find_previous
                next -> find_next
                close -> toggle_find
          mouse release=follow_link
            EditorSurface #editor-surface document<->document
              with
                line=caret_line
                column=caret_column
                disabled=!editor_enabled
          StatusBar #status-bar
            with
              cursor_label=cursor_status(caret_line, caret_column, line_count)
              busy=busy
              has_error=has_error
              error=error
            events
              dismiss_error -> dismiss_error
    layer
      ConfirmDialog #confirm action=pending name=name
        events
          save_new -> save_then_new
          discard_new -> discard_new
          save_open -> save_then_open
          discard_open -> discard_open
          save_close -> save_then_close
          discard_close -> discard_close
          cancel -> cancel_pending

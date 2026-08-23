app MarkdownEditor
  title "Markdown"
  palette active_palette
  id "dev.ducktape.ice.markdown-editor"
  font "../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-Bold.ttf"
  font "../../../../assets/fonts/IBMPlexSans-Italic.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../assets/fonts/MonoplexKR-Regular.ttf"
  text-size 14
  antialiasing true
  window
    size 1120 720
    min-size 760 520
    position centered
    exit-on-close false

use "theme.ice"
use "recipes.ice"
use "extern/editor.ice"
use "extern/document.ice"
use "extern/library.ice"
use "state.ice"
use "components/sidebar.ice"
use "components/editor.ice"
use "components/find_bar.ice"
use "components/status_bar.ice"
use "components/delete_dialog.ice"
use "handlers/app.ice"
use "tests/app.ice"

font body family="IBM Plex Sans KR" default=true
font code family="Monoplex KR"

view
  overlay
    with
      when=confirming_delete
      dismiss=cancel_delete
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
        row w=fill h=fill
          Sidebar #sidebar query<->query
            with
              notes=visible
              path=path
              dark=dark
              blocked=interaction_blocked
            events
              search -> query_changed _
              new_note -> new_note
              select -> select_note _
              toggle_theme -> toggle_theme
          box #sheet-frame
            with
              w=fill
              h=fill
              pt=10.0
              pr=10.0
              pb=10.0
            box #sheet
              with
                w=fill
                h=fill
                bg=surface
                border=border
                border-w=1.0
                r=14.0
                clip=true
              col w=fill h=fill
                if find_open
                  FindBar #find_bar query<->find_query summary=find_summary
                    events
                      changed -> find_changed _
                      previous -> find_previous
                      next -> find_next
                      close -> toggle_find
                if !find_open
                  box #sheet-top
                    with
                      w=fill
                      px=20.0
                      pt=12.0
                    row
                      with
                        w=fill
                        gap=4.0
                        align=center
                      space w=fill h=1.0
                      button "Find" #find -> toggle_find
                        with
                          label="Find · Command or Ctrl+F"
                          disabled=interaction_blocked
                          @ghost_action
                      button "Delete" #delete -> request_delete
                        with
                          label="Delete note"
                          disabled=(interaction_blocked || empty(path))
                          @ghost_action
                mouse release=follow_link
                  EditorSurface #editor-surface -> edit_document _
                    with
                      document=document
                      dark=dark
                      disabled=!editor_enabled
                      focused=editor_focused
                      find=find_query
                StatusBar #status-bar
                  with
                    cursor_label=cursor_status(caret_line, caret_column, line_count)
                    saving=saving
                    dirty=history.dirty
                    has_error=has_error
                    error=error
                  events
                    dismiss_error -> dismiss_error
    layer
      DeleteDialog #confirm title=current_title busy=loading
        events
          delete -> confirm_delete
          cancel -> cancel_delete

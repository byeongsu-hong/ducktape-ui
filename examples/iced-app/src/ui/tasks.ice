app Tasks
  title window_title
  theme app_theme
  palette active_palette
  bg app_background
  fg app_text
  id "dev.ducktape.ice.tasks"
  executor iced::executor::Default
  text-size 16
  antialiasing true
  vsync true
  scale ui_scale
  window
    icon-rgba "../../assets/app.rgba" 2 1
    size 960 720
    min-size 480 360
    position centered
    platform linux
      app-id "dev.ducktape.ice.tasks"
      override-redirect false
    platform windows
      drag-and-drop true
      skip-taskbar false
      undecorated-shadow true
      corner round-small
    platform macos
      title-hidden false
      titlebar-transparent true
      fullsize-content-view true
    platform wasm
      target "iced"
  window child
    size 640 480
    min-size 320 240
    position centered

use "extern/backend.ice"
use "theme.ice"
use "state.ice"
use "components/task_row.ice"
use "components/dialog.ice" as modal
use "handlers/tasks.ice"

preset pristine

preset seeded
  state
    draft = "Preset task"
    loading = true
  boot
    run list_tasks() -> loaded _ | failed _

test workspace_pane_operations
  dispatch resize_workspace
  dispatch drop_details
  dispatch inspect_workspace
  dispatch close_details
  dispatch appearance_changed("Renamed", "dark", "#123456", "#abcdef", 1.5)
  dispatch palette_switched
  expect child_window == none
  expect window_title == "Renamed"
  expect app_theme == "dark"
  expect app_background == "#123456"
  expect app_text == "#abcdef"
  expect ui_scale == 1.5

view
  overlay
    with
      when=about_open
      dismiss=close_about
      backdrop=black/60
      p=24.0
      align-x=center
      align-y=center
    content
      col
        with
          gap=24.0
          p=24.0
          @w-full
          @h-full
          @bg-bg
        flex
          with
            w=fill
            gap=12.0
            justify=space-between
            items=center
          text "Tasks"
            with
              size=24.0
              @font-bold
              @text-fg
          text len(tasks) size=14.0 @text-muted
          toggler "About" -> about_toggled _
            with
              checked=about_open
              disabled=loading
              size=18.0
              gap=8.0
            active checked bg=linear(1.57, primary@0.0, surface@1.0) bg-border=primary bg-border-w=1.0 fg=linear(0.0, fg@0.0, primary@1.0) fg-border=fg fg-border-w=1.0 text=fg r=7.0 r-tl=6.0 r-tr=7.0 r-br=8.0 r-bl=9.0 p-ratio=0.125
            active unchecked bg=surface fg=fg text=muted
            hovered checked bg=primary fg=fg text=fg
            hovered unchecked bg=bg fg=primary text=fg
            disabled checked bg=surface fg=muted text=muted
            disabled unchecked bg=bg fg=muted text=muted
          button "About" style=text -> open_about
          button "New window" style=secondary -> open_child
          text "Child:" size=14.0 @text-muted
          text child_width size=14.0 @text-muted
          text "×" size=14.0 @text-muted
          text child_height size=14.0 @text-muted

        row
          with
            gap=12.0
            align=center
            @w-full
          button "Capture window" style=secondary -> capture_window
          button "Change icon" style=subtle -> set_window_icon
          button "Inspect handle" style=subtle -> inspect_window_handle
          button "Read raw ID" style=subtle -> read_raw_window_id
          text raw_window_id size=14.0 @text-muted
          if snapshot_ready
            image window_snapshot
              with
                w=160.0
                h=90.0
                fit=contain
            text snapshot_width size=14.0 @text-muted
            text "×" size=14.0 @text-muted
            text snapshot_height size=14.0 @text-muted
            text snapshot_scale size=14.0 @text-muted

        row
          with
            gap=12.0
            align=center
            @w-full
          input "New task" <-> draft
            with
              hint="What needs doing?"
              disabled=loading
              submit=submit
              w=fill
              @px-4
              @py-3
              @bg-surface
              @border
              @border-border
              @rounded-lg
          button "Add" -> submit
            with
              disabled=!can_submit
              style=success
              @px-4
              @py-3
              @disabled:opacity-50
            active bg=linear(1.57, primary@0.0, surface@1.0) text=white border=primary border-w=1.0 r=8.0 shadow=black/25 shadow-y=2.0 shadow-blur=4.0 px-snap=true
            hovered bg=linear(1.57, surface@0.0, primary@1.0) text=white r=10.0
            pressed bg=primary/80 text=white r=10.0
            disabled bg=surface text=muted r=10.0

        if has_error
          row
            with
              gap=16.0
              p=16.0
              align=center
              @w-full
              @bg-danger
              @rounded-lg
            text error size=14.0 @text-white
            button "Retry" -> retry
              with
                disabled=loading
                style=danger
                @task_danger_action

        lazy loading as busy
          col
            if busy
              text "Working..." size=14.0 @text-muted

        if empty(tasks) && !loading
          text "No tasks yet." size=14.0 @text-muted

        panes #workspace w=fill h=fill gap=8.0 min-size=120.0 resize=8.0 drag click=pane_clicked(_)
          style
            hovered-region bg=linear(0.785, primary/10@0.0, primary/40@1.0) border=primary border-w=2.0 r=8.0
            hovered-split color=primary w=3.0
            picked-split color=fg w=3.0
          split vertical ratio=0.7
            pane tasks bg=linear(1.57, surface@0.0, bg@1.0) shadow=black/50 shadow-y=2.0 shadow-blur=8.0 px-snap=true border-w=1.0 r=10.0 @border-border
              title p=12.0 always-controls bg=linear(1.57, bg@0.0, surface@1.0) border=border border-w=1.0 r-tl=8.0 r-tr=8.0 shadow=black/25 shadow-y=1.0 shadow-blur=3.0 px-snap=true
                text "Task list"
                  with
                    size=18.0
                    @font-bold
                    @text-fg
              controls
                button "Inspect" style=secondary -> inspect_adjacent
              compact
                button "?" style=subtle -> inspect_adjacent
              scroll #task-list
                with
                  dir=vertical
                  w=fill
                  h=fill
                keyed task in tasks by=task.id w=fill gap=8.0
                  TaskRow task=task loading=loading
                    events
                      toggle -> toggle _ _
            pane details border-w=1.0 r=10.0 @bg-surface border-border
              title p=12.0 always-controls border-w=1.0 @bg-bg border-border
                text "Details"
                  with
                    size=18.0
                    @font-bold
                    @text-fg
              controls
                button "Maximize" style=bg -> maximize_details
              compact
                button "↗" style=warning -> maximize_details
              box
                with
                  w=fill
                  h=fill
                  p=16.0
                col gap=12.0
                  text "Drag, resize, or arrange this pane." size=14.0 @text-muted
                  canvas
                    with
                      w=fill
                      h=160.0
                      cache=detail_mode
                      cache-group=details
                      capture=true
                      cursor=(cursor_state)
                      cursor-outside=true
                    state
                      cursor_state = "crosshair"
                      hits = 0
                    event mouse pressed as button
                      set cursor_state = "grabbing"
                      set hits = hits + 1
                      emit canvas_button button
                      capture
                    event mouse released as button
                      set cursor_state = "crosshair"
                      redraw
                      capture
                    event keyboard press -> canvas_key _
                    redraw window frame after=1s
                    capture touch lost
                    rect x=0.0 y=0.0 w=canvas_width h=canvas_height fill=linear(1.57, bg@0.0, surface@1.0) stroke=border
                    circle x=48.0 y=48.0 r=28.0 fill=primary stroke=fg stroke-w=2.0
                    path fill=primary/25 stroke=primary stroke-w=2.0 cap=round join=round
                      move x=96.0 y=112.0
                      bezier ax=136.0 ay=24.0 bx=176.0 by=152.0 x=224.0 y=64.0
                      line x=224.0 y=112.0
                      close
                    text detail_mode x=16.0 y=136.0 color=fg size=14.0 font=default
                    text hits x=112.0 y=136.0 color=primary size=14.0 font=default
                    image "../../assets/checker.ppm" x=256.0 y=16.0 w=48.0 h=48.0 filter=nearest opacity=0.9 snap=true r=6.0
                    svg "../../assets/ice.svg" x=312.0 y=16.0 w=48.0 h=48.0 color=primary opacity=0.9
                  shader status_shader(1.0) w=fill h=32.0 -> shader_hovered _
                  row wrap gap=8.0
                    radio "Summary" -> detail_mode_changed _
                      with
                        value="summary"
                        selected=(detail_mode == "summary")
                        size=16.0
                        gap=6.0
                        text-size=14.0
                        line-h=1.2
                        shape=advanced
                        wrap=word
                        font=default
                      active selected bg=linear(1.57, primary@0.0, surface@1.0) dot=fg border=primary border-w=2.0 text=fg
                      active unselected bg=surface dot=primary border=border text=muted
                      hovered selected bg=primary dot=fg border=fg text=fg
                      hovered unselected bg=bg dot=primary border=primary text=fg
                    radio "Activity" -> detail_mode_changed _
                      with
                        value="activity"
                        selected=(detail_mode == "activity")
                    button "Restore" -> restore_workspace
                    button "Swap" -> swap_workspace
                    button "Move left" -> move_details_left
                    button "Open preview" -> open_preview
          pane preview closed border-w=1.0 r=10.0 @bg-surface border-border
            title p=12.0 always-controls border-w=1.0 @bg-bg border-border
              text "Preview"
                with
                  size=18.0
                  @font-bold
                  @text-fg
            controls
              button "Close" -> close_preview
            compact
              button "×" -> close_preview
            box
              with
                w=fill
                h=fill
                p=16.0
              text "This pane was opened dynamically." size=14.0 @text-muted
    layer
      modal::Dialog
        modal::Dialog.Header
          text "About Ice Tasks"
            with
              size=20.0
              @font-bold
              @text-fg
        modal::Dialog.Body
          rich-text -> about_link _
            with
              w=fill
              wrap=word
              size=14.0
              @text-muted
            span "This dialog is a structured overlay written entirely in "
            span ".ice" link="https://github.com/byeongsu-hong/ducktape-ui" bg=linear(1.57, primary/20@0.0, surface@1.0) p=2.0 r=2.0 underline @font-bold text-primary
            span "."
        modal::Dialog.Actions
          button "Close" style=primary @task_primary_action -> close_about

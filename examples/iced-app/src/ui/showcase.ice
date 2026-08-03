app Showcase
  renderer crate::backend::AppRenderer

font ui family=sans weight=medium stretch=normal style=normal default=true

use "extern/showcase.ice"
use "theme.ice"
use "handlers/task_crud.ice"

state
  tasks:[Task] = []
  draft = ""
  loading = false
  error = ""
  volume = 40.0
  precise_volume:SliderNumber = slider_number(40.0)
  notifications = true
  view_mode = 0
  display_modes = ["List", "Board", "Timeline"]
  searchable_modes:combo[str] = ["List", "Board", "Timeline"]
  display_mode:str? = none
  picker_open = false
  mode_query = ""
  hovered_mode = ""
  observed_width = 0.0
  observed_height = 0.0
  external_hover = false
  event_seen = false
  native_hover = false
  pointer_x = 0.0
  pointer_y = 0.0
  scroll_pixels = false
  scroll_x = 0.0
  scroll_y = 0.0
  scroll_relative_x = 0.0
  scroll_relative_y = 0.0
  help:markdown = "# Ice **renders** [iced docs](https://iced.rs)"
  help_images:[str] = []
  notes:editor = "fn main() { println!(\"ice\"); }"
  notes_text = ""
  editor_title = "Editor"
  last_key = "none"
  command_down = false
  key_repeat = false
  system_theme = "none"
  cpu_brand = "unknown"
  clipboard_text:str? = none
  primary_text:str? = none
  draft_focused = false
  window_width = 0.0
  window_height = 0.0
  window_maximized = false
  window_minimized:bool? = none
  window_x:f64? = none
  window_y:f64? = none
  window_scale = 1.0
  window_mode = "windowed"
  monitor_width:f64? = none
  monitor_height:f64? = none
  encoded_image = encoded(bytes(50 36 0a 31 20 31 0a 32 35 35 0a ff 00 ff))
  memory_image = rgba(2, 2, bytes(ff 00 00 ff 00 ff 00 ff 00 00 ff ff ff ff ff ff))

derived
  normalized_draft = trim(draft)
  can_submit = !loading && !empty(normalized_draft)
  has_error = error != ""

component TaskRow(task:Task, loading:bool)
  emits
    toggle(i64, bool)
  row #root
    with
      p=16.0
      align=center
      @w-full
      @bg-surface
      @border
      @border-border
      @rounded-lg
    checkbox task.title -> emit(toggle, task.id, _)
      with
        checked=task.done
        disabled=loading
        style=task_checkbox(loading)
        size=18.0
        w=fill
        gap=8.0
        text-size=14.0
        line-h=1.2
        shape=auto
        wrap=word-or-glyph
        font=default
        icon="✓"
        icon-size=12.0
        icon-line-h=1.0
        icon-shape=basic

component EditorPanel(bind content:editor, bind heading:str, busy:bool)
  emits
    editor_command(EditorCommand)
  col gap=8.0
    input "Editor heading" <-> heading hint="Editor heading" disabled=busy
    editor <-> content -> emit(editor_command, _)
      with
        hint="Write notes"
        w=640.0
        h=120.0
        min-h=80.0
        max-h=240.0
        size=14.0
        line-h=1.3
        p=8.0
        wrap=word
        font=ui
        highlighter=editor_highlight("fn")
        key-binding=editor_keys(busy)
        style=editor_surface(busy)
        disabled=busy
      active bg=surface border=border border-w=1.0 r=8.0 placeholder=muted value=fg selection=primary
      hovered bg=surface border=fg placeholder=muted value=fg selection=primary
      focused bg=surface border=primary border-w=2.0 r=8.0
      focused-hovered bg=surface border=primary border-w=2.0 r=8.0
      disabled bg=bg border=border placeholder=muted value=muted selection=primary

on draft_pasted(next)
  draft = next

on volume_changed(next)
  volume = next

on precise_volume_changed(next)
  precise_volume = next

on volume_committed

on notifications_changed(next)
  notifications = next

on view_mode_changed(next)
  view_mode = next

on render_coverage_resized(_dx, _dy)

on display_mode_changed(next)
  display_mode = some(next)

on picker_opened
  picker_open = true

on picker_closed
  picker_open = false

on mode_searched(next)
  mode_query = next

on mode_hovered(next)
  hovered_mode = next

on reset_search_modes
  display_modes = ["List", "Board", "Timeline", "Compact"]
  searchable_modes = ["List", "Board", "Timeline", "Compact"]

on add_search_mode
  combo searchable_modes push "Calendar"

on panel_measured(width, height)
  observed_width = width
  observed_height = height

on panel_hidden
  observed_width = 0.0
  observed_height = 0.0

on copy_draft
  return if empty(normalized_draft)
  task clipboard write draft

on copy_primary
  task clipboard write-primary draft

on read_clipboard
  task clipboard read -> clipboard_read _

on clipboard_read(value)
  clipboard_text = value

on read_primary
  task clipboard read-primary -> primary_read _

on primary_read(value)
  primary_text = value

on focus_draft
  task widget focus #new-task

on check_draft_focus
  task widget focused #new-task -> draft_focus_checked _

on draft_focus_checked(value)
  draft_focused = value

on previous_focus
  task widget focus-prev

on next_focus
  task widget focus-next

on draft_cursor_front
  task widget cursor-front #new-task

on draft_cursor_end
  task widget cursor-end #new-task

on draft_cursor
  task widget cursor #new-task 2

on draft_select_all
  task widget select-all #new-task

on draft_select_range
  task widget select #new-task 0 2

on task_list_snap
  task widget snap #task-list 0.0 0.5

on task_list_snap_end
  task widget snap-end #task-list

on task_list_scroll_to
  task widget scroll-to #task-list 0.0 24.0

on task_list_scroll_by
  task widget scroll-by #task-list 0.0 8.0

on window_close
  task window close

on window_drag
  task window drag

on window_drag_resize
  task window drag-resize south-east

on window_resize
  task window resize 960.0 720.0

on window_resizable
  task window resizable true

on window_min_size
  task window min-size 480.0 360.0

on window_clear_min_size
  task window min-size none

on window_max_size
  task window max-size 1920.0 1080.0

on window_resize_increments
  task window resize-step 8.0 8.0

on window_read_size
  task window size -> window_size_read _ _

on window_size_read(width, height)
  window_width = width
  window_height = height

on window_read_maximized
  task window maximized -> window_maximized_read _

on window_maximized_read(value)
  window_maximized = value

on window_maximize
  task window maximize true

on window_read_minimized
  task window minimized -> window_minimized_read _

on window_minimized_read(value)
  window_minimized = value

on window_minimize
  task window minimize false

on window_read_position
  task window position -> window_position_read _ _

on window_position_read(x, y)
  window_x = x
  window_y = y

on window_read_scale
  task window scale -> window_scale_read _

on window_scale_read(value)
  window_scale = value

on window_move
  task window move 40.0 40.0

on window_read_mode
  task window mode -> window_mode_read _

on window_mode_read(value)
  window_mode = value

on window_fullscreen
  task window set-mode fullscreen

on window_toggle_maximize
  task window toggle-maximize

on window_toggle_decorations
  task window toggle-decorations

on window_attention
  task window attention informational

on window_clear_attention
  task window attention none

on window_focus
  task window focus

on window_level
  task window level normal

on window_system_menu
  task window system-menu

on window_mouse_passthrough
  task window mouse-passthrough false

on window_read_monitor
  task window monitor-size -> window_monitor_read _ _

on window_monitor_read(width, height)
  monitor_width = width
  monitor_height = height

on window_automatic_tabbing
  task window auto-tabs false

on external_hover_changed(next)
  external_hover = next

on external_event(next)
  event_seen = next

on native_enter
  native_hover = true

on native_exit
  native_hover = false

on native_press
  native_hover = !native_hover

on native_move(x, y)
  pointer_x = x
  pointer_y = y

on native_scroll(x, y, pixels)
  pointer_x = x
  pointer_y = y
  scroll_pixels = pixels

on task_list_scrolled(x, y, _reversed_x, _reversed_y, relative_x, relative_y, _bounds_x, _bounds_y, _bounds_width, _bounds_height, _content_x, _content_y, _content_width, _content_height)
  scroll_x = x
  scroll_y = y
  scroll_relative_x = relative_x
  scroll_relative_y = relative_y

on docs_link(_url)

on editor_command(command)
  event_seen = command.save

on read_notes
  notes_text = editor_text(notes)

on clear_notes
  notes = editor("")

on extend_markdown
  markdown help append "\n\n![Ice](asset://ice)"
  help_images = markdown_images(help)

on key_pressed(event)
  last_key = event.key.kind
  command_down = event.modifiers.command
  key_repeat = event.repeat

on key_released(event)
  last_key = event.key.kind
  command_down = event.modifiers.command

on key_modifiers_changed(modifiers)
  command_down = modifiers.command

on inspect_system
  task system info -> system_inspected _

on system_inspected(info)
  cpu_brand = info.cpu_brand

on read_system_theme
  task system theme -> system_theme_changed _

on system_theme_changed(next)
  system_theme = next

on open_nested_preview
  pane #nested_workspace split nested_editor nested_preview horizontal ratio=0.4

on close_nested_preview
  pane #nested_workspace close nested_preview

on resize_nested_editor
  pane #nested_workspace resize editor_stack 0.45

on open_task_pane
  pane #nested_workspace split nested_terminal pane_task(1) vertical ratio=0.45

on close_task_pane(id)
  pane #nested_workspace close pane_task(id)

on maximize_task_pane
  pane #nested_workspace maximize pane_task(1)

on open_mode_pane
  pane #nested_workspace split nested_files mode_pane("List") horizontal ratio=0.35

on close_mode_pane(name)
  pane #nested_workspace close mode_pane(name)

subscribe
  app_events() -> external_event _
  keyboard press status=ignored -> key_pressed _
  keyboard release -> key_released _
  keyboard modifiers -> key_modifiers_changed _
  system theme -> system_theme_changed _

test showcase_behavior
  timeout 10s
  key-down "q"
  key-down "q" repeat=true
  key-up "q"
  expect key_repeat
  modifiers control logo
  expect command_down
  dispatch draft_pasted("Ship it")
  dispatch copy_draft
  dispatch read_clipboard
  expect clipboard_text == some("Ship it")
  dispatch copy_primary
  dispatch read_primary
  expect primary_text == some("Ship it")
  dispatch draft_cursor_front
  dispatch draft_cursor_end
  dispatch draft_cursor
  dispatch draft_select_all
  dispatch draft_select_range
  dispatch previous_focus
  dispatch next_focus
  dispatch task_list_snap
  dispatch task_list_snap_end
  dispatch task_list_scroll_to
  dispatch task_list_scroll_by
  dispatch window_close
  dispatch window_drag
  dispatch window_drag_resize
  dispatch window_resizable
  dispatch window_min_size
  dispatch window_clear_min_size
  dispatch window_max_size
  dispatch window_resize_increments
  dispatch window_maximize
  dispatch window_minimize
  dispatch window_fullscreen
  dispatch window_toggle_decorations
  dispatch window_attention
  dispatch window_clear_attention
  dispatch window_level
  dispatch window_system_menu
  dispatch window_mouse_passthrough
  dispatch window_automatic_tabbing
  dispatch window_resize
  dispatch window_read_size
  expect window_width ~= 960.0
  expect window_height ~= 720.0
  dispatch window_move
  dispatch window_read_position
  expect window_x == some(40.0)
  expect window_y == some(40.0)
  dispatch window_read_monitor
  expect monitor_width == some(960.0)
  expect monitor_height == some(720.0)
  dispatch window_read_maximized
  expect !window_maximized
  dispatch window_read_minimized
  expect window_minimized == none
  dispatch window_read_scale
  expect window_scale ~= 1.0
  dispatch window_read_mode
  expect window_mode == "windowed"
  dispatch task_list_scrolled(12.0, 24.0, 0.0, 0.0, 0.25, 0.5, 0.0, 0.0, 640.0, 480.0, 0.0, 0.0, 640.0, 1200.0)
  expect scroll_x == 12.0
  expect scroll_y == 24.0
  expect scroll_relative_x == 0.25
  expect scroll_relative_y == 0.5
  dispatch native_scroll(8.0, 16.0, true)
  expect scroll_pixels
  dispatch native_move(120.0, 48.0)
  expect pointer_y == 48.0

view
  col
    with
      gap=24.0
      p=24.0
      @w-full
      @h-full
      @bg-bg
    row
      with
        gap=12.0
        align=center
        @w-full
      text "Tasks"
        with
          font=ui
          style=summary_text(loading)
          size=24.0
          @font-bold
      lazy tasks as cached_tasks
        text len(cached_tasks) size=14.0 @text-muted
      text last_key size=14.0 @text-muted
      text system_theme size=14.0 @text-muted
      text cpu_brand size=14.0 @text-muted
      button "Inspect system" -> inspect_system
      button "Read theme" -> read_system_theme
      button "Copy primary" -> copy_primary
      button "Read clipboard" -> read_clipboard
      button "Read primary" -> read_primary
      button "Focus draft" -> focus_draft
      button "Check draft focus" -> check_draft_focus
      button "Read window size" -> window_read_size
      button "Toggle maximize" -> window_toggle_maximize
      button "Focus window" -> window_focus
      if draft_focused
        text "draft focused" size=14.0 @text-muted

    row
      with
        gap=12.0
        align=center
        @w-full
      input "New task" #new-task <-> draft
        with
          hint="What needs doing?"
          disabled=loading
          secure=false
          submit=submit
          paste=draft_pasted
          w=fill
          text-size=14.0
          line-h=1.2
          align=left
          font=ui
          style=form_input(loading)
          @px-4
          @py-3
        active bg=surface border=border border-w=1.0 r=8.0 icon=primary placeholder=muted value=fg selection=primary
        hovered bg=surface border=fg border-w=1.0 r=10.0 icon=primary placeholder=muted value=fg selection=primary
        focused bg=surface border=primary border-w=2.0 r=8.0
        focused-hovered bg=surface border=primary border-w=2.0 r=8.0
        disabled bg=bg border=border border-w=1.0 r=10.0 icon=muted placeholder=muted value=muted selection=primary
        icon code="+" font=ui size=14.0 gap=6.0 side=left
      button -> copy_draft
        with
          label="Copy draft"
          disabled=empty(normalized_draft)
          h=44.0
          p=8.0
          clip=true
          @bg-surface
          @text-fg
          @rounded-lg
          @disabled:opacity-50
        row gap=8.0 align=center
          text "Copy" size=14.0 @text-fg
          text "⌘C" size=12.0 @text-muted
      button "Add" -> submit
        with
          disabled=!can_submit
          style=action_button(loading)
          @px-4
          @py-3
          @bg-primary
          @text-white
          @rounded-lg
          @hover:bg-primary/90
          @pressed:bg-primary/70
          @disabled:opacity-50

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
            @px-4
            @py-2
            @bg-white
            @text-danger
            @rounded-md
            @disabled:opacity-50

    if loading
      text "Working..." size=14.0 @text-muted

    if empty(tasks) && !loading
      col
        with
          p=24.0
          align=center
          @w-full
          @bg-surface
          @border
          @border-border
          @rounded-lg
        text "No tasks yet." size=14.0 @text-muted

    box #summary
      with
        style=summary_container(loading)
        w=fill
        h=80.0
        max-w=720.0
        max-h=120.0
        align-x=center
        align-y=center
        clip=true
        p=8.0
        pl=12.0
        bg=linear(1.57, surface@0.0, bg@1.0)
        text=muted
        border=primary
        border-w=1.0
        r=8.0
        shadow=black/50
        shadow-y=2.0
        shadow-blur=6.0
        px-snap=true
      text "A native box owns one structured child tree." size=14.0 @text-muted

    rule horizontal
      with
        thickness=1.0
        style=weak
        fill=pad(12,4)
        color=border
        r=2.0
        snap=true

    grid
      with
        gap=16.0
        w=640.0
        h=aspect(16.0,9.0)
        max-cell=280.0
        @w-full
      col
        with
          gap=8.0
          p=16.0
          @w-full
          @bg-surface
          @rounded-lg
        text "Controls"
          with
            w=fill
            h=30.0
            size=18.0
            line-h-px=22.0
            font=default
            align-x=left
            align-y=center
            shape=advanced
            wrap=word
            @font-bold
            @text-fg
        text "TRACKED LABEL"
          with
            size=12.0
            tracking=1.2
            align-x=left
            w=fill
            @text-muted
        box
          with
            p=12.0
            w=fill
            border=border
            border-w=1.0
            border-dash=(4.0, 3.0)
            r=8.0
          text "Nothing final here yet" size=12.0 @text-muted
        theme tokyo-night fg=white bg=linear(1.57, bg@0.0, surface@1.0)
          qr "https://github.com/byeongsu-hong/ducktape-ui"
            with
              correction=high
              version=normal(8)
              size=112.0
              cell=fg
              bg=surface
        qr draft
          with
            cell-size=3.0
            cell=fg
            bg=surface
        toggler "Notifications" -> notifications_changed _
          with
            checked=notifications
            style=notification_toggler(loading)
            size=20.0
            w=fill
            gap=8.0
            text-size=14.0
            line-h=1.2
            shape=auto
            wrap=word
            font=default
            align=left
        slider volume -> volume_changed _
          with
            min=0.0
            max=100.0
            step=5.0
            default=50.0
            shift-step=1.0
            w=fill(2)
            h=20.0
            style=volume_slider(loading)
            release=volume_committed
          active rail-start=linear(0.0, primary@0.0, fg@1.0) rail-end=linear(1.57, border@0.0, bg@1.0) rail-w=4.0 rail-r=2.0 handle=circle(7.0) handle-color=linear(0.785, primary@0.0, fg@1.0)
          hovered rail-start=fg rail-end=border rail-w=5.0 handle=rect(12) handle-color=fg handle-r=3.0
          dragged rail-start=danger rail-end=border handle=circle(8.0) handle-color=danger handle-border=fg handle-border-w=1.0
        slider volume vertical -> volume_changed _
          with
            min=0.0
            max=100.0
            step=5.0
            default=50.0
            shift-step=1.0
            w=20.0
            h=120.0
            style=volume_slider(loading)
            release=volume_committed
        slider precise_volume -> precise_volume_changed _
          with
            min=slider_number(0.0)
            max=slider_number(100.0)
            step=slider_number(0.5)
            default=slider_number(50.0)
            shift-step=slider_number(0.1)
            style=volume_slider(loading)
        progress volume
          with
            length=fill
            girth=24.0
            style=loading_progress(loading)
            bg=linear(1.57, bg@0.0, surface@1.0)
            bar=linear(0.0, primary@0.0, fg@1.0)
            border=fg
            border-w=1.0
            r=4.0
            r-tl=2.0
        progress volume vertical
          with
            length=120.0
            girth=20.0
            style=warning
            bg=linear(1.57, bg@0.0, surface@1.0)
            bar=linear(0.0, danger@0.0, primary@1.0)
            r=3.0
        extern native_help(external_hover) -> external_hover_changed _
        extern borrowed_help(draft, external_hover) -> external_hover_changed _
        if event_seen
          text "External subscription active" size=12.0 @text-muted
        row wrap
          with
            w=fill
            h=shrink
            gap=12.0
            py=4.0
            align=center
            clip=false
            wrap-gap=8.0
            wrap-align=start
          image "../../assets/checker.ppm"
            with
              w=48.0
              h=48.0
              fit=cover
              filter=nearest
              r=8.0
          image encoded_image
            with
              w=24.0
              h=48.0
              fit=cover
              filter=nearest
          image memory_image
            with
              w=48.0
              h=48.0
              fit=cover
              filter=nearest
              rotate=rotation.solid(radians(0.1))
              r=8.0
              r-tl=2.0
              r-br=2.0
              crop=(0, 0, 1, 2)
          viewer memory_image
            with
              w=160.0
              h=96.0
              fit=contain
              filter=nearest
              p=4.0
              min-scale=0.5
              max-scale=8.0
              scale-step=0.25
          svg "../../assets/ice.svg"
            with
              w=48.0
              h=48.0
              fit=contain
              opacity=0.9
              color=fg
              hover=primary
              style=status_svg(loading)
          svg "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><rect width='1' height='1'/></svg>" memory
            with
              w=16.0
              h=16.0
              color=fg
              hover=primary
          svg bytes(3c 73 76 67 2f 3e) memory
            with
              w=16.0
              h=16.0
              color=fg
              hover=primary
          tooltip
            with
              position=bottom
              gap=4.0
              p=8.0
              delay=100
              snap=true
              style=summary_container(loading)
              bg=linear(1.57, surface@0.0, bg@1.0)
              text=fg
              border=border
              border-w=1.0
              r=8.0
              r-tl=4.0
              shadow=black/50
              shadow-x=0.0
              shadow-y=4.0
              shadow-blur=12.0
              px-snap=true
            mouse
              with
                enter=native_enter
                exit=native_exit
                press=native_press
                move=native_move
                scroll=native_scroll
                cursor=pointer
              text "Native pointer area" size=14.0 @text-fg
            col
              with
                p=8.0
                @bg-surface
                @rounded-md
              text "Native tooltip" size=14.0 @text-fg
              if native_hover
                text "Pointer is inside" size=12.0 @text-muted
              text pointer_x size=12.0 @text-muted
      col wrap
        with
          w=fill
          h=shrink
          gap=8.0
          p=16.0
          max-w=672.0
          align=start
          clip=false
          wrap-gap=8.0
          wrap-align=start
          @bg-surface
          @rounded-lg
        text "View mode"
          with
            size=18.0
            @font-bold
            @text-fg
        markdown help -> docs_link _
          with
            text-size=14.0
            h1-size=28.0
            h2-size=24.0
            h3-size=20.0
            h4-size=18.0
            h5-size=16.0
            h6-size=14.0
            code-size=12.0
            gap=10.0
            viewer=docs_viewer("showcase")
          style font=ui inline-code-bg=bg inline-code-fg=fg inline-code-font=mono code-block-font=mono link=primary inline-code-p=2.0 inline-code-px=4.0 inline-code-py=3.0 inline-code-border=border inline-code-border-w=1.0 inline-code-r=4.0
        row gap=8.0 align=center
          button "Append Markdown image" -> extend_markdown
          text len(help_images) size=12.0 @text-muted
        EditorPanel content<->notes heading<->editor_title busy=loading
          events
            editor_command -> editor_command _
        row gap=8.0 align=center
          button "Read notes" -> read_notes
          button "Clear notes" -> clear_notes
          text notes_text size=12.0 @text-muted
        pick display_modes display_mode -> display_mode_changed _
          with
            hint="Choose a view"
            w=fill
            menu-h=160.0
            p=8.0
            text-size=14.0
            line-h=1.2
            shape=advanced
            font=ui
            open=picker_opened
            close=picker_closed
            style=view_picker(loading)
            menu-style=view_menu(loading)
          active text=fg placeholder=muted handle=primary bg=surface border=border border-w=1.0 r=6.0
          hovered text=fg placeholder=muted handle=fg bg=bg border=primary border-w=1.0 r=6.0
          opened text=fg placeholder=muted handle=primary bg=surface border=primary border-w=1.0 r=6.0
          opened-hovered text=fg placeholder=muted handle=fg bg=bg border=primary border-w=2.0 r=6.0
          menu text=fg selected-text=fg selected-bg=linear(1.57, primary@0.0, surface@1.0) bg=surface border=border border-w=1.0 r=6.0 shadow=black/50 shadow-y=4.0 shadow-blur=12.0
          handle dynamic
            closed code="⌄" font=ui size=12.0 line-h=1.0 shape=basic
            open code="⌃" font=ui size=12.0 line-h=1.0 shape=advanced
        combo searchable_modes display_mode "Search views" -> display_mode_changed _
          with
            w=fill
            menu-h=160.0
            p=8.0
            text-size=14.0
            line-h=1.2
            shape=advanced
            font=ui
            input=mode_searched
            hover=mode_hovered
            open=picker_opened
            close=picker_closed
            style=form_input(loading)
            menu-style=view_menu(loading)
          active bg=surface border=border border-w=1.0 r=6.0 icon=primary placeholder=muted value=fg selection=primary
          hovered bg=bg border=primary border-w=1.0 r=6.0 icon=fg placeholder=muted value=fg selection=primary
          focused bg=surface border=primary border-w=1.0 r=6.0 icon=primary placeholder=muted value=fg selection=primary
          focused-hovered bg=bg border=fg border-w=2.0 r=6.0 icon=fg placeholder=muted value=fg selection=primary
          disabled bg=bg border=border border-w=1.0 r=6.0 icon=muted placeholder=muted value=muted selection=primary
          menu text=fg selected-text=fg selected-bg=linear(1.57, primary@0.0, surface@1.0) bg=surface border=border border-w=1.0 r=6.0 shadow=black/50 shadow-y=4.0 shadow-blur=12.0
          icon code="⌕" font=ui size=14.0 gap=6.0 side=right
        button "Reset mode options" -> reset_search_modes
        button "Add search option" -> add_search_mode
        if picker_open
          text "Picker is open" size=12.0 @text-muted
        if mode_query != ""
          text mode_query size=12.0 @text-muted
        if hovered_mode != ""
          text hovered_mode size=12.0 @text-muted
        sensor
          with
            show=panel_measured
            resize=panel_measured
            hide=panel_hidden
            key=mode_query
            anticipate=16.0
            delay=10
          responsive
            with
              size=(available_width, available_height)
              w=fill
              h=32.0
            col gap=8.0
              if available_width < 360.0
                text "Compact responsive view" size=12.0 @text-muted
              if available_width >= 360.0
                row gap=8.0
                  text "Wide" size=12.0 @text-muted
                  text available_height size=12.0 @text-muted
                  text observed_width size=12.0 @text-muted
                  text observed_height size=12.0 @text-muted
        float
          with
            scale=1.02
            x=(viewport_width - original_width)
            y=-1.0
            shadow=black/50
            shadow-y=2.0
            shadow-blur=4.0
            r=4.0
          text "Floating label" size=12.0 @text-fg
        pin
          with
            w=fill
            h=28.0
            x=4.0
            y=4.0
          text "Pinned label" size=12.0 @text-muted
        pin
          with
            w=fill(2)
            h=shrink
            x=8.0
            y=6.0
          text "Pinned with flexible bounds" size=12.0 @text-muted
        radio "List" -> view_mode_changed _
          with
            value=0
            selected=(view_mode == 0)
            style=view_radio(loading)
        radio "Board" value=1 selected=(view_mode == 1) -> view_mode_changed _
        for mode in display_modes
          text mode size=12.0 @text-muted
        resize-handle drag=render_coverage_resized cursor=resize-horizontal
          box
            with
              w=8.0
              h=24.0
              @bg-primary
              @rounded-md
            text "Resize handle" size=1.0 @text-primary
        grid
          with
            cols=2
            h=shrink
            gap=4.0
            @w-full
          text "Even" size=12.0 @text-muted
          text "height" size=12.0 @text-muted
        grid
          with
            cols=1
            h=fill
            @w-full
          text "Fill height" size=12.0 @text-muted
        grid
          with
            cols=1
            h=fill(2)
            @w-full
          text "Fill portion height" size=12.0 @text-muted
        grid
          with
            cols=1
            h=24.0
            @w-full
          text "Fixed height" size=12.0 @text-muted
        space w=fill(2) h=8.0
        stack
          with
            clip=true
            w=fill
            h=shrink
            under=1
            @p-4
            @bg-bg
            @rounded-lg
          text "Stack underlay" size=14.0 @text-muted
          text "Stack base" size=14.0 @text-muted
          text "Stack overlay" size=14.0 @text-fg

    panes #nested_workspace w=fill h=180.0 gap=4.0 resize=4.0 drag style=workspace_panes(loading)
      style
        picked-split w=4.0
      split workspace_root vertical ratio=0.65
        pane nested_files
          text "Nested files" size=14.0 @text-muted
        split editor_stack horizontal ratio=0.6
          pane nested_editor
            col gap=8.0
              button "Open nested preview" -> open_nested_preview
              button "Resize editor split" -> resize_nested_editor
              button "Open task pane" -> open_task_pane
              button "Open mode pane" -> open_mode_pane
          pane nested_terminal
            text "Nested terminal" size=14.0 @text-muted
      pane nested_preview closed
        col gap=8.0
          text "Dynamic preview" size=14.0 @text-fg
          button "Close nested preview" -> close_nested_preview
      pane pane_task in tasks by=pane_task.id maximized=task_pane_maximized
        title
          text pane_task.title size=14.0 @text-fg
        controls
          row gap=8.0
            button "Maximize" -> maximize_task_pane
            button "Close" -> close_task_pane pane_task.id
        col gap=8.0
          if task_pane_maximized
            text "Maximized task pane" size=14.0 @text-fg
          TaskRow task=pane_task loading=loading
            events
              toggle -> toggle _ _
      pane mode_pane in display_modes by=mode_pane
        title
          text mode_pane size=14.0 @text-fg
        controls
          button "Close" -> close_mode_pane mode_pane
        text "String-keyed runtime pane" size=14.0 @text-muted

    scroll #task-list
      with
        dir=vertical
        w=fill
        h=fill
        bar=visible
        bar-w=8.0
        bar-m=2.0
        scroller-w=6.0
        bar-gap=2.0
        anchor-y=start
        auto=true
        viewport=task_list_scrolled
        style=task_scroll(loading)
      keyed task in tasks by=task.id
        with
          w=fill
          h=shrink
          gap=8.0
          p=4.0
          pl=8.0
          max-w=720.0
          align=center
        TaskRow task=task loading=loading
          events
            toggle -> toggle _ _
      active y-disabled=false
        box bg=bg text=fg border=border border-w=1.0 r=8.0 shadow=black/25 shadow-y=2.0 shadow-blur=4.0 px-snap=true
        x-rail bg=surface border=border border-w=1.0 r=4.0
        x-scroller bg=primary border=fg border-w=1.0 r=4.0
        y-rail bg=surface border=border border-w=1.0 r=4.0
        y-scroller bg=primary border=fg border-w=1.0 r=4.0
        gap bg=bg
        auto bg=surface border=primary border-w=1.0 r=999.0 shadow=black/50 shadow-y=2.0 shadow-blur=4.0 icon=fg
      hovered y-hovered=true y-disabled=false
        y-scroller bg=fg
      dragged y-dragged=true y-disabled=false
        y-scroller bg=danger

    table task in tasks
      with
        w=fill
        p=4.0
        px=8.0
        py=6.0
        sep=1.0
        sep-x=2.0
        sep-y=1.0
      col w=fill align-x=left align-y=center
        header
          text "Task" @font-bold text-fg
        cell
          text task.title size=14.0 @text-fg
      col w=120.0 align-x=center align-y=center
        header
          text "Done" @font-bold text-fg
        cell
          checkbox "Complete" checked=task.done disabled=loading -> toggle(task.id, _)

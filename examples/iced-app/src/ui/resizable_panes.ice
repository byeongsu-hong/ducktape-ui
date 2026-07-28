app ResizablePanes

theme contract AppTheme
  bg
  surface
  fg
  muted
  border
  primary
  danger
palette app for AppTheme
  bg #0f172a
  surface #111827
  fg #f8fafc
  muted #94a3b8
  border #334155
  primary #7c3aed
  danger #dc2626

state
  left_width = 240.0
  dragging = false

on drag_started
  dragging = true

on drag_ended
  dragging = false

on divider_dragged(dx, dy)
  return if dx < 0.0 && left_width + dx < 160.0
  left_width = left_width + dx

view
  row w=fill h=fill
    box
      with
        w=left_width
        h=fill
        bg=surface
        p=12.0
      text "Sidebar" size=14.0 @text-fg
    resize-handle
      with
        drag=divider_dragged
        press=drag_started
        release=drag_ended
        cursor=resize-horizontal
      box
        with
          w=6.0
          h=fill
          bg=border
        text ""
    box
      with
        w=fill
        h=fill
        bg=bg
        p=12.0
      col gap=8.0
        text "Main" size=14.0 @text-fg
        text left_width size=12.0 @text-muted
        if dragging
          text "Dragging divider" size=12.0 @text-muted

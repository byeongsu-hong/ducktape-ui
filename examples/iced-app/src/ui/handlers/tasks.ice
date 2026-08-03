use "task_crud.ice"

on open_about
  about_open = true

on open_child
  task window open child -> child_opened _

on child_opened(id)
  child_window = some(id)
  task window size target=id -> child_sized _ _

on child_sized(width, height)
  child_width = width
  child_height = height

on read_raw_window_id
  task window raw-id -> raw_window_id_read _

on set_window_icon
  task window icon bytes(ff 00 00 ff 00 ff 00 ff) 2 1

on inspect_window_handle
  task window describe_window("main") -> window_handle_read _

on window_handle_read(value)
  raw_window_id = value

on raw_window_id_read(value)
  raw_window_id = value

on capture_window
  task window screenshot -> window_captured _

on window_captured(value)
  window_snapshot = rgba(value.size.width, value.size.height, value.rgba)
  snapshot_ready = true
  snapshot_width = value.size.width
  snapshot_height = value.size.height
  snapshot_scale = value.scale_factor

on about_toggled(next)
  about_open = next

on detail_mode_changed(next)
  detail_mode = next

on close_about
  about_open = false

on about_link(_url)

on pane_clicked(_name)

on canvas_button(_button)

on canvas_key(_value)

on shader_hovered(_active)

on maximize_details
  pane #workspace maximize details

on restore_workspace
  pane #workspace restore

on swap_workspace
  pane #workspace swap tasks details

on move_details_left
  pane #workspace move details left

on open_preview
  pane #workspace split details preview horizontal ratio=0.35

on close_preview
  pane #workspace close preview

on resize_workspace
  pane #workspace resize 0.5

on drop_details
  pane #workspace drop details tasks center

on close_details
  pane #workspace close details

on inspect_workspace
  pane #workspace maximized -> pane_observed _

on inspect_adjacent
  pane #workspace adjacent tasks right -> pane_observed _

on pane_observed(_name)

on appearance_changed(title, theme, bg, fg, scale)
  window_title = title
  app_theme = theme
  app_background = bg
  app_text = fg
  ui_scale = scale

on palette_switched
  active_palette = AppTheme.app

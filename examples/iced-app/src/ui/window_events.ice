app WindowEvents

use "themes/monochrome.ice"

state
  listen_frames = true
  last_window:window-id? = none

on frame

on opened(id, x, y, width, height)

on closed(id)

on moved(id, x, y)

on resized(id, width, height)

on rescaled(id, scale)

on close_requested(id)

on focused(id)
  last_window = some(id)

on unfocused(id)

on file_hovered(id, path)

on file_dropped(id, path)

on files_hovered_left(id)

subscribe
  window frame when listen_frames -> frame
  window opened with-id -> opened _ _ _ _ _
  window closed with-id -> closed _
  window moved with-id status=captured -> moved _ _ _
  window resized with-id -> resized _ _ _
  window rescaled with-id -> rescaled _ _
  window close-request with-id -> close_requested _
  window focused with-id -> focused _
  window unfocused with-id -> unfocused _
  window file-hovered with-id -> file_hovered _ _
  window file-dropped with-id -> file_dropped _ _
  window files-hovered-left with-id -> files_hovered_left _

view
  text "Window events compile fixture"

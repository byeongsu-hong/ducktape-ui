app WindowEvents

use "themes/monochrome.ice"

state
  listen_frames = true
  last_window:window-id? = none

on frame

on opened(_id, _x, _y, _width, _height)

on closed(_id)

on moved(_id, _x, _y)

on resized(_id, _width, _height)

on rescaled(_id, _scale)

on close_requested(_id)

on focused(id)
  last_window = some(id)
  listen_frames = true

on unfocused(_id)
  listen_frames = false

on file_hovered(_id, _path)

on file_dropped(_id, _path)

on files_hovered_left(_id)

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

test boot_defaults
  expect listen_frames
  expect last_window == none

view
  text "Window events compile fixture"

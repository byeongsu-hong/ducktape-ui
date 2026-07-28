app CanvasEvents

use "themes/monochrome.ice"

on ime_opened
on ime_preedit(value, start, end)
on ime_commit(value)
on key_press(value)
on key_release(value)
on modifiers(value)
on mouse_entered
on mouse_left
on mouse_moved(x, y)
on mouse_pressed(button)
on mouse_released(button)
on mouse_wheel(x, y, pixels)
on touch_pressed(id, x, y)
on touch_moved(id, x, y)
on touch_lifted(id, x, y)
on touch_lost(id, x, y)
on window_opened(x, y, width, height)
on window_closed
on window_moved(x, y)
on window_resized(width, height)
on window_rescaled(scale)
on window_focused
on window_unfocused
on file_hovered(path)
on file_dropped(path)
on files_left

view
  canvas
    with
      w=fill
      h=120.0
      capture=true
      cursor=(cursor_state)
      cursor-outside=true
    state
      cursor_state = "crosshair"
      move_count = 0
    event input-method opened -> ime_opened
    event input-method preedit -> ime_preedit _ _ _
    event input-method commit -> ime_commit _
    capture input-method closed
    event keyboard press -> key_press _
    event keyboard release -> key_release _
    event keyboard modifiers -> modifiers _
    event mouse entered -> mouse_entered
    event mouse left -> mouse_left
    event mouse moved as x, y
      set move_count = move_count + 1
      emit mouse_moved x y
    event mouse pressed as button
      set cursor_state = "grabbing"
      emit mouse_pressed button
      capture
    event mouse released as button
      set cursor_state = "crosshair"
      emit mouse_released button
      capture
    event mouse wheel -> mouse_wheel _ _ _
    event touch pressed -> touch_pressed _ _ _
    event touch moved -> touch_moved _ _ _
    event touch lifted -> touch_lifted _ _ _
    event touch lost -> touch_lost _ _ _
    redraw window frame after=16ms
    event window opened -> window_opened _ _ _ _
    event window closed -> window_closed
    event window moved -> window_moved _ _
    event window resized -> window_resized _ _
    event window rescaled -> window_rescaled _
    redraw window close-request
    event window focused -> window_focused
    event window unfocused -> window_unfocused
    event window file-hovered -> file_hovered _
    event window file-dropped -> file_dropped _
    event window files-hovered-left -> files_left
    rect x=0.0 y=0.0 w=canvas_width h=canvas_height fill=bg
    text move_count x=8.0 y=20.0 color=fg size=14.0

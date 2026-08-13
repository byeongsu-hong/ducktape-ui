app ButtonStatusChildren

theme contract AppTheme
  bg
  fg
  primary
  danger
  muted
  glyph_disabled
  danger_dim
palette app for AppTheme
  bg #101010
  fg #ffffff
  primary #333333
  danger #ff4020
  muted #888888
  glyph_disabled #443322
  danger_dim #66201a

state
  locked = true

on noop

view
  col #root gap=16.0 p=24.0
    button #icon-live label="Favorite" p=12.0 -> noop
      svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M2 2h12v12H2z'/></svg>" #live-glyph memory color=inherit w=16.0 h=16.0
      active text=muted
      hovered text=fg
    button #icon-locked label="Favorite" disabled=locked p=12.0 -> noop
      svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M2 2h12v12H2z'/></svg>" #locked-glyph memory color=inherit w=16.0 h=16.0
      active text=muted
      disabled text=glyph_disabled
    button #delete-live label="Delete" -> noop
      text "Delete" #delete-live-label @text-danger disabled:text-danger_dim
    button #delete-locked label="Delete" disabled=locked -> noop
      text "Delete" #delete-locked-label @text-danger disabled:text-danger_dim

// The svg wears `color=inherit`: its ink is the BUTTON's status-resolved text
// color. Hovering anywhere on the button's plate — the probe point sits in
// the button's padding, strictly LEFT of the glyph's own bounds — must
// brighten the glyph, and moving off the plate must restore it. The disabled
// button's glyph reads the `disabled` status arm, and hovering a disabled
// plate changes nothing: a disabled button has no hovered status.
test svg_child_ink_keys_on_button_status
  viewport 400 300
  target root = #root
  target icon_live = root/icon-live
  target live_glyph = icon_live/live-glyph
  target icon_locked = root/icon-locked
  target locked_glyph = icon_locked/locked-glyph
  expect live_glyph.x > icon_live.x + 4.0
  expect live_glyph.image_color == color.rgb8(136, 136, 136)
  move (icon_live.x + 4.0) icon_live.center_y
  expect live_glyph.image_color == color.rgb8(255, 255, 255)
  move (icon_live.x - 8.0) icon_live.center_y
  expect live_glyph.image_color == color.rgb8(136, 136, 136)
  expect locked_glyph.image_color == color.rgb8(68, 51, 34)
  hover icon_locked
  expect locked_glyph.image_color == color.rgb8(68, 51, 34)

// The text child carries an explicit `@text-danger` and still follows the
// button's disabled ramp through its `disabled:text-*` arm — no mount
// parameter involved. Both sides: the live button keeps the explicit ink,
// the disabled one wears the disabled arm.
test explicit_text_child_follows_disabled_ramp
  viewport 400 300
  target root = #root
  target delete_live = root/delete-live
  target live_label = delete_live/delete-live-label
  target delete_locked = root/delete-locked
  target locked_label = delete_locked/delete-locked-label
  expect live_label.text_color == color.rgb8(255, 64, 32)
  expect locked_label.text_color == color.rgb8(102, 32, 26)

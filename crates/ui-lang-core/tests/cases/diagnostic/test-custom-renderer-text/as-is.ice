app Demo
  renderer crate::Renderer
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344

test unsupported_text_search
  expect text "hello"

view
  text "hello"

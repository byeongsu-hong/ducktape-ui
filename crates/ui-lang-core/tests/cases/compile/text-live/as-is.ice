app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  status = "Saved"
view
  col
    text status live=polite
    text "Session expired" live=assertive
    text "Quiet"

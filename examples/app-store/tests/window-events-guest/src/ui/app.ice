app WindowEvents
extern crate::data
  pure display(value:bool) -> str
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #333333
  danger #ff0000
state
  enabled = true
  modal = false
  generic_events = 0
  focus_count = 0
  blur_count = 0
  close_requests = 0
  closes = 0
  hover_path = ""
  drop_path = ""
  hover_left = 0
  ime_opens = 0
  ime_closes = 0
  preedits = 0
  commits = 0
  captured_commits = 0
  composition = ""
  committed = ""
  range_matches = false
  draft = ""
on observed(_event)
  generic_events = generic_events + 1
on focus
  focus_count = focus_count + 1
on blur
  blur_count = blur_count + 1
on closing
  close_requests = close_requests + 1
on closed
  closes = closes + 1
  task window focus
on hovering(path)
  hover_path = path
on dropped(path)
  drop_path = path
on left
  hover_left = hover_left + 1
on ime_opened
  ime_opens = ime_opens + 1
on ime_closed
  ime_closes = ime_closes + 1
on preedit(text, start, end)
  preedits = preedits + 1
  composition = text
  range_matches = start == some(0) && end == some(6)
on commit(text)
  commits = commits + 1
  committed = text
on captured(_text)
  captured_commits = captured_commits + 1
on disable
  enabled = false
on show
  modal = true
on hide
  modal = false
subscribe
  event status=any when enabled -> observed _
  window focused when enabled -> focus
  window unfocused when enabled -> blur
  window close-request when enabled -> closing
  window closed when enabled -> closed
  window file-hovered when enabled -> hovering _
  window file-dropped when enabled -> dropped _
  window files-hovered-left when enabled -> left
  input-method opened status=any when enabled -> ime_opened
  input-method closed status=any when enabled -> ime_closed
  input-method preedit status=any when enabled -> preedit _ _ _
  input-method commit status=any when enabled -> commit _
  input-method commit status=captured when enabled -> captured _
view
  overlay #modal when=modal dismiss=hide
    content
      col gap=4.0
        input "Draft" #draft-input <-> draft
        text draft #draft
        text generic_events #events
        text focus_count #focus
        text blur_count #blur
        text close_requests #closing
        text closes #closed
        text hover_path #hover-path
        text drop_path #drop-path
        text hover_left #hover-left
        text ime_opens #ime-opens
        text ime_closes #ime-closes
        text preedits #preedits
        text commits #commits
        text captured_commits #captured-commits
        text composition #composition
        text committed #committed
        text display(range_matches) #range
        button "Disable observations" #disable -> disable
        button "Open modal" #show -> show
    layer
      box
        with
          w=260.0
          h=120.0
          bg=bg
        col gap=8.0
          input "Modal draft" #modal-input <-> draft
          button "Close modal" -> hide

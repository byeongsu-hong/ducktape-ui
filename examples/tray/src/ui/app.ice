daemon Tray
  title "Ice Tray"
  id "dev.ducktape.ice.tray"
  font "../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../assets/fonts/Geist-Bold.ttf"
  font "../../../../assets/fonts/GeistMono-Regular.ttf"
  text-size 13
  tray
    icon-rgba "icon.rgba" 22 22
    icon-template true
    label clock(remaining)
    tooltip "Focus timer"
    popover panel
  window panel
    size 268 196
    decorations false
    resizable false
    level always-on-top

use "theme.ice"

font sans family="Geist" default=true
font digits family="Geist Mono"

extern crate::timer
  pure clock(seconds:i64) -> str
  pure elapsed_width(remaining:i64, width:f64) -> f64
  pure remaining_width(remaining:i64, width:f64) -> f64
  pure phase(running:bool, remaining:i64) -> str
  pure minute_label(minutes:i64) -> str

component Choice(minutes:i64, session:i64)
  emits
    pick(i64)
  col #root w=fill
    if session == minutes * 60
      button #on -> emit(pick, minutes)
        with
          label=minute_label(minutes)
          w=fill
          p=8.0
        active bg=primary text=bg r=6.0
        hovered bg=primary_lift text=bg r=6.0
        text minute_label(minutes)
          with
            size=12.0
            w=fill
            align-x=center
            font=digits
    if session != minutes * 60
      button #off -> emit(pick, minutes)
        with
          label=minute_label(minutes)
          w=fill
          p=8.0
        active bg=surface text=muted r=6.0
        hovered bg=rail text=fg r=6.0
        text minute_label(minutes)
          with
            size=12.0
            w=fill
            align-x=center
            font=digits
            @text-muted

state
  remaining = 1500
  running = false
  page = "timer"
  session = 1500

preset midway
  state
    remaining = 1062
    running = true

on start
  running = !running

on reset
  running = false
  remaining = 1500

on tick
  remaining = remaining - 1
  running = remaining > 0

on show(next)
  page = next

on choose(minutes)
  session = minutes * 60
  remaining = minutes * 60
  running = false
  page = "timer"
  task tray close

on quit
  exit

subscribe
  every 1s when running -> tick

view
  // `popover` is true only for the window the status item opened. This daemon
  // declares no other window, so the panel is the whole program.
  box #panel
    with
      w=fill
      h=fill
      bg=bg
    col
      with
        w=fill
        h=fill
        p=20.0
        gap=0.0
      row
        with
          w=fill
          gap=8.0
          align=center
        if running
          box
            with
              w=6.0
              h=6.0
              r=3.0
              bg=primary
            space w=fill h=fill
        if !running
          box
            with
              w=6.0
              h=6.0
              r=3.0
              bg=rail
            space w=fill h=fill
        text "FOCUS"
          with
            size=10.0
            tracking=1.6
            @text-muted
        space w=fill
        if page == "timer"
          text phase(running, remaining)
            with
              size=10.0
              tracking=1.1
              @text-faint
        if page == "timer"
          button #settings p=2.0 label="Session length" -> show("settings")
            active bg=bg text=faint r=4.0
            hovered bg=surface text=fg r=4.0
            text "•••" size=10.0
        if page != "timer"
          text "SETTINGS"
            with
              size=10.0
              tracking=1.1
              @text-faint
      if page == "timer"
        space h=10.0
        if running
          text clock(remaining)
            with
              size=46.0
              w=fill
              align-x=center
              font=digits
              @text-primary
        if !running
          text clock(remaining)
            with
              size=46.0
              w=fill
              align-x=center
              font=digits
              @text-fg
        space h=6.0
        text "25 minute session"
          with
            size=10.0
            w=fill
            align-x=center
            @text-faint
        space h=16.0
        row w=fill h=2.0
          box
            with
              w=elapsed_width(remaining, 228.0)
              h=2.0
              bg=primary
            space w=fill h=fill
          box
            with
              w=remaining_width(remaining, 228.0)
              h=2.0
              bg=rail
            space w=fill h=fill
        space h=fill
        row
          with
            w=fill
            gap=8.0
            align=center
          if !running
            button #start p=9.0 label="Start the timer" -> start
              active bg=primary text=bg r=6.0
              hovered bg=primary_lift text=bg r=6.0
              text "Start"
                with
                  size=12.0
                  w=64.0
                  align-x=center
          if running
            button #pause p=9.0 label="Pause the timer" -> start
              active bg=surface text=fg r=6.0
              hovered bg=rail text=fg r=6.0
              text "Pause"
                with
                  size=12.0
                  w=64.0
                  align-x=center
          button #reset p=9.0 label="Reset to 25 minutes" -> reset
            active bg=bg text=faint r=6.0
            hovered bg=surface text=fg r=6.0
            text "Reset" size=12.0
          space w=fill
          button #quit p=9.0 label="Quit" -> quit
            active bg=bg text=faint r=6.0
            hovered bg=surface text=danger r=6.0
            text "Quit" size=12.0

      if page == "settings"
        space h=10.0
        text "Session length"
          with
            size=12.0
            w=fill
            @text-muted
        space h=12.0
        row w=fill gap=8.0
          Choice minutes=15 session=session #choice-15
            events
              pick -> choose _
          Choice minutes=25 session=session #choice-25
            events
              pick -> choose _
          Choice minutes=45 session=session #choice-45
            events
              pick -> choose _
        space h=fill
        row
          with
            w=fill
            gap=8.0
            align=center
          button #back p=9.0 label="Back to the timer" -> show("timer")
            active bg=surface text=fg r=6.0
            hovered bg=rail text=fg r=6.0
            text "Back"
              with
                size=12.0
                w=64.0
                align-x=center
          space w=fill
test tray_panel_opens_from_the_status_item
  viewport 268 196
  tray click
  expect text "FOCUS"
  expect text "25:00"
  expect text "Start"
  capture panel

test tray_panel_draws_the_run_it_has_spent
  preset midway
  viewport 268 196
  tray click
  expect text "17:42"
  capture midway

test tray_panel_starts_and_pauses_the_timer
  viewport 268 196
  target panel = #panel
  target start = panel/start
  tray click
  click start
  expect running
  expect text "Pause"
  capture running

test tray_panel_walks_to_the_session_page_and_back
  viewport 268 196
  target panel = #panel
  target settings = panel/settings
  target choice = panel/choice-45/root
  target pick = choice/off
  tray click
  click settings
  expect page == "settings"
  expect text "Session length"
  capture settings
  click pick
  expect session == 2700
  expect page == "timer"
  expect text "45:00"

app Starter
  title "Ice starter"
  id "dev.ducktape.ice.starter"
  text-size 16
  window
    size 640 480
    min-size 480 360
    position centered

theme contract StarterTheme
  bg
  fg
  primary
  danger
  surface
  muted
  primary_fg

palette starter for StarterTheme
  bg #f5f7fb
  fg #172033
  primary #315efb
  danger #c93445
  surface #ffffff
  muted #667085
  primary_fg #ffffff

state
  name = "World"
  count = 0

on increment
  count = count + 1

view
  box #app
    with
      w=fill
      h=fill
      bg=bg
      p=32.0
      align-x=center
      align-y=center
    col #content
      with
        w=fill
        gap=16.0
        align=center
      text "Ice starter" size=30.0 @text-fg
      text name #greeting size=20.0 @text-muted
      input "Your name" #name <-> name w=320.0
      row gap=12.0 align=center
        button "Increment" #increment -> increment
          active bg=primary text=primary_fg r=8.0
          hovered bg=primary/90 text=primary_fg r=8.0
          pressed bg=primary/80 text=primary_fg r=8.0
        text count #count @text-fg

test starter_flow
  viewport 640 480
  target app = #app
  target name_input = #app/content/name
  target increment_button = #app/content/increment
  expect app.width ~= 640.0
  expect name_input.value == "World"
  expect a11y increment_button name "Increment"
  click increment_button
  expect count == 1
  capture ready

app WidgetFixture
extern crate::bridge
  task canceled_focus() -> unit
theme contract AppTheme
  bg
  fg
  primary
  danger
  green
  blue
  yellow
  magenta
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
  green #00ff00
  blue #0000ff
  yellow #ffff00
  magenta #ff00ff
font ui family="Geist" default=true
recipe focus_action for button
  @px-4 py-2 font-semibold bg-primary text-fg rounded-8px hover:bg-primary disabled:opacity-50 focus-visible:border-danger
state
  first = "abcd"
  second = "other"
  focused = false
  pulses = 0
on mount
  task widget focus #first
on focus_first
  sequential
    task widget focus #first
    task widget focused #first -> queried _
on focus_second
  task widget focus #second
on next
  task widget focus-next
on previous
  task widget focus-prev
on query
  task widget focused #first -> queried _
on queried(value)
  focused = value
on select_range
  sequential
    task widget focus #first
    task widget select #first 1 3
on cursor_front
  sequential
    task widget focus #first
    task widget cursor-front #first
on cursor_end
  sequential
    task widget focus #first
    task widget cursor-end #first
on cursor_at
  sequential
    task widget focus #first
    task widget cursor #first 2
on select_all
  sequential
    task widget focus #first
    task widget select-all #first
on snap
  task widget snap #list 0.0 0.5
on snap_end
  task widget snap-end #list
on scroll_to
  task widget scroll-to #list 0.0 100.0
on scroll_by
  task widget scroll-by #list 0.0 -24.0
on canceled_focus
  task canceled_focus() -> ignored
on ignored
on pulse
  pulses = pulses + 1
subscribe
  every 16ms -> pulse
view
  col
    row
      button #ink -> ignored
        with
          label="Ink"
          w=80.0
          h=48.0
          p=0.0
        row gap=8.0
          svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><rect width='16' height='16'/></svg>" memory
            with
              w=16.0
              h=16.0
              color=inherit
          svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><rect width='16' height='16'/></svg>" memory
            with
              w=16.0
              h=16.0
              color=magenta
        active text=danger
        hovered text=green
        pressed text=blue
      button #disabled-ink -> ignored
        with
          label="Disabled ink"
          w=48.0
          h=48.0
          p=0.0
          disabled=true
        svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><rect width='16' height='16'/></svg>" memory
          with
            w=16.0
            h=16.0
            color=inherit
        disabled text=yellow
    input "First" #first <-> first
    input "Second" #second <-> second
    button "Cancel focus" #cancel-focus -> canceled_focus
    button "Focus first" #focus-first -> focus_first
    button "Focus second" #focus-second -> focus_second
    button "Next" #next -> next
    button "Previous" #previous -> previous
    button "Query" #query -> query
      with
        checked=focused
        expanded=focused
        description="Reports input focus"
        w=160.0
        h=40.0
        @focus_action
    button "Select range" #select-range -> select_range
    button "Cursor front" #cursor-front -> cursor_front
    button "Cursor end" #cursor-end -> cursor_end
    button "Cursor at" #cursor-at -> cursor_at
    button "Select all" #select-all -> select_all
    button "Snap" #snap -> snap
    button "Snap end" #snap-end -> snap_end
    button "Scroll to" #scroll-to -> scroll_to
    button "Scroll by" #scroll-by -> scroll_by
    scroll #list h=100.0 w=fill
      col
        text "Start" @text-fg
        space h=600.0
        text "End" @text-fg
    if focused
      text "focused" #focused @text-fg
    if !focused
      text "unfocused" #unfocused @text-fg
    text pulses #pulses @text-fg

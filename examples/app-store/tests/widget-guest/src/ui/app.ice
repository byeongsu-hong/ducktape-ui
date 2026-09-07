app WidgetFixture
extern crate::bridge
  task canceled_focus() -> unit
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
    input "First" #first <-> first
    input "Second" #second <-> second
    button "Cancel focus" #cancel-focus -> canceled_focus
    button "Focus first" #focus-first -> focus_first
    button "Focus second" #focus-second -> focus_second
    button "Next" #next -> next
    button "Previous" #previous -> previous
    button "Query" #query -> query
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

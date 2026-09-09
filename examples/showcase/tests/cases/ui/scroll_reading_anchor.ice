app ScrollReadingAnchor
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

extern crate::backend
  State()
  Event()
  Transition(state:State)
  sync initial() -> State
  pure items(rows:[i64]) -> Event
  pure prepend(rows:[i64]) -> [i64]
  pure remove_first(rows:[i64]) -> [i64]
  pure remove_visible(rows:[i64], state:&State) -> [i64]
  sync apply(state:State, event:Event) -> Transition
  task effects(transition:Transition) -> Event
  component transcript(rows:&[i64], state:&State) -> Event

state
  scroller:State = initial()
  rows = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
  version = 0

on mount
  let transition = apply(scroller, items(rows))
  scroller = transition.state
  task effects(transition) -> event _

on event(next)
  let transition = apply(scroller, next)
  scroller = transition.state
  task effects(transition) -> event _

on prepend
  rows = prepend(rows)
  version = version + 1
  let transition = apply(scroller, items(rows))
  scroller = transition.state
  task effects(transition) -> event _

on remove
  rows = remove_first(rows)
  version = version + 1
  let transition = apply(scroller, items(rows))
  scroller = transition.state
  task effects(transition) -> event _

on delete_visible
  rows = remove_visible(rows, scroller)
  let transition = apply(scroller, items(rows))
  scroller = transition.state
  task effects(transition) -> event _

view
  Page #page
    col w=fill h=fill gap=12.0
      row gap=8.0
        button "Prepend" #prepend @secondary_action -> prepend
        button "Remove first" #remove @secondary_action -> remove
        button "Delete visible" #delete-visible @secondary_action -> delete_visible
      extern transcript(rows, scroller) #transcript -> event _

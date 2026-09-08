app Transactions
  title "Editor transactions fixture"
  id "dev.ice.editor-transactions-fixture"

extern crate::fixture
  History(undo:[bytes], redo:[bytes], commits:i64, ordered:bool, accepted:bool, last_sequence:i64, last_kind:str)
  Outcome(before:bytes, after:bytes, sequence:i64, kind:str, history:str)
  pure initial_history() -> History
  pure record(history:History, outcome:Outcome, document:editor) -> History
  editor-binding keys(history:History) -> Outcome

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #ff0000
  danger #ff00ff

state
  draft:editor = editor("ab")
  history:History = initial_history()
on committed(outcome)
  history = record(history, outcome, draft)

component IndependentEditor()
  lifetime retained
  state
    draft:editor = editor("")
    history:History = initial_history()
  on committed(outcome)
    history = record(history, outcome, draft)
  editor #local <-> draft key-binding=keys(history) -> committed _
    with
      w=120.0
      min-h=30.0
      max-h=30.0
      size=12.0
    active bg=bg value=fg selection=primary

view
  col w=260.0 gap=8.0
    editor #document <-> draft key-binding=keys(history) -> committed _
      with
        w=260.0
        min-h=100.0
        max-h=140.0
        size=16.0
        p=8.0
      active bg=bg value=fg selection=primary
    text editor_text(draft) #echo
    text history.commits #commits
    if history.accepted
      text "state accepted before route" #accepted
    if !history.accepted
      text "route preceded state" #rejected
    if history.ordered
      text "commits ordered" #ordered
    if !history.ordered
      text "duplicate or reordered commit" #out-of-order
    row
      IndependentEditor #left
      IndependentEditor #right
    editor #same-document <-> draft
      with
        w=120.0
        min-h=30.0
        max-h=30.0
        size=12.0
      active bg=bg value=fg selection=primary

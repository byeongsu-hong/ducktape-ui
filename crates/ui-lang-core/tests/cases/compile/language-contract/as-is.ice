app Contract
  palette active_palette
theme contract ProductTheme
  bg
  fg
  primary
  danger
palette light for ProductTheme
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
palette dark for ProductTheme
  bg #111111
  fg #ffffff
  primary #88aaff
  danger #ff6677
recipe action for button
  @p-4
recipe primary_action for button extends action
  @bg-primary
enum Selection
  idle
  page(str)
state
  active_palette:palette[ProductTheme] = ProductTheme.light
  selection:Selection = Selection.idle
  draft = "Draft"
component Field(bind value:str)
  emits
    commit(Selection)
  col
    input "Value" <-> value
    button "Apply" @primary_action -> emit(commit, Selection.page(value))
    if provided(Footer)
      slot Footer?
component Shell(bind value:str)
  emits
    commit(Selection)
  Field value<->value
    forward
      commit
on committed(next)
  selection = next
view
  col
    Shell value<->draft
      events
        commit -> committed _
    match selection
      Selection.idle
        text "Idle"
      Selection.page(page)
        text page

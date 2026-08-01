app ViewHir
extern crate::backend
  Item(id:i64, name:str)
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
  prefix = "Ice"
  items:[Item] = []
  choice:str? = some("ready")
component Card(title:str)
  col
    text title
    if provided(Footer)
      slot Footer?
view
  col
    Card title=prefix
    for row in items
      text row.name
    match choice
      some(label)
        text label
      none
        text "none"
    keyed keyed_row in items by=keyed_row.id
      text keyed_row.name
    lazy choice as cached
      text "cached"
    table table_row in items
      col
        header
          text "Name"
        cell
          text table_row.name
    panes #work
      pane files maximized=files_maximized
        col
          if files_maximized
            text "files"
      pane pane_item in items by=pane_item.id maximized=pane_maximized
        col
          if pane_maximized
            text pane_item.name
    responsive size=(available_width, available_height)
      col
        if available_width < available_height
          text "portrait"

extern crate::data
  pure many_rows() -> [i64]

app KeyedFixture
  title "Keyed fixture"
  id "dev.ice.keyed-fixture"

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #00aa44
  danger #ff0000

state
  rows = [1, 2, 3]
  use_plain = false
on many
  rows = many_rows()
on jump
  task widget scroll-to-key #list 150
on missing
  task widget scroll-to-key #list 999
on ordinary
  use_plain = true
on reorder
  rows = [2, 3, 1]
on prepend
  rows = [0, 1, 2, 3]
on remove
  rows = [1, 3]

component Row(number:i64)
  state
    draft = ""
  col gap=4.0
    text number #number
    input "Row draft" #draft <-> draft

view
  col
    with
      w=fill
      h=fill
      gap=8.0
    row gap=8.0
      button "Reorder" #reorder -> reorder
      button "Prepend" #prepend -> prepend
      button "Remove" #remove -> remove
      button "Ordinary" #ordinary -> ordinary
      button "Many" #many -> many
    row
      button "Jump" #jump -> jump
      button "Missing" #missing -> missing
    if use_plain
      keyed number in rows by=number #ordinary_entries w=fill gap=4.0
        Row number=number #ordinary_row
    if !use_plain
      scroll #list
        with
          w=fill
          h=200.0
          anchor-y=keep
        keyed number in rows by=number #entries
          with
            virtual-row=60.0
            w=fill
            gap=4.0
          Row number=number #row
    scroll #plain w=fill h=80.0
      col
        with
          virtual-row=24.0
          w=fill
          max-w=320.0
        for number in rows
          text number

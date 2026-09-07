app ClipboardFixture
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
  standard:str? = none
  primary:str? = none
  reads = 0
on copy
  task clipboard write "module copy"
on copy_primary
  task clipboard write-primary "module selection"
on read
  task clipboard read -> read_done _
on read_primary
  task clipboard read-primary -> primary_done _
on read_done(value)
  standard = value
  reads = reads + 1
on primary_done(value)
  primary = value
  reads = reads + 1
view
  col
    button "Copy" #copy -> copy
    button "Copy primary" #copy-primary -> copy_primary
    button "Read" #read -> read
    button "Read primary" #read-primary -> read_primary
    text reads #reads @text-fg
    match standard
      some(value)
        text value #standard @text-fg
      none
        text "standard empty" #standard-empty @text-fg
    match primary
      some(value)
        text value #primary @text-fg
      none
        text "primary empty" #primary-empty @text-fg

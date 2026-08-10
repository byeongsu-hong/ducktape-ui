app Demo
extern crate::backend
  Entry(note:str)
  Fault(message:str)
  read_wallet(phrase:secret) -> Entry ! Fault
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
  note = ""
secret phrase
on check
  return if empty(phrase)
  run every read_wallet(phrase) -> read _ | failed _
on read(entry)
  note = entry.note
  phrase = ""
on failed(fault)
  note = fault.message
view
  col
    input "Recovery phrase" #phrase <-> phrase
    button "Check" #check -> check
      with
        disabled=empty(phrase)
    text note

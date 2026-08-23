app PerfLazyLoop
extern crate::backend
  Message(rev:i64, seq:i64, body:str, tags:[str])
  Print(ts:i64, price:f64)
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
  messages:[Message] = []
  prints:[Print] = []
on messages_loaded(next)
  messages = next
on prints_loaded(next)
  prints = next
view
  col
    button "Reload" -> messages_loaded []
    button "Tape" -> prints_loaded []
    for message in messages
      lazy message as row
        text row.body
    keyed message in messages by=message.seq
      lazy message by message.rev, message.seq as row
        text row.body
    for print in prints
      lazy print as printed
        text printed.ts
    lazy messages as all
      for message in all
        text message.body

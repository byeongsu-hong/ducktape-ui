app HandlerMatch

extern crate::backend
  sync handler_match_expensive(values:[str]) -> i64

use "themes/monochrome.ice"

enum LiveKind
  chat
  tip

state
  messages:[str] = ["one", "two"]
  count = 0

on updated(kind)
  match kind
    LiveKind.chat
      count = handler_match_expensive(messages)
    LiveKind.tip
      count = 7

view
  col
    text count
    button "Chat" -> updated(LiveKind.chat)
    button "Tip" -> updated(LiveKind.tip)

extern crate::library
  Note(path:str, title:str, snippet:str, stamp:str, search:str)
  Library(home:str, notes:[Note], path:str, source:str)
  Saved(path:str, saved_revision:i64, notes:[Note])
  open_library(home:str) -> Library ! EditorError
  switch_note(home:str, path:str, source:str, revision:i64, dirty:bool, next:str) -> Library ! EditorError
  save_note(home:str, path:str, source:str, revision:i64) -> Saved ! EditorError
  flush_note(home:str, path:str, source:str, revision:i64, dirty:bool) -> unit ! EditorError
  delete_note(home:str, path:str) -> Library ! EditorError
  pure filter_notes(notes:[Note], query:str) -> [Note]
  pure selected_title(notes:[Note], path:str) -> str

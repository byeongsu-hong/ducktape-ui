// The whole Rust boundary of this window.
//
// A chat is held in Rust, because the same conversation is both what gets
// resent to the API and what gets drawn, and the two must not drift. Ice is
// handed the drawn half: an ordered list of transcript rows that only grows.
//
// A turn arrives on two channels on purpose. `codex_turn` carries text token by
// token, which is constant; `codex_entries` carries the row list, which changes
// only when a tool starts, a tool finishes, or a block settles. Putting the
// list on every token would copy the whole transcript once per token.
//
// Nothing here hands a token to Ice. The access token never leaves
// `crate::codex`.
extern crate::codex
  Session()
  Entry(id:i64, kind:str, title:str, detail:str, body:str, status:str, open:bool, dark:bool)
  Chunk(answer:str, thinking:str, status:str)
  CodexError(message:str)
  sync codex_session() -> Session
  sync codex_account() -> str
  sync codex_model() -> str
  sync push_user(session:Session, text:str) -> [Entry]
  sync set_palette(session:Session, dark:bool) -> [Entry]
  sync toggle_row(session:Session, id:i64) -> [Entry]
  pure sample_entries(dark:bool) -> [Entry]
  sync sample_session(dark:bool) -> Session
  stream codex_entries(session:Session) -> [Entry]
  sip codex_turn(session:Session) progress=Chunk -> [Entry] ! CodexError

// A settled row's Markdown, parsed once and kept against its row id. Ice keeps
// only cloneable values in component state, so a parsed document cannot live in
// the language; this adapter is where it lives. The route carries the URL of a
// clicked link.
extern crate::render
  component markdown_body(id:i64, source:str, size:f64, dark:bool) -> str

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
  sync signed_in() -> bool
  sync sign_out() -> bool
  sync codex_model() -> str
  sync codex_models() -> [str]
  sync set_model(session:Session, model:str) -> str
  sync codex_effort() -> str
  sync codex_efforts(model:str) -> [str]
  sync set_effort(session:Session, effort:str) -> str
  sync session_effort(session:Session) -> str
  sync new_chat(session:Session) -> Session
  sync push_user(session:Session, text:str) -> [Entry]
  sync set_palette(session:Session, dark:bool) -> [Entry]
  sync toggle_row(session:Session, id:i64) -> [Entry]
  sync stop_turn(session:Session) -> str
  sync queue_message(session:Session, text:str) -> str
  sync steer_turn(session:Session, text:str) -> str
  sync take_pending(session:Session) -> str
  pure sample_entries(dark:bool) -> [Entry]
  pure sample_running(dark:bool) -> [Entry]
  sync sample_session(dark:bool) -> Session
  stream codex_entries(session:Session) -> [Entry]
  sip codex_turn(session:Session) progress=Chunk -> [Entry] ! CodexError

// A settled row's Markdown, parsed once and kept against its row id. Ice keeps
// only cloneable values in component state, so a parsed document cannot live in
// the language; this adapter is where it lives. The route carries the URL of a
// clicked link.
extern crate::render
  component markdown_body(id:i64, source:str, size:f64, dark:bool) -> str
  markdown-viewer answer_viewer(dark:bool) -> str

// Signing in without leaving the window: ask the host for a short code, show
// it, and wait while it is typed into a browser. This app keeps what it is
// granted in its own file and never writes the CLI's, because a refresh here
// would rotate a token `codex` is still holding.
extern crate::auth
  DeviceCode(user_code:str, verification_uri:str, device_auth_id:str)
  begin_sign_in() -> DeviceCode ! CodexError
  finish_sign_in(code:DeviceCode) -> str ! CodexError

// Enter sends; every other way of pressing it writes a line.
extern crate::composer
  Send()
  editor-binding composer_keys() -> Send

// Chats the CLI has already had. The listing reads only each rollout's head;
// opening one streams it, because these files run to tens of megabytes.
extern crate::history
  Chat(path:str, title:str, when:str, cwd:str)
  pure sample_chats() -> [Chat]
  recent_chats() -> [Chat]
  open_recent(session:Session, path:str) -> [Entry] ! CodexError

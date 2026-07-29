extern crate::terminal
  Session()
  Environment(shell:str, directory:str, ssh_available:bool, claude_available:bool, codex_available:bool)
  Notice(running:bool, title:str)
  Started(session:Session, title:str)
  TerminalError(message:str)
  sync idle_session() -> Session
  sync detect_environment() -> Environment
  start_session(kind:str, target:str, directory:str) -> Started ! TerminalError
  task focus_terminal(session:Session) -> unit
  component terminal_surface(session:&Session) -> unit
  subscription terminal_events(session:Session) -> Notice

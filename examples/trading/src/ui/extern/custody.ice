// Custody: what the app may sign with, and what it took to get there.
//
// `Session` is opaque here on purpose. The rules that move it are a pure state
// machine in Rust with an exhaustive test suite of its own, and a copy of them
// in Ice would be a second opinion about when an order may be signed. So Ice
// holds one value, routes two acts at it, and asks it questions.
//
// The two acts are fallible for different reasons and both routes are wired: a
// declined sheet, a first run, and a build with no keychain are *states* the
// machine has, so they come back through the success route carrying the
// sentence that tells them apart — while a venue that would not say which keys
// are live is a read that failed like any other and belongs in the same alarm
// line as the rest.
extern crate::custody
  Session()
  Entry(session:Session, note:str)
  CustodyFault(message:str)
  pure session_start() -> Session
  pure lock_agent() -> Session
  pure tick_agent(session:Session, now:i64) -> Session
  // Takes the clock because the state alone cannot answer it: a window closes
  // on a schedule the exchange set, so a session that answered on its variant
  // would keep saying yes through every millisecond between expiry and the
  // next tick — and forever if the ticks stop, which is exactly when a laptop
  // that slept through the expiry starts asking.
  pure session_can_trade(session:Session, now:i64) -> bool
  pure session_badge(session:Session, now:i64) -> str
  pure session_reason(session:Session) -> str
  pure session_agent(session:Session) -> str
  pure session_account(session:Session) -> str
  pure session_window(session:Session, now:i64) -> str
  pure session_unlockable(session:Session) -> bool
  pure session_refusal(venue:Venue, session:Session) -> str
  unlock_agent(venue:Venue, address:str) -> Entry ! CustodyFault
  enrol_agent(venue:Venue, address:str) -> Entry ! CustodyFault
  // Sessions a preset can be drawn in. Each is built by driving the real state
  // machine rather than by naming a variant, so a fixture cannot be a state the
  // machine would never reach.
  pure demo_session_unenrolled() -> Session
  pure demo_session_unavailable() -> Session
  pure demo_session_unapproved() -> Session
  pure demo_session_ready(now:i64) -> Session
  pure demo_session_expired(now:i64) -> Session

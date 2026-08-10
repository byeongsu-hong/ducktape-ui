// What the terminal answers on the keyboard, as four mappings rather than four
// branches: an Ice handler cannot branch, so each of these answers the one key
// it owns and hands back what it was given for every other key. One
// subscription and one handler carry the whole scheme, and one press moves
// exactly one thing.
//
// Nothing here sends. The keys reach the confirmation and stop, and the
// subscription is off while one is standing.
extern crate::hotkeys
  Hotkey(keys:str, act:str)
  // The scheme itself, so Settings prints the list the handler answers rather
  // than a second copy of it kept by hand.
  pure hotkey_list() -> [Hotkey]
  pure hotkey_note() -> str
  pure hotkey_side(pressed:key, current:bool) -> bool
  pure hotkey_share(pressed:key) -> f64
  pure hotkey_price(pressed:key, typed:str, book:Book?) -> str

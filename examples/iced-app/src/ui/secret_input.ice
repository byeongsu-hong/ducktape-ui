// A field whose text never becomes an Ice value.
//
// The buffer behind `phrase` lives in the runtime, not in the application
// struct, so no preset constructs it, no snapshot serializes it, and no
// `expect` has a typed field to read. What Ice can do with the name is the
// whole of the feature: bind one input to it, ask it `empty` or `len`, clear
// it with `= ""`, and hand it to an extern parameter declared `secret`.
app SecretInput

extern crate::backend
  AppError(message:str)
  // The one guarded read. `derive_address` receives a
  // `ui_lang_runtime::Secret`, which is not `Clone`, prints redacted, and
  // wipes itself when the call returns.
  derive_address(phrase:secret) -> str ! AppError

theme contract AppTheme
  bg
  fg
  primary
  danger

palette app for AppTheme
  bg #111111
  fg #eeeeee
  primary #3366ff
  danger #cc3333

state
  address = ""
  note = ""

secret phrase

on derive
  return if empty(phrase)
  note = ""
  run every derive_address(phrase) -> derived _ | refused _

// The phrase has done its work, and the shortest life it can have is the one
// that ends the moment it has.
on derived(value)
  address = value
  phrase = ""

on refused(error)
  note = error.message
  phrase = ""

on forget
  address = ""
  note = ""
  phrase = ""

// The buffer is what the field draws from and what CHECK spends. Typing it and
// reading the address back proves the whole loop; that the same test cannot
// name the text is the point rather than a gap in the test.
test secret_input_derives_without_the_text_becoming_state
  viewport 480 320
  target field = #phrase
  target derive = #derive
  target shown = #address
  expect a11y derive disabled true
  expect empty(phrase)
  focus field
  replace "abandon abandon about"
  // The facts Ice is allowed: how much is in the buffer, and whether anything
  // is. Both are already on screen — the mask draws one bullet per character.
  expect !empty(phrase)
  expect len(phrase) == 21
  expect a11y derive disabled false
  click derive
  expect address == "0x3"
  expect a11y shown value "0x3"
  // And the buffer goes the instant the address exists.
  expect empty(phrase)
  expect len(phrase) == 0

// What a reader, a screenshot, and a screen reader are each told.
//
// This is the assertion the feature exists for, so it is made three ways: the
// accessibility tree carries the protected role and no value, nothing paints
// the characters, and `forget` leaves the buffer empty. The compile-time half
// of the claim — that `expect phrase == "abandon abandon about"` does not
// compile — is a `secret-read-as-text` diagnostic fixture in Core, because a
// test that cannot be written cannot be run.
test secret_input_never_shows_or_announces_what_was_typed
  viewport 480 320
  target field = #phrase
  focus field
  replace "abandon abandon about"
  expect !empty(phrase)
  expect a11y field role "password-input"
  expect a11y field name "Recovery phrase"
  // The protected role carries no value at all — not an empty one, not a
  // masked one. That absence is asserted where it is produced, by
  // `semantic_target_merging_never_exposes_password_text` in the runtime,
  // which holds that reading such a target panics and that the panic does not
  // carry the text. Here the claim is the visible half: nothing paints it.
  expect no text "abandon abandon about"
  expect no text "abandon"
  capture typed_secret
  dispatch forget
  expect empty(phrase)

view
  col gap=12.0 p=16.0
    text "Import a wallet"
    input "Recovery phrase" #phrase <-> phrase
      with
        label="Recovery phrase"
        description="Typed here and read once, when you press CHECK"
        hint="abandon abandon about"
    button "CHECK" #derive disabled=empty(phrase) -> derive
    text address #address
    text note #note

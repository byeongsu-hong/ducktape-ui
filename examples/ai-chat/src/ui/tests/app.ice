// A settled row is drawn behind `lazy`, which redraws it only when the row
// itself changes. That is why a row carries whether it is folded: state held
// anywhere else would be invisible to that boundary, and the fold would appear
// to do nothing. This test holds that design in place.
test folding_a_settled_reasoning_row_opens_it
  preset conversation
  viewport 920 800
  target thoughts = #app/transcript/rows/key(-2)/reasoning(-2)/root
  target toggle = thoughts/toggle
  expect a11y toggle name "Checking the crate before answering"
  expect thoughts.height < 30.0

  click toggle
  expect thoughts.height > 60.0

  // And it folds back, so the height follows the row rather than only ever
  // growing.
  click toggle
  expect thoughts.height < 30.0

// An empty composer must not be able to open a turn, and neither must one
// holding only spaces: the draft is trimmed before it is judged.
test the_composer_will_not_send_a_blank_draft
  preset conversation
  viewport 920 800
  target composer = #app/composer/draft
  target send = #app/composer/send
  expect a11y send disabled true

  click composer
  type "   "
  expect composer.value == "   "
  expect a11y send disabled true

  type "hello"
  expect a11y send disabled false

// Everything a turn produces reaches the screen. A chat window that draws only
// the answer is not reporting the turn.
test a_turn_draws_its_reasoning_searches_and_cost
  preset conversation
  viewport 920 800
  expect text "Checking the crate before answering"
  expect text "Searched the web"
  expect text "site:crates.io/crates/iced latest version  ·  iced-rs releases"
  expect text "Opened a page"
  expect text "https://crates.io/crates/iced"
  expect text "2,914 in · 268 out · 192 reasoning"
  capture conversation

// The whole turn, driven the way a person drives it: type into the composer,
// press Send, and let everything the turn produces land in the transcript.
// Nothing here is dispatched or preset — the widgets are the real ones and so
// is the path behind them.
test sending_a_message_runs_a_whole_turn
  preset signed_in
  viewport 920 800
  target composer = #app/composer/draft
  target send = #app/composer/send
  expect text "What are we working on?"

  click composer
  type "Which iced is newest?"
  expect a11y send disabled false

  click send
  expect composer.value == ""
  expect no text "What are we working on?"
  expect text "Which iced is newest?"
  expect text "Planning a source search"
  expect text "Searched the web"
  expect text "site:crates.io/crates/iced latest version  ·  iced-rs releases"
  expect text "Reading the changelog"
  expect text "Opened a page"
  expect text "https://raw.githubusercontent.com/iced-rs/iced/master/CHANGELOG.md"
  expect text "22,875 in · 321 out · 257 reasoning"
  capture chatted

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

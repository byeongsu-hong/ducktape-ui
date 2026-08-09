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
  expect thoughts.height > 40.0

  // And it folds back, so the height follows the row rather than only ever
  // growing.
  click toggle
  expect thoughts.height < 30.0

// An empty composer must not be able to open a turn, and neither must one
// holding only spaces: the draft is trimmed before it is judged.
test the_composer_will_not_send_a_blank_draft
  preset conversation
  viewport 920 800
  target composer = #app/composer/field/draft
  target send = #app/composer/field/send
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
  target composer = #app/composer/field/draft
  target send = #app/composer/field/send
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

// What will answer, and how hard it will think, are both on screen. A picker
// showing something other than what the next turn asks for is worse than no
// picker at all.
test the_header_shows_what_will_answer_and_how_hard
  preset conversation
  viewport 920 800
  target model_chip = #app/header/model/root
  target effort_chip = #app/header/effort/root
  expect text "gpt-5.6-sol" within model_chip
  expect text "xhigh" within effort_chip

// What this can and cannot prove, stated plainly: the overlay's own contents
// are outside the tree the harness scans, and this palette paints an opened
// control the same as a hovered one — so "the menu opened" is not observable
// here. What is observable is that the control answered the click at all, and
// the capture is what the menu's appearance is actually reviewed from.
test clicking_a_chip_raises_it_and_captures_its_menu
  preset conversation
  viewport 920 800
  target model_chip = #app/header/model/root
  // Closed, the control is the header it sits in. Opened, it lifts onto the
  // accent — which is the only part of the menu the harness can read, the
  // overlay's own text being outside the scanned tree.
  expect model_chip.background == background.color(color.rgb8(255, 255, 255))

  click model_chip
  window redraw
  expect model_chip.background == background.color(color.rgb8(243, 242, 239))
  capture menu

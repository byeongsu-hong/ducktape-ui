// A finished turn is read as its answer. Everything it did to get there folds
// under one line, and unfolds when someone wants to see it — twice over, since
// a reasoning summary keeps its own fold inside that one.
//
// The rows are carried on the row itself rather than in component state,
// because a settled row is drawn behind `lazy`, which redraws it only when the
// row changes and cannot see state held anywhere else.
test folding_a_finished_turn_reveals_what_it_did
  preset conversation
  viewport 920 800
  target work = #app/transcript/rows/key(-2)/work(-2)/root/toggle
  target thoughts = #app/transcript/rows/key(-3)/reasoning(-3)/root
  target thoughts_toggle = thoughts/toggle

  expect text "Worked for 12s · 4 steps"
  expect no text "Checking the crate before answering"
  expect no text "Searched the web"

  click work
  expect text "Checking the crate before answering"
  expect text "Searched the web"
  expect text "Opened a page"
  expect thoughts.height < 30.0

  // And the summary inside it opens on its own.
  click thoughts_toggle
  expect thoughts.height > 40.0

  click work
  expect no text "Checking the crate before answering"

// A tool call says what it did; what it was given is there when asked for.
test a_tool_call_keeps_its_arguments_behind_its_own_fold
  preset conversation
  viewport 920 800
  target work = #app/transcript/rows/key(-2)/work(-2)/root/toggle
  target search = #app/transcript/rows/key(-4)/tool(-4)/root/toggle
  click work

  expect text "Searched the web"
  expect no text "site:crates.io/crates/iced latest version  ·  iced-rs releases"

  click search
  expect text "site:crates.io/crates/iced latest version  ·  iced-rs releases"

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

// What will answer, and how hard it will think, sit with the message about to
// be sent. A picker showing something other than what the next turn asks for is
// worse than no picker at all.
test the_composer_shows_what_will_answer_and_how_hard
  preset conversation
  viewport 920 800
  target model_chip = #app/composer/model/root
  target effort_chip = #app/composer/effort/root
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
  target model_chip = #app/composer/model/root
  expect model_chip.background == background.color(color.rgb8(255, 255, 255))

  click model_chip
  window redraw
  expect model_chip.background == background.color(color.rgb8(243, 242, 239))
  capture menu

// The whole turn, driven the way a person drives it: type into the composer,
// press Send, and let everything the turn produced land in the transcript.
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

  // The working-out folded itself away the moment there was an answer.
  expect text "Worked for 0s · 4 steps"
  expect no text "Searched the web"
  expect text "22,875 in · 321 out · 257 reasoning"
  capture chatted

// A turn already running can be ended, cut short and redirected, or left to
// finish with something waiting behind it. Which of those are offered depends
// on whether anything has been typed to redirect it with.
test a_running_turn_offers_stop_steer_and_send_after
  preset steering
  viewport 920 560
  target stop = #app/composer/field/stop
  expect a11y stop name "Stop"
  expect missing #app/composer/field/send
  expect exists #app/composer/steer
  expect exists #app/composer/queue

// Nothing typed, nothing to steer with: stopping is all that is on offer.
test a_running_turn_with_nothing_typed_offers_only_stop
  preset streaming
  viewport 920 560
  expect exists #app/composer/field/stop
  expect missing #app/composer/steer
  expect missing #app/composer/queue

// And with no turn running there is nothing to stop.
test an_idle_composer_offers_only_send
  preset conversation
  viewport 920 560
  expect exists #app/composer/field/send
  expect missing #app/composer/field/stop
  expect missing #app/composer/steer

// iced draws non-editable text without selection, so there is no dragging
// over an answer to copy it. The button is how the text leaves the window,
// which makes it load-bearing rather than a convenience.
test copying_a_message_puts_its_own_text_on_the_clipboard
  preset conversation
  viewport 920 800
  target ask = #app/transcript/rows/key(-1)/prompt(-1)/root/copy
  target reply = #app/transcript/rows/key(-6)/answer(-6)/root/copy
  expect a11y ask name "Copy"
  expect a11y reply name "Copy"

  // The clipboard itself is outside what the harness can read, so what is
  // asserted is that each button carried its own row's text to the handler
  // that writes it — which is the part that goes wrong.
  click ask
  expect copied == "Which version of iced is current, and how do I stream a reply into a Markdown view?"
  expect text "Copied"

  click reply
  expect copied != "Which version of iced is current, and how do I stream a reply into a Markdown view?"

// Chats already had are offered by name and date, and picking one opens it.
// The list is fetched when the panel is asked for rather than at startup,
// because it touches every rollout on the machine.
// An overlay's contents are outside the tree the harness scans — the same
// limit the picker menu has — so what is asserted is that the panel is there
// with a row per chat, and the capture is what its appearance is reviewed from.
test the_history_panel_offers_chats_by_name
  preset history
  viewport 920 700
  expect exists #history-panel
  expect exists #history-panel/chat-list
  capture history

// And it is not in the way until it is wanted.
test the_history_panel_is_shut_until_it_is_asked_for
  preset conversation
  viewport 920 700
  target history = #app/header/history
  expect a11y history name "History"
  expect missing #history-panel

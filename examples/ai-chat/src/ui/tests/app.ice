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
  target work = #shell/app/transcript/rows/key(-2)/work(-2)/root/toggle
  target thoughts = #shell/app/transcript/rows/key(-3)/reasoning(-3)/root
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
  target work = #shell/app/transcript/rows/key(-2)/work(-2)/root/toggle
  target search = #shell/app/transcript/rows/key(-4)/tool(-4)/root/toggle
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
  target composer = #shell/app/composer/field/draft
  target send = #shell/app/composer/field/send
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
  target model_chip = #shell/app/composer/model/root
  target effort_chip = #shell/app/composer/effort/root
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
  target model_chip = #shell/app/composer/model/root
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
  target composer = #shell/app/composer/field/draft
  target send = #shell/app/composer/field/send
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
  target stop = #shell/app/composer/field/stop
  expect a11y stop name "Stop"
  expect missing #shell/app/composer/field/send
  expect exists #shell/app/composer/steer
  expect exists #shell/app/composer/queue

// Nothing typed, nothing to steer with: stopping is all that is on offer.
test a_running_turn_with_nothing_typed_offers_only_stop
  preset streaming
  viewport 920 560
  expect exists #shell/app/composer/field/stop
  expect missing #shell/app/composer/steer
  expect missing #shell/app/composer/queue

// Starting over cannot detach a turn that is still producing output.
test new_chat_is_unavailable_while_a_turn_is_running
  preset streaming
  viewport 920 560
  target new_chat = #shell/sidebar/new-chat
  expect a11y new_chat disabled true

// The handler keeps the same boundary even when invoked without the button.
test reset_cannot_abandon_a_running_turn
  preset streaming
  viewport 920 560
  dispatch reset
  expect busy == true
  expect status == "Responding"

// A pending disk read owns the same session until its result is delivered.
test new_chat_is_unavailable_while_a_chat_is_opening
  preset opening
  viewport 920 560
  target new_chat = #shell/sidebar/new-chat
  expect a11y new_chat disabled true
  dispatch reset
  expect loading_chat == true
  expect open_path == "/sessions/2.jsonl"

// Idle controls stay live; the guards are state boundaries, not dead routes.
test an_idle_chat_can_start_over
  preset conversation
  viewport 920 560
  target new_chat = #shell/sidebar/new-chat
  expect a11y new_chat disabled false
  click new_chat
  expect busy == false
  expect text "What are we working on?"
  expect no text "Worked for 12s · 4 steps"

test an_idle_chat_can_sign_out
  preset conversation
  viewport 920 560
  target sign_out = #shell/app/header/sign-out
  expect a11y sign_out disabled false
  click sign_out
  expect signed == false
  expect text "Sign in to Codex"
  expect no text "Worked for 12s · 4 steps"

// Signing out cannot detach either kind of work that still owns the session.
test sign_out_is_unavailable_while_a_turn_is_running
  preset streaming
  viewport 920 560
  target sign_out = #shell/app/header/sign-out
  expect a11y sign_out disabled true
  dispatch forget
  expect signed == true
  expect busy == true
  expect status == "Responding"

test sign_out_is_unavailable_while_a_chat_is_opening
  preset opening
  viewport 920 560
  target sign_out = #shell/app/header/sign-out
  expect a11y sign_out disabled true
  dispatch forget
  expect signed == true
  expect loading_chat == true
  expect open_path == "/sessions/2.jsonl"

// And with no turn running there is nothing to stop.
test an_idle_composer_offers_only_send
  preset conversation
  viewport 920 560
  expect exists #shell/app/composer/field/send
  expect missing #shell/app/composer/field/stop
  expect missing #shell/app/composer/steer

// Part of an answer leaves by being dragged over; the whole of one leaves by
// this button, which is still the shorter route when the whole of it is what
// is wanted.
test copying_a_message_puts_its_own_text_on_the_clipboard
  preset conversation
  viewport 920 800
  target ask = #shell/app/transcript/rows/key(-1)/prompt(-1)/root/copy
  target reply = #shell/app/transcript/rows/key(-6)/answer(-6)/root/copy
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

// The whole reason this window keeps its own record: a chat had here is kept
// here, and it is in the sidebar the moment the turn ends rather than one
// restart later. Nothing is preset — the turn is driven through the real
// composer and the list comes back off disk.
test a_chat_just_had_is_listed_the_moment_it_ends
  preset signed_in
  viewport 1180 700
  target composer = #shell/app/composer/field/draft
  target send = #shell/app/composer/field/send
  target list = #shell/sidebar/chat-list
  expect no text "Where does this window keep its chats?" within list

  click composer
  type "Where does this window keep its chats?"
  click send

  // The turn finished, so it was written out and read back.
  expect text "22,875 in · 321 out · 257 reasoning"
  expect text "Where does this window keep its chats?" within list

// Chats already had are a place on screen, not something that appears over
// one: the list is beside the transcript, the chat being read is marked in it,
// and picking one opens it.
test the_sidebar_lists_chats_already_had
  preset history
  viewport 1180 700
  target list = #shell/sidebar/chat-list
  target first = #shell/sidebar/chat-list/chat("/sessions/4.jsonl")/root
  expect exists list
  expect text "Recent"
  expect text "Which version of iced is current?"
  expect text "Write a test for the SSE reader"
  expect text "2026-08-08"
  expect a11y first name "Which version of iced is current?"
  capture history

// An empty history is still a state, not a blank panel that looks unfinished.
test an_empty_recent_sidebar_explains_itself
  preset signed_in
  viewport 760 480
  target list = #shell/sidebar/chat-list
  expect exists list
  expect text "No recent chats" within list
  expect text "Start a new chat to see it here." within list
  capture empty_recent

// A thousand rollouts take long enough to read that nothing happening reads as
// nothing working. The list fills as it is found and says how far it has got,
// and both of those go away when there is nothing left to say.
test the_sidebar_says_how_far_it_has_read
  preset scanning
  viewport 1180 600
  target bar = #shell/sidebar/scan
  expect exists bar
  expect text "reading…"
  expect text "Which version of iced is current?"
  capture scanning

test a_finished_scan_says_nothing
  preset history
  viewport 1180 600
  expect missing #shell/sidebar/scan
  expect no text "reading…"
  expect text "Which version of iced is current?"

// Reading a chat off disk takes anything from milliseconds to seconds and
// happens on another thread. Saying so is the only thing that separates a
// window working from a window frozen, and it has to replace the chat being
// left rather than sit above it.
test opening_a_chat_says_it_is_opening
  preset opening
  viewport 1180 700
  expect exists #shell/app/transcript/opening
  expect text "Opening that chat…"
  expect no text "Worked for 12s · 4 steps"
  capture opening

test a_chat_that_is_open_says_nothing_about_opening
  preset history
  viewport 1180 700
  expect missing #shell/app/transcript/opening
  expect text "Worked for 12s · 4 steps"

// Dragging across an answer selects it where it stands, and keeps going when
// it leaves the paragraph it started in. The rendering is rich text, which this
// toolkit paints without selection until something owns the paragraph —
// `src/select.rs` is that, and a selection there belongs to the answer rather
// than to any one block of it.
//
// The drag runs from the middle of the middle paragraph to below the whole
// answer, so what is selected is the tail of one block and the whole of the
// next. Copying it is what says so exactly, because a selection leaves this
// window by the same route a clicked link and the Copy button use. The
// highlight itself is in the capture.
test dragging_across_an_answer_selects_past_the_block_it_started_in
  preset one_answer
  viewport 920 400
  target answer = #shell/app/transcript/rows/key(-31)/answer(-31)/root
  target body = answer/body
  target copy = answer/copy
  expect copied == ""

  press body
  move copy
  release
  capture dragged
  // A drag is not a click on what either end landed on.
  expect copied == ""

  // The middle of the row lands mid-word, so the tail starts a letter in — and
  // the blank line is the block boundary the drag crossed.
  chord control "c"
  expect copied == "nd extend it as it grows.\n\nThe view rebuilds one row."

  // And the button beside it still takes the whole of the answer.
  click copy
  expect copied == "The document is parsed once.\n\nHold the parsed document and extend it as it grows.\n\nThe view rebuilds one row."

// A window shows one selection. A prompt is plain text, which the runtime
// makes selectable; an answer is rendered Markdown, which `src/select.rs` does.
// They keep their own anchors and cursors and agree on nothing except which of
// them holds it — so starting a drag in either has to put the other out.
//
// Copying is what makes that observable: an answer's selection leaves by the
// clipboard handler, so the answer answering a key it should have gone quiet
// for shows up as `copied` changing when it must not.
test starting_a_selection_takes_it_from_wherever_it_was
  preset one_answer
  viewport 920 400
  target ask = #shell/app/transcript/rows/key(-30)/prompt(-30)/root/body
  target ask_copy = #shell/app/transcript/rows/key(-30)/prompt(-30)/root/copy
  target answer = #shell/app/transcript/rows/key(-31)/answer(-31)/root
  target body = answer/body
  target copy = answer/copy

  // The answer takes the selection, and answers for it.
  press body
  move copy
  release
  chord control "c"
  expect copied == "nd extend it as it grows.\n\nThe view rebuilds one row."

  // Something else on the clipboard, so the answer answering again would show.
  click ask_copy
  expect copied == "How does an answer grow?"

  // The prompt takes it away, and the answer is quiet.
  press ask
  move ask_copy
  release
  capture moved
  chord control "c"
  expect copied == "How does an answer grow?"

// A reply being written is drawn by two surfaces — what the model is working
// out, and what it is answering with. Both are rebuilt from nothing on every
// frame, so neither can hold an identity of its own; where they sit on the page
// is what tells them apart, and a drag that ends inside one of them ends there.
// Told apart by anything they carry instead, this drag highlighted the reply,
// at the same offsets, in the other box entirely.
test dragging_over_the_working_out_selects_it_and_not_the_reply
  preset steering
  viewport 920 800
  target thinking = #shell/app/transcript/live/live-work/live-thinking

  press thinking
  move 300.0 330.0
  release
  capture working_out
  chord control "c"
  expect copied == "hecking the"

// A selection belongs to the transcript, not to the row it started in. A
// question is drawn as plain text and an answer as rendered Markdown — two
// different widgets, sharing no numbering — so which run of text comes before
// which is read off the page rather than counted: down it first, then across.
//
// The drag runs from the middle of the question to below the whole answer, and
// what comes back is the tail of the question and every block of the answer,
// each set apart the way it is drawn.
test dragging_from_a_question_runs_on_into_its_answer
  preset one_answer
  viewport 920 400
  target ask = #shell/app/transcript/rows/key(-30)/prompt(-30)/root/body
  target answer = #shell/app/transcript/rows/key(-31)/answer(-31)/root
  target copy = answer/copy
  expect copied == ""

  press ask
  move copy
  release
  capture across

  chord control "c"
  expect copied == "answer grow?\n\nThe document is parsed once.\n\nHold the parsed document and extend it as it grows.\n\nThe view rebuilds one row."

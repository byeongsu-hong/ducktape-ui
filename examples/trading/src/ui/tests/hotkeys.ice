// The keyboard, and the two rules it is built on.
//
// **A focused field owns what is typed into it.** Every test below drives the
// real widgets: it focuses the search box, types the scheme's own letters into
// it, and asserts that the ticket did not move. That is the disaster this
// scheme has to be proof against, and `status=ignored` is what makes it
// structural — a key a widget consumed never reaches the app at all.
//
// **No key sends.** Nothing here reaches a confirmation past its own panel, and
// the last test asserts the whole scheme goes quiet the moment one is standing.

// The side keys, from both directions: `b` buys and `s` sells, and each is
// asserted against a ticket that was on the other side first, so a binding that
// never fired and a binding that always fired both fail.
test trading_the_side_keys_move_the_side
  preset held
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target review = app/terminal-fit/trade/ticket-panel/ticket-review
  expect ticket_buy
  key "s"
  expect !ticket_buy
  expect a11y review name "Send this sell of 3 BTC on Hyperliquid, REAL MONEY"
  key "b"
  expect ticket_buy
  expect a11y review name "Send this buy of 3 BTC on Hyperliquid, REAL MONEY"

// The size keys are the share row's own arithmetic, so a keyed 50% and a
// pressed 50% fill in the same number — including which thing it is a share of
// once CLOSE POSITION has set reduce-only.
test trading_the_size_keys_are_the_share_row
  preset held
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target close = ticket/close-held
  target half = ticket/share-50/root
  // Opening: the key and the button agree.
  key "2"
  expect !empty(ticket_size)
  click half
  expect ticket_size == "0.08593"
  key "2"
  expect ticket_size == "0.08593"
  // Closing: the same key, and now it is half the position rather than half the
  // buying power, because the key routes through the same sizing the row does.
  click close
  expect ticket_reduce
  key "2"
  expect ticket_size == fmt_size(position_held(positions, coin) * 0.5)
  key "4"
  expect ticket_size == fmt_size(position_held(positions, coin))

// The arrows move the limit by the market's own tick, read off the book rather
// than assumed. The fixture book is quoted a dollar apart.
test trading_the_arrows_move_the_limit_one_tick
  preset held
  viewport 1660 900
  expect ticket_price == "64,000.00"
  key arrow-up
  expect ticket_price == "64,001.00"
  key arrow-down
  key arrow-down
  expect ticket_price == "63,999.00"

// The rule the whole scheme stands on: the search box owns plain typing.
//
// Every letter and digit in the scheme is typed into the field here, and the
// ticket is asserted unmoved on the far side of it. This is the test the
// mutation is for: a scheme that listened to captured keys as well would turn
// a search for "SOL" into a sell, and a search for "1INCH" into a size.
test trading_typing_in_the_search_box_is_not_a_hotkey
  preset held
  viewport 1660 900
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target search = markets/search
  expect ticket_buy
  expect ticket_size == "3.00"
  expect ticket_price == "64,000.00"
  focus search
  type "sb1234"
  // The letters went where they were typed.
  expect query == "sb1234"
  // And nowhere else. The side is the one the ticket opened on, the size is the
  // one it opened with, and the price never moved a tick.
  expect ticket_buy
  expect ticket_size == "3.00"
  expect ticket_price == "64,000.00"
  // Escape still belongs to the search box, which is the one binding that was
  // here before this scheme was.
  key escape
  expect query == ""
  expect ticket_buy

// The same rule for the ticket's own fields, which are the ones a trader is
// actually typing in while the ticket is the thing they are looking at.
test trading_typing_in_the_size_field_is_not_a_hotkey
  preset held
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target size = ticket/ticket-size
  expect ticket_buy
  focus size
  type "1"
  expect ticket_size == "3.001"
  // Emptied and typed into again. The field keeps the cursor across going
  // empty, which is the case that used to lose it: a line above the field came
  // and went with the size, iced matches widget state by position, and the
  // keystroke after it reached the app's shortcuts instead of the box the
  // reader was in.
  cursor end
  repeat backspace 5
  expect ticket_size == ""
  type "s"
  expect ticket_size == "s"
  expect ticket_buy
  clear
  type "12"
  expect ticket_size == "12"
  // The digits sized the order by being typed, not by being heard: a scheme
  // that heard them too would have overwritten the field with a share of the
  // account between the two keystrokes.
  expect ticket_buy
  type "b"
  expect ticket_buy
  expect ticket_size == "12b"

// Enter reviews, and it is the field's own submit rather than a key the app
// listens for — so it cannot fire from a widget the reader is not in.
test trading_enter_in_a_ticket_field_opens_the_review
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target size = ticket/ticket-size
  target markets = app/terminal-fit/trade/markets
  target search = markets/search
  target panel = #confirm
  // From the search box it does nothing, because the confirmation is opened by
  // the ticket's fields and the search box is not one of them.
  focus search
  type "BTC"
  key enter
  expect missing panel
  // From the size field it opens the confirmation — and stops there.
  focus size
  key enter
  expect exists panel
  expect confirm_size(confirm) == 3.0

// The safety rule, as the scheme's own off switch.
//
// With a confirmation standing, every binding is dead: the keys that would have
// moved the ticket move nothing, and the confirmation is the order it froze
// when the press was made. SEND IT has no key at all.
test trading_no_key_reaches_past_the_confirmation
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #confirm
  target send = panel/confirm-send
  click review
  expect exists panel
  expect a11y send name "Send this buy of 3 BTC on Hyperliquid, REAL MONEY"
  // Every key in the scheme, and the two a reader might expect to commit.
  key "s"
  key "4"
  key arrow-up
  key enter
  key " "
  // Nothing was sent: the panel is still standing over the order it froze, and
  // no venue was asked anything.
  expect exists panel
  expect empty(error)
  expect empty(status)
  expect confirm_price(confirm) == 64000.0
  // And the ticket behind it did not move either, because the whole scheme is
  // off rather than merely blocked at the send.
  expect ticket_buy
  expect ticket_size == "3.00"
  expect ticket_price == "64,000.00"

// The scheme is documented in the app, and the documentation is the scheme: one
// list in Rust answers the keys and prints the rows, so a binding cannot change
// without its row changing with it.
test trading_settings_says_what_the_keyboard_does
  preset unlocked
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target rows = settings/settings-content/shortcuts
  target side = rows/shortcut("B")
  dispatch navigate(Page.settings)
  expect exists rows
  expect exists side
  expect text "Buy / long" within rows
  expect text "Size to 25%, 50%, 75%, all" within rows
  expect text "Move the limit price one tick" within rows
  // Including what it will not do.
  expect text "No key sends an order. The keys above reach the confirmation and stop there, and they are off entirely while one is open — SEND IT is pressed by hand. A field you are typing in keeps its own keystrokes, so these do nothing while the search box or a ticket field has the cursor." within rows
  capture settings_keyboard

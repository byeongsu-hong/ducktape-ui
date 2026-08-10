// What a live list does under a reader who has scrolled into it.
//
// `push_fills` and `push_trades` both put a beat's rows in front of the ones
// already listed, so these two panels grow at the top — the end nobody is
// looking at. iced keeps a scroll offset as an absolute distance from the top
// of the content and never revises it when the content changes, so before
// `anchor-y=keep` a reader 120px into the recent fills had the row they were
// reading move 26px down the screen for every fill that landed. Measured on
// the 200-row screen: one beat of four fills moved the watched row from
// y=1024 to y=1128.
//
// The offset is the assertion because the offset is the mechanism. A row's own
// `y` is its position in the content, which genuinely moves when content is
// inserted above it; what has to move with it — and did not — is the offset
// the viewport reads that content through.

test trading_new_fills_do_not_move_the_fills_a_reader_is_reading
  preset unlocked
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target lower = terminal/lower
  target printed = lower/fills
  target list = printed/fill-list
  // Forty fills, older than anything the beat below carries, so the beat lands
  // above them rather than among them.
  dispatch fills_streamed(demo_fills_many(40))
  scroll-to list 0.0 120.0
  expect list.scroll_y ~= 120.0
  // Three fills the account has just made. Each row is 26px, so the content
  // above the viewport grew by 78 and the offset owes exactly that much.
  dispatch fills_streamed(demo_fills())
  expect list.scroll_y ~= 198.0

// What the panel does not lay out, and what that costs. The list is a keyed
// column with `virtual-row=26.0`, so on the 200-fill screen only the rows the
// viewport reaches are measured, drawn, and published to assistive tech — and
// the rest are absent from this snapshot rather than merely invisible in it.
// That absence is the whole saving: text is shaped during layout, so a row
// never laid out never shapes, which is 314us of the frame on the dense screen.
//
// The key is what makes it safe. Under an index-diffed column the beat below
// would move all 200 rows' memo, cursor, and measured height onto their
// neighbours; keyed, a fill printing on top disturbs nothing already listed,
// which the two tests above hold from the scroll offset's side.
test trading_a_long_fills_list_lays_out_only_the_rows_it_can_show
  preset busy
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target lower = terminal/lower
  target printed = lower/fills
  target list = printed/fill-list
  // `demo_fills_many` counts tids up from 4000000, newest first, so tid
  // 4000000 is the top row and 4000199 is 199 rows below it.
  target newest = list/key(4000000)/fill(4000000)/root
  target oldest = list/key(4000199)/fill(4000199)/root
  expect exists newest
  expect missing oldest

  // Scrolling mounts what it reaches and lets go of what it left. 199 rows at
  // 26px is 5174, past the end, so this rests at the bottom of the content.
  scroll-to list 0.0 5174.0
  window redraw
  expect exists oldest
  expect missing newest

// The other half, and the one that would be quietly ruined by a fix that only
// looked at content height: a reader who has not scrolled is resting on the
// newest row, and that is the one place following the content is what they
// want. Landing three fills must leave them on the newest, not carry them 78px
// down into the history.
test trading_a_fills_list_at_rest_stays_on_the_newest_fill
  preset unlocked
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target lower = terminal/lower
  target printed = lower/fills
  target list = printed/fill-list
  dispatch fills_streamed(demo_fills_many(40))
  expect list.scroll_y ~= 0.0
  dispatch fills_streamed(demo_fills())
  expect list.scroll_y ~= 0.0
  expect text "BTC" within printed

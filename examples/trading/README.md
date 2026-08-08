# Trading

A live Hyperliquid terminal written in Ice: the perpetuals list, candles and
the order book for the selected market, and — for any address you point it at
— that account's open positions, resting orders, recent fills, and every one
of those fills marked on the candle it landed in.

```bash
cargo run -p trading-example
cargo test -p trading-example
```

The app opens on an address prompt, prefilled with a well-known account so
there is something to look at on the first run. Press **Connect** to read it,
type your own, or **Browse markets** to use market data only; the positions
panel offers the prompt again if you change your mind. Browsing without one,
the three panels that need an account say so rather than reporting that the
account has nothing in it.

An address is checked before it is sent, because the exchange answers a
malformed one with a plain-text parser complaint rather than JSON — so without
the check, a typo reads as "Hyperliquid sent bad JSON", the one error that
blames the exchange for something you just typed.

![Trading](screenshots/trading.png)

## Design

The screen is an instrument panel, so it is set like one. Every figure is
Monoplex KR and every word is IBM Plex Sans KR — the same skeleton drawn twice,
one monospaced so a column of prices aligns on its digits, one proportional for
prose. The ground is a warm ink-black rather than the blue-black every exchange
ships, and the two money colours are a ledger green and an oxide red: printer's
inks, not phosphor. They mean one thing and are spent nowhere else: which way
money went, which side an order takes, and how far a position has run toward
losing all of it. A border, a tab, a rule, a heading, a failure — none of them
may be either colour, so long and short read at a glance. A feed that dropped
is the app's problem and not the market's, so it says so in plain ink.

The one thing this layout gives you that an exchange table does not is the
**risk rail** under each liquidation price: a bar showing how far the mark has
travelled from your entry toward the cliff. Distance to liquidation is the
number a leveraged position actually turns on, and it is the one number every
table makes you compute. Here it is a length.

The same rail runs under the equity figure, because cross positions do not die
one at a time: the account goes when its equity falls under the maintenance
requirement the margin engine holds against it. That bar is how much of the
equity the requirement has already claimed — empty with nothing open, full at
the call. Two rails, one reading, one for the position and one for everything.

That one carries its share as a number beside it as well as a length. The
position rail can be a bare bar because everything it measures — the entry, the
mark, the liquidation price — is written out in the row beneath it, so a reader
who cannot see the bar can still do the subtraction. Nothing else on screen
carries the maintenance requirement, so a bar alone would be its only copy, and
a bar has no accessible value.

The tape's header carries which side is crossing, weighted by size. The same
price with buyers taking it and with sellers hitting it are two different
markets, and that is not something the price alone says.

The **tape** under the book is everybody's trades rather than this account's:
the socket was already open and one more subscription costs nothing, so the
panel that tells you whether anything is happening at all is close to free.

A print is checked against the market on screen before it is folded in, the
same way a pushed candle is: switching markets clears the tape, but a message
already in flight for the market you just left arrives after that and would
otherwise read as this one's.

It reads the way the market traded rather than the way the wire reported. One
aggressing order that eats four resting orders arrives as four messages sharing
a hash — four rows would be the exchange's bookkeeping, not the market's. They
become one row, priced at what that order actually paid across the levels it
took and marked with how many it took. A sweep is the thing worth seeing, and
it is exactly what a raw message-per-row tape buries.

The tape takes whatever height the rail has left, because it is the panel that
gets better with more rows. Resting orders are few and keep a fixed slot.

A fill the account just printed is pushed onto the top of the list wearing its
side's colour, which fades over two beats and leaves the row cold. It is the
only motion on screen that is not a number changing, so it is the only thing
that can mean *something happened while you were looking elsewhere*. The
divider under the chart drags: positions and fills are worth more rows on some
days than others, and it stops at its limits rather than at the gesture: a
drag that overshoots pins to the bound instead of refusing to move.

A row that names a market is a way back to it — a position, a resting order, a
fill. An account holding a hundred of them has no other route to any but the
one already charted.

Funding is the one figure here written to four decimals. Two is what every
other number wants, and it is exactly what funding cannot use: the hourly rate
is a hundredth of a percent on most of the exchange, so 166 of the 177 funded
markets would print the identical `+0.00%`. The header pays for those two
digits by calling volume `VOL` and open interest `OI`, which is what a book
calls them anyway, and it fits the window's own 1400pt minimum again.

The book quotes its spread in basis points rather than in dollars, because a
spread only means something against the price under it: two dollars is the
tightest market on the exchange on Bitcoin and no market at all on a coin worth
three. One number you can carry between markets beats one you have to divide
first.

## The ticket sends nothing

The ticket is a rail beside the book, not a dialog over it. An order is priced
against what the book is doing, and a modal that covers the book to ask about
it has the relationship backwards. It prices an order and stops there.
It seeds the limit price from the book's mid, takes a size and a leverage held
inside what the market allows, and answers the only three questions worth
asking before an order exists: what it is worth, what it ties up, and where it
dies. The liquidation is isolated-margin arithmetic against the maintenance
requirement this market holds — a cross position dies against the whole
account instead, which is the rail under the equity figure.

Every row that does something is named by what it does, not by what it shows:
a book level announces the order it would start rather than the price it
displays, and a position row its side and size rather than its ticker. A row
carrying five figures is worth more than one of them to somebody who cannot
see the other four.

A level in the book fills it: clicking an ask starts a buy at that price,
clicking a bid a sell, because the side you want is the side you just clicked
across. Changing market resets it — 0.5 means a different order on every one
of them, and carrying it over is how you place an order you did not mean.

Four rails need width the old minimum did not have. The window asks for 1660
now, which is what the columns actually measure: 610 for the positions row,
310 for the fills, and 719 for the market list, the book and the ticket.

An order that closes something ties up nothing and has no cliff, and the panel
says so: the trade still has a value, but the margin is zero and there is no
liquidation to quote, because nothing was opened. Past the position it is both
at once — all of it trades, only the excess opens, and only the excess can be
liquidated.

A position in this market puts a **close** on the ticket, which fills the size
that flattens it and takes the side that does — both read off the same signed
number, which is the only place the two agree by construction rather than by
you doing the sign in your head.

It also says what the order would do to what you already hold. Opening and
closing are different acts on the same ticket, and the only thing that
separates them is the sign of a number two panels away — so the ticket reads it
for you: a buy against a short reduces it, closes it, or closes it and opens
the other way.

A market the app has not read yet gets no cliff quoted at all. What an order
is worth and what it ties up are multiplication and always answerable, but the
liquidation needs the venue's requirement, and treating an unknown requirement
as zero puts the cliff further from the entry than it really is. That is the
one direction a risk number must never be wrong in, so the panel says it does
not know.

Leverage is reported as it was priced rather than as it was typed. The field
takes anything; the market does not, so a 400 typed into a 5x market is held at
5 and the ticket says 5. A liquidation quoted at a leverage the panel is not
showing is the one number here that must never be wrong.

Escape closes it, and the subscription that listens for Escape exists only
while it is open. Escape with a search in the box clears the search instead,
on its own subscription with its own condition, so neither key listener exists
when there is nothing for it to do.

Nothing is signed and nothing is sent, and the panel says so in the place a
submit button would be. Sending would mean this app holding the key that signs
an EIP-712 order, which is not a thing an example should ask for. The boundary
is the interesting part: everything up to the signature is arithmetic worth
having, and the signature is where a real client starts.

The one figure in that arithmetic that is not arithmetic is the maintenance
requirement, and it belongs to the venue: Hyperliquid holds half the margin at
a market's maximum leverage, and another exchange holds something else. So the
market carries it and the ticket reads it, rather than the shared math knowing
one exchange's rule. It is stated once, next to the parser that knows whose
rule it is.

## Fixtures are read as evidence

The account's requirement is summed from the positions held against the whole
account, and an isolated one does not enter it — it dies alone. A hand-typed
requirement had the equity bar reading 38% loaded beside a cross position
whose own rail read nothing travelled: two risk figures on one screen, in
disagreement, both drawn convincingly.

The pair of account fixtures is at rest and against the engine, because a
safety indicator that has only ever been rendered at rest has never been
rendered.

Every capture in this directory is an argument that the panel is right, so a
fixture has to be a state the exchange could actually report. Five bugs in
this example were impossible states drawn convincingly, and a wrong number in
the right column is the one kind of wrong a render cannot show.

So the fixture positions derive their figures from the four that are chosen,
through the same arithmetic the panel uses, and a test holds the relations
that survive: unrealized from entry, mark and size; margin from the leverage
beside it; return on equity as that return over that equity; the rail as how
far the mark has travelled; the cliff on the correct side of the entry.

Writing that test found two numbers that had been on screen all along — a
return on equity of 811.79% where the position's own pnl and margin say
857.41%, and a 24h change rounded away from the prices it is computed from.

The markets are three rather than one, for the same reason. A list of one
answers no question a list is asked: which row is selected, what a search
leaves behind, whether a price landed on the market it belongs to.

## One market on the screen at a time

Every panel is quoting the same market, so a fixture has to price them
together: the mark a position is held at is the feed's price for its market,
the book sits inside a spread of it, the tape prints against it, and the chart
ends on it. Each of those is one number appearing in several places, and a
fixture that let them drift showed a book from one market beside a chart of
another — convincingly, because each panel was internally fine.

The at-risk fixture is where that first bit: it moved bitcoin to 58,000 and
left the book, the tape and the chart at 64,000. So the fixtures take the
price they are drawn around, and a test walks each pair.

## What the position costs to keep

A perpetual has no expiry, so a position is rented rather than bought, and
the rent arrives hourly forever. `RENT PER DAY` is that rate against this
order's notional: `-$57.60/day` on three bitcoin at the current funding.

Longs pay a positive rate and shorts are paid it, so the sign is the reader's
side rather than the venue's convention. It is the part of a carry that never
appears on a ticket, and the reason one that looks free is not.

Single-letter shortcuts for the side are not here. The market search listens
to the same keys, and the app has no notion of which surface holds focus, so
typing `b` to find bitcoin would flip the ticket to a buy instead.

## The rate belongs to the market, not the position

A market capped at 40x holds every position in it to half of that cap,
whether the trader opened at 40x or at 2x. Reading the requirement off the
position's chosen leverage overstates a conservative position by exactly the
factor it was conservative by — a 5x position on a 40x market reads eight
times closer to the engine than it is.

Where the venue reports what an account is held to, that figure is used
rather than reassembled. `AGAINST THE ENGINE` starts from
`crossMaintenanceMarginUsed` and only computes the part the order changes.

## The order, against the engine

`AGAINST THE ENGINE` reads `91% → 100%`: where the account stands against its
maintenance requirement now, and where this order leaves it. The panel already
said what an order costs in margin; it did not say what it costs in distance,
which is the figure a cross account is actually liquidated on — and the one
that has to be readable before sending rather than after.

Only cross positions count. An isolated one is liquidated against its own
margin and asks nothing of the account.

## The other price

The ticket quotes a price the reader typed. `IF YOU CROSS` is the other one:
the size walked through the resting side of the book, level by level, at the
prices actually there, with the distance from the mid beside it. The gap
between the two is the whole question of whether to cross or to rest.

The walk starts at the best price, which is not the first row. The asks are
stored reversed so the panel can draw them downward into the spread, and a
walk that trusted the order would have quoted the worst level in the book as
the first one filled. A test holds that, because nothing on screen would.

When the size is past what the book holds, it says so rather than pricing
depth that is not there.

## A price that has stopped arriving

The dangerous state in a terminal is not an error, it is data that has gone
still while still looking current. When the feed dropped, the mark stayed
green, the change stayed at +1.25%, the book and the tape kept their last
values, and the only two signs were a dash in the far corner and a line of
11px text in the positions gutter.

The mark now stops being coloured as a move, and says `NOT LIVE` beside
itself, because that is where the number is read. One feed drives the mark,
the book, the tape and the chart, so one badge qualifies all of them; marking
every cell would be the same statement, repeated until it is ignored.

It is a flag rather than a latency reading. A venue fast enough to report 0ms
would otherwise read as a venue that had stopped.

## Levels worth being told about

**WATCH THIS LEVEL** puts the ticket's price on a list under the book. Nobody
is asked which side it is waiting on, because that is a fact rather than a
question: a level above the mark can only be reached from below.

Firing is one-way. A price that touches a level and wobbles back has still
touched it, so a level chimes once and then reads as reached rather than
flickering with the tape. The header counts what is still waiting, which is
the only number a header can act on.

The alerts live where the market is, not where the account is: the same rail
as the book and the tape, because they are watching a price rather than a
position. They outlive the market they were set from, so every row names its
own — and dismisses by it, rather than by whatever is on screen.

## What a second venue has to provide

The panels, the folds, the ticket's arithmetic, the formatters and the chart
adapter do not know which exchange they are looking at. What does is a short
list, and it is the whole of a second adapter:

| | |
| --- | --- |
| Two endpoints | one REST, one websocket |
| Six requests | the universe, a candle window, an account, its resting orders, and whatever the websocket needs to open |
| Five channels | mids, book, market context, candles, and this account's fills |
| One field map per response | every number arrives as a string here; another venue will disagree about names and types both |
| One margin rule | the share of a position's value held against it — Hyperliquid keeps half the margin at the market's maximum leverage; the market carries the answer, so nothing shared learns the rule |
| One interval vocabulary | `1m`, `5m`, `1h` are this venue's spelling |
| One side encoding | `B` and `A` here, for both fills and prints |

Everything else is already venue-neutral, and the boundary test keeps it that
way: `SymbolRow`, `Position`, `Account`, `Book`, `Trade`, `Fill`, `Order` and
`Ticket` are shapes the panels read, not shapes Hyperliquid returns.

The one thing not yet done is the module split that would put those two halves
in separate files. It is mechanical — the venue half needs `Tape.candles`,
`MarketTick.mids`, `MarketTick.context` and `Fill.tid` visible to the crate,
because the venue writes what the panels read — and it is worth doing when
there is a second adapter to shape it against.

## What talks to the exchange

Everything the exchange pushes arrives on a websocket; everything it only
answers when asked goes through the `info` endpoint as a blocking `ureq` POST
moved off the UI thread with `smol::unblock`. Both live in
[`src/hyperliquid.rs`](src/hyperliquid.rs).

Two sockets, each a thread pumping into a channel that Ice consumes as a
`stream`:

| Ice stream | Subscriptions | Feeds |
| --- | --- | --- |
| `hl_market_feed` | `allMids`, `l2Book`, `activeAssetCtx`, `candle`, `trades` | every mid price, the book, the header's figures, the live candle, and the public tape |
| `hl_fill_feed` | `userFills` | a snapshot of recent fills, then each new one as it prints |

| Ice call | Request | Reads |
| --- | --- | --- |
| `hl_symbols` | `metaAndAssetCtxs` | the tradeable universe: tickers, maximum leverage, and the day's volume |
| `hl_candles` | `candleSnapshot` | 500 candles when a market or interval is opened |
| `hl_history` | `candleSnapshot` | 500 more, ending where the tape begins, when the chart is panned back that far |
| `hl_account` | `clearinghouseState` | equity, margin, open positions with PnL, ROE, leverage, and funding paid |
| `hl_orders` | `openOrders` | resting orders, listed with their age and drawn on the chart as levels |

Responses are read as `serde_json::Value` and mapped by hand, because the
exchange sends every number as a string — a derive would need a custom
deserializer per field. Prices, sizes, and PnL that are missing or unparsable
read as zero rather than failing the whole message.

Positions and resting orders are the one thing still polled, every 5s, because
Hyperliquid publishes no channel that pushes them. The universe is re-read once
a minute for the figures that move on a daily clock; the market on screen gets
its own `activeAssetCtx`, so the header is live. Both stop while the address
prompt is up (`when` conditions on the `subscribe` block, so iced drops the
timers instead of ignoring their messages), and `abort feeds` closes the
sockets with them.

Waiting five seconds to find out what a position is worth is not a position
panel, so between polls the feed values them itself. Every beat re-marks each
position at the price that just came in and moves its PnL by what the price
did — a delta, not a recomputation from the entry: the entry price the
exchange reports is an average rounded to five significant figures, and a
position of sixty million units turns that rounding into real money. The
return moves the same way, over the margin the position opened with, which is
what the exchange's own `returnOnEquity` divides by. The risk rail re-measures
against the new mark, equity moves by what the positions just made, and each
poll re-anchors all of it. What is withdrawable, what margin is tied up, and
what the maintenance requirement is stay with the poll: those are the margin
engine's answers, not arithmetic over positions. The health rail still closes
between polls, though — the requirement it measures against is fixed until the
next one, but the equity falling toward it is not.

The market feed re-reads the tape's focus on every beat, so switching markets
costs an unsubscribe and a subscribe on the socket already open rather than a
new connection. Candles are merged straight into the shared tape and never
cross into Ice: the chart repaints on its own 100ms beat, so a tick costs no
app message and no view rebuild. Everything else the feed carries is coalesced
into at most one message per beat, and each one holds the latest of everything
rather than only what changed, so a handler can assign it without asking which
kind of update it was.

A failure and a progress message share one slot under the positions header,
and a failure wins it. They are different things: "Loading candles" is the app
working and "Hyperliquid unreachable" is the app stopped, so they are separate
state and only one of them is red. The slot is also the chart's hover readout,
and that loses to both — a candle's open and close are worth less than knowing
the feed is gone.

Latency is the round trip of the socket's own ping, which needs no agreement
between our clock and the exchange's, and the ping is required anyway: a socket
that goes quiet for a minute is closed. A dropped feed clears it back to an em
dash rather than leaving the last good number in the header, because a stale
`42ms` is the panel claiming to be live while it reconnects. Only the market
feed's own failures do that; a poll that fails says so in the status line and
leaves the socket's reading alone.

`hl_candles` keeps one tape for the whole session. An empty tape backfills 500
candles and adopts the market that filled it, and the feed replaces the live
candle in place from there. Switching markets re-points the tape, and a
response — or a pushed candle — for the market you just left is dropped instead
of overwriting the one you are looking at. Panning back past the oldest candle
asks `hl_history` for the window before it, once per tape length; the chart
moves its own viewport by however many candles land in front of it, so the
screen stays on the bars it was showing.

## Marking trades on the chart

The chart is `candle-chart` from [`crates/ui`](../../crates/ui/src/ui/candle_chart.rs)
with three annotation overlays:

```rust
candle_chart_shared(tape, &chart_theme())
    .price_lines(position_lines(positions, coin))  // entry, liquidation
    .price_lines(order_lines(orders, coin))        // resting orders
    .markers(fill_markers(fills, coin))            // one glyph per fill
```

A buy is a triangle pointing up out of its fill price, a sell points down into
it, and each carries its size or, for a closing fill, what it realized. All
three are ordinary `ChartOverlay` implementations — the same extension point a
caller uses for anything the built-ins do not cover — so nothing about trading
leaks into the chart widget.

The chart is drawn by Rust and everything around it by Ice, so the palette
exists twice: as tokens in `theme.ice`, and as literals in `chart_theme`.
Nothing makes them agree, and the chart is half the screen — a drift would be
plain at runtime and invisible until then. A test reads the tokens out of
`theme.ice` and holds the chart to them.

## Boundary

One extern block
([`src/ui/extern/hyperliquid.ice`](src/ui/extern/hyperliquid.ice)): an opaque
`Tape` handle, five async fetches and two streams that return checked structs,
a set of `sync` formatters and list folds, and one `component` adapter that
renders the chart from the tape plus the current fills, positions, and orders.
Candles never cross into Ice; everything the panels list does, because the
panels list it — and only that. A struct crossing the boundary carries the
fields the screen reads and no others, so the extern block stays a description
of the interface rather than of the exchange. A test holds it to that, because
the rule does not hold itself: five fields and one whole `sync` had drifted
across it before the test existed, and a declared function nothing calls is
how you find out that the edit meant to wire it up matched nothing.

The chart adapter reports back one `ChartSignal`: the candle under the cursor,
and whether the view has reached the oldest candle loaded. One handler reads
the first and guards on the second, so hovering and paging share a route
without the chart knowing what an exchange is.

## Tests

`cargo test -p trading-example` parses recorded payloads for every response
shape and checks the arithmetic they feed: the tape merge and its market
guard, how one aggressor's prints fold into one row, the book's depth and
spread, how fills stack and cool, the valuation between polls, and both rails.
It prices the ticket against the closed form of the liquidation it quotes, and
against the cases where it must refuse to quote one at all.

It also drives the app. The address prompt refuses a malformed address; the
ticket takes a price and a size, prices them, and closes on Escape; a search
survives being typed and clears on Escape; the panels that need an account say
so when there is none; and a failure outranks the progress line it shares a
slot with. None of those reach the network, so they run wherever the rest does.
One test reads the palette out of `theme.ice` and holds the chart to it, because
the chart is drawn in Rust and would otherwise drift in silence.

One market, one position, a book and a few prints live in the source as
fixtures, behind a named preset. Everything that only exists when an account does — the ticket's
figures, what an order would do to a position, what it asks for in margin —
is asserted against them, so the readings that were only ever visible in a
picture are now checked without one.

Two tests talk to the live exchange — one per endpoint shape, so the subscription names and payloads
are checked against Hyperliquid rather than against a recording, and the
account's own marks are fed back through the valuation to prove they are a
fixed point of it — and are opt-in:

```bash
cargo test -p trading-example -- --ignored
```

To capture the prompt as evidence:

```bash
ICE_TEST_ARTIFACT_DIR=target/trading-evidence \
  cargo test -p trading-example __ice_tests::trading_gate_gates_the_app -- --exact --nocapture
```

![Address prompt](screenshots/gate.png)

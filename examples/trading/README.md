# Trading

A live perpetuals terminal written in Ice, reading any of several networks:
the market list, candles and the order book for the selected market, and — for
any address you point it at — that account's open positions, resting orders,
recent fills, and every one of those fills marked on the candle it landed in.

A *network* is an exchange and one of its deployments — Hyperliquid,
Hyperliquid Testnet, Lighter — because one exchange can have more than one, and
holding "which exchange" and "which deployment" as two values is how a mainnet
book comes to price a testnet order with both halves of the screen looking
right. Which one is being read is named in the header beside a badge saying
whether being wrong on it costs anything; the picker is on settings. It is not
a filter over one exchange's data: networks disagree about which markets exist,
what they are called, and what the engine holds against a position in them, so
switching throws every panel away and reads the new one from nothing. What the
network being opened cannot answer is said in the panel that would otherwise be
empty — see [the network registry](#the-network-registry).

```bash
cargo run -p trading-example
cargo test -p trading-example
```

The app opens on an address prompt, prefilled with a well-known account so
there is something to look at on the first run. Press **Connect** to read it,
type your own, or **Browse markets** to use market data only; the terminal's
positions panel offers the prompt again if you change your mind, and so does
the settings page. Browsing without one, every panel that needs an account
says so rather than reporting that the account has nothing in it.

An address is checked before it is sent, because the exchange answers a
malformed one with a plain-text parser complaint rather than JSON — so without
the check, a typo reads as "Hyperliquid sent bad JSON", the one error that
blames the exchange for something you just typed.

![Trading](screenshots/trading.png)

## The menu bar mini status

On macOS the terminal also lives in the menu bar: a `tray` block keeps the
focused market's coin and last price beside a status icon, so the price is
readable with every window closed.

Left-clicking the item raises a native menu: the focused market's coin and
last price, the feed's latency, and **Quit**. The platform owns that menu — it
opens, places itself and dismisses itself — so the terminal declares no window
for it and subscribes to nothing but the row a reader chose.

A dead feed marks the label and the row `NOT LIVE`, the same words the header
stamps beside the price it greys. The menu bar is read without the header
there to qualify it, so a last price printed there in the words a live one
uses is the one stale figure nothing on screen would correct.

The terminal stays a `daemon` rather than an `app` so that closing the window
leaves it in the menu bar rather than exiting; **Quit** in the menu ends it. On
other platforms the same source builds and runs with the tray as a no-op.

If the status item ever looks inert, `ICE_TRAY_DEBUG=1 cargo run -p
trading-example` traces the native boundary: whether the menu was built, and
which row the platform reported.

## One terminal, and two pages beside it

Everything a market read needs is on one screen, because the readings are one
reading: the ticket is priced against the book, the tape says whether anything
is happening at all, and the position the next print re-marks is the reason
you are watching. Those were dealt onto four pages once, and the cost was
paid every time a beat landed — the chart moved on one page and the position
it re-marked on another.

| Surface | What it holds |
| --- | --- |
| **Terminal** | the market rail, the chart and its intervals, the book, the tape, the levels being watched, the order ticket, and the account's positions, resting orders and recent fills |
| **Portfolio** | what the account *is*, rather than what it is doing: equity and its parts, exposure, margin health, funding, and what the fill history says |
| **Settings** | the address being read, what the feed is doing, and what this app will and will not do |

The line is between watching a market and reading an account. The terminal is
the first, whole. The portfolio is the second, and it is not the terminal's
lists moved: it is folds over them — realized against unrealized, funding paid
against funding received, gross exposure against the equity behind it — which
are answers the rows cannot give you by being listed.

`Page` is an enum and the view is a `match` over it. There is no router,
because there is nothing to route: no history to walk, no path to parse, and
no state a page holds that the app does not already hold.

The header stays on all three. It carries the market, the account's equity and
the feed's round trip, and those three are true wherever you are standing. The
page tabs live in it, named by the act like every other control here:
`Show the portfolio page`, and the page already drawn appends
`, already showing` rather than renaming itself, because a button carries no
state a reader can hear.

Picking a market goes to the terminal — from the market rail, and from any
position, order or fill row, which have always been ways back to a market. A
row that answers a request by leaving you where you were has not answered it.

### What the dashboard refuses to say

Every figure on the portfolio is a fold over what the venue actually served,
and where a venue serves nothing the panel says so instead of totalling to
zero. Lighter serves this account's fills only to an API-key-signed token, so
REALIZED PNL reads *Not served here* and FILL HISTORY carries the reason; a
`$0.00` there would be the same pixels as a flat book and the opposite fact.
A win rate with no round trip closed reads `—`, not `0%`. Funding is drawn on
both venues, because both publish it per position.

### What the window costs, and what it does when it cannot have it

The window's minimum is **1180×720**, which is a 1366 or 1440 laptop, or a 14"
MacBook Pro at 1512. Drawn whole, one screen wants 1660×820 — but a minimum is a
promise about the smallest window, not about the roomiest layout, so below two
widths the terminal folds panes by priority instead of demanding the pixels.

The arithmetic is the positions table's. Its seven columns are 540px of fixed
widths, gaps and padding, and the panes beside it are fixed too: 232 markets,
232 book, 252 ticket, three rules — 719 — plus 311 for the recent-fills column.
So 540 + 719 + 311 = 1570 is the width below which recent fills has to fold, and
540 + 719 = 1259 is where the market rail follows it. The rule is written at
1580 and 1280, and it is a `responsive` node reading its own width rather than
anything the app stores.

Nothing that trades folds: the chart, the order book, the ticket, positions and
open orders are on the screen at every width. The tape is not folded either — it
lives inside the book column, which never collapses, so folding it would free no
width at all and cost the flow. Alerts stay for their own reason: 88 pixels is
the cheapest pane here and it is the one that says a level was hit.

What folds gets a toggle on the chart bar, beside the interval tabs, and never
beside the page tabs — a control up there would read as a fourth page, which is
the shape this screen was put back together to be rid of. Unfolding one brings
its pane back onto this same screen next to everything already on it. The rail
is a picker, so a pick folds it again and hands the 232 pixels back to the
table it borrowed them from.

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
gets better with more rows. The levels being watched are few and keep a fixed
slot under it.

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

A control is named by what pressing it does, not by what it currently reads. An
alert row is the button that drops the level, so it says `Stop watching BTC
above 64,400.00` rather than describing the level it is about to delete. The
interval tabs read by the same rule: six tabs spelled the same, all six named
`Show 1m candles` for the act. Which one the chart is drawing was said in
highlight colour and nowhere else, and that is a state rather than a second
act — a button carries no state a reader can hear — so the drawn one appends
it: `Show 1m candles, already showing`. Naming that one `Showing` instead would
be the one control on screen named for what it reads.

Funding is the one figure here written to four decimals. Two is what every
other number wants, and it is exactly what funding cannot use: the hourly rate
is a hundredth of a percent on most of the exchange, so 166 of the 177 funded
markets would print the identical `+0.00%`. A column of them on the markets
page is what those digits are for: four decimals down a column separates the
markets that pay from the markets that do not, and two decimals would draw the
same figure 166 times.

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

The ticket is 252 wide beside a 232 book, and it is one of the two panels that
set the terminal's share of the window's minimum.

An order that closes something ties up nothing and has no cliff, and the panel
says so: the trade still has a value, but the margin is zero and there is no
liquidation to quote, because nothing was opened. Past the position it is both
at once — all of it trades, only the excess opens, and only the excess can be
liquidated.

A position in this market puts a **close** on the ticket, which fills the size
that flattens it and takes the side that does — both read off the same signed
number, which is the only place the two agree by construction rather than by
you doing the sign in your head.

That fill only closes the position if the size survives being written down. A
size is quoted at the precision it carries, never at one picked from how large
it is: the venue quantizes a size to the instrument's step before it sends it,
so every digit that arrives is the market's and none of them may be dropped.
The old rule dropped them by magnitude — two decimals above 1, three above a
thousandth — so a market quoted a small size more finely than a large one, and
**close** on a position of 30.12345 filled in 30.12 and left a residual open.
Past the position the same rounding is a flip into the other side.

The one size this app works out rather than reads is the share buttons', and
that one is put back on the instrument's step: the market carries its
`szDecimals` beside its maximum leverage, because a step is the asset's fact
and not the day's. It rounds down, because a MAX rounded up is an order the
margin engine refuses. On a market that trades in whole units that rounding can
take the whole size away, and then the button offers nothing rather than `0`:
an order for none of the instrument is not a size the panel may fill in.

The same number may not be written two ways on one screen. The header's PnL is
the sum of the positions the portfolio lists, so with one position open it is
that position's PnL — and it was exact where the row beside it was compact,
`-$30,000.00` above `-$30.0K`. Both are written by the rule the fills already
used: exact while it is small enough to read, compact once it is not.

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
showing is the one number here that must never be wrong — which is why the
readout is written to the leverage that priced it, decimals and all, out to the
hundredth — the field is free text and the cell is a fixed width, so the digits
stop where a leverage stops being one. It used to round to a whole number, so a
ticket levered at 2.5 said `3x` beside a margin and a cliff computed from 2.5,
and the panel showed one number while using another.

Escape closes it, and the subscription that listens for Escape exists only
while it is open. Escape with a search in the box clears the search instead,
on its own subscription with its own condition — the terminal, showing, with
something typed — so neither key listener exists when there is nothing for it
to do, and Escape on the portfolio does not clear a box that is not there.

Nothing is signed and nothing is sent, and the ticket says so where a submit
button would be a heading: it opens by naming what it does and what it does
not, rather than closing with a badge. What may be signed is now a real
question with an answer — see [custody](#custody) — but nothing is wired to
this ticket yet, and the settings page says exactly that rather than implying
otherwise. The boundary is still the interesting part: everything up to the
signature is arithmetic worth having.

The one figure in that arithmetic that is not arithmetic is the maintenance
requirement, and it belongs to the venue: Hyperliquid holds half the margin at
a market's maximum leverage, and another exchange holds something else. So the
market carries it and the ticket reads it, rather than the shared math knowing
one exchange's rule. It is stated once, next to the parser that knows whose
rule it is.

## Custody

The app can hold one key and it is not the account's. An **agent key** is a
separate keypair the account's own wallet approved at the exchange: it places
and cancels orders, it cannot withdraw, and the exchange stops honouring it on
a date the exchange chose. Losing it costs an approval, not a balance. That
property is the whole reason there is a key here at all.

On macOS its secret lives in the keychain behind Touch ID — one guarded
generic-password item, biometry or passcode, this device only, and not in this
process or in a file. Unlocking is that prompt. On a build without a keychain
there is nowhere to keep a secret and nothing to unlock, and the panel says so
rather than offering a prompt that can only refuse. The keychain item is filed
under the deployment *and* the address, because a key approved on mainnet is
unknown on testnet and a secret read back under the wrong one is a signer the
venue has never heard of.

Three acts, and the app cannot perform the middle one:

1. **NEW KEY** generates an agent key, stores the secret, and prints the
   address.
2. **Approve** — done by the account's own wallet, at the exchange, somewhere
   that is not this app. An `approveAgent` is signed by the master wallet,
   which is the one key this design exists to avoid holding.
3. **UNLOCK** raises Touch ID, reads the secret back, asks the venue which of
   this account's keys are live, and a listing naming ours is what a tradeable
   session is made of.

The middle step being somebody else's is the property, not a gap.

### What the screen says, and what it must never blur

The header carries a badge on every page, beside the equity strip rather than
instead of it: the two answer different questions, and they disagree the moment
there is a key to hold — an unlocked session over an address with no account,
and a read-only session over a funded one, are both ordinary. It reads
`UNLOCKED` only while the session may actually trade, and it takes the clock to
say so, because a window closes on the exchange's schedule rather than on an
event: a badge read off the state alone would keep saying yes through every
millisecond between expiry and the next tick, and forever if the ticks stop —
which is exactly when a laptop that slept through an expiry starts asking.
`KEY EXPIRED` is its own word, because a reader whose key lapsed has something
to renew and a reader who never had one has something to make.

A **refusal and a fault are not the same event** and the panel must not draw
them alike. Cancelling Touch ID released nothing, is nobody's mistake, and
leaves the button live with a sentence beside it. A keychain that failed, or a
build that has none, is not a thing to ask again — the keystore is chosen at
compile time, so a second prompt fetches the same refusal — so the button goes
dead and states the platform's own words. `session.rs` draws that line in a
pure function outside every `cfg`, which is why a Linux machine can test it;
this seam's job is not to blur it on the way to the screen.

A network this app cannot sign for **yet** is refused before any sheet, by
name. Lighter is that case today, and the distinction is worth stating
precisely because the first version of this sentence got it wrong: the *venue*
has a write path. Lighter's orders are L2 transactions signed by an API key the
account registers — the `api_key_index` in the token `lighter_sign.rs` already
mints, over the curve it already implements. What is missing is this app's
transaction half. "Nothing in this module can place an order" is that module
saying what it implements, not what Lighter can do.

So `Network.chain` is `None` there, meaning "no write path here yet", and the
refusal says the same. Both stop being true when the Lighter path lands, and
both go in that change rather than outliving it.

Changing network or address forgets the key. Carried across either, it is a
session claiming the app may trade somewhere the key is unknown, and the first
thing that would say otherwise is a rejected order.

### What needs a Mac

Everything decidable without a Keychain is decided in CI, on Linux: the state
machine exhaustively, this seam's projections and both refusals, and the panel
in every state a preset can put it in. None of it touches a keychain, because a
build without one answers `Unavailable` — a state with a test rather than a gap.

What no runner reaches is the sheet. `security-framework` is a macOS-only
dependency, so the macOS jobs compile that path and nothing executes it: a CI
runner has no window server to raise a Touch ID sheet in front of and no
enrolled finger to answer it with. The nine experiments `session.rs` lists on
its `impl Keystore` are still owed, and `custody.rs` adds four that only exist
now that something calls it — one sheet per unlock rather than two, a cancelled
sheet leaving a live button, the full enrol-approve-unlock round trip, and a
re-enrolment keeping the secret it replaces when the add fails. Until a person
on a Mac reports those, the honest claim is that this seam's logic is tested and
its platform half is compiled, reviewed and unrun.

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

## Funding, as the money that moved

Both venues report funding as a CHARGE — positive means the position paid —
and that is the sign the arithmetic wants, so the field keeps it. A column
headed FUNDING means something else to the person reading it: money that left
the account is negative there, the way it is in every other money column on
this screen.

Shown as the charge, a position that had been paid funding read as a loss, in
the colour a loss is drawn in. The column shows the flow now, and the colour
follows it.

## The equity the engine can actually spend

A cross position is liquidated against the account's cross equity, not its
total. Margin posted behind an isolated position is not money the cross engine
can call on, and dividing the cross requirement by the total reads an account
at its cliff as comfortable by however much is locked away. The venue reports
both — `crossMarginSummary.accountValue` beside `marginSummary.accountValue` —
and the rail takes the first while the EQUITY figure keeps the second.

## What the reader typed, and what they were quoted

Every figure on the ticket is priced at the market's cap, because that is what
the margin engine will hold the order to. The share buttons were the exception:
they levered at the number typed into the field, so MAX on a market capped at
10x with 40 typed filled in four times what the account could deploy. They take
the leverage the ticket was priced at now, and it is an `f64` rather than the
raw string, so the old form no longer compiles.

## What a beat does

A beat is where most of this app happens: it applies every mid, re-marks
every position, folds the prints into the tape, checks the levels, and
reprices the ticket. None of that had been walked, because a beat needs a
`MarketTick` and nothing could produce one outside the feed.

A fixture beat can, so a test moves bitcoin to 64,500 and then to 63,000 and
reads the screen. The mark follows, the short's unrealized goes from
+$523.8K to +$508.8K to +$553.8K, and the level waiting at 64,400 loses its
arrow on the way up and does not get it back on the way down. That last part
is the one-way firing rule, and until now it had only been checked as
arithmetic.

## A failure that ends when the thing it describes ends

The feed's failure is the feed's, so it is held apart from the errors a
request returns and cleared by the next beat. It used to share one field with
them, and nothing on the beat path cleared it: the socket would come back,
prices would move, the NOT LIVE badge would go, and the message saying the
feed had dropped would sit there until a poll happened to overwrite it —
sixty seconds with no address to poll for.

Two statements on one screen contradicting each other, which is the same
shape as an equity bar reading 38% beside a rail reading nothing travelled.

A message about a live socket cannot outlive the socket being dead, so the
badge and the message now go together, and a dispatched beat in the test
proves it.

Leaving an address is the same rule against a different clock. The gate opens
over the terminal rather than replacing it, so whatever the last address put
on screen is still drawn behind it — which is why going back to the prompt
clears the fills, the orders, the positions, the account, the live badge and
its round trip. Both lines in the chart's own bar go with them. The feed's
complaint describes a socket that `abort feeds` is closing on the way out, and
a request's failure names the address the request was made for: every fetch
this app makes is made for one account, so `Hyperliquid unreachable` about the
account you just left is a failure reported over the next one's positions.
That is the same defect twice, and the same fix.

## Lists longer than the panels that hold them

Every capture held four markets and three prints, and a list that fits
answers nothing about a list that does not. There is a venue-sized fixture
now — twenty four markets and a tape at the depth the feed keeps it — and
the terminal is captured against it.

The long list is generated rather than typed, and checked by the same test
as the short one: volume descending, one selection, each maintenance half
the margin at its cap, each change the price against its own close. A
generated fixture that broke the ordering the panel assumes would be a panel
drawing in the wrong order, quietly.

Nothing was clipped. The book, the tape, the alerts, the orders and the
market list each scroll inside their own bounds, and the half row at a
boundary is the affordance saying so. They share a screen again, so one
capture holds all of them at once.

## Two states nothing had drawn

A position the venue reports no liquidation price for says `none` rather
than printing a zero, and the rail beside it is empty because there is
nothing to travel toward — not because nothing has been travelled. Those
read the same and mean opposite things, so the fixture holds one.

The crosshair's readout — open, high, low, close and volume of the candle
under the pointer — had shipped without ever being captured. It is drawn
from a candle taken out of the fixture tape, so what the row says is what
the chart is drawing under it.

A picture is not an assertion, though, and the fixture tape walks a sine: five
figures a test could only agree with by transcribing them, and a transcription
checks the arithmetic against a copy of itself. The test asks the fixture for
the candle instead. It asks each cell rather than the strip, too: four of the
five are prices a few dollars apart, so a strip that holds all five holds them
just as convincingly with the close under the `O`. Each cell is asked for its
own letter and its own figure, so the readout labelling its close as an open
fails rather than captures.

Every capture now asserts the ticket does not say `market not loaded`. The
handlers reprice, but a preset sets state directly and bypasses them, so
each new fixture could reintroduce a bug that was fixed three cycles ago —
and the one added in this cycle did, before the assertion caught it.

## A market worth a fraction of a cent

Most of a perp venue is priced under a dollar, and every column here was
sized and formatted around bitcoin. The fixture holds one now — kPEPE at
0.008421 — and the terminal is captured focused on it.

It found the chart tagging the last price `0.00842` while every other panel
said `0.008421`. The chart derived its decimals from the gridline step, which
is enough to tell two gridlines apart and not enough to tell two ticks apart.
Those are different questions: the axis answers "which line is this" and the
tag answers "which price is this". The tag is written to the instrument's
quote precision now, and the axis to whatever its own step needs.

The book and the tape take a tick as well as a price. A dollar between levels
is a reasonable book on bitcoin and the whole market on a coin worth a
fraction of a cent.

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
11px text in the bar under the chart.

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

## One exchange, several books

Hyperliquid is not one market list any more. HIP-3 lets anyone deploy a perp
dex on the same exchange, and read live the day this was written `perpDexs`
answered with the canonical universe plus nine builder deployments — `xyz`
("XYZ") alone listing 94 live markets against USDC, `hyna` ("HyENA") 18 against
USDe. Flattened into one rail those read as Hyperliquid's own markets, which
they are not.

So the rail is grouped: the exchange's own perps first, then each builder dex
under the name it deployed with. A group that settles in something other than
the exchange's own collateral says so beside its name, because that is the fact
that separates two lists which otherwise look interchangeable. One group is not
a categorization — a venue listing one flat universe is drawn with no headings
at all, which is the whole of what Lighter's rail does.

Grouping organizes one list; it does not create destinations. There is no dex
page, no dex filter and no dex tab. The search box still reaches every category
at once, and a group whose first row a search removes is re-headed by whatever
is left of it — the heading is decided by the filter that orders the rows
rather than stamped when the universe is read. A header is a heading, so each
row also names its own group and collateral in its accessible name: a reader
moving row by row does not carry a header down the list with them.

### The name is the whole identity

A builder market is named `dex:SYMBOL` on the wire, and that string is all that
is needed to address it. Verified live: `l2Book` and `candleSnapshot` answer
for the coin `xyz:NVDA` with no `dex` parameter at all, and answer `null` for a
bare `NVDA`; the `l2Book`, `trades`, `candle` and `activeAssetCtx` websocket
subscriptions take it the same way. So picking one from the rail charts, quotes
and books it through exactly the paths every other market uses.

The one thing that cannot take it apart is the tape's focus, which holds the
market and interval as one colon-joined key. Split from the left, `xyz:NVDA:1m`
reads as the coin `xyz` at the interval `NVDA:1m`, and every subscription the
feed then holds is for a market that does not exist. It splits from the right.

`allMids` is the exception on the read side: it answers for one dex at a time,
so the feed holds one subscription per dex and merges what they carry. Assigned
rather than merged, the last message of a beat would be the only prices the
rail saw.

### What the ticket will not quote

A builder dex is a separate clearinghouse, not a section of the exchange's own.
Read live, one address held $127,575 against canonical Hyperliquid and
$5,235,542 against the `xyz` dex in the same second — four open positions on
the second, none on the first.

So every ticket figure measured against the account on screen is about an
account the order would never touch. AGAINST THE ENGINE says *separate margin
account* instead of quoting the wrong one, and the share buttons decline to
size a position out of a balance that is not there. A wrong liquidation price
is the worst lie this app could tell, and a confident wrong margin load is the
same lie with a percent sign.

What is not gated is the market's own arithmetic, because it is the market's.
The maintenance rule holds across dexs — checked live, `xyz:SKHX` at 10x
maintains at exactly 1/20th of its position value, the same half-of-max-leverage
rule the app already prices canonical markets with — so ORDER VALUE, PRICED AT
and the isolated LIQUIDATION are quoted for a builder market exactly as for any
other. MARGIN REQUIRED is quoted in the token it is actually posted in: `hyna`
margins in USDe, `flx`/`vntl`/`km` in USDH and `cash` in USDT0, and a dollar
sign in front of any of those is the panel claiming a peg it never checked.

Placing an order on a builder dex is not in scope here — the ticket sends
nothing on any venue. What is in scope is that it never quotes a figure it
cannot stand behind.

### Lighter has no equivalent

Checked rather than assumed. `/orderBookDetails`, the endpoint this app reads
Lighter's universe from, answers with 222 markets whose `market_type` is `perp`
and nothing else, and not one of its forty-four per-market fields names a
deployer, a builder, a sub-exchange or a second collateral; `quote_asset_id` is
0 on every one of them. There is no Lighter equivalent of a HIP-3 dex to
reflect, so Lighter's rail is drawn as one flat list with no headings over it.
Inventing a single group to sit under a header for symmetry with the other
venue would be a categorization the venue does not have.

## The venue switch

## The network registry

`NETWORKS` in `venue.rs` is the only place a network is enumerated. One entry
carries its name, whether it is a test deployment, the `Chain` a signature made
for it would be pinned to, the reads it answers, the sentence it owes when it
will not answer one, and the note a reader needs beyond its name. Adding a
network is that entry, the `Venue` variant it names, and the arm `Network::of`
will not compile without — the exhaustive match is the point, because a network
whose reads were wired and whose capability sentence was forgotten is a screen
that silently claims the wrong exchange answers a panel it will leave empty.

Two facts that used to be constants in `hyperliquid.rs` are entries here now.
The `info` endpoint is `Chain`'s, and the heading over the exchange's own perp
group is the network's own name — a rail on the test deployment headed
"Hyperliquid" over markets that are not the live exchange's would be the one
place on screen that contradicts the header. What canonical Hyperliquid margins
in stayed a constant, because it is one: both deployments settle their own
canonical perps in their own USDC, and what differs is which builder dexs are
listed beside them. Read live, testnet answers `perpDexs` with around a hundred
of them — `test dex`, `unit dex` — where mainnet answers five.

Nothing else enumerates them. The picker is a loop over `venue_list()`, so a
network added in Rust is drawn on settings without the view being touched, and
a Rust test holds `venue_list()` to the registry's own length and `Network::of`
to round-tripping every entry — that arm is the one a copy-paste gets wrong,
and `Venue::HyperliquidTestnet => Network::HYPERLIQUID` compiles, draws the
right name, and points every read on the testnet at mainnet.

### Which deployment, and what it costs to be wrong

Every read is addressed to a `Chain`, and `Chain` is also what a signature is
pinned to — it is the phantom agent's `source` and the user-signed
`hyperliquidChain`, so a mainnet signature cannot be replayed on testnet. One
value carrying both is what stops the screen and the order disagreeing about
which deployment is being traded; the endpoints for reads live on it beside the
one for writes for the same reason.

The header names the network and, under it, states its kind: **REAL MONEY** or
**TESTNET**, in a box either way, with only the colour moving. Both are stated
on purpose. A badge drawn only on testnet is a badge whose absence carries the
dangerous half of the message, and nobody notices an absence — so the network
that can lose money says so in the same place and the same shape, and the
reader learns where to look on the day it is free to get wrong rather than on
the day it is not. Every row of the picker says its own kind for the same
reason: a picker is where this mistake is actually made, so the row a finger is
travelling towards has to answer it before it is pressed.

Hyperliquid's test deployment answers every read the live one does, over the
same protocol and the same parser, so its panels fill exactly like mainnet's.
What is different about it is a note on settings rather than a sentence under
an empty panel — written there it read as a venue refusing to serve orders,
which is the opposite of what a testnet is for. A live test drives the seam and
asserts the two universes *disagree*, because a testnet reading mainnet's
markets passes "the reads work", draws a plausible screen, and prices orders
against a book its own exchange has never seen.

### What switching throws away

The network already being read is still a button, and a button carries no state
a reader can hear — so it says which it is in its own accessible name (*Read
Lighter* against *Read Lighter, already reading*) rather than in its highlight
colour.

Pressing it is not a filter and not an undo. Everything on screen belongs to
the exchange it was read from, and a row kept across the switch would be drawn
under the other exchange's name and look entirely plausible: a book at one
venue's prices under the other's mid, a position at a maintenance requirement
that market does not hold. So `switch_venue` clears the universe and the
focused row, the book, the tape, the levels being watched, the account, its
positions, orders and fills, the ticket's price and size, the chart's hover and
the feed's own reading — and hands the feeds a **new** tape rather than a
re-pointed one. That last one is the defect that hides: `tape_focus` turns away
candles for a market it was not asked for, and both venues ask for the same
market at the same width, so the feed being aborted would go on merging its own
candles into the chart the next venue is drawing until its thread noticed. A
Rust test holds the switch to handing over a tape the old feed cannot reach.

Switching to the venue already on screen is not a switch, and returns early
rather than re-reading a loaded terminal.

The picker is on settings rather than in the header because a list that grows
with the registry cannot live in 58 pixels, and because switching network
throws the whole screen away and can change which network your money is on —
which is a deliberate act rather than a header toggle. The header carries the
answer, not the choice, and is the same shape on every network.

The header was already exactly full at the window's minimum, so naming the
network had to be paid for: the account strip gave up its **FREE** figure. It is the one thing there that
does not move between polls — the margin engine answers what is withdrawable
once every five seconds, and it is a tile on the portfolio page — so it was
the cheapest of the five to lose. Nothing is clipped: everything the header
still carries is on screen at the minimum size, and the page tests run there.

### What each venue can serve today

| | Hyperliquid | Lighter |
| --- | --- | --- |
| Market list, with the day's figures and the margin rule | yes | yes |
| Mid prices, book, public tape, market context | yes, on the socket | yes, on the socket |
| Candle history | yes — 500 bars on open, 500 more per pan back | yes — the same, at `GET /candles` |
| Live candles | yes | yes |
| Account equity, margin and positions | yes | yes, on the 5s poll |
| Resting orders | yes | **no** |
| Fills, as they print | yes, on the socket | **no** |
| Liquidation prints on the tape | yes | **no** — see below |

The gaps are stated on screen rather than left as empty panels, because an
empty list reads as *nothing has happened* and on Lighter nothing *can* happen:

- **Orders and fills.** Lighter keys its account channels by account index
  rather than L1 address, and the order and notification channels want an
  API-key-signed token an address alone cannot get (`code 20001`). An address is
  all this app asks a reader for. Both panels say so where their rows would be,
  and the settings page says it once more beside the venue's name. Connecting
  an address would not change it, so the sentence does not offer that.
- **Liquidation prints.** Lighter carries them in a second array on the trade
  channel, keyed to merge by trade id, and the copy that arrives with the
  subscription is hours stale — so including them would put old prints on screen
  as new. The tape is the venue's ordinary prints only.

A gap answers empty rather than failing. A venue that does not carry a channel
has not broken, and an `Err` here would raise the app's alarm line over
something working exactly as documented.

### What a second venue has to provide

The panels, the folds, the ticket's arithmetic, the formatters and the chart
adapter do not know which exchange they are looking at. What does is a short
list, and it is the whole of a third adapter:

| | |
| --- | --- |
| Two endpoints | one REST, one websocket |
| Seven answers | the universe, a candle window, the window before it, an account, its resting orders, and the two feeds — the fields of `Reads` in [`src/venue.rs`](src/venue.rs) |
| Five channels | mids, book, market context, candles, and this account's fills |
| One field map per response | every number arrives as a string on Hyperliquid; Lighter mixes strings and numbers in one object, sometimes for the same quantity |
| One margin rule | the share of a position's value held against it — Hyperliquid keeps half the margin at the market's maximum leverage, Lighter publishes both fractions in basis points and its maintenance is *not* half its initial |
| One interval vocabulary | `1m`, `5m`, `1h` are each venue's own spelling; an interval a venue does not quote is refused rather than drawn at the wrong width |
| One side encoding | `B`/`A` on Hyperliquid, a maker-is-ask flag on Lighter |

Everything else is already venue-neutral, and the boundary test keeps it that
way: `SymbolRow`, `Position`, `Account`, `Book`, `Trade`, `Fill`, `Order` and
`Ticket` are shapes the panels read, not shapes either exchange returns.

The one thing not yet done is the module split that would move those neutral
shapes out of `hyperliquid.rs`, where the second adapter still imports them
from. It is mechanical, and `Fetch`, `HlError` and `Event` are misnamed until
it lands.

## What talks to the exchange

Ice cannot choose a function at the call site, so every read the app makes is
one `venue_*` extern taking the venue it is holding, and the choice is made in
Rust against `Reads` in [`src/venue.rs`](src/venue.rs) — one table per
exchange, so a venue that cannot answer something has to say so there rather
than in a handler. The two adapters are
[`src/hyperliquid.rs`](src/hyperliquid.rs) and
[`src/lighter.rs`](src/lighter.rs); neither is named anywhere under `src/ui`.

Everything an exchange pushes arrives on a websocket; everything it only
answers when asked is a blocking `ureq` request moved off the UI thread with
`smol::unblock` — one POST to `info` on Hyperliquid, a REST path with a query
string on Lighter.

Two sockets, each a thread pumping into a channel that Ice consumes as a
`stream`:

| Ice stream | Hyperliquid subscriptions | Lighter channels | Feeds |
| --- | --- | --- | --- |
| `venue_market_feed` | `allMids`, `l2Book`, `activeAssetCtx`, `candle`, `trades` | `market_stats/all`, `order_book/<id>`, `trade/<id>`, `candle/<id>/<res>` | every mid price, the book, the header's figures, the live candle, and the public tape |
| `venue_fill_feed` | `userFills` | — | a snapshot of recent fills, then each new one as it prints |

| Ice call | Hyperliquid | Lighter | Reads |
| --- | --- | --- | --- |
| `venue_symbols` | `metaAndAssetCtxs` | `orderBookDetails` + `fundingRates` | the tradeable universe: tickers, maximum leverage, the margin rule, and the day's volume |
| `venue_candles` | `candleSnapshot` | — | 500 candles when a market or interval is opened |
| `venue_history` | `candleSnapshot` | — | 500 more, ending where the tape begins, when the chart is panned back that far |
| `venue_account` | `clearinghouseState` | `account` | equity, margin, open positions with PnL, ROE, leverage, and funding paid |
| `venue_orders` | `openOrders` | — | resting orders, listed with their age and drawn on the chart as levels |

The three account reads share one rule, and it lives at the seam rather than in
the handlers: no address is not a failure and not an empty account, it is a
read the app did not make. `venue_account` answers `none`, `venue_orders` an
empty list, and `venue_fill_feed` a stream that has already ended. A handler
could not hold that guard anyway — a task group has to be a handler's last
statement, so guarding one read in Ice would mean a second copy of the whole
group.

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

A failure and a progress message share one slot in the bar above the chart,
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

`venue_candles` keeps one tape for as long as one venue is being read. An empty
tape backfills 500 candles and adopts the market that filled it, and the feed
replaces the live candle in place from there. Switching markets re-points the
tape, and a response — or a pushed candle — for the market you just left is
dropped instead of overwriting the one you are looking at. Switching venues
does not re-point it: it hands over a new one, because the market the old feed
was asked for is the market the new one is being asked for, and only a tape the
old feed cannot reach severs that. Panning back past the oldest candle asks
`venue_history` for the window before it, once per tape length; the chart moves
its own viewport by however many candles land in front of it, so the screen
stays on the bars it was showing.

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
retained-identity/environment `sync` calls confined to top-level state
initialization and immediately evaluated handler expressions, a set of `pure`
formatters and list folds, and one `component` adapter that renders the chart
from the tape plus the current fills, positions, and orders.
Candles never cross into Ice; everything the panels list does, because the
panels list it — and only that. A struct crossing the boundary carries the
fields the screen reads and no others, so the extern block stays a description
of the interface rather than of the exchange. A test holds it to that, because
the rule does not hold itself: five fields and one whole function had drifted
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
slot with, in the terminal's plain ink rather than in either money colour.
None of those reach the network, so they run wherever the rest does.

A categorized universe has fixtures and tests of its own. The rail heads the
exchange's own perps and each builder dex; a search reaches every category and
re-heads what it leaves; each row announces its group and collateral; picking a
builder market opens every panel at its qualified name and the self-pick guard
still reads it as one market rather than as a dex; and the ticket declines the
account it is not held against while still quoting the market's own cliff. The
colon-joined focus key has a test of its own at the Rust boundary, because a
focus split from the left is not visible as a failure — it publishes a beat
with no book, no prints and no context, and the panels sit empty over a market
the rail says is selected.

The live opt-in run asserts the shape rather than a named dex: builder
deployments are third-party and come and go, so it checks that a second group
exists at all, that its markets are named `dex:SYMBOL`, that each states what it
settles in, and that the exchange answers a dex-qualified coin's book as itself.

The network switch has a file of its own. Switching names a panel at a time and
asks for what was in it — the book, the tape, the levels, the market list, the
account, its positions, orders and fills — and switching back reads the first
network again rather than restoring what was on screen before it left. Every
socket and every REST read refuses to open under test, so a dispatched switch
starts the feeds of the network being opened without any of them reaching an
exchange; a feed that cannot reach one ends rather than retrying, or the app
would never settle and no test could dispatch a switch at all.

Two of those tests are about the label rather than the data, because the label
is the whole of what separates two screens drawn from the same fixtures. One
asserts both kinds in the header — **REAL MONEY** present and **TESTNET**
absent on the live network, and the reverse after a switch — and one asserts
that every registry entry reaches the picker with its kind beside it. Each was
run against a mutation before it was kept: a `venue_kind` that always answers
**REAL MONEY** fails three of them on the missing word, a `venue_list` that
filters test deployments out fails the picker on the missing row, and an arm
resolving `HyperliquidTestnet` to the mainnet entry fails five.

There is one test per surface, and each asks for something only that surface
draws — a test that asserted the header would pass on all three, which is the
failure a navigation test exists to catch. They run at 1660×820, the size one
screen wants when it can have it. Each captures its page, and the portfolio is
captured twice because it is taller than the window it opens in.

Two more run at the two ends of the responsive rule, because a fold nobody
asserts is a fold nobody notices breaking. The wide one holds every pane on the
screen and both toggles off it. The narrow one runs at 1180×720, the window's own
minimum, and asks for the five panes that may not fold, then that the two that
did are gone, then that positions still has all seven of its columns — which is
what the folding was for. It presses the markets toggle, gets the rail back
without leaving the page, picks a market from it, and finds the table whole
again.
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

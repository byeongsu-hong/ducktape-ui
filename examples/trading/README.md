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
panel offers the prompt again if you change your mind.

![Trading](screenshots/trading.png)

## Design

The screen is an instrument panel, so it is set like one. Every figure is
Monoplex KR and every word is IBM Plex Sans KR — the same skeleton drawn twice,
one monospaced so a column of prices aligns on its digits, one proportional for
prose. The ground is a warm ink-black rather than the blue-black every exchange
ships, and the two money colours are a ledger green and an oxide red: printer's
inks, not phosphor. Nothing else on screen — no button, border, tab, or rule —
is allowed to be green or red, so long and short read at a glance.

The one thing this layout gives you that an exchange table does not is the
**risk rail** under each liquidation price: a bar showing how far the mark has
travelled from your entry toward the cliff. Distance to liquidation is the
number a leveraged position actually turns on, and it is the one number every
table makes you compute. Here it is a length.

A fill the account just printed is pushed onto the top of the list wearing its
side's colour, which fades over two beats and leaves the row cold. It is the
only motion on screen that is not a number changing, so it is the only thing
that can mean *something happened while you were looking elsewhere*. The
divider under the chart drags: positions and fills are worth more rows on some
days than others.

## What talks to the exchange

Everything the exchange pushes arrives on a websocket; everything it only
answers when asked goes through the `info` endpoint as a blocking `ureq` POST
moved off the UI thread with `smol::unblock`. Both live in
[`src/hyperliquid.rs`](src/hyperliquid.rs).

Two sockets, each a thread pumping into a channel that Ice consumes as a
`stream`:

| Ice stream | Subscriptions | Feeds |
| --- | --- | --- |
| `hl_market_feed` | `allMids`, `l2Book`, `activeAssetCtx`, `candle` | every mid price, the book, the header's figures, and the live candle |
| `hl_fill_feed` | `userFills` | a snapshot of recent fills, then each new one as it prints |

| Ice call | Request | Reads |
| --- | --- | --- |
| `hl_symbols` | `metaAndAssetCtxs` | the tradeable universe: tickers, maximum leverage, and the day's volume |
| `hl_candles` | `candleSnapshot` | 500 candles when a market or interval is opened |
| `hl_history` | `candleSnapshot` | 500 more, ending where the tape begins, when the chart is panned back that far |
| `hl_account` | `clearinghouseState` | equity, margin, open positions with PnL, ROE, leverage, and funding paid |
| `hl_orders` | `openOrders` | resting orders, listed and drawn on the chart as levels |

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

The market feed re-reads the tape's focus on every beat, so switching markets
costs an unsubscribe and a subscribe on the socket already open rather than a
new connection. Candles are merged straight into the shared tape and never
cross into Ice: the chart repaints on its own 100ms beat, so a tick costs no
app message and no view rebuild. Everything else the feed carries is coalesced
into at most one message per beat, and each one holds the latest of everything
rather than only what changed, so a handler can assign it without asking which
kind of update it was.

Latency is the round trip of the socket's own ping, which needs no agreement
between our clock and the exchange's, and the ping is required anyway: a socket
that goes quiet for a minute is closed.

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

## Boundary

One extern block
([`src/ui/extern/hyperliquid.ice`](src/ui/extern/hyperliquid.ice)): an opaque
`Tape` handle, five async fetches and two streams that return checked structs,
a set of `sync` formatters and list folds, and one `component` adapter that
renders the chart from the tape plus the current fills, positions, and orders.
Candles never cross into Ice; everything the panels list does, because the
panels list it.

The chart adapter reports back one `ChartSignal`: the candle under the cursor,
and whether the view has reached the oldest candle loaded. One handler reads
the first and guards on the second, so hovering and paging share a route
without the chart knowing what an exchange is.

## Tests

`cargo test -p trading-example` parses recorded payloads for every response
shape, checks the tape merge, the market-switch guard, the book's depth and
spread, how pushed fills stack and cool, and the risk-rail arithmetic, then
renders the address prompt headlessly. Two tests talk to the live exchange —
one per endpoint shape, so the subscription names and payloads are checked
against Hyperliquid rather than against a recording — and are opt-in:

```bash
cargo test -p trading-example -- --ignored
```

To capture the prompt as evidence:

```bash
ICE_TEST_ARTIFACT_DIR=target/trading-evidence \
  cargo test -p trading-example __ice_tests::trading_gate_gates_the_app -- --exact --nocapture
```

![Address prompt](screenshots/gate.png)

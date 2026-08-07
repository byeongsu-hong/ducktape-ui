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

## What talks to the exchange

Everything goes through Hyperliquid's one `info` endpoint
([`src/hyperliquid.rs`](src/hyperliquid.rs)), a blocking `ureq` POST moved off
the UI thread with `smol::unblock`:

| Ice call | Request | Reads |
| --- | --- | --- |
| `hl_symbols` | `metaAndAssetCtxs` | tradeable perps, mark price, 24h change, volume, open interest, funding |
| `hl_candles` | `candleSnapshot` | OHLCV for the selected market and interval |
| `hl_book` | `l2Book` | ten levels a side, with cumulative depth and the spread |
| `hl_account` | `clearinghouseState` | equity, margin, open positions with PnL, ROE, leverage, and funding paid |
| `hl_fills` | `userFills` | executed trades, marked on the chart and listed beside it |
| `hl_orders` | `openOrders` | resting orders, listed and drawn on the chart as levels |

Responses are read as `serde_json::Value` and mapped by hand, because the
exchange sends every number as a string — a derive would need a custom
deserializer per field. Prices, sizes, and PnL that are missing or unparsable
read as zero rather than failing the whole poll.

Polling replaces a websocket here: markets, candles, and the book every 3s, the
account and its orders every 5s, fills every 30s, and all of it stops while the
address prompt is up (`when` conditions on the `subscribe` block, so iced
actually drops the timers instead of ignoring their messages).

`hl_candles` keeps one tape for the whole session. An empty tape backfills 500
candles and adopts the market that filled it; a loaded one asks only for the
candles that can still change and merges them in place, replacing the live
candle and appending closed ones. Switching markets re-points the tape, and a
response for the market you just left is dropped instead of overwriting the one
you are looking at.

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
`Tape` handle, six async fetches that return checked structs, a set of `sync`
formatters, and one `component` adapter that renders the chart from the tape
plus the current fills, positions, and orders. Candles never cross into Ice;
everything the panels list does, because the panels list it.

## Tests

`cargo test -p trading-example` parses recorded payloads for every response
shape, checks the tape merge, the market-switch guard, the book's depth and
spread, and the risk-rail arithmetic, then renders the address prompt
headlessly. One test talks to the live exchange and is opt-in:

```bash
cargo test -p trading-example -- --ignored
```

To capture the prompt as evidence:

```bash
ICE_TEST_ARTIFACT_DIR=target/trading-evidence \
  cargo test -p trading-example __ice_tests::trading_gate_gates_the_app -- --exact --nocapture
```

![Address prompt](screenshots/gate.png)

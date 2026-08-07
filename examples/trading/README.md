# Trading

A live Hyperliquid front end written in Ice: the perpetuals list, candles for
the selected market, and — for any address you point it at — that account's
open positions with unrealized PnL, its entry and liquidation levels drawn
across the chart, and every fill marked on the candle it landed in.

```bash
cargo run -p trading-example
cargo test -p trading-example
```

The app opens on an address prompt. Paste an address to read an account, or
**Browse read-only** to use markets only; the positions panel offers the
prompt again if you change your mind.

![Trading](screenshots/trading.png)

## What talks to the exchange

Everything goes through Hyperliquid's one `info` endpoint
([`src/hyperliquid.rs`](src/hyperliquid.rs)), a blocking `ureq` POST moved off
the UI thread with `smol::unblock`:

| Ice call | Request | Reads |
| --- | --- | --- |
| `hl_symbols` | `metaAndAssetCtxs` | tradeable perps, mark price, 24h change, volume, funding |
| `hl_candles` | `candleSnapshot` | OHLCV for the selected market and interval |
| `hl_account` | `clearinghouseState` | account value, margin, open positions with PnL and ROE |
| `hl_fills` | `userFills` | executed trades, marked on the chart |

Responses are read as `serde_json::Value` and mapped by hand, because the
exchange sends every number as a string — a derive would need a custom
deserializer per field. Prices, sizes, and PnL that are missing or unparsable
read as zero rather than failing the whole poll.

Polling replaces a websocket here: markets and candles every 3s, the account
every 5s, fills every 30s, and all of it stops when the address prompt is up
(`when` conditions on the `subscribe` block, so iced actually drops the
timers instead of ignoring their messages).

`hl_candles` keeps one tape for the whole session. An empty tape backfills 500
candles and adopts the market that filled it; a loaded one asks only for the
candles that can still change and merges them in place, replacing the live
candle and appending closed ones. Switching markets re-points the tape, and a
response for the market you just left is dropped instead of overwriting the
one you are looking at.

## Marking trades on the chart

The chart is `candle-chart` from [`crates/ui`](../../crates/ui/src/ui/candle_chart.rs)
with two annotation overlays:

```rust
candle_chart_shared(tape, &theme::DARK)
    .price_lines(position_lines(positions, coin))  // entry, liquidation
    .markers(fill_markers(fills, coin))            // one glyph per fill
```

A buy is a triangle pointing up out of its fill price, a sell points down into
it, and each carries its size. Both are ordinary `ChartOverlay` implementations
— the same extension point a caller uses for anything the built-ins do not
cover — so nothing about trading leaks into the chart widget.

## Boundary

One extern block
([`src/ui/extern/hyperliquid.ice`](src/ui/extern/hyperliquid.ice)): an opaque
`Tape` handle, four async fetches that return checked structs, a handful of
`sync` formatters, and one `component` adapter that renders the chart from the
tape plus the current fills and positions. Candles never cross into Ice; the
positions and fills do, because the panel below the chart lists them.

## Tests

`cargo test -p trading-example` parses recorded payloads for every response
shape, checks the tape merge and the market-switch guard, and renders the
address prompt headlessly. One test talks to the live exchange and is opt-in:

```bash
cargo test -p trading-example -- --ignored
```

To capture the prompt as evidence:

```bash
ICE_TEST_ARTIFACT_DIR=target/trading-evidence \
  cargo test -p trading-example __ice_tests::trading_gate_gates_the_app -- --exact --nocapture
```

![Address prompt](screenshots/gate.png)

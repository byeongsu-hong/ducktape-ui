# Candles

A native lightweight financial chart written in Ice. The `candle-chart`
widget in [`crates/ui`](../../crates/ui/src/ui/candle_chart.rs) renders
OHLCV candlesticks, a volume histogram, price/time axes, a last-price line,
and a crosshair on an iced canvas. Grid, candles, volume, and axes live in a
cached geometry layer that is only rebuilt when the data or the visible range
changes; crosshair moves reblit that cache and redraw only the overlay.

Scroll to zoom around the cursor, drag to pan, hover for the OHLCV readout.
The app streams a deterministic synthetic tape: every 500ms the last candle
ticks, and occasionally a fresh candle rolls over.

```bash
cargo run -p candles-example
cargo test -p candles-example
```

To render the authored Ice test headlessly and capture a screenshot:

```bash
ICE_TEST_ARTIFACT_DIR=target/candles-evidence \
  cargo test -p candles-example __ice_tests::candles_smoke -- --exact --nocapture
```

The capture lands in `target/candles-evidence/candles_smoke/` as `ready.png`
plus a `ready.json` with geometry, paint, and accessibility evidence.

![Candles](screenshots/candles.png)

## Performance

Frame-build cost of the widget at 1280x720, measured headlessly on the
tiny-skia backend in release mode (geometry recording; compositing is
bounded by viewport pixels for any chart):

| candles | view | static-layer rebuild | cached frame |
| ------: | :-- | ---: | ---: |
| 1k - 1M | last 120 | ~85us | ~2us |
| 10k | all visible | 0.6ms | ~2us |
| 100k | all visible | 1.4ms | ~2us |
| 1M | all visible | 11.5ms | ~2us |

The cached (per cursor move) frame is flat at any zoom and any dataset size:
scale and axes are memoized under the same fingerprint that keys the cached
geometry. Rebuilds are bounded too — once candles are narrower than a pixel
they fold into per-pixel columns (M4-style), so tessellation is O(plot width);
what remains at extreme sizes is the O(visible) numeric scan (see the
`ponytail:` marker in `candle_chart.rs`). Reproduce with:

```bash
cargo test -p ducktape-ui --release --features candle-chart,tiny-skia,x11 \
  --lib candle_chart::tests::bench_frame_costs -- --ignored --nocapture
```

## Using the widget outside this workspace

`candle-chart` is a plain Rust widget behind a cargo feature; no `.ice`
sources from `crates/ui` are involved, because the extern block below lives in
your app. Until the first crates.io release, depend on the repository directly
and keep the crates on one revision (iced stays pinned to `=0.14.0`):

```toml
[dependencies]
iced = { version = "=0.14.0", features = ["smol"] }
ducktape-ui = { git = "https://github.com/byeongsu-hong/ducktape-ui", features = ["candle-chart"] }
ui-lang = { git = "https://github.com/byeongsu-hong/ducktape-ui" }
ui-lang-runtime = { git = "https://github.com/byeongsu-hong/ducktape-ui" }

[build-dependencies]
ui-lang-build = { git = "https://github.com/byeongsu-hong/ducktape-ui" }
```

Then copy this example's three-file recipe: the adapter module that re-exports
`Candle`/`CandleHit` and wraps `candle_chart(...)` in a component fn
([`src/market.rs`](src/market.rs)), the extern block declaring it
([`src/ui/extern/market.ice`](src/ui/extern/market.ice)), and the
`extern chart(candles) -> candle_hovered _` call site
([`src/ui/app.ice`](src/ui/app.ice)).

## Boundary

The Ice/Rust boundary is one extern block
([`src/ui/extern/market.ice`](src/ui/extern/market.ice)): `Candle` and
`CandleHit` record views, three `sync` helpers for the tape and number
formatting, and one `component` adapter that returns the chart element. The
view mounts it with `extern chart(candles) -> candle_hovered _` and feeds the
reported hit back into the header readout.

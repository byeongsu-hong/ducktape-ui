# Design metrics evidence

Source: [design_metrics.ice](../../tests/cases/ui/design_metrics.ice).
The view and density tests use the same `MetricsScreen` composition. The
Rust comparison adapter calls the library's actual `typography` function.
All captures use scale 1, light app palette, ko-KR, Linux metadata and reduced
motion. IBM Plex Sans KR regular/semibold/bold and Geist Mono regular are
loaded from repository assets.

| Capture | Viewport | Page / section gap | Input / action height |
| --- | --- | --- | --- |
| [Standard](standard.png) | 640×720 | 24 / 16px | 40.2 / 38.25px |
| [Compact](compact.png) | 360×720 | 12 / 12px | 32.25 / 32.25px |
| [Compact wide](compact_wide.png) | 640×720 | 12 / 12px | 32.25 / 32.25px |

These captures follow Korean text entry and a pointer click 2px above the
button's bottom edge. Each test then edits a different value and saves with
Tab/Enter. Assertions cover exact section/field gaps, preserved hit areas,
centered text, named inputs/actions and longer wrapped copy.

All 13 text roles are rendered as two-line Ice and Rust samples at 520×260.
In the [before body](body_before.png) and [after body](body_after.png) captures,
Ice is on top and Rust below. The previous Rust body line height was 20.925px
against Ice's 20.25px. Eleven role line-height comparisons failed before the
fix; all now agree on font, size, line height, total height and first-baseline
offset. Four additional semantic-color failures identified headings using
`foreground` instead of `primary` and field labels using `muted_foreground`
instead of `foreground`.

The composition assertions also reject these temporary changes:

- Compact action vertical padding 8→4: height 24.25 instead of 32.25px.
- Remove the three Korean font assets: heading glyph width 72.32 instead of
  121.728px, with visibly missing glyphs in the
  [mutation capture](missing_font_mutation.png). The named font declaration
  alone remains present and cannot prove loaded glyph coverage.
- Remove the page inset: left edge 0 instead of 24px.
- Omit explicit input labels: accessible name is empty instead of
  `Workspace name` even though the Field has a visible text label.

All temporary mutations were restored. The final focused fixture has 19
passing tests (16 authored scenarios, three generated harness checks).
The component/showcase regression run passed 968 tests with four ignored;
only the focused fixture was rerun after the final example labeling change.

```sh
cargo test -p showcase --test design_metrics -- --nocapture
cargo ice inspect examples/showcase/tests/cases/ui/design_metrics.ice \
  --viewport 640x720 --theme light --scale 1 --locale ko-KR \
  --platform linux --reduced-motion --frames 60 --name design_metrics_frames
```

Native test PNG/JSON pairs are under
`examples/showcase/target/ice-test-artifacts/`.
All 13 pre/post role pairs were compared with `cargo ice diff`. Their manifest
changes are the expected native line boxes, baselines, semantic colors,
corresponding container/visible heights and source-line movements. For example,
body changes 0.317% of pixels, section title 1.358%, and caption 0.655%.
At the 9px badge size, raster snapping keeps the pixels identical while the
line-height manifest and assertion still detect the correction.

These are native tiny-skia checks. They do not claim Tree parity, touch-target
standards or platform font-fallback behavior. The separately documented Rust
inline-code helper retains its own compact wrapper metrics.

Production-view inspection (640×720, default density), 60 debug frames:

```text
frames: 60 @ debug | view p50 51us p95 70us | layout p50 3us p95 3us | update p50 15us p95 20us | rev_memo 60/0 | memo_lazy 0/0
```

This small form measurement is not a general performance guarantee.

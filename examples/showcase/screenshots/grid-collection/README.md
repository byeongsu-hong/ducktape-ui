# Native minimum-cell grid evidence

Source: [`grid_collection.ice`](../../tests/cases/ui/grid_collection.ice).
The view and card mounts use the same `Collection` component. Each card contains
an existing one-cell 4:3 grid, a padded surface, a heading/number and a real
button. No new Ice syntax or typed rendering adapter is needed.

All captures use the app's light palette, scale 1, en-US, Linux and reduced
motion. The after captures follow a pointer click on card 5, so that button
shows its hover state. Tests assert the delivered selection as well as geometry.

| Capture | Viewport | Minimum / gap / page inset | Resulting columns / cell width |
| --- | --- | --- | --- |
| [Narrow](narrow.png) | 280 × 1400 | 320 / 12 / 24 | 1 / 232 |
| [Two columns](two_columns.png) | 480 × 800 | 180 / 12 / 24 | 2 / 210 |
| [Three columns](three_columns.png) | 721 × 600 | 180 / 12 / 24 | 3 / 216.3333 |
| [Custom](custom.png) | 640 × 600 | 160 / 20 / 12 | 3 / 192 |

Before the production fix, the unchanged geometry assertions failed:

- [Narrow](before_narrow.png): first width 320, expected 232; the capture clips
  every card at the window's right edge.
- [Two columns](before_two_columns.png): final width 432, expected 210.
- Three columns: final width 330.5, expected 216.3333.
- Custom: final width 298, expected 192.
- Exact 420px reflow threshold: final width 372, expected 180.

The eight scenarios additionally cover 419px immediately before that threshold,
empty and single-item updates, naturally unequal row heights with grid padding,
exact row/column placement, equal 4:3 cells, painted heading/button containment
and pointer routing. A temporary floor-to-ceil mutation makes the 419px test fail at 179.5 instead of 371. Removing the cap
by item count makes the single-item test fail at 210 instead of 432. Restoration
passes the same assertions. Ignoring grid padding in the owning flex limits
makes the natural-row test fail at a 230px cell instead of 218; restoring padding
restores the expected columns.

Reproduce with:

```sh
cargo test -p showcase --test grid_collection -- --nocapture
```

Capture JSON/PNG pairs are generated under
`examples/showcase/target/ice-test-artifacts/grid_collection_*/`.
Compare pre-click `*_geometry.json` pairs with `cargo ice diff` to isolate the
layout delta from the after-click button state.

This evidence is native. Tree still sends ordinary flex item rules for
`min-cell`; it does not yet carry the equal-column sizing mode.

The 721 × 600 production view was also inspected for 60 frames:

```text
frames: 60 @ debug | view p50 106us p95 132us | layout p50 7us p95 8us | update p50 46us p95 59us | rev_memo 60/0 | memo_lazy 0/0
```

Pre-click capture diffs show the intended geometry/paint/visibility changes:
35.59% of pixels at 280px, 28.34% at 480px, 24.05% at 721px, and 23.00%
for the custom 640px view. At 280px, the formerly clipped card numbers become
visible; this also changes the ordered paint entries in ancestor manifests.
Theme, font and accessibility metadata remain consistent. Capture source line
numbers move where assertions were added.

# First-class Ice tests: driver and evidence reference

The README shows the shape of an authored `test`; this is the full driver,
determinism, and evidence contract. `SPEC.md` remains the grammar of record.

## Environment pinning

A conformance case can pin its environment and drive the same input, window,
accessibility, and renderer paths used by the application:

```ice
test dark_keyboard_and_focus
  viewport 800 600
  theme dark
  scale 2.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  target field = #form/name
  target submit = #form/submit

  focus field
  replace "Ice"
  chord control enter
  hover submit
  expect a11y submit role "button"
  expect a11y submit action click
  capture dark_keyboard_and_focus
```

`theme` replaces the headless program theme result with `Theme::default(mode)`;
it does not switch application-owned palette state, which should be selected
through a preset or `dispatch`. `scale` overrides the headless program scale
result. Locale, platform, and reduced-motion pin test metadata and driver
context; they do not invent application state or an OS setting event. Rust
harnesses can pin the independent boot OS preference with
`Config::system_theme`; an Ice `system-theme` step is a later notification.

## The driver

The test driver is a semantic boundary over Iced rather than a public generated
application event enum. Ice steps and Rust harnesses share the std-like
`ui_lang_runtime::testing::Action` enum and
`Driver::perform_action(Action, Location)` entry point, so adapters can replay
the same input without depending on private generated messages.

Semantic steps cover pointer buttons and coordinates, drag/drop, exact focus,
held keys/modifiers and chords, selection and IME composition, scrolling,
multi-touch, window/system/file events, bounded waiting and redraw-time
advancement, named RGBA capture, and AccessKit actions/assertions. The driver
keeps cursor, focus, modifiers, touches, viewport, and widget-local state in
sync with the native events it emits. `advance` controls deterministic redraw
timestamps but does not virtualize arbitrary `iced::time` futures. A
task-issued window open replaces the single headless current window with fresh
widget/focus/input state while retaining application state.

Interactions replay emitted messages through generated update code and drain
real tasks recursively before the next statement. Checked `sync` and task
externs therefore call the same Rust functions used by the app. Deterministic
test behavior belongs behind a named preset or Rust `cfg(test)` boundary; Ice
does not add a mock layer. Subscriptions are re-established around simulated
events; intentionally infinite timer/I/O subscriptions are sampled rather than
awaited as finite work. See the layout and interaction contracts in
[`component_state.ice`](../examples/iced-app/src/ui/component_state.ice).

## Targets and assertions

Tests use the same checked components, handlers, presets, expressions, and Rust
extern boundary as production code. IDs select rendered widgets after real
Iced layout. A component call ID is a scope rather than a synthetic layout box,
so a test selects an identified descendant such as `#counter/root`. Target
aliases may reuse an earlier target as a path prefix, while `#` paths remain
absolute. Geometry assertions use logical-pixel bounds; paint assertions
inspect unambiguous tiny-skia quad or text commands for backgrounds, borders,
radii, shadows, colors, fonts, sizes, and line heights without comparing
screenshots. Primitive counts, text/image bounds, shaped text baseline,
scale-aware pixel alignment, focus, and accessibility fields are also available
when a conformance report needs more than the single-primitive convenience
accessors.

Each target generated from an Ice view also records its originating `.ice`
path, line, and column. A target constructed wholly inside a Rust widget may
report no finer provenance.

## Captures and evidence

`capture` writes a PNG and structured JSON frame manifest to
`target/ice-test-artifacts/<sanitized-test-name>/` while retaining RGBA output
for a Rust harness. `ICE_TEST_ARTIFACT_DIR` replaces the artifact root, and the
runtime `Config::artifact_dir` sets an exact per-test directory. Capture does
not impose exact pixel equality. It records configured, resolved-render, and
system theme fields separately and limits physical output to 16,777,216 pixels
(64 MiB RGBA8).

`cargo ice inspect` exposes the same real headless app `Program` without
requiring an authored test capture, while `cargo ice diff` compares two
manifests and their PNGs outside the runtime. `cargo ice review` runs selected
first-class Ice tests and packages their captures, diagnostics, accessibility
inventory, baseline diffs, and source-mapped changes into one JSON/HTML
evidence bundle — see [tooling.md](tooling.md) for their flags and policies.

## Performance contracts

Performance tests are `#[ignore]`d so an ordinary `cargo test` stays fast, which
means **a performance test only runs if CI names it**. The `Run remaining
performance contracts` step in `.github/workflows/ci.yml` selects them two ways:
by the name filter `performance_contract`, or by an explicit `--test <file>` for
an integration target. A test named anything else, in a file no line mentions,
never runs anywhere and is decoration. Three had gone dead that way before this
was written down. When adding one, add its CI line in the same change, and audit
with:

```sh
grep -rn -A2 '#\[ignore' crates/*/src crates/*/tests | grep 'fn '
```

Two shapes of assertion, and the choice matters:

- **Metric counts** — the layout metrics a widget records (paragraphs shaped,
  lines highlighted, allocations, line-vector slots). These are exact,
  machine-independent, and say *why* something got slower, so prefer them.
- **Wall-clock budgets** — keep them, but only as a backstop, and size them to
  the work rather than to the machine. A budget left far above the real cost
  cannot fail: contracts here once asserted a 30-second budget against 8.7
  seconds of pure waste.

Absolute microsecond budgets do not survive a shared runner. Where timing is the
point, assert a **ratio between two measurements taken in the same run** — a busy
machine slows both, so the comparison holds. `tests/frame_probe.rs` does this to
guard virtualization: it requires a measured virtual timeline to beat a plain
lazy column by >5x at equal row count, and 6.7x the rows to cost <2.5x the time.

A contract that asserts today's number pins today's behaviour, including its
waste. When a fix makes a metric drop, the contract asserting the old value is
part of the fix — update it, and bring its budget down with it, or the next
regression has nowhere to land.

## Probes: where the time actually goes

Contracts guard a number that is already understood. A probe finds the number in
the first place — it prints a phase split, asserts nothing, and is `#[ignore]`d
or excluded from debug builds, so it never runs in CI. Reach for one before
optimizing anything, because both of the loops below turned out to be dominated
by a phase that was not the obvious suspect.

### The edit → run loop

```sh
scripts/build_bench.py --packages showcase iced-app --runs 5 --json before.json
# change something
scripts/build_bench.py --packages showcase iced-app --runs 5 --compare before.json
```

Three medians per package: `noop` (cargo's own overhead), `script` (the package
build script run directly — the Ice compiler alone), and `edit` (one byte
changed in a root `.ice`, which is what an author waits for). `edit - script` is
rustc's share.

Do **not** measure the Ice compiler by bumping `ICE_DEV_BUILD_FINGERPRINT`:
cargo marks the whole crate dirty on an env change, so that number is mostly
rustc. `build_bench.py` runs the build-script binary directly instead.

On showcase (2170 lines of `.ice`, 15.5k lines generated) the split is
`script` 0.3s against `edit` 6.5s, and `-Ztime-passes` on the incremental
rebuild attributes rustc's share to `type_check_crate` 2.7s, `link` 0.9s,
`MIR_borrow_checking` 0.8s, `codegen_crate` 0.55s. So the loop is a *rustc
front-end* cost on generated code, not an Ice compiler cost. Two profile levers
were measured and rejected because of that: `[profile.dev.build-override]
opt-level = 3` (no effect — the Ice compiler is not the bottleneck) and
`debug = "line-tables-only"` / `debug = 0` (0.92x at best — debug info is not
the bottleneck either).

What the crate is actually spending it on is visible in the dependency graph
(`RUSTC_BOOTSTRAP=1 cargo rustc -p showcase --bin showcase -- -Zincremental-info`,
run twice so the second pass does not discard the cache over changed flags).
showcase's is 1.29M nodes and 12.45M edges, led by `layout_of` (117k nodes),
`impl_trait_header`, `implementations_of_trait`, `symbol_name` (77k) and
`items_of_instance` (48k) — roughly 77k monomorphized instances out of 15.5k
generated lines. The front-end cost is instantiating iced's widget machinery,
not parsing or checking volume.

Three more things were measured against that and rejected, so nobody repeats
them:

- **Content-addressed generated filenames breaking incremental reuse.** They
  are not content-addressed. `generated_group_file_name` hashes the source path
  and slug, so an edit leaves every generated file name byte-identical; the
  suffix only differs between checkouts because the absolute path does.
- **Boxing style closures to collapse instantiations.** Emitting container
  styles as `Box<dyn Fn(&Theme) -> Style>` instead of a distinct closure type
  per container left rustc unchanged: total 14.66s against 14.13s,
  `monomorphization_collector_graph_walk` 2.00s against 1.93s. iced already
  erases the closure at that boundary.
- **Per-file incremental isolation.** Generated fenced groups land in separate
  files so an edit to one leaves the others' spans untouched, but editing
  `components/catalog.ice` costs the same as editing the app root — 6.4–7.6s
  either way. The isolation is real and does not show up in the wall clock.

So the remaining lever on the build side is the count of distinct widget types
the generated view instantiates, which is a question about how views are lowered
rather than about compiler flags.

### The frame

```sh
cargo test --release -p showcase -- --ignored --nocapture frame_cost
```

`examples/showcase/src/frame_probe.rs` drives the real generated app through
`testing::Driver` and prints p50/p95 per phase. `crates/ui-lang-runtime/tests/frame_probe.rs`
is its counterpart for hand-written iced trees; use that one when the question
is about a runtime widget rather than about generated code.

Release only — the module is `#![cfg(not(debug_assertions))]`, because `-O0`
numbers measure rustc, not the app.

The phase that matters is `__view build only` against `idle redraw`: the first
is the code the Ice compiler emits, the second adds iced's layout and event
walk. On showcase that is ~0.72ms against ~3.3ms, so roughly three quarters of a
frame is layout, and optimizing generated code alone cannot reach it.

`idle redraw @480x320` says what that layout is proportional to. The small
viewport holds a fraction of the same catalog — 8.4x less area — and costs
~3.1ms against ~3.3ms, a 6% difference. **A frame costs what the view contains,
not what it shows.** Every widget below the fold is laid out on every frame.

That is the fact to design against. What moves it is a boundary the layout walk
can stop at, and the repo has two:

- **`lazy`** lowers to `ui_lang_runtime::memo_lazy`, which is iced's `Lazy`
  plus a memoized layout node — while the dependency hash and the incoming
  `Limits` are unchanged, `layout()` clones the stored node instead of walking
  the subtree. (Plain `iced::widget::Lazy` caches only the element and still
  re-walks; the distinction is the whole point of the fork.) The runtime probe
  re-lays-out 150 lazy chat rows in ~35us, against showcase's ~3.3ms for a
  comparable tree with no lazy boundary anywhere.
- **`virtual_list`** mounts only the rows a viewport can hold, so nothing
  off-screen exists to lay out. `tests/virtual_list_performance.rs` covers 1000
  rows in ~1.0ms where a plain lazy column needs 13.1ms for 150.

A boundary only pays while its dependency is stable, which is why showcase is
the worst case rather than a bug: the catalog is a demo of interactive widgets,
threaded with `bind` parameters and ~45 pieces of state, so almost no subtree
in it holds still long enough to cache. Read its 3.3ms as the cost of a view
that cannot memoize, not as a number every Ice app pays.

Micro-optimizing emitted code has the ~0.72ms `__view` share as its ceiling.
Removing 984 redundant scope clones from showcase's generated view — every one
of them real waste — was worth ~19us. Measure before spending effort there.

### A second app, and where the boundary runs out

`examples/trading/src/frame_probe.rs` measures the opposite shape from
showcase: lists whose rows are a near-pure function of one row value.

```sh
TRADING_PROBE_SYMBOLS=0   cargo test --release -p trading-example -- --ignored --nocapture frame_cost
TRADING_PROBE_SYMBOLS=120 cargo test --release -p trading-example -- --ignored --nocapture frame_cost
```

Every list on that screen is filled by a network task and starts empty, so a
headless boot measures the chrome alone; the probe seeds the symbol universe
the way the task would. Run the two counts back to back — a busy machine moves
both, and an earlier reading taken minutes apart was wrong by 40%.

At 1600x1000: **1276us with no symbols, 2094us with 120** — the rows are 39% of
the frame, ~6.8us each, and a real perp universe is larger than 120.

Those rows are exactly what a `lazy` boundary is for, and they cannot have one:

- `MarketRow` depends on the row **and** on `coin`, the selected symbol, while
  `lazy` takes a single dependency and exposes only that alias inside.
- Folding the selection into the row does not rescue it. A `lazy` dependency
  must be `Hash`, and `SymbolRow` carries `price`, `change_pct`, `funding_pct`
  — `f64` does not implement `Hash`, which is why SPEC rejects float-bearing
  values as lazy identity in the first place.

So the lever that reaches the ~75% of a frame that is layout is unavailable
precisely where market data lives. Closing that would mean letting `lazy` take
an author-supplied key the way `keyed` already does, rather than deriving
identity from the whole dependency — which is a language change, not a tuning
one. Recorded here so the next pass starts from the constraint rather than
rediscovering it.

Two controls in that probe are there to foreclose the easy explanations, and
both come back negative:

- `__view build only (chrome)` against `idle redraw` — 142us against 1276us.
  Generated code is 11% of trading's frame, matching showcase's 22%. Whatever
  is expensive, it is not the code the Ice compiler emits.
- `cold redraw` against the steady-state p50 — 1296us against 1313us. The first
  frame, which has to shape every paragraph from nothing, costs what the
  sixtieth costs. This is not a warm-up cost that amortizes; the walk is paid
  again on every frame.

Both apps therefore land in the same place: iced re-lays-out the whole tree per
frame, that walk is 78-89% of the cost, and it does not shrink with the
viewport, with warm-up, or with anything the code generator emits. Only a
boundary the walk can stop at moves it.

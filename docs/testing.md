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
awaited as finite work.

A task stream is not a subscription and does not get that treatment. A handler
that starts an endless `stream` — a websocket pump, a worker feed — leaves work
the driver waits on, and every later step waits with it:

```
statement: dispatch browse()
expected: quiescence within 2s
actual: 1 task stream(s) still pending after 2.000930993s
```

Raising `timeout` does not help, because the stream never ends. Reach the state
that handler would have produced with a `preset` instead, and dispatch only the
handlers whose tasks finish. A preset is also the only way to set state no
handler can construct from Ice — an extern error payload, say, whose type has
no Ice constructor. See the layout and interaction contracts in
[`component_state.ice`](../examples/iced-app/src/ui/component_state.ice).

## Targets and assertions

Tests use the same checked components, handlers, presets, expressions, and Rust
extern boundary as production code. IDs select rendered widgets after real
Iced layout. A component call ID is a scope rather than a synthetic layout box,
so a test selects an identified descendant such as `#counter/root`. Target
aliases may reuse an earlier target as a path prefix, while `#` paths remain
absolute.

An absolute path reaches what the tested view exposes at its top; a layout ID
nested below that is reached by chaining from an alias, not by spelling out a
longer `#` path. The chain names every identified ancestor, not only the
outermost — an alias does not reach past an intervening ID. Given a dialog
holding a button, `#connect` and `#gate/connect`
are both `E194 unknown rendered widget target`, and this is the form that
resolves:

```ice
target dialog = #gate
target connect = dialog/connect
target field = dialog/address-input
focus field
```

Actions and assertions take the alias too, so `focus #address-input` fails for
the same reason `target` did.

Deeper in a real view the chain grows to match. A message rendered inside a
column inside a panel is reached one identified ancestor at a time, and
skipping a middle name fails as though the ID were missing rather than as
though the path were short:

```ice
target app = #app
target lower = app/lower
target pos = lower/positions
target line = pos/failure
``` Geometry assertions use logical-pixel bounds; paint assertions
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

Those rows are exactly what a `lazy` boundary is for, and getting one took two
changes on the app side, neither of them to the language:

- **One dependency, not two.** `MarketRow` read the row *and* `coin` to know
  whether it was the market on screen, and `lazy` exposes only its alias inside
  the subtree. `filter_symbols` now marks the row it selects, so the row is the
  whole dependency and the comparison happens where the list is built. Anything
  that changes what a row renders — including which one is picked — has to
  rebuild `visible`, which is what
  `picking_a_market_moves_the_mark_onto_its_row` holds down.
- **A hand-written `Hash`.** Every number on a market row is an `f64`, which is
  not `Hash`, so `SymbolRow` could not be derived into a dependency. The
  checker's `lazy_hashable` already admits any named extern type and leaves the
  obligation to Rust, so the app implements `Hash` over the float bits, folding
  `-0.0` onto `0.0` so rows that render alike also cache alike. Only a bare
  `f64` dependency is rejected outright.

The result, measured back to back at 1600x1000:

| | before | after |
|---|---|---|
| chrome only | 1276us | 1274us |
| 120 symbols | 2094us | 1445us |

The rows went from 818us to ~172us, **4.8x**, and 31% came off the whole frame.
The chrome did not move, which is the check that the win is the rows and not
the weather.

Two things this does not show. The universe is rebuilt on every market tick, so
a row whose price moved misses its cache and re-lays-out — the win is on frames
where the data holds still, which is most of them. And the remaining 1274us of
chrome has no boundary in it at all; that, not the rows, is now the largest
single number on this screen.

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

### Why an edit costs what it costs

The `edit` number is not a fixed price. It tracks the size of the generated
module the edit dirties, because rustc's incremental cache is per **item** and
a component's whole view is one function:

| edited | dirtied module | generated lines | rebuild |
|---|---|---|---|
| `showcase` `components/navigation.ice` | navigation | 673 | 3.0s |
| `trading` `app.ice` | app + root | 2.6k + 4.0k | 3.0s |
| `showcase` `app.ice` | root | 5.5k | 6.3s |
| `showcase` `components/catalog.ice` | catalog | **6.9k** | **6.5s** |

There is a floor near 3.0s (link, cargo, the work no cache can avoid) and above
it the cost follows the dirtied module. showcase's `catalog` module is **one
6,894-line function** — its `Catalog` component takes 45 parameters and mounts
16 `Panel`s — so changing one character in it re-checks all of it. There is no
smaller unit for the cache to reuse.

`codegen/view/outline.rs` already exists to split those functions up, and its
own notes measure typecheck as superlinear in function size (`~M^1.7`). It does
not fire here: a component use whose slot content is present renders inline
(`call.slots.is_empty()` in `view/content.rs`).

Removing that condition outright takes showcase's worst edit from **6.5s to
3.3s** and leaves total generated lines flat (15,506 -> 15,632), but it is
**unsound** — `iced-app`'s `render_surface` stops compiling with `cannot find
value 'item'`, because slot content snapshots the call-site environment and can
interpolate a loop variable that an outlined method has no binding for.

Narrowing it to "outline unless the slot content actually reached a render-site
local" is sound and recovers nothing, because the slots stop short for a
different reason: they read the enclosing component's parameters, and
`RecordingEnv::record` hard-captures a `ComponentParam` whose self-backed
marker is absent from the snapshotted slot environment
(`expr.rs`, the `None => self.hard_capture.set(true)` arm). Locals are empty in
every case measured; the flag is the marker's absence, not a real capture.

So the ~2x on the worst edit is real and reachable, and the work is to carry
those markers into the slot snapshot (or resolve them through to the parent
environment) so slot content parameterizes the way arguments already do. It is
a change to capture analysis, where a mistake miscompiles rather than fails, so
it wants its own pass with the workspace suite as the oracle — that suite is
what caught the unsound version.

### The floor, and what is under it

Once the dirtied module is small the front end is no longer the cost. Splitting
a 3.0s rebuild (`showcase`, editing `components/navigation.ice`) with
`-Ztime-passes`:

| pass | time |
|---|---|
| `link` / `run_linker` | **1.45s** |
| `codegen_crate` | 0.54s |
| `LLVM_passes` | 0.57s |
| `serialize_dep_graph` | 0.18s |
| `type_check_crate` | not in the top 14 |

Half the floor was the linker, and most of that was not the linker's own work:
the dev profile wrote full debug info *into* the binary on every link.
`split-debuginfo = "unpacked"` leaves it complete but beside the binary:

| | rebuild | binary |
|---|---|---|
| packed (was the default) | 3.14s | 443 MB |
| unpacked | **2.55s** | **158 MB** |
| unpacked + `line-tables-only` | 2.51s | 135 MB |

`line-tables-only` buys almost nothing more and costs variable inspection, so
only the split is taken. An earlier pass measured this same setting at 0.92x
and dismissed it — that measurement bundled it with `line-tables-only` on the
6.5s worst case, where the same ~0.6s is a smaller fraction. The absolute win
was always there.

Two other link levers were measured and are not taken. `rust-lld` is already
the default on this target. Pointing the final link at `mold`
(`-Clink-arg=-fuse-ld=mold`) moved 3.00s to 2.81s — 6%, for a toolchain
dependency every contributor would have to install.

### Chasing the outlining 2x to its last blocker

The 2x above is real, and the search for it went through one wrong diagnosis
before landing. Both are recorded, because the wrong one is easy to repeat.

Counting the decision directly — one line per component use, printing each
clause of the gate — says that on showcase, of 164 uses in `__view`:

| Rejected by | Uses |
| --- | --- |
| `call.slots.is_empty()` | 90 |
| a hard capture | 0 |
| a render-site local value | 0 |
| a callback with no signature marker | 0 |

One clause. Everything the recorder was built to detect fires on nothing here.

The earlier note in this file claimed the blocker was `__ice_reconciliation_scope`
falling into `RecordingEnv::record`'s catch-all arm, and that the last mile was
a judgement about whether a scope expression's free identifiers are available
inside an outlined method. That was measured on a build that had *already*
lifted the slot gate, so it described a consequence of the experiment rather
than the state of the tree. There is no free-identifier judgement to make.

Lifting the gate outright breaks exactly one thing, in `iced-app`:
`error[E0425]: cannot find value 'item'`. Slot content is snapshotted at the
call site and rendered from inside the callee, so a call-site loop item it
reads is not in scope in the method the body was moved to. The recorder never
saw the read, because by render time the content's environment is a flat copy
with nothing in front of it.

So give the snapshot the recorder that stood at its call site, and replay its
reads into it — no more and no less. Replaying into *every* open recorder
instead is sound but over-blocks: a `lazy` dependency read by slot content is
bound inside the enclosing component's body and travels with it, and blocking
on it costs that component its own method (the `lazy-component-context`
fixture catches this).

The reconciliation scope then does surface in the catch-all — and it is not a
capture at all. `set_reconciliation_scope` at a slot render site writes the
scope the content renders under, which comes from the render site, not the
call site. Reading it back is only a capture because the write went *under*
the recorder. Layering it above instead makes the question disappear.

That is the whole change: all 164 showcase uses outline, and the `.ice` edit
falls from 5.91s to 3.49s — 41%, measured with `scripts/build_bench.py` in
both directions on one warm target directory.

| Package | edit before | edit after |
| --- | --- | --- |
| showcase | 5.91s | 3.49s |
| music-example | 2.42s | 2.45s |
| trading-example | 2.89s | 2.90s |
| iced-app | 2.15s | 2.16s |

Only showcase moves, and that is the expected shape rather than a
disappointment: the other three already sit at the link-dominated floor
measured above, where the type check is not what the wall clock is waiting on.
Outlining buys nothing there, and showcase lands close to that same floor.

Outlining is not free at runtime — each use becomes a call through
`grow_stack` with its scope locals cloned in — so the frame was checked too.
`__view build only` on showcase reads 707/709/719/731us across four runs
against a 715us baseline: no change that the run-to-run spread does not
already cover.

# First-class Ice tests: driver and evidence reference

The README shows the shape of an authored `test`; this is the full driver,
determinism, and evidence contract. `cargo ice schema` remains the vocabulary of
record.

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
  move submit
  expect a11y submit role "button"
  expect a11y submit action click
  capture dark_keyboard_and_focus
```

`a11y activate|focus|increment|decrement <target>` performs an assistive
technology's action on a semantic node — activate runs its click route, focus
moves keyboard focus to it, and increment/decrement run a range control's
change route with the value one step away — and `expect a11y <target> action
click|focus|increment|decrement [bool]` asserts whether the node exports it.

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

## One process, many applications

A shipped process runs one application, so application state that belongs to
the whole machine is a `static` there and correct: a key store, a device
handle, a connection pool. A test binary is not that process. It runs one whole
application per test thread against the same `static`, and nothing says so
until the suite is busy enough for two of them to overlap — then one test's
press clears what another had just stored, and which test fails moves with the
machine rather than with the code.

`ui_lang_runtime::testing::app_instance()` is the key such state hangs on in a
test build:

```rust
#[cfg(test)]
fn vault() -> &'static Mutex<Option<Vault>> {
    static HELD: OnceLock<Mutex<HashMap<u64, &'static Mutex<Option<Vault>>>>> = OnceLock::new();
    per_instance(&HELD)
}
```

The driver claims an instance before its program boots, and enrols the executor
threads its tasks and subscriptions are polled on — so an application reaches
its own store from wherever its work runs, not only from the test thread. A
thread with no driver behind it, an ordinary `#[test]`, gets an instance of its
own for the same reason; tests that once took turns on a mutex to avoid the
collision no longer need to.

Interactions replay emitted messages through generated update code and drain
real tasks recursively before the next statement. Checked `pure`, `sync`, and
task externs therefore call the same Rust functions used by the app. Deterministic
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
[`component_state.ice`](../examples/showcase/tests/cases/ui/component_state.ice).

## Targets and assertions

Tests use the same checked components, handlers, presets, expressions, and Rust
extern boundary as production code. IDs select rendered widgets after real
Iced layout. A component call ID is a scope rather than a synthetic layout box,
so a test selects an identified descendant such as `#counter/root`. Target
aliases may reuse an earlier target as a path prefix, while `#` paths remain
absolute.

A target path names every identified ancestor between the tested view and the
node, not only the outermost. An absolute `#` path spells that chain out, and an
alias is the shorter spelling of the same path. Skipping an intervening ID fails
with `E194 unknown rendered widget target`. Given a dialog `#gate` holding a
button `#connect`, `#connect` alone does not resolve, while `#gate/connect` and
the alias form both do:

```ice
target dialog = #gate
target connect = dialog/connect
target field = dialog/address-input
focus field
```

Actions and assertions take either spelling, so `focus #gate/address-input`
resolves and `focus #address-input` fails for the same reason `target` would.

Component-local state is read with `expect component`:

```ice
target browser = #main/browser
expect component browser.draft == ""
```

The target is the component call's `#id` — the scope that is not a rendered
widget — and the field is one of the component's declared `state` entries,
checked against its type. The read is compiled onto the generated component
seam (`__ice_test_state_<name>`), keyed by the same scope string
the view keys the instance's state by, so there is no second naming scheme:
the alias that fails `click` because it names a scope is the alias that works
here, and `#path.field` works too. An instance that has rendered but never
handled an event reads as its declared initial state; a scope no render sighted
fails and lists the live instances of that component. It is a read only —
seeding still goes through rendered controls or a preset.

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

`expect text` asks what is painted, and text drawn with `tracking=` is painted
one widget per grapheme — iced has no letter-spacing, so the lowering spaces a
row of them. No primitive holds the whole string. The query rebuilds those
runs: consecutive single-grapheme primitives sharing a baseline and a style
and evenly spaced, with a gap about one space wider than the run's own read as
a space, because a space paints nothing and arrives as a hole. Even spacing is
what keeps two tracked labels along the same row from merging. A run one
grapheme long — a count beside a heading — has only the gap crossing to the
next label to learn its spacing from, so it reads the gap after that one too:
tracking is the tightest spacing on a row, and a first gap wider than the one
following it is a label boundary.

`expect text` also reads what is drawn on a `layer`. An open `overlay` draws
its panel through iced's overlay rather than in place, so the query lays that
overlay out before asking; a target inside a modal can be scoped with `within`
exactly like one on the base screen.

Both matter most for the negative form. Before the runs were rebuilt,
`expect no text` passed for any tracked label; before layers were laid out, it
passed for anything on a modal — assertions that could not fail, on text that
was plainly on screen.

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

Opt-in interaction traces instrument `Driver::perform_action` at stable
semantic boundaries: view construction, UI build/layout, event dispatch,
program update, widget operations, task settling, and total action time.
Tracing is disabled by default and keeps no action counter or recorder state in
ordinary tests. Trace campaigns use release builds; raw samples remain in the
artifact and summaries report p50/p95/p99/max plus 60/120 Hz misses. Evidence
capture and drawing are outside the measured interval, and `draw` is explicitly
listed as unavailable instead of being inferred from capture cost. The ignored
`performance_contract_interaction_trace_overhead` contract compares the same
actions in the same process and caps enabled tracing at 1.5x disabled time.

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
scripts/build_bench.py --packages showcase trading-example --runs 5 --json before.json
# change something
scripts/build_bench.py --packages showcase trading-example --runs 5 --compare before.json
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
cargo test --release -p showcase -- --ignored --nocapture --test-threads=1 frame_probe
```

Every example but `terminal` and `two-windows` carries one:
`examples/<app>/src/frame_probe.rs` drives the real generated app through
`testing::Driver` and prints p50/p95 per phase, each probing what that app's
shape makes expensive — starter is the framework's own floor, trading its dense
terminal, cef-browser the browser's 16ms tick.
`crates/ui-lang-runtime/tests/frame_probe.rs` is their counterpart for
hand-written iced trees; use that one when the question is about a runtime
widget rather than about generated code.

The part of such a file that rots is not written at all. Nine of them carry an
`every_target` that calls `ui_lang_runtime::testing::probe::measure_interactions`
(`tray` does not — a daemon with no window and no `#id` has nothing to census),
which asks the *running* app what it has — every identified target — and drives each one with the
interaction it affords: a keystroke into anything writable, a wheel over
anything whose content is taller than its box, a click on anything
accessibility says can be activated, a hover over the rest. Every target gets a
driver of its own, settled and warmed the way `measure_frames` warms one, so a
row reports that interaction from the app's boot state rather than from
whatever the previous row's click left behind, and rows print worst-frame
first:

```text
6 targets, 20 rounds each, release build
target                         kind        action  n  action p50/p95   frame p50/p95
Candles/app/chart-frame/chart  image       hover  20    1138/1150         36/49
Candles/app/header             semantic    hover  20    1138/1142         31/41
Candles/app/header/pick-duck   button      hover  20    1123/1136         26/43
Candles/app/header/pick-tape   button      click  20    3318/3373         25/36
```

The `action` column is a driver number, not an app number: each simulated event
builds a `UserInterface` and pays the `cfg(test)` accessibility walk, which is
why every hover costs about a millisecond here. Read the column *across rows*,
never as what a click costs a user; `frame` is the redraw the interaction
caused, and that one is comparable to the phases above.

`pick-duck` is a button and still gets a hover: it is the tab already selected,
so it carries no `on_press`, and what the probe reads is what the target
supports rather than what it is. `n` is the rounds a row actually measured —
an interaction that takes its own target off the screen (a button that opens a
bar over itself) gets a rebuilt app for the next round, and a row that cannot
come back keeps the rounds it got.

Nothing there is a constant a `.ice` edit can invalidate, which is what the
hand-written phases in the same file cannot say: those name targets and
messages, and a moved node makes them wrong in a way no compile catches. Keep
writing them for what a census cannot pose — a 1MiB paste, 100k rows, a
particular state — and let the census cover the rest. It *drives the app*: a
click runs the handler behind the button with its real extern, so an id whose
handler must not run goes in the skip list.

Release only — the module is `#![cfg(not(debug_assertions))]`, because `-O0`
numbers measure rustc, not the app. That also hides it from every debug build
in CI, so the performance-contracts job type-checks the probes under the
release cfg: a probe that rots against a generated API fails there.

`cargo ice inspect ROOT.ice --frames N [--release]` is the generated form of
this probe: it measures any Ice app with no hand-written Rust file, over the
same three `Driver::redraw_phases` phases and the same p50/p95 index rule, and
writes them into the capture manifest's `frames` object beside the `rev_memo`
and `memo_lazy` totals. Reach for the probe here when the question is which of
showcase's interactions costs what; reach for `--frames` when the question is
what one idle frame of an arbitrary app costs.

The phase that matters is `__view build only` against `idle redraw`: the first
is the code the Ice compiler emits, the second adds iced's layout and event
walk. On showcase that is ~0.72ms against ~3.3ms, so roughly three quarters of a
frame is layout, and optimizing generated code alone cannot reach it.

Every example's probe closes with a `PHASES` line from
`probe::report_frame_phases`, the medians of those same three phases over the
probe's own rounds. It exists so one app's frame can be read against another's:
the table below is one machine's release run of all ten, sorted by total, and
what it is for is the *shape* of each row, not the absolute microseconds.

| app | view | layout | walk | total |
|---|---|---|---|---|
| tray | 0 | 0 | 0 | 0 |
| starter | 2 | 1 | 0 | 3 |
| cef-browser | 7 | 3 | 0 | 10 |
| candles | 20 | 7 | 1 | 28 |
| apple-music | 67 | 89 | 17 | 173 |
| markdown-editor | 107 | 28 | 126 | 261 |
| ai-chat, 500 rows | 242 | 31 | 27 | 300 |
| hotreload, 500 KB | 406 | 24 | 2 | 432 |
| trading, dense | 42 | 998 | 134 | 1174 |
| showcase | 635 | 577 | 147 | 1359 |

No two rows are limited by the same thing, and that is the point. Hotreload
spends 94% of its frame in the view build, where a layout boundary buys
nothing. Markdown-editor's largest column is the event walk, which under
`cfg(test)` carries the accessibility snapshot the shipped app builds only for
a screen reader — read that row as an upper bound, not as the app's frame.

**A column is where the work is charged, not where it is done.** Trading's row
reads as 85% layout, and it is not: `responsive #terminal-fit` wraps the whole
terminal — about 1500 lines of `view.ice` — and `responsive` runs its closure
once per layout pass, so building that subtree is charged to layout. Its memos
all hit while it happens (component 113/0, lazy 57/0), which is the tell: a
frame that were really spending 961us walking layout would be missing them.

Deleting the wrapper proves it. With `term_w` replaced by the literal the
1760px viewport gives it — every read of it is a comparison against 1280.0 or
1580.0, so the same branches are taken — the same screen measures:

| `responsive #terminal-fit` | view | layout | walk | total |
|---|---|---|---|---|
| as written | 38 | 988 | 133 | 1159 |
| removed | ~1000 | ~260 | ~170 | ~1420 |

The work did not go away, it changed columns — and it did not get cheaper.
Read the totals as "not recovered", not as "worse": the probe's own footer
says an absolute is not comparable across two builds of the app, because a
boundary added anywhere in `__view` re-optimizes all of it. What the pair does
establish is within one build, where the two columns swap. So the lever
trading needs is whatever makes that build rarer — not a layout boundary,
which its memos already are and which already hit.

Measured on the same screen and *not* worth doing: `tracking=` lowers to one
`text` widget and one `String` per grapheme inside a flex row, 55 sites in
trading, which reads like the obvious next target. Deleting every one of them
measures 1131–1191us against the 1159us baseline — no difference, because those
labels sit under memo boundaries that hit and their rows are never rebuilt.

Measured on the same screen and worth a great deal, though nothing has been
spent on it yet: the identity chain. Every node's generated code builds an
accessibility key by formatting its parent's scope, and identified nodes build
a scope string of their own. Replacing every view-path emitter in
`codegen/view.rs` and `codegen/view/content.rs` with `String::new()` and
running the two release binaries interleaved, twelve pairs with the order
rotated each round:

| | frame, `redraw_phases` total |
|---|---|
| as written | 1137us |
| no identity strings | 890us |

Twelve pairs of twelve, no overlap between the sets — the ablated maximum is
940us and the unablated minimum is 1126us. That is **247us of a 1137us frame,
22%**, and a floor: the `for`-loop scopes in `codegen/expr/children.rs` are
still in the ablated build. Memo counts are identical in both binaries
(component 113/0, lazy 57/0), so no boundary moved.

Read it as an upper bound on any fix rather than a forecast. A real interning
still needs one `iced::widget::Id` per node that carries one, and
`Internal::Unique` is private, so each of those is `Id::from(hash.to_string())`
— an allocation this ablation does not keep. What the number does settle is
that the size of this slice belongs to the app and not to the language: the
same chain measured about 3% of showcase's 70us view build, and an argument
that declines it on a 25-widget catalog is not evidence about a 302-node
terminal.

Take a phase split before choosing a fix, and when one column dominates, check
what defers into it before believing it. `lazy` and `responsive` both move work
across the view/layout line; the probe footers in `examples/*/src/frame_probe.rs`
say so for `lazy`, and this is the same rule for `responsive`.

`idle redraw @480x320` says what that layout is proportional to. The small
viewport holds a fraction of the same catalog — 8.4x less area — and costs
~3.1ms against ~3.3ms, a 6% difference. **A frame costs what the view contains,
not what it shows.** Every widget below the fold is laid out on every frame.

That is the fact to design against. What moves it is a boundary the layout walk
can stop at, and the repo has three:

- **`lazy`** lowers to `ui_lang_runtime::memo_lazy`, which is iced's `Lazy`
  plus a memoized layout node — while the dependency hash and the incoming
  `Limits` are unchanged, `layout()` clones the stored node instead of walking
  the subtree. The memo keeps three `(Limits, Node)` pairs, not one, because
  `ui_lang_runtime::flex` (what `flex`, a `grid` with `min-cell`, and any
  `row`/`col` carrying a flexbox option lower to) lays each child out with up
  to three different limits per pass — measure, final, stretch — and a single
  slot missed on every one of them and on the next frame's first pass too. (Plain `iced::widget::Lazy` caches only the element and still
  re-walks; the distinction is the whole point of the fork.) The runtime probe
  re-lays-out 150 lazy chat rows in ~35us, against showcase's ~3.3ms for a
  comparable tree with no lazy boundary anywhere. Unmounting parks the subtree
  in a per-thread lot keyed `(codegen site, dependency hash)` and capped at
  1024, so re-entering a torn-down screen reclaims it instead of rebuilding —
  `frame_probe`'s `memo_parking_cost` prices that at **4.3ms against a 21ms
  first mount** for trading's dense terminal, whose 400 boundaries (200 markets,
  200 fills) sit flat in the lot across 60 mount/unmount cycles because a
  remount reclaims exactly what it parked. Flat, but not small against the cap:
  this one screen is 39% of it, and the eviction scan is O(n).
- **`virtual_list`** mounts only the rows a viewport can hold, so nothing
  off-screen exists to lay out. `tests/virtual_list_performance.rs` covers 1000
  rows in ~1.0ms where a plain lazy column needs 13.1ms for 150.
- **A component use** is a boundary the compiler inserts on its own: when
  every read below the use is revision-keyed and every widget below it lays
  out from its element and `Limits` alone (SPEC, "Components"), the generated
  code wraps it in `ui_lang_runtime::rev_memo`, which keeps the layout node
  and, while the key holds, skips both iced's diff of the nodes below and
  their layout walk. A `for` or keyed row and a `match` payload key on the
  list or value their view takes them from, so row components are boundaries
  too; a body holding a `lazy` is not, since a lazy needs its diff every
  pass. The element is still built every pass, so nothing about borrowing
  changes. `frame_cost` prints the memo hits per idle frame
  (`take_rev_memo_counts`, `take_memo_lazy_counts`) and a phase split from
  `Driver::redraw_phases` — the generated `view`, iced's diff and layout, and
  the event walk; `ICE_MEMO_DEBUG=1` at build time prints why a use was
  refused. On showcase, 81 uses hit and 0 miss on an idle frame, and diff +
  layout went from ~1080us to ~570us with the layout skip alone. On trading's
  dense terminal, diff + layout went 1300us → 1202us when its row components
  became boundaries (68 → 113 hits) and → 1074us when a held key stopped the
  diff below it, and 1200us → 970us when an identified node's scope was
  bound once per pass instead of every descendant re-formatting the ancestor
  chain (`__ice_node_scope`; the generated view went from 1619 nested scope
  `format!`s to none outside test targets, and `view` 53us → 47us), and
  ~1–2% more once an accessible node compared its semantic snapshot against
  the tree's copy instead of cloning it every diff (most of trading's nodes
  sit under a held memo key that already skips the diff). Judge a
  boundary on that split only: the probes' end-to-end
  `idle redraw` carries ~1.3ms of the driver broadcasting the redraw to every
  subscription and settling, and `frame_panels` deltas taken end-to-end once
  read the rows as owning a walk the build phase shows them to be a fraction
  of.

A boundary only pays while its dependency is stable, which is why showcase is
the worst case rather than a bug: the catalog is a demo of interactive widgets,
threaded with `bind` parameters and ~45 pieces of state, so almost no subtree
in it holds still long enough to cache. Read its 3.3ms as the cost of a view
that cannot memoize, not as a number every Ice app pays.

Micro-optimizing emitted code has the ~0.72ms `__view` share as its ceiling.
Removing 984 redundant scope clones from showcase's generated view — every one
of them real waste — was worth ~19us. Measure before spending effort there.

### A resize is not its own frame

Reported as "the trading window stutters while it is being resized", which
sounds like a resize path and is not one. `resize_cost` drags the dense
terminal three pixels a frame and prices the frame it produces against a
settled one, on both axes separately, because they are not the same work: a
height change moves every panel and re-aims every virtual column, while a width
change does that *and* re-shapes every text run on the screen.

At 1760x940, 200 markets / 200 fills / 60 prints, p50 of 60 interleaved rounds:

| | |
| --- | --- |
| idle redraw, settled | 2139us |
| resize, height only | 2160us |
| resize, width only | 2169us |
| resize, corner | 2112us |
| resize, corner, no rows | 964us |

**A resize frame costs what any other frame on that screen costs.** There is no
resize-specific work in the Ice or iced layer to remove: what a drag changes is
only that those frames arrive back to back, at whatever rate the compositor
asks. The rows are 1148us of the 2112, and they cost that on every frame,
resizing or not.

Two things that read like resize costs and are not, both checked rather than
assumed:

- **The virtual columns do not double-layout a resize.** `virtual_children`'s
  `update` invalidates layout when the viewport it remembers has moved, which
  would cost the real loop a second view build and layout for every resize
  event. It does not fire: `virtual_scroll`'s `layout` re-aim already ran
  `sync_viewport` over every column against the same rectangle iced hands the
  scrollable's content — `Rectangle { x: bounds.x + translation.x, y: bounds.y +
  translation.y, ..bounds }`, spelled identically in both places — so the
  comparison in `update` is equal by the time the event arrives.
- **`responsive` rebuilds its content on every layout call**, unconditionally,
  not only when the size changed. The whole terminal lives inside one, so that
  cost is in the settled frame above too, and a resize does not add it.

Take the baseline settled. A redraw timed straight after a resize is the frame
the columns re-aim on; an earlier revision of this probe used one as its
baseline and read the difference as zero for the wrong reason.

What this cannot reach is the platform half, and the remaining suspects are all
in it: wgpu reconfigures the surface on each `Resized`, and AppKit runs a live
resize in a nested loop that wants a frame inside the callback. Neither is
measurable from a headless Linux runner, and neither is a claim this file
makes. What is settled is that the app-side frame is the same frame.

### A second app, and where the boundary runs out

`examples/trading/src/frame_probe.rs` measures the opposite shape from
showcase: lists whose rows are a near-pure function of one row value.

```sh
cargo test --release -p trading-example -- --ignored --nocapture frame_        # cost, panels, scaling
cargo test --release -p trading-example -- --ignored --nocapture resize_cost   # a window being dragged
cargo test --release -p trading-example -- --ignored --nocapture beat_cost     # a beat of the feed
cargo test --release -p trading-example -- --ignored --nocapture direct_call_cost  # direct Rust call costs
cargo test --release -p trading-example -- --ignored --nocapture memo_parking  # the memo lot
```

Read `frame_cost`'s own footer before reading its table: two of its rows are
not what they look like, and it says which.

Every list on that screen is filled by a network task and starts empty, so a
headless boot measures the chrome alone; the probe seeds the symbol universe
the way the task would. Two row counts have to be measured inside one run —
which is what `frame_scaling` does — because a busy machine moves both, and an
earlier reading taken minutes apart was wrong by 40%.

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
where the data holds still, which is most of them. And the chrome around the
lists has no boundary in it at all.

### The other list, and what an unbounded list is worth

The claim that the chrome was then the largest number was wrong, because only
the market list had been measured. `frame_panels` prices every panel by paired
ablation, and on a connected account the **recent-fills list** — 200 rows, the
cap `push_fills` imposes — was larger than the market list had ever been:
re-measured against 200 distinct fills, 1858us of a 6029us frame, 9.3us a row
against the market rows' 2.5us behind their boundary.

One row of that table used to price nothing, and it is worth knowing why it
now does: the probe seeded the **portfolio** page, which was where the fills,
the positions and the orders had gone, and the market list was drawn on a page
of its own. `market rows` therefore ablated a list that was not on screen and
came back inside its own noise — negative as often as not, which is exactly
what ablating nothing looks like.

The terminal is one page again and the probe seeds that page, so every list the
table names is rendered while it is ablated. `market rows` against
`full screen` is now a resolving row rather than a noise row. No figure is
quoted for it here: it is a wall-clock reading off a shared machine, and the
two rows are what a reader runs to see it. The market list also keeps its own
count-based contract in `markets_stay_memoized_performance_contract`, which
counts rows rebuilt rather than microseconds and is the one CI can hold.

**The trap either contract sets, and it is quiet.** Both count through a row's
accessibility label — `market_label`, `fill_label` — with a `#[cfg(test)]
count(&…)` as the function's first line, while the probes that read the counters
are `#![cfg(not(debug_assertions))]`. So a debug `cargo test --workspace` never
compiles the reader, and a `fill_label` that loses its counting line is green
everywhere except the release job. Seen for real: a merge took the upstream side
of that function and dropped the line, and the whole workspace stayed green
until `cargo test --release -p trading-example` reported **0 rows built cold for
200 rows**. Read a zero there as a missing counter first and a broken memo
second. Before trusting either contract after touching those functions or the
panes that draw them:

```sh
cargo test --release -p trading-example memoized_performance_contract -- --ignored --nocapture
```

`selected` is the one `SymbolRow` field that contract cannot move directly:
`visible` is derived by `filter_symbols(symbols, query, coin)`, which sets
`selected` from `coin`. The way to move it is to move the selection, and that
rebuilds two rows — the one losing the highlight and the one taking it. Two is
the assertion; a `Hash` blind to `selected` would rebuild neither.

It took the same three things the market list took, and one more:

- **`Hash` on `Fill`**, over the float bits, like `SymbolRow`'s.
- **The whole row as the dependency.** A fill row already read nothing but its
  fill, so no field had to move onto it.
- **An identity that comes from the row, not from the loop.** This is the one
  the market list hid. A `lazy` subtree is built from its dependency alone, so
  the generated body cannot see which iteration mounted it and the enclosing
  `@for:(index)` scope is not in scope inside it. Rows that carry their own id
  — `#market(market.name)`, `#position(held.coin)` — never noticed. A fill has
  no natural name, so all 200 rows landed on one runtime id and every `capture`
  in the suite failed as `E194`-shaped ambiguity. The fix is to publish the
  exchange trade id `push_fills` already dedupes on and spell the row
  `#fill(printed.tid)`. **A list going behind `lazy` needs a per-row id drawn
  from the row.**
- **And something has to guarantee that id is distinct**, which nothing did.
  `tid` came out of the payload through `unwrap_or_default`, so a fill the
  exchange sent no trade id for got the id `0` — and so did the next one. A
  duplicate row identity is not cosmetic here: the rows share a memo entry and
  a parking slot, and until the runtime fix above, one of them was dropped
  inside the lot's own borrow. The requirement is now written where every
  listed fill passes: `parse_fills` drops a fill with no `tid` rather than
  inventing one, and `push_fills` admits each `tid` once — across the history
  and within the incoming batch, which it did not do either.
  `a_fill_without_a_trade_id_is_not_listed` and
  `push_fills_lists_each_trade_id_once` are the two halves.

#### Two wrong bases, and the number after correcting both

An earlier reading of this claimed **31% off the whole frame**, from
`__view build 2875us -> 1400us` and `idle redraw 5346us -> 3692us`. Both halves
of that were measured wrong, in opposite directions, and the corrected answer is
smaller. Everything below is a re-measurement.

**The fixture was three fills.** `frame_probe`'s 200-fill screen was
`demo_fills()` — three of them — cycled up to the cap, on the reasoning that a
repeated row is the same widget count and the same formatter work. That stopped
being true the moment the rows went behind a `lazy` keyed on the fill: 200 rows
over three values are 200 rows over **three cache keys**. What that actually
does is worth stating precisely, because two of the three answers are nothing:

- *Mounted, it does nothing.* Each row's `MemoLazy` owns its own slot in the
  widget tree, paired by position, so rows never read each other's cache and
  the screen is correct. Measured, too: fifteen interleaved runs of `frame_cost`
  on one binary, distinct fixture against cycled, moved no number in the table
  by more than 1% and none of them resolved from its own spread.
- *Unmounted, it throws the list away.* The lot is keyed `(site, hash)`, so 200
  rows park **three** entries and 197 live subtrees are displaced. `frame_probe`'s
  `memo_parking_cost` prices the difference: with distinct fills the lot holds
  **400** entries (200 markets + 200 fills) and a remount costs **4.3ms**
  against a 21ms first mount; with the cycled fixture it holds **203** and the
  remount costs **7.0ms**, because 197 rows it had thrown away have to be built
  again. The fixture was hiding half the lot and overstating remount by 63%.
- *Displacing a parked subtree was a latent panic.* `park` dropped the entry it
  replaced inside the lot's `RefCell` borrow, and dropping a parked subtree
  re-enters the lot to park its own nested lazy state — `RefCell already
  borrowed`. Eviction had been written carefully around exactly this; the
  replacement path had not, and needs no full lot to reach: two rows sharing a
  key are enough. `parking_a_key_twice_reparks_the_subtree_it_displaces` is that
  test, and it panicked before the fix.

**And an absolute number is not comparable across two builds of the app.**
`__view` is one enormous function, so removing a boundary anywhere inside it
re-optimizes all of it. The two binaries differ by **299us [10%]** on a screen
holding *no fills at all* — work that is identical in both. `frame_cost` now
drives that empty screen in the same rounds as the full one, so the fills'
worth is a difference taken inside one binary rather than between two.

Twenty-five interleaved runs of `frame_cost` per binary, alternating which goes
first, 60 samples a run, p50 with the interquartile spread:

| | no boundary | behind `lazy` |
|---|---|---|
| **what the 200 fill rows cost** (paired, one binary) | **1380us [1354..1424]** | **319us [310..341]** |
| idle frame, end to end | 4497us [4419..4606] | 3129us [3053..3190] |
| the same screen with no fills — control | 3128us [3075..3190] | 2829us [2774..2925] |
| `__view` build only | 2410us [2377..2464] | 1237us [1225..1263] |
| cold redraw | 20758us [20436..21300] | 21063us [20423..21600] |

The row that means something is the first: **the 200 fill rows cost 1380us
unmemoized and 319us memoized, so the boundary takes ~1.06ms off an idle
frame** — 4.3x on the rows, and **24% of the 4497us frame**, not 31%. The
cross-binary idle frames differ by 1368us, of which 299us is the control, and
1069us is left: the same answer twice, which is why both rows are printed.

`frame_panels`' paired ablation agrees from the other side — five interleaved
runs per binary, 300 pairs each: the fills' own rows are **1858us [1706..2174]
of the frame without the boundary and 408us [295..455] with it**, and
1351us -> 137us of the view build.

Read only the large rows of that table. Each variant is a driver of its own, so
alternating the order inside a pair cancels which one runs first but not what a
particular driver's allocations happen to cost — and the small panels come back
*negative*: removing the alerts is worth -248us, the book -174us, the chart
-220us. Those are not savings from deleting rows, they are the per-driver bias
showing through a panel too small to clear it. The fills, the markets and the
everything-at-once ablation are the three that resolve.

`__view build only` halving (2410us -> 1237us, 0.51x) is real and is **not**
1.17ms off the frame. `lazy` lowers to a widget holding a closure and its
dependency: building the view stores those, and the row is built — or found
cached — later, in the tree walk inside the redraw. Work crossing that line
leaves the build and arrives in the frame, and only the frame is a frame. On a
steady screen the deferred work is then skipped entirely, which is why the
frame moved at all; on the frame where a row does change, it is paid.

The correctness contract is split where ownership actually lies. Runtime test
`diff_keeps_the_layout_memo_only_while_the_dependency_holds` proves that an
unchanged dependency retains the memoized layout and a changed dependency
drops it. Trading test `lazy_row_hashes_cover_every_rendered_field` changes
each rendered field of `SymbolRow` and `Fill` in turn and requires its
dependency hash to change. Together they prove invalidation without mutating
otherwise `pure` Rust formatters through `cfg(test)` counters.

The nine `capture` PNGs come out byte-identical with the boundary and without
it, which was also worth re-checking rather than re-quoting: when it was first
claimed, every preset held the same three fills, so nine identical captures
proved nine renderings of three rows. The `busy` preset — the one whose test is
named `trading_lists_longer_than_their_panels_render` — now holds
`demo_fills_many(200)`, so the comparison runs over 200 distinct rows, 200
distinct runtime ids and 200 distinct cache keys. Nine of nine still match, from
two binaries three seconds apart into two fresh `ICE_TEST_ARTIFACT_DIR`s.

What is left is inherent, and worth naming so nobody re-measures it:

- **The chrome is most of the frame now, and it has no boundary available.** No
  percentage is quoted here, because there is no honest one to quote: the
  boundary took cost out of the rows and left the chrome untouched, so the
  chrome's *share* necessarily rose — any figure read before the change is wrong
  for this sentence — and the share moves with the machine besides. The reading
  is `frame_panels`, `without any rows` against `full screen`, both out of one
  interleaved sweep. The ticket panel is the largest block in it and holds four
  `input`s, which `lazy` rejects outright — *input cannot live in lazy because
  iced text input borrows app state*, `check/options.rs`; the header strip and
  the ticket's quote both move on every beat, so a boundary there would miss
  every time it mattered.
- **Which block of the chrome is which cannot be measured, and the bound says
  it does not need to be.** Chrome is drawn unconditionally, so no edit to
  `Trading` takes the header, the tab bar, the ticket or the book's own frame
  off the screen the way an empty `Vec` takes its rows off — and a guard added
  for the probe would change the view the number is about. What can be swapped
  is the page under the header: `frame_panels`' `without any rows, portfolio`
  is the same screen with the same rows gone on a page whose generated view
  carries 231 nodes against the terminal's 302. The floor moves 501us to
  485us. Swapping the entire terminal chrome for a different page's is worth
  16us of a 1145us frame, so no single block inside it — the ticket is 138 of
  those 302 nodes, the largest by more than double — is worth a boundary, even
  before `lazy` refuses the ticket's inputs.
- **The memo the compiler already inserts covers most of the chrome.**
  `beat_cost` prints the count: 73 of the 113 component layout memos a frame
  reaches still hit on the frame after a feed beat, and the 40 that miss are
  the rows and the figures that just moved — the header price, the equity, the
  latency, the book, the tape, the alerts, the positions. On an idle frame all
  113 hit. The chrome that can be skipped for free already is.
- **The frame after a beat still costs ~1.5ms more than an idle one**, before
  and after this change alike — +1491us without the boundary, +1593us with it,
  n=11 interleaved runs of `beat_cost` each. `allMids` republishes every market,
  so in the probe's worst case all 200 rows genuinely changed and all 200 memos
  correctly missed. Only mounting fewer rows — `virtual_list`, not `lazy` —
  moves that one.
- **The win is steady-state only.** The cold redraw is 20758us against 21063us,
  spreads overlapping, n=25 each: not resolvable, which is the expected answer —
  the first frame builds all 200 rows either way, and a boundary can only skip
  work it has already done. What the boundary does buy off the cold path is
  *re*mounting: a screen torn down and re-entered reclaims its rows from the
  parking lot for 4.3ms instead of building them for 21ms.

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

### Not laying the rows out at all

The boundary the walk can stop at is the viewport, and the fills list now uses
it: `keyed fill in fills by=fill.tid virtual-row=26.0`. Both halves are load
bearing, and the reason the list went unvirtualized for so long was that only
one of them existed. `virtual-row=` was a plain column's property, and dropping
the key to get it costs more than it saves — every row's `lazy` memo is matched
by tree position, so a fill printing on top of an index-diffed column
invalidates all 200 — besides putting `FillFlash` under `W008`.

Two binaries built from `origin/main` and this branch, run alternately A, B, A,
B, four rounds each, `frame_panels` at 200 fills / 200 markets / 1760x940,
release. Medians of the four rounds:

| | keyed | keyed + `virtual-row=` |
|---|---|---|
| **fill rows, paired ablation** | **550us [529..570]** | **236us [225..253]** |
| all rows, paired ablation | 1344us | 1034us |
| idle redraw, full screen | 2093us [2010..2154] | 1705us [1653..1743] |
| `__view` build, full screen | 39us | 39us |
| rows rebuilt: cold / unchanged / one fill moved | 200 / 0 / 1 | 200 / 0 / 1 |

**The fill rows cost 550us and now cost 236us**, and the whole-list ablation
moves by 310us against the fills' own 314us, which is the same answer from the
other side. The frame is 2093us -> 1705us.

Both binaries were built after `cargo clean -p trading-example`, and each
generated view was grepped for `virtual_keyed_children` before it was run. That
is not ceremony. A first attempt at this table used a shared target directory
without the clean, and `ui-lang-build` served a generated view from a build
hours earlier — the compiler changed, the `.ice` sources did not, and nothing
in the fingerprint notices the first without the second. The debug build of the
same edit ran a whole 478-test suite green against a view that did not contain
the feature. See "An environment variable that no fingerprint tracks" below:
the trap is the same one, and the compiler is a build input like any other.

Three things that table says, none of them optional to say:

- **The build column did not move, and neither did the memo counts.** A
  virtualized-out row is still *built*; it is not laid out. Text is shaped in
  `layout`, which is where the 314us was. Any account of this change that
  claims fewer rebuilds is wrong, and `fills_stay_memoized_performance_contract`
  is the receipt: 200/0/1 on both binaries.
- **236us remains and is not a rounding error.** The visible rows are really
  laid out, 200 memo boundaries still hash their `Fill` every pass, 200 children
  are still diffed, and the `FillFlash` beside each `lazy` is outside the memo
  by design. Virtualization takes the shaping, not the list.
- **Offscreen rows leave the accessibility tree**, and `.ice` tests read that
  same snapshot — `trading_a_long_fills_list_lays_out_only_the_rows_it_can_show`
  asserts exactly that, in both directions, and is the test that fails on a
  build where the property is silently ignored. A read-mostly log of fills is
  what that trade is acceptable for; a collection that has to read correctly to
  assistive tech needs `virtual_list`, which owns its rows.

The earlier decline of this change priced it at 301us from `frame_panels`
before that probe was corrected, and rejected it on the two language gaps
above. The gaps were real; the number was not the reason.

### A wheel transaction can outrun a virtual column

The viewport boundary above originally learned about scrolling only from the
events iced forwarded to the column. That is not every physical wheel event:
iced treats consecutive wheel input as one scroll transaction and, after the
first event is captured by the scrollable, stops forwarding the rest to its
content. Four rows of overscan hid ordinary movement. Eight 100px wheel events
did not — the scroll translation moved beyond every mounted row while the
column still held the first window, producing the empty/frozen frame reported
when the list was shaken rapidly.

Generated scrolls that own `virtual-row=` content now synchronize the virtual
viewport from the scrollable's resulting translation after every captured
wheel event. Layout reopens only once the translation leaves mounted overscan.
`rapid_wheel_scrolling_never_runs_past_the_mounted_rows` drives twenty wheel
events through one native `UserInterface::update`; the trading test drives
eight down and eight immediately back through the first-class Ice test API.
Both fail by drawing or publishing none of the rows reached without the sync,
and pass with it. Positions, order book, alerts, and open orders now use the
same fixed-row boundary as markets, fills, and tape, so every scrolling data
list on the terminal follows that contract.

### Chart zoom was waiting on a quadratic history prepend

The chart's cached layer was not the zoom stall. In release at 1280x720 its
geometry rebuild measured 76–96us for the usual 120-bar view and at most 755us
with one million candles all visible. In the dense trading screen, six wheel
zooms in followed by six out measured 3.1ms p50 end to end.

The discontinuity came when zooming reached the oldest loaded bar. A history
page held the chart's shared-candle mutex while binary-inserting every older
candle at the front of the existing `Vec`. Prepending 1,000 candles before a
200,000-candle tape moved the held tape 1,000 times and measured **235.2ms**.
Feed-sized snapshots still take the tiny binary-update path; history pages now
sort once and linearly merge the two slices, with a fresh overlapping candle
replacing the held one. The same probe measures **4.74ms**, about **50x** less
time under the mutex. The merge regression deliberately covers an unsorted
page and an overlapping live candle so the fast path cannot silently reorder
or stale the tape.

### Two reported symptoms, and which of them was a frame

The terminal's owner reported list scrolling that "bounces" and a chart/positions
splitter that "sometimes lags". They read like one complaint about frame cost.
Only one of them was.

**The splitter is not a dependency bug.** `on lower_resized(_dx, dy)` fires once
per `CursorMoved` while dragging, and the suspicion was that `lower_height` had
leaked into something a row memo depends on. Counted rather than clocked — the
same `FILL_LABELS`/`MARKET_ROWS` counters the memo contracts use — 40 drag steps
on the 200-fill/200-market screen rebuild **0 fill rows and 0 market rows**, and
a drag step plus redraw costs what an idle redraw costs:

| 200 fills, 200 markets, 1760x940 | release (fat LTO) | debug |
|---|---|---|
| idle redraw | 3322us | 8987us |
| drag step + redraw | 3306us | 9058us |
| rows rebuilt over 40 drag steps | 0 | 0 |

So a drag costs exactly one frame, and the whole of the symptom is what a frame
costs. There is no invalidation defect to narrow.

**What was changed is the dev profile.** `cargo run -p trading-example` is a
**debug** build, and 9ms of build and layout before any paint is the only
reading here that leaves no headroom at 60Hz. The app now compiles at
`[profile.dev.package.trading-example] opt-level = 1`. Measured by running two
builds' `frame_cost` alternately on one shared machine, so a load spike lands
on both — absolute numbers run high there and only the ratio is a reading:

| idle redraw, dense terminal, dev profile | p50 | against opt-level 0 |
|---|---|---|
| app at opt-level 0 | 11221us | — |
| app at opt-level 1 | 7155us | **0.62x** |
| app at 1, dependencies at 2 | — | 0.51x |

Why the *app*, when the frame is spent inside iced: a frame is iced's generic
widget machinery monomorphized into the crate that *instantiates* it, and a
monomorphization is codegen'd at the instantiating crate's opt-level, not at
iced's. Raising the framework crates (`ui-lang-runtime`, `ui-lang-components`) to 2
moved nothing, for the same reason — they instantiate very little of what a
frame walks.

Optimizing the app cost no iteration speed **on the crate it was measured
on**: touching a source file and rebuilding was 2.51s against 2.58s at
opt-level 0. The crate then grew into the full HTS, and the edit rebuild grew
to **24.5s** — of which `-Ztime-passes` attributed **23.2s to LLVM_thinlto**:
at any opt-level above zero, rustc runs *local* ThinLTO across the crate's
codegen units unless told not to. The workspace now sets
`[profile.dev] lto = "off"` (2026-08-12), which returned the measured edit to
**3.7s** while `frame_cost`'s idle-redraw p50 moved 4433us → 4514us (~1.8%,
interleaved A/B on one box) — inside the probe's own ~0.5ms cross-binary
noise floor. Crates at opt-level 0 never ran the pass, so dev behavior is
otherwise unchanged. The dense-terminal debug frame itself now reads ~4.4ms,
well inside 60Hz either way.

**`[profile.dev.package."*"] opt-level = 2` was measured and rejected**, so
nobody repeats it. It is the last row above, a further 0.82x, and it costs
every job in CI: on run 31359419019 `Rust and Ice` went 7m12s to 22m20s
against a 30-minute cap, `Review bundles` 12m26s to 21m15s, `Workspace
(windows)` 5m16s to 22m09s. sccache is content-keyed on the compiler flags, so
changing them misses the entire dependency graph in every job at once.

None of this replaces `--release`, which is still what a real trading session
runs.

**The row fade is not it either.** #490's `window::frames()` subscription is
global — while any `FillFlash` is animating, the whole app rebuilds at refresh
rate — but the fade is mounted *beside* the `lazy` rather than inside it, and
the memo boundaries absorb it exactly as designed: 40 redraws with 200 rows
mid-fade cost **2309us** against **2343us** once settled, and rebuild **0** rows
either way. Scrolling during a fade rebuilds 0 rows as well.

**Nor does a beat over-invalidate.** The third suspicion was that a feed merge
dirties more than the rows it moved. Counted: one `market_ticked` rebuilds
**200 market rows and 0 fill rows**, and the same beat replayed with identical
numbers rebuilds **0 and 0**. The 200 are the data genuinely moving — `allMids`
republishes the whole universe — and nothing leaks into the panel beside it.
That is the invalidation rule working, and it is why the note above holds: only
mounting fewer rows moves the beat frame, not a narrower dependency.

**And the derived reads inside `responsive` were noise.** At the time,
`responsive #terminal-fit` wrapped the whole terminal page, every derived read
inside a responsive closure was tagged as escaping, and that vetoed the
per-view derived snapshot for the whole view — trading's generated view called
`__ice_derived_quote()` 8 times per build, each recomputing a chain that walks
the book. Priced with `bench`, that whole waste was **34.7us against 10.2us**
if each were computed once: **~25us of a 3300us frame**. Reading this table
before writing a codegen fix is what stopped one being written then. The
snapshot has since been replaced by a cache on the app struct that survives
frames and is cleared by the writes that can change it
(`SPEC.md` section 6), so a derived read now costs a
reference and the 8 reads compute once per feed beat, not once per read.

**The scrolling report was a different kind of bug, and it reproduced.**
`push_fills` and `push_trades` both put a beat's rows in front of the ones
already listed, so those two panels grow at the top. iced keeps a scroll offset
as an absolute distance from the top of the content and `Scrollable::diff`
touches nothing but the child tree, so the offset is never revised when content
is inserted above it. A reader 120px into the recent fills, one beat of four
fills later:

| | before | after |
|---|---|---|
| fills content height | 1040 | 1144 |
| fills `scroll_y` | 120 | 120 |
| the watched row's `y` | 1024 | **1128** |

The row they were reading moved 104px down the screen while the offset held.
That is the "bounce", and it is not a frame at all — no amount of frame budget
fixes it.

`anchor-y=keep` is the fix: `Anchor::Start`'s resting place with `Anchor::End`'s
correction, applied by `ui_lang_runtime::scroll_anchor` from inside `layout()`,
where the growth becomes visible and the offset is still read as a translation
at draw time. `End` alone cannot serve these lists — it stores the offset from
the bottom, so a list whose newest row is on top would open on its oldest.

What it does not cover is worth stating so nobody re-measures it: it reads
**growth**, which is all a wrapper around a scrollable can know. The tape caps
at 60 rows within seconds of a live market, and a capped list slides rather than
grows — constant height, no growth to read, and the rows still move. Covering
that needs row identity, which is where `virtual_list` already keeps its own
anchor (`RowsMeasured`, the `anchor`/`anchor_gap` pair). The fills list, which
is the one a human actually reads, sits below its 200 cap for a whole session.

### Why an edit costs what it costs

The `edit` number is not a fixed price. It tracks the size of the generated
module the edit dirties, because rustc's incremental cache is per **item** and
a component's whole view is one function. Every figure in this section was
measured on 2026-08-08, before outlining reached component uses with slot
content:

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
own notes measure typecheck as superlinear in function size (`~M^1.7`, still
its note today). It did not fire here: a component use whose slot content was
present rendered inline, on a `call.slots.is_empty()` clause in
`view/content.rs` that no longer exists.

Removing that condition outright took showcase's worst edit from **6.5s to
3.3s** and left total generated lines flat (15,506 -> 15,632), but it was
**unsound** — the native `render_surface` fixture stopped compiling with
`cannot find value 'item'`, because slot content snapshots the call-site
environment and can interpolate a loop variable that an outlined method has no
binding for.

Narrowing it to "outline unless the slot content actually reached a render-site
local" measured as sound and recovering nothing, and that measurement was
wrong: it was taken on a build that had already lifted the slot gate, so the
`ComponentParam` hard-capture it blamed (`expr.rs`, the
`None => self.hard_capture.set(true)` arm — still there today, now with a
`Some(marker)` arm beside it) was a consequence of the experiment rather than
the state of the tree. Counting the gate directly finds no showcase use that
hard-captures at all.

That ~2x was real and it was taken. #421 handed the slot snapshot the recorder
that stood at its call site and replayed its reads through it; all 164 showcase
uses now outline, and showcase's edit fell from 5.91s to 3.49s. What follows
supersedes this section — `Chasing the outlining 2x to its last blocker` keeps
the wrong turn on the record because it is easy to repeat.

### The floor, and what is under it

Once the dirtied module is small the front end is no longer the cost. Splitting
a 3.0s rebuild (`showcase`, editing `components/navigation.ice`) with
`-Ztime-passes`, measured 2026-08-08:

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

Lifting the gate outright breaks exactly one thing, in the native
`render_surface` fixture:
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

Only showcase moves, and that is the expected shape rather than a
disappointment: the other two already sit at the link-dominated floor
measured above, where the type check is not what the wall clock is waiting on.
Outlining buys nothing there, and showcase lands close to that same floor.

Outlining is not free at runtime — each use becomes a call through
`grow_stack` with its scope locals cloned in — so the frame was checked too.
`__view build only` on showcase reads 707/709/719/731us across four runs
against a 715us baseline: no change that the run-to-run spread does not
already cover.

### What the release profile was leaving on the table

Everything above is about the debug profile, because that is what the edit
loop uses. The release profile had no settings at all — `opt-level = 3`,
`lto = false`, `strip = "none"`, sixteen codegen units — and it turned out to
be the cheapest remaining lever on both axes at once.

Measured on showcase and trading, each configuration built from an emptied
release directory:

| Release profile | showcase | trading |
| --- | --- | --- |
| stock | 33.0 MB | 35.6 MB |
| `strip` | 25.4 MB | 29.2 MB |
| `strip` + `lto = "thin"` + `codegen-units = 1` | 22.3 MB | 26.6 MB |
| `strip` + `lto = "fat"` + `codegen-units = 1` | **20.5 MB** | **24.5 MB** |

Size alone would not settle the choice, because LTO also changes the frame,
so the probe was run under each. Thin LTO moves nothing a re-run does not
cover. Fat LTO does, and the two distributions do not overlap:

| showcase, p50 over three runs | stock | `lto = "fat"` |
| --- | --- | --- |
| `__view` build only | 736 / 735 / 721us | 657 / 637 / 646us |
| idle redraw | 3491 / 3448 / 3496us | 3026 / 3041 / 3020us |
| click + redraw | 13711 / 13851 / 13845us | 12267 / 12287 / 12221us |
| click + redraw, p95 | 15987 / 16557 / 15549us | 13283 / 13108 / 13095us |

Eleven to thirteen percent off every phase, and eighteen percent off the p95
of the worst one. That shape is what a generated view should give LTO: the
frame is thousands of tiny calls across the crate boundary into iced, and
inlining across that boundary is the whole point of the pass.

The bill is release build time, 2.2x — showcase and trading together go from
77.8s to 173.9s from an empty release directory. Nothing in the edit loop
pays it. CI pays it in one place, the performance-contracts job, whose test
binaries go from 34.2s to 95.8s: about a minute on a job that runs thirteen.
`cargo-ice` release builds (tag publishes only) go from 47.5s to 123.7s, and
the published binary from 14.1 MB to 9.7 MB.

`strip = "symbols"` is the one setting with a cost that is not build time: a
release panic backtrace loses its symbol names. Worth it at 8 MB a binary,
and debug builds are untouched.

### One anchor is not the edit loop

Every build number in this file came from `scripts/build_bench.py`, which
edits one configured literal per app — for showcase, the window `title`. That
is one anchor, and it turns out the cost of an edit depends on which part of
the app it lands in. Splitting the same rebuild four ways with `-Ztime-passes`:

| edit | rustc total | `type_check_crate` | `MIR_borrow_checking` |
| --- | --- | --- | --- |
| `app.ice` window title | 3.11s | 0.77s | 0.21s |
| `app.ice` window id | 3.25s | 0.83s | 0.20s |
| a component fragment | 2.48s | 0.04s | 0.02s |
| `handlers/app.ice` | **4.22s** | **0.93s** | **0.37s** |

The benchmark's anchor was neither the worst case nor a typical one. Editing a
handler — which is ordinary work, not a corner — cost the most, and almost all
of the excess was type and borrow checking.

`__update` and `__view` were the reason. They are the two large items a
generated app has, they answer to different sources, and they sat in the same
generated file. Each now gets its own fenced group, which `ui-lang-build`
splits into its own file, exactly as component methods already were. A handler
edit stops re-checking the view:

| | before | after |
| --- | --- | --- |
| `handlers/app.ice` edit | 3.77s | **2.90s** |
| `type_check_crate` | 0.93s | 0.04s |
| `MIR_borrow_checking` | 0.37s | 0.03s |

Fragment edits and top-of-app edits do not move, and are not claimed to.

`scripts/build_bench.py` now measures both anchors — a `handler` phase beside
`edit` — because documenting the hazard would not have stopped the next person
quoting one number. On every app that has a handler fragment the two differ,
and which one is worse changed with this fix:

| | root edit | handler edit |
| --- | --- | --- |
| showcase | 3.54s | 2.79s |
| music-example | 2.50s | 1.97s |
| markdown-example | 1.92s | 1.77s |

### Two ways these numbers went wrong first

Both mistakes produced clean-looking tables, so they are worth naming.

**A leftover anchor.** One script flipped several anchors in sequence and left
the tree dirty; the next measurement compared a build that still carried three
edits against a clean one. The tell was a diff of generated output showing
changes nobody had asked for.

**A flag change between the seed and the measurement.** A reset build run
without `-Ztime-passes`, between two builds that had it, discards the
incremental cache — the next "incremental" edit measured 11s instead of 3s.
Every build in a series has to carry identical flags.

**A corrupted build cache, read as a flaky machine.** rustc takes intermittent
SIGSEGVs on this box, and a process killed mid-write leaves artifacts behind
that fail in ways that look like a real defect: `rust-lld: section has a
sh_offset + sh_size that cannot be represented`, or `no method named to_owned
found for reference &str` reported against a third-party crate. Retrying does
not clear either — the corrupt file is an input to the next build. `cargo clean
-p <package>` does, in seconds. Three times in one session the reflex to retry
cost several full workspace builds before the error text got read properly. A
transient failure moves around; a failure that reproduces on the same crate
four times is not transient, whatever the signal number says.

**A stale build directory.** `target/debug/build/<pkg>-*` matches several
directories, and `ls ... | head -1` picks whichever sorts first, not the live
one — so generated sizes and build-script timings come from an old build and
look stable while the thing under test changes. `ls -t | head -1`. Two rounds
of numbers on the hot-reload track were retracted for this, and the same glob
appears in `scripts/build_bench.py`, which sorts by name and takes the last.
Relatedly, never `rm -rf` a package's `OUT_DIR` to force regeneration: cargo's
fingerprint then skips re-running the build script and `include_app!` fails
with "generated Rust is missing". Touch `build.rs` instead.

**An environment variable that no fingerprint tracks.** Anything read at
codegen time is a build input, and a build input nothing tracks is not a
switch — it is a switch-shaped thing that does nothing. Cargo does not rerun a
build script for a variable nobody declared `rerun-if-env-changed` on, and
touching `build.rs` is not enough either, because `ui-lang-build` then compares
its own input fingerprint and keeps the output it already has when the `.ice`
sources have not moved.

`ICE_TEMPLATE_VIEW=0` was such a switch, meant to force a view back onto the
compiled path so a capture from it could be diffed against the published one.
Setting it, touching `build.rs` and rebuilding produced a byte-identical
generated view three times running — on, off, and unset. The failure is silent
and it flatters you: both captures come from whichever path the directory
happened to hold, they are byte-identical because they are the same program,
and the diff reads as proof of parity. That result was produced and retracted,
and the switch was deleted rather than repaired, because `main` is now the
parity reference — diff against a revision where the node kind is not yet
published.

The general rule outlives the variable. To compare two codegen configurations,
give each its own target directory and build from scratch; a shared one carries
the other's output. And confirm which path a build actually took by grepping
the generated Rust for `__ICE_TEMPLATE_JSON`, across every file in
`ui-lang-generated/` — the view has its own `*__app_view.rs`, so
`ls -t | head -1` hands you the update phase instead.

And the machine is shared. A blocked A/B run put a load spike entirely on one
side and produced a 4.81s baseline against a 2.64s result for an edit that in
truth does not move at all. Interleaving the sides — A, B, A, B — and pooling
each side's samples is what settled it. When a result is large and one side's
spread is much wider than the other's, the spread is the finding.

### Splitting the groups finer buys nothing

Component methods are grouped per source fragment, which on showcase makes the
default component library one 8728-line module. rustc partitions codegen units
by module, so a single component's edit looked like it should be re-codegening
the whole library — `codegen_crate` plus `LLVM_passes` is 1.30s on that edit,
against 0.04s of type checking.

Grouping per component instead splits that module into about fifty, and the
generated file count goes from 12 to 60. Interleaved, under a quiet machine:

| edit | per fragment | per component |
| --- | --- | --- |
| `crates/ui-lang-components/src/ice/components.ice` | 2.38s | 2.35s |
| `components/navigation.ice` (control) | 2.55s | 2.61s |

Nothing. Whatever decides the codegen cost of an edit here, it is not the
module the outlined methods sit in — rustc's unit partitioning does its own
merging and splitting at 256 units and does not follow the module tree that
literally. Reverted; the fragment grouping stands.

### Where the loop stands

Per `scripts/build_bench.py`, three runs each, on one warm target directory:

| package | noop | script | root edit | handler edit |
| --- | --- | --- | --- | --- |
| showcase | 0.18s | 0.30s | 2.44s | 2.41s |
| trading-example | 0.21s | 0.27s | 2.81s | — |
| music-example | 0.21s | 0.28s | 2.00s | 2.04s |
| markdown-example | 0.20s | 0.27s | 1.75s | 1.79s |
| terminal-example | 0.19s | 0.02s | 1.33s | — |
| candles-example | 0.18s | 0.01s | 1.21s | — |

The two anchors have converged everywhere, which is the result worth reading
off this table: after the macro-invocation split there is no longer a part of
an app that is expensive to edit. showcase started this work at 6.5s.

The Ice compiler itself (`script`) is never the cost. What is left is rustc's
floor: link, codegen and LLVM, and the monomorphization walk plus unit
partitioning plus dep-graph serialization — roughly 0.6s, 0.9s and 0.6s of a
2.4s rebuild, all of which scale with the whole crate rather than with the
edit. Cutting further means generating fewer monomorphizations, not shuffling
the ones there are.

### The frame is one number, counted several times

`examples/showcase/src/frame_probe.rs` prints seven phases, and reading them as
seven costs is a mistake. The test driver simulates one event per
`UserInterface` build, so that a test can observe the state between a press and
a release; a running app batches a frame's events into one build. Every phase
comes out an integer multiple of a single build:

| phase | showcase | builds |
| --- | --- | --- |
| `__view` alone | 0.65ms | — |
| idle redraw | 3.02ms | 1 |
| cursor move | 3.07ms | 1 |
| state update + redraw | 6.82ms | 2 |
| scroll | 6.11ms | 2 |
| click + redraw | 12.27ms | 4 |

So the 12ms click is not a user-visible 12ms — a click costs an app two builds,
not four. The labels now carry the count. There is one number to optimize:
**one build and layout, 3.0ms**, of which `__view` is 0.65ms and the rest is
layout. Layout does not shrink with the viewport (2.99ms at 480x320 against
3.02ms at 1440x900), so it is the whole tree every time.

The repo's two answers to that are `lazy` (which memoizes the layout node, not
just the element) and `virtual_list`. Neither applies to showcase, and the
reason is worth recording rather than rediscovering: its catalog is one
`Catalog` component taking about fifty app-state parameters, so a `lazy` over
it would depend on essentially all state, and several of those types are not
hashable at all. The showcase view is a worst case by construction — every
component in the library, wired to live state.

`lazy` is also not free on the build side. Nothing inside a `lazy` closure
outlines: its content has to be `'static` and an outlined method borrows
`self`, so `outlining_active()` is false at any lazy depth. Wrapping a subtree
to win frame time moves its component uses back inline, which is the cost the
outlining work above removed. Measure both sides before taking that trade.

### The unit rustc re-checks is the macro expansion

After the outlining work and the `__update`/`__view` split, one number would
not go away: any edit landing in the app's root generated file cost ~0.75s of
`type_check_crate`, while the same size edit landing in a group file cost
0.04s. Five hypotheses, four of them wrong, and the wrong ones are the useful
part because each looked reasonable:

| hypothesis | experiment | result |
| --- | --- | --- |
| `__program`'s RPIT inference | fence it into its own file | no change, slightly worse |
| `include!` spans shifting the group files | emit the includes first | noise |
| the `impl` block is the unit | close and reopen `impl` around one item | no change |
| a fixed per-app cost | a 225-line root on another app | absent entirely |
| **the lint macro invocation is the unit** | give one phase its own invocation | **0.69s to 0.05s** |

Two of those experiments were wasted on the same unexamined assumption: they
moved the *suspect* out of the root and left the thing being *edited* behind,
so the edit still landed in the root either way. The experiment that settled it
came from fencing something trivial — a one-line `__title` — and editing that:
0.75s to 0.017s. If the cost follows a one-line function, it was never about
what the function contains.

The generator wraps everything it writes in one
`__ice_generated_items_*! { ... }` invocation, which exists only to attach
`#[allow(warnings, clippy::all)]` to each item — attributes on `include!` do
not reach included items, and a module wrapper would change name resolution.
But rustc re-checks a macro expansion as a unit, so every item in that
invocation shares one fate. Group files sit outside it, which is why they were
always cheap.

The fix is to close the invocation and open the next one at each generation
phase. `impl` blocks repeat freely (proven above by the experiment that found
nothing), and the boundary always falls between whole items:

| showcase edit | one invocation | per phase |
| --- | --- | --- |
| `app.ice` window title | 3.31s | **2.60s** |
| `state.ice` | 3.32s | **2.17s** |
| `handlers/app.ice` (control) | 2.56s | 2.60s |

The control matters: a handler edit already lands in the `__app_update` group
file, so it should not move, and it does not. The win is proportional to what
is left in the root — a small app has little there and gains little.

### How much of a view actually reloads

A hot reload is only as wide as the published template, so the number worth
tracking is not "does this root publish one" but "how much of it is data".
Every root publishes something; a construct the vocabulary cannot model
becomes a `subtree` hole, and a hole at the root swallows the whole view.

`cargo ice expand` is the instrument — the published form is a `&str` literal
in the generated Rust:

```
cargo ice expand examples/terminal/src/ui/app.ice \
  | grep -o '__ICE_TEMPLATE_JSON: &str = .*'
```

Counting `subtree` and `group` nodes in that JSON across every root in
`examples/`, over the two changes that widened the vocabulary:

| | padding refused | padding published | `if` became a group | overlay published |
| --- | --- | --- | --- | --- |
| roots publishing a template | 65 | 65 | 65 | 66 |
| roots that are *only* a hole | 43 | 19 | 9 | **6** |
| roots with no hole at all | 13 | **28** | 28 | 28 |
| published nodes | — | 223 | 333 | **367** |

The first row is why the second one is the measurement. `p=16.0` on a root
`col` refused the node, the refusal became a hole, and the hole was the entire
application — 24 roots reported a healthy template that reloaded nothing.

So read the hole count, not the template's existence, and expect the blockers
to be shallow and near the root. What refuses today, in descending order of
what it costs: components, which are 16 of the remaining holes and take a
whole subtree each — and `mount`ed ones keep a view off the template path
entirely; the contents of a group, since a branch publishes its structure but
not what is inside it (this is what still holds `trading` to three nodes); and
`lazy`, whose whole purpose is to keep its subtree out of a rebuild.

One trap this measurement set off, worth knowing before the next widening: the
id-to-source map that gives a captured widget its `.ice` line used to reset on
the first render-source guard pushed onto an empty stack. A view fills its slot
table — where every compiled hole builds its widgets and registers their ids —
before the renderer walks the node tree and pushes a guard for the root, so
that walk counted as a new pass and discarded the registrations moments before
they were read. It only surfaced once a template root had both a source of its
own and a named widget inside a hole, and it surfaced as one null in a manifest
against thousands of matching pixels. The pass now begins where the frame does,
in `Slots::with_capacity`.

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

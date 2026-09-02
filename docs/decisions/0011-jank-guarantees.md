# 0011: What the language guarantees against stutter, freeze, and rehydration

- Status: Accepted. `W019`/`W020` and the row boundaries they found (#801),
  the multi-slot layout memo (#800), the compiler-inserted component memo
  (#807), and its row keys and skipped diff (#812) are shipped; the freeze
  boundary (G3) is proposed with its rules and sequence.
- Date: 2026-09-02

## Context

Three kinds of jank reach an Ice user, and they have different mechanisms:

- **Stutter** — a frame costs more than its budget because per-frame work is
  proportional to content: rows built per pass, widgets rebuilt from state,
  text reshaped, keys formatted. iced rebuilds the whole view on every
  message and lays the whole tree out on every frame, with no dirty region.
  Split by phase (`Driver::redraw_phases`, added with #807), an idle
  showcase frame is ~0.65 ms of generated view, ~1.1 ms of iced diff and
  layout, and ~0.1 ms of event walk; the probes' end-to-end numbers carry
  another ~1.3 ms of driver overhead, which earlier readings had booked as
  layout (`docs/testing.md`, "The frame").
- **Freeze** — the event-loop thread blocks in `update()`: a `sync` extern
  doing I/O, a `stream`/`task` constructor opening a socket, an `editor`
  clone re-parsing a document, a markdown re-parse per token.
- **Rehydration** — a write to one field rebuilds and re-lays-out subtrees
  that never read it. Every message is a whole-view rebuild by construction;
  today only an explicit `lazy` is a boundary the layout walk stops at
  (0009), and a component is a rustc-time outlining with no cache line.

A 2026-09-01 audit read the compiler, the runtime, iced 0.14's loop, and
every example, and produced 81 mechanism claims; 42 were adversarially
verified from source (none refuted) and 14 of those judged material at the
examples' data sizes. The numbers that shaped this record, measured on the
dev box in release:

| | |
| --- | --- |
| showcase idle frame / view build | 2.9 ms / 0.52 ms |
| trading dense terminal idle frame: view / diff + layout / event walk | 57 µs / 1.30 ms / 117 µs |
| trading: 200 unbounded `FillRow`s, paired ablation | 1.35 ms of the frame |
| runtime: 150 lazy chat rows unchanged / all new | 14 µs / 6.5 ms |
| trading: a11y key chain + semantics clone per pass | ~100–150 µs |
| component memo, showcase idle frame, diff + layout (81 uses hit) | 1082 µs → 571 µs |
| component memo, trading dense terminal, diff + layout (68 uses hit) | 1300 µs → 1277 µs |
| row components keyed on their list, then a held key skipping the diff below (113 hits) | 1202 µs → 1074 µs |
| `cargo ice check` over every example before this record | 0 warnings |

That last row is the finding: the checks the language had were clean on
every example, and the jank was still there.

## What a guarantee can be in Ice

The design contract (SPEC §1) fixes the shape of an answer. A guarantee is
one of:

1. **A local diagnostic** over the resolved view — the checker sees every
   widget (a checked vocabulary), every repetition, every extern boundary,
   and where each read is rooted (state, derived, prop, row). The
   expression language has no loops and no collection builtins, so
   content-proportional work can only enter through a repetition or an
   extern, both of which it can name.
2. **A canonical construct** the diagnostic points at — one spelling for the
   fix, already in the language (`lazy … by … as`, `virtual-row=`,
   `&type` params, `derived`), never a second way to say it.
3. **A hidden mechanism** — generated messages, borrows, revisions, and
   caches the author never sees (0008, 0009). Where the compiler can prove
   a boundary sound it inserts it; the author changes nothing.
4. **Dev-time attribution** — the one dynamic tool, for costs the compiler
   cannot see through the extern boundary: the dev runner reports the `.ice`
   span that overran, with no runtime scheduling behind it.

Rejected as not Ice: runtime scheduling (React transitions, time-slicing);
stability annotations on types (Compose `@Stable`) — a second vocabulary
that leaks Rust; a declared cost class on `sync` (`sync fast`) — the
compiler cannot check it and the author would guess; a hard ban on `sync`
in handlers — `cef-browser` pumps a native message loop with one on a
16 ms timer, and that is the correct spelling for it.

## Decision

### G1 · Stutter: every repeated component row has a boundary

**`W019`** reports a `for`, or a keyed column, over a list rooted in state,
a derived value, component state, or a prop, whose row instantiates a
component, an extern component, or a nested repetition, and which has no
per-row boundary: not every child is a `lazy`, the repetition is not under
a `virtual-row` column (through `if`/`match`/outer `for`, which flatten
into the column), and the keyed column carries no `virtual-row` of its own.
Leaf rows are silent: the memo bookkeeping is not worth a row of text.

**`W020`** reports a plain `lazy` inside a repetition whose dependency is a
call or operator over the row — `lazy label_of(message) as label` — because
the memo must evaluate it on every pass before it can compare the key, so
the work it wraps is never skipped. A bare row value (priced by `W017` when
it owns a list) and a value rooted in state (keyed by revisions) are silent.

The canonical construct is the row memo idiom the language already had:
`lazy row, <state extras> as alias` around a component call, whose id stays
on the call and derives from the alias; the keyed form where the row owns a
list; a `virtual-row` column where the row height is fixed. Applied to the
twenty sites the checks found (trading 8, apple-music 8, app-store 2,
ai-chat 1, markdown-editor 1), every example is warning-free again with no
widget id changed. Two things the rollout taught, now in SPEC prose: a row
with an `f64` field needs a hand-written `Hash` over the bits (`LyricLine`),
and a row whose `selected` reads state beside a list-owning record splits
into two guarded keyed lazies rather than clone the record per pass.

**Multi-slot layout memo.** `ui_lang_runtime::flex` — which `flex`, a
`grid` with `min-cell`, and any `row`/`col` carrying a flexbox option lower
to — lays each child out with up to three different `Limits` per pass
(measure, final, stretch). A memo remembering one `(Limits, Node)` pair
missed on the second and third pass and on the next frame's first; the
whole apple-music shell sits under such a `flex`. The memo now keeps three
pairs, keyed by `Limits`, oldest evicted; `BustMemoLayouts` clears them all.

### G2 · Rehydration: a state write rebuilds only what read it

What holds today: a derived value is recomputed once per write to a
dependency (0008); every state field has a revision; a `lazy` rooted in
state hashes nothing on an unchanged frame (0009). What does not: a
component is not a unit of invalidation. Trading factors its screen into
176 outlined component uses and gets 176 rustc items and zero cache lines;
one animating badge subscribes the whole app to `window::frames()` and
re-lays-out every panel at 60 Hz.

**The compiler inserts a revision-keyed layout memo at a component use**
(`rev_memo`, #807) when three facts hold, all decided statically:

1. *Key.* The union of the revision reads of every expression the body and
   the call site evaluate is `Some` — the same `revision_reads` that keys
   `lazy`, folded over the outlined body and the arguments — and every
   callback argument's captured route payloads are state-rooted or `_`. A
   row-local, secret, or clock read anywhere in the union means no memo,
   as it does for `lazy` today.
2. *Element lifetime.* The memo caches the **layout node only**; the element
   is rebuilt every pass as it is now, so a body that borrows `&self` (a
   `&type` extern argument, a component-state borrow, an `editor`) is not
   excluded and `E150` does not apply. This is `memo_lazy`'s `MemoLayout`
   without its `'static` element cache. The build is 11–22% of a frame; the
   walk it skips is the other 78–89%.
3. *Layout purity.* A cached node is only correct if `layout(tree, limits)`
   is a function of the element tree and the `Limits`. From the runtime and
   iced 0.14 sources, the widgets whose layout reads state written *outside
   their own subtree's layout* are: the virtual family (`virtual-row`
   columns read a viewport the enclosing scrollable writes), `virtual_list`,
   `data_grid`, `scroll_anchor` (operates and corrects scroll inside
   `layout`), and `rich_text_editor` (mutates highlighter state across
   frames). `scroll`, `table`, `responsive`, and `pane_grid` are pure by this
   rule: what they read or rebuild in `layout` lives in the memo's own child
   tree and is keyed on the same `Limits`. The static half: a component
   whose expanded tree contains one of the impure widgets lowers unchanged.
   The dynamic half, for the day one is wanted inside a memo anyway: a
   `layout_invalidates(&self, tree) -> bool` hook on those widgets, which
   the enclosing memo consults before serving its node — the honest
   generalisation of `BustMemoLayouts`, scoped to the branch that moved.

A memo hit skips only the walk; the body is still built, so nothing about
mounting or booting changes. What shipped keys the direct callee's own
state through its instance scope and refuses a nested stateful component,
whose instance the outer site cannot name; lifting that needs the nested
scope expression at the outer site, and is the first follow-up. A body
that reads the implicit animation clock is refused too, so a 60 Hz fade
today relays out its own badge and everything the badge's parents do not
memoize — ticking the animating instances' revisions instead is the second.

Measured on the examples, memo off against on in one session: showcase's
diff + layout halves (1082 µs → 571 µs, 81 hits and 0 misses per idle
frame) and its `scroll` probe drops 5.6 ms → 4.3 ms; trading's dense
terminal barely moved at first (1300 µs → 1277 µs, 68 hits). The reading
that its rows owned the walk came from `frame_panels` deltas taken
end-to-end, which is driver overhead; on the build phase alone the virtual
columns and their live rows are ~165 µs of ~1.2 ms. Two follow-ups shipped
in #812: a `for`/keyed row or match payload keys on the revisions of the
list or value its view takes it from (a row is a function of its list; the
compiler's key, never `lazy`'s, which hashes the row so a prepend rebuilds
one row), which made every trading row component a boundary (68 → 113
hits) without moving the number; and a held key now returns from `diff` as
well as `layout`, which did (1202 µs → 1074 µs). A body holding a `lazy` is
refused for that, since a lazy hands its cached element to each new
instance in `diff`. What is left in the build is not a walk the memo can
skip: a `perf` profile puts allocation, string formatting (scope and id
`format!`s, float display), and the accessibility wrapper's per-pass
semantics clone at roughly a quarter of the redraw between them.

**`emit` in the same update turn.** An emitted component event is
`Task::done(msg)` today, which iced delivers on the *next* loop cycle after
a full view build, layout, and `subscription()` — one whole frame per hop,
two for showcase's one real path. The target is a compiler-known handler on
the same instance, so generated code calls `__update` directly after the
emitting handler's writes, as the run-lane arms already do, under a
compile-time acyclicity check over the emit graph. Same ordering, no frame.

Deferred with its reason: `virtual_list` publishes `Scrolled` as an app
message on every scroll event, so each scroll frame is a whole-app rebuild;
the fix is view-local state a widget owns without re-entering `view()`,
which is a language construct, not a lowering, and waits for a second
consumer. `overlay` keeps its covered base fully live; with the component
memo the base costs its hits, which is the cheaper fix.

### G3 · Freeze: only `sync` may block, and the loop knows when it did

Ice's promise is the arrow in SPEC §1: interaction → handler → *async*
extern → result handler. Three places the lowering breaks it without the
author spelling `sync`:

- **`stream`/`task` constructors run inline.** `Task::stream` defers the
  first poll behind `yield_now`, but the constructor — `fn(...) -> impl
  Stream` — is called on the loop thread; a socket opened there is a
  freeze the source cannot show. The constructor becomes `async fn` in the
  checked ABI and lowers to `Task::future(f(args)).then(Task::run)`; the
  generated probe enforces the signature. Breaking for every stream extern,
  updated in the same change.
- **`editor` clone and markdown re-parse** are handled at their sites by
  `&editor` params (0009) and the `markdown … append` form; the remaining
  gap — `state = markdown(text)` per message — gets no new construct until a
  second app needs it.
- **`sync` in a hot handler.** A cadence-aware `W021`: a `sync` extern
  called from a handler reachable from an input binding, a stream, a raw
  event, or a sub-second `every` is reported, naming the async form. The
  `cef-browser` pump is the known exception and keeps its `sync`; the
  warning is the language saying so at that line, not a ban.
- **Dev-time attribution.** `cargo ice dev` times every handler arm and
  every extern call and prints the `.ice` span of any that exceeds a frame,
  StrictMode-style. A debug build measures against 16ms; a release build,
  where the timings are the app's own, measures when `ICE_PERF` names the
  budget in milliseconds — no logging dependency and no build flag for it. It prevents nothing and attributes
  everything the extern boundary hides; wall-clock budgets stay out of CI
  (`docs/testing.md`, "Performance contracts").

### Runtime hygiene the audit priced

Ordered by measured share of a frame, all mechanism-verified: the a11y key
chain (a leaf's key is up to seven nested `format!`s, then FNV, then a
`SemanticSnapshot` clone per pass; ~100–150 µs on trading — intern the
suffix, hash incrementally from the parent's `StableId`, gate `logical_id`
on `test-runtime`, drop the identity `container` wrapper where the widget
carries an `Id`); `BustMemoLayouts` dropping every memo under a scrollable
when one virtual column escapes (scope it to the escaping branch); the memo
parking lot's O(n) scan on park and reclaim (unreachable at steady state,
paid on screen switches); `responsive` re-running its closure per layout
pass (a bucketed lowering when every size read is a literal comparison).

## Consequences

- A repeated component row without a boundary is a warning, and the
  examples carry the canonical fix at every site, so the frame cost of a
  list is bounded by what is in view or by what changed — not by its length.
- The language keeps one memo construct. The component memo is invisible;
  `lazy` remains the explicit spelling for a boundary inside a component.
- `W016`–`W020` are warnings, not errors, on purpose: the checker cannot
  see list sizes, and a four-row list of components is a real, cheap
  program. The evidence rule in `COVERAGE.md` is that the in-repo examples
  stay at zero.
- Every number above came from a probe, not a reading; the probe files
  (`examples/*/src/frame_probe.rs`) are the place a regression lands.

## Sequence

1. `W019`/`W020` + example boundaries — #801.
2. Multi-slot layout memo — #800.
3. Component-use layout memo (G2) — #807, with `Driver::redraw_phases`
   and the memo counters as its evidence.
4. Row components keyed on their list; a held key skips the diff — #812.
5. The build's remaining floor: scope/id `format!`s and the a11y semantics
   clone per pass (profile-led), then the nested-stateful and
   animation-revision follow-ups above.
6. a11y key interning and `logical_id` gating — runtime + codegen.
7. `emit` in-turn delivery with the acyclicity check.
8. `stream`/`task` async constructors; `W021`; dev-runner timing.

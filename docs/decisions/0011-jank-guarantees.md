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
- **Dev-time attribution.** Every view build, every handler arm and every
  extern call but a `pure` one is timed, and the `.ice` span of any that
  exceeds a frame is printed, StrictMode-style — the three answer *which* of
  a frame's parts overran before any of them says why. A debug build measures
  the two event-shaped spans against 16ms; a release build, where the timings
  are the app's own, measures when `ICE_PERF` names the budget in milliseconds
  — no logging dependency and no build flag for it. The view span reports only
  under a named budget in either profile, because it runs every frame and a
  `-O0` frame is not a measurement of the app. `pure` is the exception because it is the one kind the language holds a
  promise about and the one a view calls per node per frame: a guard there
  would grow the generated view, which is what decides Ice's build time, for
  work `--frames` already prices as view time. The same spans report on the
  way out of a panic — the `.ice` construct that was running, innermost first
  — since the Rust location a panic prints is inside generated code or inside
  the extern's body, neither of which the author wrote. It prevents nothing and attributes
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

That last one has since been priced, on trading's dense terminal, where
`responsive #terminal-fit` wraps the whole screen: the rebuild is 988 us of
a 1159 us frame, and it is charged to layout — which is why that frame reads
as 85% layout while every memo under it hits. Deleting the wrapper and
substituting the literal width each read compares against does not recover
it. The subtree simply moves into `__view`, 38 us to about 1000 us, and the
total does not come down — about 1420 us against 1159 us, which is a
different build of `__view` and so not a strict comparison, but plainly not a
saving. A bucketed lowering has to make that build rarer; a version that only
relocates it is already measured, and buys nothing.

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
   animation-revision follow-ups above. The clone is done; the
   `format!`s are measured and declined in 6. The floor also held one
   thing that was not a string at all: every generated view function
   rebuilt the app's `iced::Theme`, so a build ran iced's oklch palette
   generation once a frame for a value that only changes on a theme
   switch. Memoized on the palette in #865 — apple-music's
   `idle frame: view` 75 -> 68 us, its whole frame 198 -> 189.
6. `logical_id` gating — shipped in #861. The generated view hands its
   key to `logical_id_maybe` only under `cfg!(test)`, joining the
   render-source push and the id registration, which were already gated
   that way, and an identified node that only reads its key borrows its
   scope binding instead of cloning it. Measured on apple-music's
   `__view build only`: a test build stays at 1273 allocations and
   195,739 bytes exactly, and a shipped one becomes 1175 and 189,280 —
   98 allocations, 6,459 bytes.

   The gate belongs in the generated view rather than inside
   `logical_id`, and that is the load-bearing part. Gating on an active
   render source instead — the obvious runtime spelling — reads as
   equivalent and is not: it silently drops the logical id of every
   widget built by a *hand-written* view under the driver, which six
   existing runtime tests caught. `cfg!(test)` at the call site is exact
   where the render source is a proxy.

   Related and unchanged by that: the `widget::Id` a node carries must
   have the same spelling in a test build and a shipped one. `cfg(test)`
   may add a string; it may never change an id's form. #809 is what the
   alternative costs — codegen skipped a navigation root the test mount
   wrapped, so a shipped daemon had no Tab traversal that every test
   said it had. The gating above obeys this by construction: it drops
   metadata nothing outside a test reads, and touches no id.

   **Interning the key chain is measured and declined.** The `format!`
   chain that builds scopes and keys is the larger half of the identity
   cost — emitting no scope or key strings at all leaves 886
   allocations, so the chain is 251 of the 1273 against the 98 the
   gating removed, 2.6x more. It is still not worth buying, and the
   reason is the size of the slice rather than the size of the
   allocator. A sampled profile puts about 38% of this build in libc's
   `malloc`, `cfree` and `realloc` — allocation is the largest single
   thing in it — so 251 allocations of 1273 is roughly 3% of the build,
   about 2 us of 70, at or below what a p50 comparison resolves on this
   box. The shipped gating measured exactly that: dropping 98
   allocations did not move `__view build only`, which is 91-96 us
   before and after. Do not read this entry as "allocations do not
   matter here"; 98 of them are simply too few to see. The price would
   be an invariant no compiler checks — `codegen/view.rs`
   and `codegen/statement.rs` rebuild the same path independently, the
   latter for a handler's `focus`/`scroll` target — for 251 allocations
   and 25 KB that do not show up in a frame. Two things would reopen it:
   a build where allocation count is itself the constraint (a fixed
   arena, a no-allocation contract over the view build, a target where
   the allocator is the bottleneck), or a numeric constructor for
   `iced::widget::Id` upstream. `Internal::Unique` is private today, so
   even a hashed id is `Id::from(hash.to_string())` — a short allocation
   at 322 sites rather than none, which is most of why interning cannot
   reach zero.

   **The first of those two conditions is now met, and the 3% above is
   showcase's number, not the language's.** Measured on trading's dense
   terminal by ablating the view-path emitters in `codegen/view.rs` and
   `codegen/view/content.rs` — every identified scope, every keyed scope,
   every `@kind:line` key and the identity `container`'s id all replaced
   by `String::new()` — against an unablated build of the same source.
   Two release binaries, twelve interleaved runs with the order rotated
   each round:

   | | frame, `redraw_phases` total |
   | --- | --- |
   | as written | 1137 us |
   | no identity strings | 890 us |

   Twelve pairs out of twelve, with no overlap at all: the ablated
   maximum is 940 us and the unablated minimum is 1126 us. That is
   **247 us of a 1137 us frame, 22%** — against the ~3% of a 70 us build
   this entry declined on. Memo counts are identical in both binaries
   (component 113/0, lazy 57/0), so the ablation moved no boundary. The
   `for`-loop scopes in `codegen/expr/children.rs` are still in the
   ablated build, so 22% is a floor.

   What that does and does not establish: the ablation is an **upper
   bound on any fix**, because a real interning keeps one
   `Id::from(hash.to_string())` at every node that carries a widget id,
   and this ablation keeps none. It does not say how much of the 247 us a
   fix recovers. What it does say is that the size of the slice is a
   property of the app, not of the language: a 25-widget catalog screen
   and a 302-node terminal do not sit in the same regime, and an entry
   that declines on one is not evidence about the other. The same shape
   turned up the same day in `virtual_children`, whose design premise —
   "construction is under half a percent of the bill" — was an ai-chat
   leaf row at 0.24 us against a trading component row at 8.5-9 us.

   Still not taken here: the price named below is unchanged, and 247 us
   of a frame that is already almost entirely construction argues at
   least as strongly for the boundary that skips the build. Whoever
   reopens it should price both against each other rather than take this
   number as a decision.

   If it is ever taken, the drift between the two spellings
   needs a test that fails when they diverge: a generated app whose
   handler focuses a keyed target, asserted through a real widget op
   rather than through the driver's own target resolution.
7. `emit` in-turn delivery with the acyclicity check.
8. `stream`/`task` async constructors; `W021`; dev-runner timing.

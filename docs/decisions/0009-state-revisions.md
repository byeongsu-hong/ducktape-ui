# 0009: State fields carry compiler-owned revisions, and `lazy` keys on them

- Status: Accepted
- Date: 2026-08-23

## Context

`lazy value as alias` memoized on a hash of the value: every view pass cloned
the value into the memo tuple and hashed the clone, so a `lazy` over a state
list paid a deep clone per frame to discover that nothing had changed. The
keyed form avoided the clone for its value but still cloned every `str` key it
was given, and a key that was a component state field was cloned out of the
instance map each pass. Derived values had the same blind spot: nothing in the
generated program knew whether a state field had been written since the last
frame.

## Decision

Every app state field and every component state field has a `u64` revision in
a `__ice_rev` array beside the fields, indexed by declaration position. One
generated write helper (`codegen::state_write::state_write_code`) is the only
path through which a state write is emitted — handler assignments, the
`mem::take` self-assignment, markdown append, combo push, animation start,
controlled input and editor bindings, component bind props, task handles, and
debug spans — and it ticks the written field's revision and clears the
derived cells that read the field (0008), as one event. An assignment
compares first through autoref specialization (`ui_lang_runtime::rev`), so an
extern type need not implement `PartialEq`; a type that cannot compare, an
in-place mutation, and a self-assignment tick unconditionally.

`lazy` keys its memo off those revisions wherever the dependency is rooted in
state — read directly, through a `derived` value (whose app-state read set the
checker already decided for its cache clears), or through a component prop
whose call site baked a state-rooted expression — and materializes the value
inside the memo builder on a miss. A dependency that reads a row-local
binding keeps the value-hashed lowering: a row's identity is the row. An
explicit `by` key that is exactly a state field is subsumed by that field's
revision.

A dependency that reads something no revision tracks keeps the value-hashed
lowering as well: a secret, whose store is written by the input directly and
wiped without a tick; a recomputation-unsafe builtin such as the implicit
animation clock (`animation.animating(fade)`) or `window_id.unique()`, whose
result moves with no write behind it.

Every instance seeds its revisions from a process-wide counter
(`ui_lang_runtime::rev::seed`: instance number in the high 32 bits, writes in
the low 32). The memo parking lot is per thread and keyed by site, scope, and
the hashed revision tuple, so without the seed a second app instance on the
same thread — one test driver after another, `*driver.state_mut() = ...` — or
a mounted component leaving and coming back would reclaim a subtree built
from the previous instance's state whenever the write counts happened to
match.

Revisions are not a language feature: there is no `rev()` built-in and user
code cannot read or write them.

## Consequences

- An unchanged frame deep-clones and hashes nothing for any `lazy` over
  state, including `str` keys of component state.
- A write that stores an equal value rebuilds nothing when the type
  compares. A write through an extern type without `PartialEq` always
  rebuilds — the trade-off for never requiring the bound; deriving
  `PartialEq` on the extern type restores the comparison.
- A keyed `lazy` whose value is itself a state field now rebuilds on every
  write to that field, not only when an explicit key moves; the author's
  "stale until a key moves" contract survives only for row values.
- A plain `lazy` whose dependency mixes state with a row-local binding, or
  reads a prop fed from an enclosing component's state, keeps the eager
  lowering — correct, just not revision-keyed.
- The memo parking lot keys on a hash of small `u64` tuples; `memo_lazy`
  itself is unchanged.
- Only generated code ticks a revision. A Rust write that bypasses the
  handlers — `driver.state_mut().field = value` in a probe, or a `&mut`
  reached through an extern — leaves the revision where it was, and a
  state-rooted `lazy` over that field keeps its cached subtree with no
  diagnostic. A probe that mutates through `state_mut` must not rely on a
  state-rooted `lazy` to reflect the write; assert through `dispatch`, or
  keep the probe's lazies row-keyed.

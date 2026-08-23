# 0008: Derived values are cached until a write they depend on

- Status: Accepted
- Date: 2026-08-23

## Context

A `derived` value lowered to `fn __ice_derived_x(&self) -> T`, recomputed and
cloned on every read. A narrower per-view snapshot coalesced some reads inside
one `view` call, but it had to prove that no read escaped into a retained
closure, so most real views got nothing from it, and the specification promised
that no derived value survives a frame.

The promise shaped applications. The ducktape app keeps `derived` to six
scalars and maintains eight list-shaped state mirrors by hand in its handlers,
with a test that greps the `.ice` source to keep them in sync. That is the
runtime dependency graph the language declined to build, rebuilt per
application, with the same bugs available each time.

The constraint that made caching unsafe no longer exists. The checker already
rejects `sync` externs and every recomputation-unsafe built-in inside a
derived expression, so a derived value is a pure function of app state and
other derived values. A pure function of state is safe to cache for exactly as
long as that state does not change.

## Decision

Every derived value owns one `OnceCell` in a generated `__IceDerivedCache`
stored on the application struct. Its accessor is
`fn __ice_derived_x(&self) -> &T { self.__ice_derived.x.get_or_init(|| expr) }`,
and reads bind to the dereferenced reference: a borrowing use site borrows the
cell, an owning one clones it. The per-view snapshot, its read profile, and
its escape analysis are deleted.

Invalidation is direct. The compiler reads each derived expression, follows
derived-to-derived references, and inverts the result into a map from app-state
field to the derived values that read it. Generated code writes an app value
through **one helper**, `codegen::derived::state_write`, which emits the write
followed by `self.__ice_derived.<d>.take();` for each dependent. Every emitter
of an app-state write routes through it: handler assignment in all its forms
(plain, self-moving, combo replacement and `push`, animation `go`, markdown
`append`, `abortable` handle capture, `debug start`/`finish`, secret wipe), the
controlled `input` and `editor` update arms (which is also where a component's
`bind` prop lands when it is bound to app state), and therefore every test step
that dispatches into them. A write clears the cells before the handler
continues, so a read after a write in the same handler is fresh.

Fields the language does not expose as state — run-lane generations and task
handles, pane-grid layouts, the secret store — are written directly: no
derived expression can read them, so there is nothing to clear, and
`codegen/tests/derived_cache.rs` pins that a secret wipe clears nothing.
Component-local state is out of reach of a derived expression by the checker's
scope rules, so a component handler's writes clear nothing either.

A self-moving assignment (`rows = f(rows, ...)`, lowered through
`mem::take`) declines the move when its right-hand side reads the target
through a derived value, because the cell may be empty at that statement and
the recomputation would otherwise read the emptied field.

## Consequences

- A derived value is evaluated at most once per write to a dependency, across
  any number of frames and reads. Handler-maintained mirrors of list-shaped
  derived values are unnecessary.
- The write-helper invariant is the correctness boundary. A new emitter of an
  app-state write that bypasses `state_write` leaves a stale cell, and
  `every_app_state_write_clears_the_derived_cells_that_read_the_field` in
  `crates/ui-lang-core/src/codegen/tests/derived_cache.rs` fails on the
  generated output of every write form the language has. Extend that fixture
  when adding a write form.
- The helper covers generated code only. Rust that writes an app-state field
  directly — a probe through `Driver::state_mut`, an example unit test
  assigning `app.field` — clears nothing. Such a write must happen before the
  first derived read, or be followed by
  `app.__ice_derived = Default::default()` (the cache is `pub(crate)`), or the
  next frame reads the stale cell.
- `SPEC.md` no longer promises that nothing survives a frame; it promises the
  opposite, with the dependency rule that makes it sound.
- Revision counters and `lazy` are untouched: a `lazy` dependency that is a
  derived value is still hashed per frame, it is just no longer recomputed per
  frame.

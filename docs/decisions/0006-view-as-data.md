# 0006: A view is data the runtime renders

- Status: Proposed (proof of concept)
- Date: 2026-08-08

## Context

Editing one `.ice` line costs a full Rust rebuild and a process restart.
Measured on this repository: 1.4s for `starter` (45 lines of Ice), 5.0s for
`showcase` (294 lines plus the imported component library). The 2026-08-07
performance campaign established that this loop is ~95% rustc and front-end
bound, on roughly 77k monomorphized instances of iced's widget machinery.

Counting what those instances come from says where the cost lives.
`showcase` compiles to 15,497 lines of generated Rust containing 1,921 iced
widget constructions, 761 accessibility wrappers, 165 style closures — and
**351 references to `self`**. Barely two percent of the generated view is
genuinely dynamic. The rest is structure and literals expressed as code, and
it is precisely the part that multiplies into monomorphizations.

Decision 0005 records that live reload was built once
(`feat/ice-live-reload-parity`, 45 commits) and deleted on 2026-08-03. Its
recorded shape explains why: a *second* lowering path, `live.rs`, that
deliberately rejected everything it could not interpret, and at deletion
supported only `row` and `column`. Two implementations of one language, one of
them permanently behind.

## Decision

A view compiles into two halves.

The static half — widget structure, literals, style tables, geometry,
accessibility segments, and source coordinates — is published as **data**: a
`ui_lang_runtime::template::Template`, rendered by one renderer that lives in
the runtime crate. The dynamic half — every expression that reads application
state or names a message — stays compiled Rust, and reaches the renderer as a
positional **slot table** that the generated `__view` refills each frame.

There is one renderer, used in development and in release. The prior attempt's
failure mode was two implementations diverging; the defence is not to have a
second one. A dev-only interpreter is explicitly rejected.

A construct the vocabulary does not model becomes a **hole**: the compiler
renders that subtree exactly as before and hands it to the renderer through the
slot table, and the structure around it stays data. Falling back per subtree
rather than per view is what lets real applications reload at all — one `if` no
longer costs a whole screen its template. This is the permanent boundary, not
scaffolding: control flow, components, and native `extern` surfaces are
expected to stay compiled, and what is inside a hole reloads only when the
binary does.

`if`, `for`, and `match` are the exception, because they contribute a variable
number of children to their parent rather than rendering as one element. A
layout containing any of them becomes the hole instead, so the fallback
boundary rises to the nearest enclosing layout.

A running process may accept a new template when, and only when, the slot table
it fills each frame is unchanged — same expressions, in the same order,
identified by a fingerprint over their generated code. Structure, literals,
colours, spacing, and accessibility segments may all change freely. Anything
needing a value the binary does not compute goes through the compiler, via the
rebuild-and-restart path decision 0004's tooling already provides.

## Evidence from the proof of concept

Scope: layouts, containers, text, inputs, and buttons — 5 of 41 view node
kinds, with everything else falling back per subtree. **60 of the repository's
67 app roots publish a template**; 2 keep a fully compiled view (mounted
components), and 5 are fragments that do not analyse standalone.

- **Rendering is identical.** `cargo ice inspect` captures on the template path
  and the compiled path produce byte-identical PNGs and zero manifest
  differences, including source provenance — for `starter`, and for
  `markdown-editor`, a real application whose view is largely holes. The whole
  workspace suite passes with 60 roots on the template path. The first draft
  failed the capture check on accessibility paths, which is how the oracle
  earned its keep: only an author's `#name` opens a scope for descendants, and
  the template initially opened one for every node.
- **A binary renders a template it was never built against.** Editing the
  emitted JSON — new heading, larger type, wider spacing, a reversed row, a
  restyled button — and pointing the compiled `starter` at it renders every
  change, while the state-backed slots keep working. No compiler ran.
- **The reload path costs ~0.6ms** (parse, check, lower, emit), against ~1.0s
  for the rebuild it replaces on that app. Diagnostics are unaffected: the
  reload path runs the same front end.
- **Rebuilds roughly halve at scale.** A generated probe of rows built from the
  modelled vocabulary, rebuilt after touching its `.ice`:

  | probe | compiled path | template path |
  | --- | --- | --- |
  | 200 rows (600 slots) | 7.23 s | **3.65 s** |
  | 600 rows (1800 slots) | 23.5 s | **13.0 s** |

  Generated Rust for the 200-row probe falls from 1.63 MB carrying 2,007 iced
  widget constructions to 0.67 MB carrying none.

Two things this does **not** establish, both worth naming.

The remaining template cost is not the published JSON. A 1.9 MB string literal
compiles in 0.03 s standalone, and a 1.57 MB template with **no** slots rebuilds
in 3.40 s against a 1.35 s floor. What costs is the slot table: 1,800 slot
expressions in one `__view` body account for ~9.5 s, and emitting them as
statements rather than as one array literal recovered only ~2.5 s of that. The
compiled path already solves this problem — `codegen::view::outline` moves large
views into per-fragment methods precisely because type and borrow checking are
superlinear in function size — and the slot table needs the same treatment.
Until it gets it, the measured win understates what the design can reach.

Nor does it establish anything about a `showcase`-scale application, whose views
are built from components. A component is a hole today.

## Rejected alternatives

### Keep compiled views and reload nothing

The status quo. It is the honest baseline, and decision 0004's staged restart
already makes it safe. It leaves the measured 95%-rustc edit loop in place.

### A dev-only interpreter beside the compiler

Slint sustains this by sharing one frontend across two backends, so it is not
unworkable. It is rejected here because this repository has already run the
experiment and lost, and because the build-time win only materialises when
release uses the same path.

### Binary patching (subsecond) or `hot-lib-reloader`

Both patch compiled code rather than replacing data. Subsecond is the stronger
option and reaches Rust handler bodies that a template never will, but it is
experimental, tip-crate-only, and linker-sensitive. It is complementary to this
decision, not a substitute: it would cover the half that stays compiled.

## Consequences

Views the vocabulary covers lose rustc's type check as a backstop on generated
view code, leaving that guarantee entirely to the Ice checker. `cargo ice lint`
maps generated Rust errors back to `.ice` lines; for a templated view there is
no generated Rust to map, so that safety net shrinks to the compiled half. The
pixel-diff evidence in `cargo ice review` becomes the load-bearing check, and
widening the vocabulary must be gated on it.

Node kinds must be implemented once in the renderer and once in the emitter,
and the two are only proven consistent by capture diffing. That is one
implementation plus a serializer, not two implementations of the language, but
it is not free.

Template strings are cloned per frame rather than borrowed, so an element never
outlives the template it came from and a reload can drop the previous tree.

## Revisit trigger

Revisit if release frame cost regresses measurably against the compiled path,
in which case a template-to-Rust compiler can be reintroduced as a pure
optimisation over the same data — with the renderer as the oracle to
differential-test it against. Revisit the all-or-nothing rule if a view mixing
modelled and unmodelled nodes becomes common enough that whole-view fallback
wastes the mechanism.

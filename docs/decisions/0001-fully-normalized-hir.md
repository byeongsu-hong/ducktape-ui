# 0001: Fully normalized HIR boundary

- Status: Accepted
- Date: 2026-07-31

## Context

Successful Ice analysis currently produces a nominal `CheckedDocument`, but it
still contains and dereferences to the source `Document`. The Rust code
generator consumes that checked AST directly and imports checker helpers for
some semantic decisions. As component defaults, bindings, named events, slots,
recipes, typed matches, scoped widget identity, and asynchronous lifetimes have
grown, this makes it increasingly easy to repeat or diverge on the same
decision in the checker and code generator.

New Core syntax is frozen while this boundary is established. The objective is
not to introduce another public language surface or prepare speculative
backends. It is to make every accepted Ice construct have one checked,
canonical meaning before backend emission begins.

## Decision

Ice will introduce a complete, private, typed high-level intermediate
representation named `LoweredProgram`. The target pipeline is:

```text
Source AST
  -> CheckedDocument
  -> LoweredProgram
  -> Rust/Iced code generation
```

The source AST preserves how a program was written for parsing, formatting,
editor features, and diagnostics. `LoweredProgram` preserves what the checked
program means. It contains Ice semantics and runtime-required identities, but
no Rust token fragments or Iced-specific types. Concrete Rust and Iced API
choices remain the responsibility of the backend emitter.

The lowering boundary has these invariants:

1. Only the lowering module can construct a `LoweredProgram`.
2. Rust code generation accepts a `LoweredProgram`, never a source `Document`
   or `CheckedDocument`.
3. Code generation does not import checker helpers or repeat semantic
   validation.
4. Every HIR node carries an `OriginId`. A shared origin table retains the root
   or imported source location and its origin stack without copying paths into
   every node.
5. Invalid or unresolved states are not representable in HIR. Lowering either
   produces a complete program or returns a source-mapped diagnostic.

Before code generation, lowering resolves or removes all language sugar,
including:

- declaration and reference names into stable typed IDs;
- component prop ordering, defaults, read/bind capability, and concrete state
  references;
- named event routes, default-event shorthand, and `forward` blocks;
- required and optional slots and folded `provided(Slot)` expressions;
- recipe inheritance, utilities, variants, palettes, and theme token IDs;
- exhaustive match arms and payload bindings;
- component scopes, widget identity, and reconciliation identity;
- task, stream, `run latest`, and `run replace` call-site identity;
- retained or mounted lifetime behavior and its cleanup obligations;
- all remaining type, capability, and ownership choices required by emission.

The HIR specification covers the whole accepted language. Delivery remains
incremental: component calls and contracts first, then styles and themes,
expressions and matches, asynchronous behavior and lifetime, application
settings and tests, and finally the remaining backend surface. Each migrated
slice removes its old code-generation path in the same change. There is no
permanent mixed AST/HIR fallback or compatibility shim.

### Implementation status

The component-contract and component-call slice is implemented. The private
owned `LoweredProgram` is now the only input accepted by Rust generation.
Component and call IDs, ordered props with selected defaults, writable state
references, direct/forwarded named events, ordered required/optional slots,
output routes, scope identity, storage lifetime, and parented root/import
origins are fixed before emission. The old component render path that searched
source names and repeated those decisions has been removed. Component state
storage, boot, update, mounted cleanup, and call rendering consume the resolved
contract.

The program still owns AST-backed nodes for semantic families not yet migrated.
Styles/themes, expressions/matches, asynchronous call sites, application
settings, tests, and the remaining views therefore remain open implementation
slices; this status does not satisfy the migration-complete criteria below.

The migration is complete when:

- the code-generation module has no source-AST or checker dependency;
- `CheckedDocument` no longer dereferences to `Document` as an escape hatch;
- structured HIR fixtures cover every accepted semantic family;
- backend tests assert only backend-specific output where possible;
- imported-source diagnostics and generated Rust source maps retain their
  existing behavior; and
- lowering and generation remain within explicit performance budgets.

## Rejected alternatives

### Keep generating from the checked AST

This leaves syntactic alternatives visible to the backend and permits semantic
decisions to spread between checking and emission. The cost grows with every
feature even while Core syntax is frozen.

### Add isolated, permanent mini-IRs

Feature-specific lowering can help migration, but leaving unrelated AST and HIR
paths as the final architecture preserves the same ambiguous boundary. Mini-IRs
are accepted only as steps toward the complete `LoweredProgram`.

### Put Rust or Iced constructs in HIR

Backend-shaped nodes would make semantic tooling depend on emission details and
would prevent API inspection and other consumers from sharing the canonical
program meaning. Backend-specific planning stays private to code generation.

### Replace the pipeline in one large change

A big-bang conversion would make source-map and generated-program regressions
difficult to isolate. The destination remains complete HIR, but migration is
split into independently verified vertical slices.

## Consequences

The compiler gains another owned representation and an explicit lowering pass.
That adds implementation work and some build-time memory, which must be
measured. In return, the checker and lowering boundary become the single source
of semantic truth, code generation becomes a translation step, HIR fixtures can
test meaning without comparing large Rust strings, and future tooling can
consume the same canonical program model.

During migration, every pull request must identify the semantic family it
moves, the old path it deletes, its invalid cases, its source-origin evidence,
and any performance change. New Core syntax remains out of scope until the full
boundary is complete and the Core admission rule independently justifies it.

## Revisit trigger

Revisit the representation when measurements show that HIR construction or
retention materially violates compiler performance budgets, or when an accepted
Ice semantic cannot be represented without backend knowledge. A revisit may
change storage, IDs, or internal node boundaries; it must not restore an AST or
checker backdoor into code generation.

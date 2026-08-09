# 0001: Fully normalized HIR boundary

- Status: Accepted
- Date: 2026-07-31

## Context

At the time of adoption, successful Ice analysis produced a nominal
`CheckedDocument` that contained and dereferenced to the source `Document`.
The Rust code generator consumed that checked AST directly and imported checker
helpers for some semantic decisions. As the language grew, this made it easy to
repeat or diverge on the same decision in the checker and code generator.

The objective was to give every accepted Ice construct one checked, canonical
meaning before backend emission without adding a public language surface or
preparing speculative backends.

## Decision

Ice uses a complete, private, typed high-level intermediate representation named
`LoweredProgram`:

```text
Source AST
  -> CheckedDocument
  -> LoweredProgram
  -> Rust/Iced code generation
```

The source AST preserves how a program was written for parsing, formatting,
editor features, and diagnostics. `LoweredProgram` preserves what the checked
program means. It contains Ice semantics, runtime-required identities, and
resolved native-boundary names or paths, but no prebuilt Rust tokens or Iced
library types. Concrete Rust construction and Iced API calls remain backend
responsibilities.

The lowering boundary has these invariants:

1. Only lowering can construct a `LoweredProgram`.
2. Rust code generation accepts a `LoweredProgram`, never a source `Document`
   or `CheckedDocument`.
3. Code generation does not import checker helpers or repeat semantic
   validation.
4. HIR contracts that emit source markers or backend diagnostics carry an
   `OriginId`; a shared origin table stores root and imported locations.
5. Invalid or unresolved states are not representable in HIR. Lowering either
   produces a complete program or returns a source-mapped diagnostic.

Before emission, lowering resolves language sugar and all choices that would
otherwise require semantic interpretation in the backend. This includes typed
declaration identities, component contracts and calls, event routes, slots,
recipes and themes, matches, widget identity, asynchronous call sites,
lifetime behavior, application settings, and tests.

The HIR covers the whole accepted language. There is no mixed AST/HIR fallback
or compatibility shim.

## Rejected alternatives

### Keep generating from the checked AST

This leaves syntactic alternatives visible to the backend and permits semantic
decisions to spread between checking and emission.

### Add isolated, permanent mini-IRs

Feature-specific lowering can help migration, but permanent independent paths
would preserve an ambiguous compiler boundary.

### Put Rust or Iced constructs in HIR

Backend-shaped nodes would make semantic tooling depend on emission details.
Backend-specific planning stays private to code generation.

### Replace the pipeline in one large change

A big-bang conversion would have made source-map and generated-program
regressions difficult to isolate. The migration instead used verified vertical
slices while retaining one complete destination.

## Consequences

The compiler has another owned representation and an explicit lowering pass,
adding some build-time work and memory. In return, lowering is the single source
of semantic truth, code generation is a translation step, HIR fixtures can test
meaning without comparing large Rust strings, and other tooling can consume the
same canonical model.

## Revisit trigger

Revisit the representation when measurements show that HIR construction or
retention materially violates compiler performance budgets, or when an
accepted Ice semantic cannot be represented without backend knowledge. A
revisit may change storage, IDs, or internal node boundaries; it must not
restore an AST or checker backdoor into code generation.

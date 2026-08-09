# 0007: Ice covers the complete application-facing Iced surface

- Status: Accepted
- Date: 2026-08-09

## Context

Ice was described as a small Core language whose extended native surface was
not an API-parity roadmap. The implemented product no longer matched that
description. Its versioned coverage ledger proves every public,
application-facing row in the pinned iced baseline through direct declarative
syntax or a typed Rust boundary, with parser, checker, generated-code, and
runtime evidence.

Treating that breadth as incidental makes future iced upgrades ambiguous: a
new public capability can be recorded as missing without deciding whether the
language still claims to cover its backend. Conversely, mapping every Rust
method to another keyword would duplicate iced mechanically and weaken the
closed, statically checked language model.

## Decision

Ice is the statically checked declarative frontend for the complete public,
application-facing surface of its pinned iced baseline.

Completeness belongs to the combined authoring surface:

- common UI concepts use one canonical direct Ice form;
- higher-order and custom native behavior uses a canonical typed Rust
  boundary;
- arbitrary Rust expressions and a runtime interpreter remain outside Ice;
- domain validation, persistence, networking, security, and platform-specific
  policy remain Rust responsibilities.

`COVERAGE.md` is the versioned completeness contract. A pinned iced upgrade is
not complete while an application-facing row is partial or missing. A row is
native only after the evidence rule proves its syntax or typed interface,
invalid cases, lowering against the pinned release, and runtime behavior where
applicable.

Completeness does not require a dedicated language construct for every iced
type or method. Each gap receives a language-design decision choosing the
smallest checked representation that preserves native semantics. An existing
typed boundary may be the final representation rather than a temporary escape
hatch.

## Rejected alternatives

### Keep complete coverage as an implementation accident

This leaves backend upgrades without a definition of done and contradicts the
published 100% ledger.

### Add one syntax form for every iced method

This copies an imperative Rust API into the grammar, creates overlapping
spellings, and makes static semantics harder to understand.

### Admit arbitrary Rust expressions

This bypasses the checker, source-level diagnostics, schema, formatter, and
first-class test contract. Typed boundaries retain native power without losing
those guarantees.

## Consequences

Iced baseline upgrades must audit the complete public application-facing
surface and close every ledger gap before claiming completion. Language and
agent documentation must make the canonical representation discoverable so
authors do not need to memorize the full surface. Direct syntax remains
selective, but typed boundaries are first-class parts of the language contract.

This decision does not claim every state, theme, viewport, or value combination
has been tested. Those combinations remain application behavior; completeness
covers the public construction and interaction capabilities of the pinned
backend.

## Revisit trigger

Revisit the baseline definition if iced stops exposing a coherent public
application-facing surface or Ice adopts another production backend.

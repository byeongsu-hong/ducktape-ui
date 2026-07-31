# 0002: Core feature admission and syntax freeze

- Status: Accepted
- Date: 2026-08-01

## Context

Ice can express the repository's complete native reference applications, but
every new Core construct expands the parser, formatter, checker, HIR, code
generator, schema, LSP, source-map, test, documentation, and performance
surface. Product needs can usually be delivered through a native runtime
widget, a reusable `ducktape-ui` component, or an application-local typed Rust
boundary without adding syntax.

The fully normalized HIR migration is still in progress. Adding syntax during
that migration would multiply mixed AST/HIR paths and make the boundary harder
to finish.

## Decision

New Ice Core syntax is frozen until the HIR completion criteria in decision
0001 are met. Its `Accepted` status records the target architecture, not HIR
completion; decision 0001's completion criteria remain authoritative. Every
proposed feature must name exactly one primary semantic owner before
implementation:

| Owner | Admitted work |
| --- | --- |
| Ice Core | broadly repeated concepts whose static checking materially matters and which have one canonical syntax |
| `ui-lang-runtime` | native widget state, renderer integration, IME, platform events, or reusable reconciliation behavior |
| `ducktape-ui` | reusable screen composition, interaction patterns, and design-system contracts |
| application Rust boundary | domain logic or product/platform-specific lifecycle |

A vertical feature may require integration evidence in adjacent layers without
giving those layers duplicate semantic ownership. For example, a runtime-owned
native widget can also require a public `ducktape-ui` interface and an Ice extern
example; that does not make it a Core feature.

A feature becomes a Core candidate only after the same problem appears in at
least three independent applications or screens and implementing it as a
runtime widget, component, or typed Rust boundary would materially destroy its
meaning or static safety. Meeting that threshold starts a design review; it
does not authorize syntax by itself.

Every admitted Core proposal must provide one canonical form, explicit invalid
cases, formatter idempotence, checked semantics, normalized HIR, source-mapped
backend evidence, schema/LSP support, generated-program tests, documentation,
and a measured complexity budget. The change removes any superseded path; it
does not retain aliases or compatibility syntax.

## Rejected alternatives

### Expose Iced APIs one construct at a time

This optimizes for surface count rather than stable language semantics and
turns upstream API churn into language churn.

### Promote a useful runtime primitive immediately

Importance does not imply syntax. Virtualized data surfaces, editors, and
terminal widgets need native runtime behavior first; real component usage must
demonstrate that a Core form is necessary.

### Keep the admission rule informal

An informal rule cannot stop parallel feature work from creating inconsistent
precedents. The owning layer and required evidence are review inputs.

## Consequences

Product features continue through typed runtime and component boundaries while
Core remains stable. Some convenient sugar is deferred, but the implementation
surface, diagnostics, and canonical syntax stay reviewable. The HIR migration
can delete old backend paths instead of supporting newly introduced branches.

## Revisit trigger

Revisit the freeze only after decision 0001's HIR completion criteria are met.
Revisit an individual layer assignment when three independent uses provide
evidence that its current typed boundary loses essential meaning or safety.

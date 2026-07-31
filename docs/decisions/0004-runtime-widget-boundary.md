# 0004: Native data surfaces start at the runtime boundary

- Status: Accepted
- Date: 2026-08-01

## Context

Large lists, trees, grids, editors, terminals, and timelines need native widget
state, layout, event routing, focus, accessibility, and reconciliation. They
are important product capabilities, but that does not make them Ice syntax.
Premature syntax would freeze assumptions before real applications establish
the reusable contract.

## Decision

A reusable native data surface is implemented first in `ui-lang-runtime` with
a typed Rust API. `ducktape-ui` exposes the reusable component/interface and
design-system behavior. Ice applications consume that typed extern component;
Core does not gain a special loop, selector, or widget syntax.

The runtime API must define:

- typed inputs and messages without unchecked dynamic payloads;
- state ownership, stable identity, reconciliation, and removal behavior;
- mouse, keyboard, focus, and scrolling interaction;
- Unicode and IME behavior for text surfaces;
- AccessKit semantics, or an explicit documented limitation;
- headless geometry and semantic inspection;
- a native WGPU first-draw or renderer-specific smoke; and
- a realistic large-data performance contract.

The component layer separately owns props, events, slots, visual interaction
states, semantic theme/font inheritance, responsive examples, and accessibility
names and keyboard behavior. The showcase must consume the same public
interface as downstream applications.

Virtualization begins with fixed row height, stable keys, overscan, selection,
keyboard navigation, scroll-to-item, visible-range inspection, item count/index
semantics, and a 100,000-item budget. Variable-height measurement is a later
runtime capability, not part of the first contract.

## Rejected alternatives

### Add `virtual-for` to Core first

This commits the language to a reconciliation and measurement model before the
runtime behavior is proven.

### Implement only a `ducktape-ui` composition

Ordinary composition cannot provide bounded layout and event work for very
large collections without a native stateful widget.

### Expose an untyped escape hatch

Dynamic payloads hide capability and lifecycle errors that the Rust/Ice
boundary is intended to catch.

## Consequences

Runtime and component APIs may evolve together without expanding Core syntax.
The first version intentionally omits variable-height rows and domain-specific
tree/grid policy. If repeated use later satisfies decision 0002, a Core
proposal can be evaluated against working semantics rather than speculation.

## Revisit trigger

Revisit the boundary when three independent applications demonstrate that the
typed extern component cannot preserve essential static semantics or
composition. Performance alone is not a Core admission reason.

# Multi-child slots for component-owned layout

Status: implementation design; the syntax below is not supported yet.
Owner: UI quality worklist L04b. This does not close L04 or any other audit row.

## Problem and evidence

The card example in `examples/showcase/tests/cases/ui/action_layout.ice`
needs a caller-owned `row wrap` to keep long action labels at useful widths.
Without it, the 280px card keeps its labels inside their buttons but squeezes
the second label into three lines. The wrapping pattern is verified at 280px
and 640px, but every author must choose that structure again.

This is a composition boundary, not a missing renderer layout algorithm.
`ComponentSlot.content` is a boxed single view, the parser rejects sibling
roots, and slot codegen renders that view into one `Element`. An outer wrapping
row consequently sees the caller's whole row as one child. Merely changing
Card.Footer's outer row cannot arrange the buttons inside it.

## Decision

Add an explicit multi-child slot cardinality, written `slot children*` (or a
named equivalent such as `slot actions*`). A component layout consumes the
supplied children individually, without an implicit row, column, widget,
accessibility node, or reconciliation boundary.

```ice
component Actions(gap:f64=8.0)
  row #root wrap w=fill gap=gap wrap-gap=gap
    slot children*

Actions
  button "Discard all changes" @secondary_action -> cancel
  button "Save workspace settings" @primary_action -> save
```

The example illustrates the contract; it does not require a new public Actions
component. First apply the contract to existing Card.Footer and Dialog.Actions,
which already own action spacing. Inspect ButtonGroup's connected appearance
before deciding its reflow policy: a connected group need not use card spacing.

Do not implement an action-label model, a fixed primary/secondary pair, or an
opaque native action-bar adapter. Callers retain normal buttons, custom content,
conditions, loops, event routes and local component state.

## Cardinality and placement

- `slot` / `slot name` keeps its current required single-root contract.
- `slot name?` keeps its current optional single-root contract.
- `slot name*` accepts zero or more roots and may be omitted. There is no `?*`
  combination and no additional shorthand spelling in this change.
- Multiple direct component children supply the default `children` slot. Named
  `name:` blocks may contain multiple roots when the declared slot is multi-child.
  Existing qualified compound children keep their named-slot mapping.
- A multi-child slot must be consumed where sibling children are legal: a
  row, column, grid, flex or stack child list, including transparent conditional
  and iteration branches in that list. Forwarding to another multi-child slot
  is valid and retains cardinality.
- A box, button, scroll viewport or component root cannot consume a multi-child
  slot as one element. Diagnose that misuse at the slot placement, with a hint
  to supply an explicit layout. Do not silently insert a column or choose the
  first child. Forwarding many children into a single-root slot is also invalid.
- A single-root slot still rejects multiple syntactic roots. Its existing root
  control-flow behavior is unchanged. Multi-child slots expand nested `if`,
  `for` and `match` branches using the enclosing layout's existing rules.
- An omitted or empty multi-child slot contributes no child and therefore no
  spacing. Spacing belongs to the consuming layout and counts rendered children.

A caller can deliberately pass one nested layout to a multi-child slot. That
layout remains one child; the compiler never peels arbitrary rows or columns
apart. This preserves deliberate custom grouping and its styling.

## Compiler representation and identity

Parse caller slot contents as an ordered list with source spans. The checker
uses the component declaration's cardinality to validate that list and its
placement. Carry the cardinality and ordered resolved child IDs through HIR;
codegen must not infer them from names or source strings.

When emitting layout children, resolve the slot's captured caller environment
and expand the list into the same child collection as direct layout content.
Reuse existing condition/iteration expansion. Preserve the current split:
expressions and handlers bind in the caller; rendered identity is scoped at the
slot's placement in the receiving component. Forwarding must preserve that split
through each component. Explicit keys continue to own state during reorder.

The scalar rendering path accepts only single-root cardinalities. Invalid many
placement must fail in checking, before codegen needs to choose a fallback.
Native and existing Tree codegen must agree on child structure; this is a shared
language change, not a new host or platform feature.

Formatter output preserves caller siblings and named blocks. Schema, LSP,
component API extraction and API diff must expose the cardinality. Extend the
existing slot API's `required` information with whether it accepts multiple roots;
update exact baselines and their diff tests together. Do not retain stale
single-root-only diagnostics or documentation for multi-child slots.

## Acceptance evidence

1. Parser/formatter/checker fixtures cover direct and named sibling lists,
   zero/one/many children, optional single slots, invalid unary placement,
   invalid forwarding, and unambiguous compound-slot names.
2. A native authored test places real caller buttons directly in Card.Footer:
   at 280px they occupy separate lines with natural labels; at 640px they share
   a row. Both remain contained. Tab order and click/Enter routes still work.
3. Conditional empty content leaves no spacer. Adding/removing content updates
   the consuming layout. A keyed list preserves each child's local state across
   reorder; route payloads still identify the intended child.
4. A forwarding component preserves binding, keys and rendered target scopes.
   A custom nested row stays grouped rather than being flattened accidentally.
5. The existing Tree compilation path produces the same sibling structure and
   route identity, using its existing test harness. No host changes are assumed.
6. Mutating sibling expansion to a single column must fail the real layout
   assertions. Swapping a route or losing a key must fail the corresponding
   interaction/state assertion. Parse/setup errors are not UI Red evidence.
7. Update library callers, compiling examples, README, SPEC, COVERAGE, skills,
   schema/API baselines and worklist evidence in the implementation change.
   Inspect narrow/wide captures and review the complete diff before delivery.

Do not mark L04 Done from the new syntax alone. Card.Footer, Dialog.Actions and
ButtonGroup still need their own measured layout and customization contracts.

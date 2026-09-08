# Multi-child slots implementation plan

Design: [component-owned layout](../specs/2026-09-08-multi-child-slots-design.md).
Status: pending implementation. Keep the implementation in one focused review
scope; a partial parser-only merge must not advertise supported syntax.

1. Add cardinality and ordered caller content to AST/HIR. Update parser,
   formatter and checker together; use focused fixtures to establish the new
   contract and reject many-slot placement in scalar content positions.
2. Update all traversal/analysis consumers, slot forwarding, rendered-target
   discovery, schema/LSP and API extraction/diff. Exhaustive matches and focused
   tests should identify omissions; do not add compatibility branches.
3. Emit multi-child slots through existing layout-child generation with the
   captured caller environment and receiving scope. Keep scalar rendering
   restricted to single-root slots. Verify native and shared Tree structures.
4. Move Card.Footer and Dialog.Actions to own their wrapping action children;
   update every affected library/example caller. Audit ButtonGroup's connected
   presentation before choosing its policy. Leave intentional nested groups
   intact.
5. Run native layout/interaction tests across narrow/wide, conditional empty,
   keyed reorder, forwarding and custom grouping. Prove the specified behavior
   mutations fail, restore, rerun, and inspect captures. Reuse the existing
   action-layout evidence rather than retaining a second obsolete recipe.
6. Update public docs, skills, exact API baselines and the UI worklist. Run
   affected Core and showcase tests, Rust/Ice formatting, appropriate workspace
   checks and independent review. Commit/push/open PR. With high confidence,
   use normal merge permissions without waiting for optional CI per the user.

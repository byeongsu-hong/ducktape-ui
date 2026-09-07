# Phase 4a: layered module layouts

Continue the authorized Ice prerequisites without changing Ducktape. The audited
chat rows use `stack` and native `hover` toolbars; pages and menus also require
modal `overlay`. Implement these related layering constructs together, then
batch the checks and actual wasm interaction evidence.

Reuse the existing native behavior. Normal `stack` uses `zstack` union sizing;
`under=` uses the native Iced stack's base/under-layer semantics. Preserve length,
clip and surface styling. Hover has a base and reveal child, optional tint/radius
and guest-owned `open`; pointer presence is decided by the native host without a
guest state round trip. Overlay must block base focus and input while open,
swallow panel presses, dismiss only through the declared backdrop route, and
release the layer on close. Unsupported float callbacks remain explicit E190.

Wire records carry copied values and children with the existing depth/node/text
budgets. Route-free event swallowing needs a host-local ignored output, not a
fabricated guest handler index. Update wire patch/sanitize traversals and the
runtime's retained input/picture traversal together. Responsive structural
conditions must splice into stacks consistently with rows/columns/grids.

- [x] Advance this task branch from origin/main after responsive #956 lands.
- [x] Add wire fields, Tree emitters and native host rendering as one code batch.
- [x] Build a representative actual wasm layered-layout fixture: union size,
      top-layer click precedence, hover reveal/held-open behavior, modal backdrop
      dismissal, base focus/input blocking and hidden layer lifetime.
- [x] Demonstrate meaningful Red/Green assertions, then run root/app-store checks,
      native tests, real bundles, Clippy and formatting as a batch.
- [ ] Update support claims and the module phase ledger; independent review,
      focused PR, all CI green, merge and worktree cleanup.

Flex child sizing/wrapping and keyed/lazy identity/virtualization remain subsequent
phase-4a scopes; do not claim this makes an entire Ducktape screen portable yet.

Validation and review:
- Random hostile frames exposed missing Props child-list handling for
  Hover/Overlay; bounded child vectors now survive all six model checks.
- The actual wasm backdrop-dismissal assertion exposed GuestView dropping native
  overlays. Forwarding them and routing their outputs fixes modal interactions.
- Independent review exposed lost inferred Fill on stacks. Dimensions are now
  applied only when explicit; the real wasm test checks union/base/Fill sizes.
- The bundled host test checks top-layer clicks, native hover pixels without guest
  ticks, held-open reveal, modal focus/input blocking, panel/backdrop routing and
  surface release on close. Deliberately removing inferred Fill and the focus
  barrier each fails its intended assertion; restoration passes.
- Local checks passed: workspace check with tests; core 987, guest 13, wire 45,
  hostile frames 6, runtime view-tree tests 15 and ordinary host tests 21.
  Affected-crate and host Clippy pass with warnings denied. The actual modal
  screenshot is recorded in the README. Independent review has no open findings.
- Advanced to resources/component main 1015d279, preserving both fixtures and
  evidence. The union-size case uses an Ice component to exercise their combined
  generated code through actual wasm.

PR checks, merge and worktree cleanup remain delivery steps.

# UI quality worklist

## Objective

A normal Ice screen should look coherent from its default components. An author
should be able to change a specific visual decision without rebuilding input,
focus, accessibility or layout behavior. Agents should select established screen
structures rather than invent a design system for every task.

This is the full initial worklist for that objective, not a claim that the listed
features are absent. It covers UI authoring, default composition and the native
rendered result. It does not expand into an app platform, visual studio, renderer
replacement or unrelated language features. New syntax requires evidence that
existing components, recipes and typed boundaries cannot express the contract.

Inventory inspected against `origin/main` at `480a5290` (2026-09-08). The form
slice was delivered in [PR #1015](https://github.com/byeongsu-hong/ducktape-ui/pull/1015).
The existing [63-component parity ledger](../crates/ui-lang-components/docs/parity.md)
is evidence of supplied behavior, not evidence that all default compositions
and customizations below have been verified.

## States and delivery rules

- **Audit**: relevant implementation exists or is identifiable; the acceptance
  scenario has not yet been verified. This is not a missing-feature finding.
- **Gap**: a specific limitation has been observed in the cited source or run.
- **PR**: a focused implementation and local evidence exist; delivery gates remain.
- **Done**: the acceptance scenario is supported by inspected evidence, with any
  necessary PR merged. Existing behavior may satisfy a row without new code.

Every row is handled in order: inspect the complete production path, render the
smallest representative example, record the actual failure if any, choose its
owning layer, implement a focused fix, prove assertion-level Red/Green, inspect
captures, review and deliver. Do not implement the entire table speculatively.
If a row contains independent defects, list child tasks before changing them.
Keep one review scope per worktree/PR. Update this file with evidence and the PR.

## Layout foundations

| ID | Priority/state | Inspect first | Acceptance scenario |
| --- | --- | --- | --- |
| L01 | P0 · Audit | [view layout rules](../skills/design-ice-ui/references/views-and-style.md), [Item/InputGroup](../crates/ui-lang-components/src/ice/components.ice) | A label + flexible input + trailing action share a narrow parent without overlap; the action stays usable, fill/shrink ownership is explicit, and custom padding remains inside the parent. |
| L02 | P0 · Done | [PR #1020](https://github.com/byeongsu-hong/ducktape-ui/pull/1020): optional descriptions, conditional rows and bounded wrapping | An omitted/empty subtitle allocates no blank row; a long title/subtitle wraps inside its container without overlapping the next element. Local: 6 header tests and 324 existing showcase tests pass; empty-row and wrapping assertion Reds recorded. Semantic heading roles remain tracked under A02; merged as b709ea51. |
| L03 | P0 · Audit | [scroll guidance](../skills/design-ice-ui/references/design-workflow.md), [MessageScroller contract](../crates/ui-lang-components/docs/parity.md) | A short window can reach the last action; headers/actions that should stay visible do so; nested scroll ownership and reading-position preservation are explicit. C01 covers only a single form scroller. |
| L04 | P0 · Gap | [Card.Footer/Dialog.Actions/ButtonGroup](../crates/ui-lang-components/src/ice/components.ice) | Long action labels fit or wrap into an intentional layout at narrow width; button labels remain unclipped and focus order follows the visual order. |
| L05 | P1 · Audit | [responsive design guidance](../skills/design-ice-ui/references/design-workflow.md), [showcase root](../examples/showcase/src/ui/app.ice) | A sidebar/detail screen has a declared compact presentation; resizing preserves selection and editing state and does not silently discard essential controls. |
| L06 | P1 · Audit | [grid/flex surface](../skills/design-ice-ui/references/extended-surface.md), [catalog](../examples/showcase/src/ui/components/catalog.ice) | Repeated cards use a consistent minimum useful width and aspect ratio; odd item counts and narrow windows leave no clipped or zero-width cells. |

## Design defaults and customization

| ID | Priority/state | Inspect first | Acceptance scenario |
| --- | --- | --- | --- |
| D01 | P0 · Audit | [Ice recipes](../crates/ui-lang-components/src/ice/recipes.ice), [Rust theme](../crates/ui-lang-components/src/ui/theme.rs) | A screen has a coherent spacing/control-size hierarchy; a compact variant changes the intended metrics without losing minimum hit areas or text alignment. Establish whether a shared mechanism is needed from real repeated overrides. |
| D02 | P0 · Audit | [Typography components](../crates/ui-lang-components/src/ice/components.ice), [font ownership](../crates/ui-lang-components/README.md) | Heading, body, caption and monospace roles have consistent baselines and line heights; longer and non-Latin text stays readable with the documented font loading path. |
| D03 | P0 · Audit | [default palette](../crates/ui-lang-components/src/ice/default.ice), [theme contract](../SPEC.md) | Changing an application's semantic palette keeps fields, surfaces and control states coherent; light/dark and custom accent examples do not require copying component source. Distinguish existing complete palettes from proposed partial overrides. |
| D04 | P0 · Audit | [control/action recipes](../crates/ui-lang-components/src/ice/recipes.ice), [button-status tests](../examples/showcase/tests/cases/ui/button_status_children.ice) | Default, hover, focus, pressed, disabled and invalid states remain legible; overriding one geometric or color property does not reset unrelated states. |
| D05 | P1 · Audit | [component slots and custom content](../crates/ui-lang-components/README.md) | Replace a row/trigger/body visual through the public interface while retaining its selection, dismissal, keyboard and accessibility contract. Identify actual source-copy requirements before adding new extension points. |

## Common compositions

| ID | Priority/state | Inspect first | Acceptance scenario |
| --- | --- | --- | --- |
| C01 | P0 · Done | [PR #1015](https://github.com/byeongsu-hong/ducktape-ui/pull/1015) | Default and customized fields at 360/960px; wrapping errors; no empty-help gap; short-window scroll; binding, Tab/Space/Enter and focus preserved. Local: 8 settings tests, 330 showcase tests pass; five mutation Reds; merged as 607f7ce3. |
| C02 | P1 · Audit | [Item/Attachment/Breadcrumb](../crates/ui-lang-components/src/ice/components.ice), [VirtualList](../crates/ui-lang-components/docs/virtual-list.md) | Leading visual, long content and trailing metadata/actions remain aligned; stable-key selection survives reorder/filter; optional content does not leave blank columns. |
| C03 | P1 · Audit | [navigation examples](../examples/showcase/src/ui/components/navigation.ice), [parity navigation contracts](../crates/ui-lang-components/docs/parity.md) | A list/detail or settings screen has one obvious selected location, coherent title/actions and predictable back behavior, with a reusable default composition. |
| C04 | P1 · Audit | [Dialog composition](../crates/ui-lang-components/src/ice/components.ice), [modal runtime contract](../crates/ui-lang-components/docs/parity.md) | A long dialog fits a small window, body scrolls as needed, actions stay reachable, focus is contained/restored and custom content preserves dismissal policy. |
| C05 | P1 · Audit | [Select/Combobox/menus/popovers](../crates/ui-lang-components/docs/parity.md) | A searchable anchored selector near a window edge keeps the active result visible, handles empty results, and restores focus after selection/Escape, including a custom trigger. |
| C06 | P2 · Audit | [Markdown/editor examples](../examples/markdown-editor), [AI chat](../examples/ai-chat) | Rich text, images and editor content respect readable width; long links/code and streaming additions have an explicit overflow policy and preserve reading position. |
| C07 | P2 · Audit | [DataGrid](../crates/ui-lang-components/docs/data-grid.md), [TreeView](../crates/ui-lang-components/docs/tree-view.md), [LogTimeline](../crates/ui-lang-components/docs/log-timeline.md) | Default table/tree/log compositions handle empty, selected and editing states; documented virtualization limits stay explicit; appending data preserves active work and fits frame budgets. |

## UI state and feedback

| ID | Priority/state | Inspect first | Acceptance scenario |
| --- | --- | --- | --- |
| S01 | P1 · Audit | [EmptyState/Alert](../crates/ui-lang-components/src/ice/components.ice), [catalog](../examples/showcase/src/ui/components/catalog.ice) | Empty, loading, error and success presentations use a consistent hierarchy and useful action; changing state does not introduce accidental blank space or obscure the main task. |
| S02 | P1 · Audit | [music handlers](../examples/apple-music/src/ui/handlers/app.ice), [chat handlers](../examples/ai-chat/src/ui/handlers.ice) | Repeated submit, stale completion and failed save have clear feedback; unrelated edits remain intact; no simultaneous contradictory success/error state. Fix ownership/policy before inventing a new async DSL. |

## Interaction and accessibility

| ID | Priority/state | Inspect first | Acceptance scenario |
| --- | --- | --- | --- |
| A01 | P0 · Audit | [focus tests](../examples/showcase/tests/cases/ui/focus_visible.ice), [modal/selection contracts](../crates/ui-lang-components/docs/parity.md) | Keyboard-only completion works through default and custom content; focus stays visible, moves predictably after removal and returns from overlays. C01 is evidence for its form only. |
| A02 | P1 · Audit | [accessibility contract](../SPEC.md), [test driver](testing.md) | Visible names, descriptions, heading levels, errors and disabled states produce matching semantic output; interactive content is not a decorative text substitute. |
| A03 | P1 · Audit | [direction/localization support](../crates/ui-lang-components/docs/parity.md), [environment tests](testing.md) | Translated long labels, larger text and RTL reading order have explicit tested layouts; scale/locale metadata alone is not proof of translated content or larger text. |
| A04 | P2 · Audit | [drawer/carousel behavior](../crates/ui-lang-components/docs/parity.md), [motion guidance](../skills/design-ice-ui/references/design-workflow.md) | Pointer/touch cancellation and interrupted motion preserve state; reduced motion remains usable; customization does not remove the alternative keyboard action. |

## Agent authoring and evidence

| ID | Priority/state | Inspect first | Acceptance scenario |
| --- | --- | --- | --- |
| G01 | P0 · Audit | [default import instructions](../crates/ui-lang-components/README.md), [starter](../examples/starter/README.md) | An agent can discover/import the default library and run one representative screen without copying catalog-only adapters or inventing an import mechanism. |
| G02 | P1 · Audit | [design workflow](../skills/design-ice-ui/references/design-workflow.md), [showcase](../examples/showcase/README.md) | Forms, list/detail, dialog and collection screens each have a small canonical production example with clear owners and supported customization. Guidance points to compiling examples, not hypothetical syntax. |
| G03 | P0 · Audit | [first-class tests and inspection](testing.md), [coverage ledger](../COVERAGE.md) | The changed component is checked across relevant width/content/state variants with geometry and interaction assertions plus inspected images; each regression oracle rejects a real behavior mutation. Reuse existing tooling. |
| G04 | P1 · Audit | Existing task histories when available; otherwise a recorded prospective task | Give an agent the same bounded UI request before/after an improvement; record structural mistakes, manual interventions and time to a verified usable result. Compile success and fewer lines alone do not close this row. |

## Processing order

1. Deliver C01 and resolve any CI/review finding.
2. L02: verify and fix empty/long header descriptions in the existing shared components.
3. L01 and L04: narrow rows and action groups; separate defects if they have different owners.
4. L03: scroll ownership for longer compositions, building on the form evidence.
5. D01–D04: spacing, typography, theme and control-state consistency.
6. G01/G03 alongside those deliveries, then measure G04 on the established defaults.
7. L05/L06 and C02–C07, with D05, S01/S02 and A01–A04 checked at each affected boundary.

This order is based on source inspection, not a measured defect ranking. A
verified regression that blocks an earlier item takes priority. No row is Done
merely because its component exists or its code compiles.

## Evidence log

- 2026-09-08: initial inventory recorded; inspected shared Ice components,
  recipes/default palette, Rust theme roles, design workflow and parity ledger.
- 2026-09-08: C01 initial CI results: Rust/Ice, Windows workspace, native WGPU,
  accessibility and MSRV checks pass. App-store clipboard fixture loading fails
  with `tick exceeded 100 ms`; performance checks remain pending. The fixture does not import the changed default components. The user
  subsequently authorized high-confidence delivery without waiting for CI;
  PR #1015 merged as 607f7ce3 using normal merge permissions. The timeout was
  not fixed or claimed green. Local evidence is linked in PR #1015.
- 2026-09-08: L02 PR #1020 merged as b709ea51. Empty descriptions previously added
  26.75 px in both headers; assertion-level Red/Green confirms removal.
  A no-wrap mutation fails the long-title assertion. All three rendered
  captures were inspected; 6 header and 324 existing showcase tests pass.
  API diff reports two additive optional arguments and no breaking changes.

- 2026-09-08: after rebasing L02 onto the delivered form slice, all 6 header
  and 8 settings tests pass together. Merge conflict resolutions retain both
  documentation sections and regenerate the combined component API baseline.
  User-authorized high-confidence merges do not wait for CI; required repository
  protections remain in force.

### L04 follow-up: action layout ownership

The first 280px card audit passed text containment and keyboard assertions, yet
its first button consumed most of the row and the second label wrapped into
three lines. Containment alone is insufficient evidence of a useful action
layout. The explicit `row wrap` pattern in
[`action_layout.ice`](../examples/showcase/tests/cases/ui/action_layout.ice)
preserves natural button widths at 280px and keeps the actions together at
640px. Its tests assert line placement, text bounds and keyboard/pointer routes.

This supplies L04a (the explicit pattern and assertion-level evidence), but
does **not** close L04. Follow-up work remains:

- L04b: resolve default layout ownership. A slot accepts one root, so callers
  currently construct the inner row. Making Card.Footer's outer row wrap cannot
  reflow the buttons inside that caller-owned row. Determine the smallest
  shared composition or slot contract that can own sibling arrangement before
  adding an automatic action-layout API.
- L04c: verify Dialog.Actions and ButtonGroup, including long labels and focus
  order; the card test alone does not establish their behavior.

- L04b implementation design: [multi-child slots](superpowers/specs/2026-09-08-multi-child-slots-design.md)
  and [implementation plan](superpowers/plans/2026-09-08-multi-child-slots.md).
  This is a planned language contract, not supported syntax or completed layout
  behavior. Existing source inspection confirms that scalar slot codegen is the
  boundary preventing the parent from arranging caller siblings individually.

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
| L01 | P0 · Done | [view layout rules](../skills/design-ice-ui/references/views-and-style.md), [Item/InputGroup](../crates/ui-lang-components/src/ice/components.ice) | A label + flexible input + trailing action share a narrow parent without overlap; the action stays usable, fill/shrink ownership is explicit, and custom padding remains inside the parent. |
| L02 | P0 · Done | [PR #1020](https://github.com/byeongsu-hong/ducktape-ui/pull/1020): optional descriptions, conditional rows and bounded wrapping | An omitted/empty subtitle allocates no blank row; a long title/subtitle wraps inside its container without overlapping the next element. Local: 6 header tests and 324 existing showcase tests pass; empty-row and wrapping assertion Reds recorded. Semantic heading roles remain tracked under A02; merged as b709ea51. |
| L03 | P0 · Audit | [scroll guidance](../skills/design-ice-ui/references/design-workflow.md), [MessageScroller contract](../crates/ui-lang-components/docs/parity.md) | A short window can reach the last action; headers/actions that should stay visible do so; nested scroll ownership and reading-position preservation are explicit. C01 covers only a single form scroller. |
| L04 | P0 · Done | [Card.Footer/Dialog.Actions/ButtonGroup](../crates/ui-lang-components/src/ice/components.ice) | Long action labels fit or wrap into an intentional layout at narrow width; button labels remain unclipped and focus order follows the visual order. |
| L05 | P1 · Audit | [responsive design guidance](../skills/design-ice-ui/references/design-workflow.md), [showcase root](../examples/showcase/src/ui/app.ice) | A sidebar/detail screen has a declared compact presentation; resizing preserves selection and editing state and does not silently discard essential controls. |
| L06 | P1 · Audit | [grid/flex surface](../skills/design-ice-ui/references/extended-surface.md), [catalog](../examples/showcase/src/ui/components/catalog.ice) | Repeated cards use a consistent minimum useful width and aspect ratio; odd item counts and narrow windows leave no clipped or zero-width cells. |
| L07 | P0 · Audit | [text/layout emission](../crates/ui-lang-core/src/codegen/view), [text runtime](../crates/ui-lang-runtime/src) | Horizontal and vertical text alignment inside assigned bounds work independently of parent placement, including fixed/fill/shrink sizes, multiline wrapping and padding. Verify painted text geometry, not only the enclosing widget's rectangle. Added from the user's explicit two-axis text-alignment request. |

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

L04b supplies the shared [multi-child slot contract](superpowers/specs/2026-09-08-multi-child-slots-design.md)
and [implementation](superpowers/plans/2026-09-08-multi-child-slots.md):
`slot children*` lets the receiving component arrange ordinary caller siblings.
Card.Footer now owns wrapping and spacing; the same narrow/wide tests use direct
buttons. Empty content adds no placeholder or gap, explicit caller layouts stay
grouped, and forwarded keyed state survives reordering. Both native and freshly
bundled Tree interaction evidence exercise the composition boundary.

L04c additionally verifies ButtonGroup's narrow wrapping, zero-gap wide row,
single-line labels and keyboard/pointer routes. Dialog.Actions has the same
direct-child interface, but its absolute right-edge assertions exposed an Iced
wrapping-alignment defect. Keep L04 open until the separate owning-layer fix and
the narrow/wide Dialog assertions are verified together.

### L01 follow-up: viewport and surface edge spacing

A user-reported recurring failure is content touching the viewport or its parent
surface. The action card reproduced the viewport case: text containment passed
while its left/top edges were at zero. `Page(padding=24.0)` supplies an explicit
ordinary-screen boundary; the card example uses it and asserts the outer inset.
A 12px customization test separately checks Panel's retained 20px inner padding;
an explicit zero-inset test preserves intentional full-bleed composition.

This edge-spacing slice does not close L01's flexible label/input/action-row
audit. Form already owns outer padding and is not wrapped in Page. Additional
surface-specific edge defects require their own reproductions.

- 2026-09-08: L04 integration now passes after the owning Iced wrapping fix
  [PR #1027](https://github.com/byeongsu-hong/ducktape-ui/pull/1027). All 10
  native multi-slot tests and 5 action-layout tests pass on that merged base.
  Dialog.Actions previously stopped at x=225.25 instead of 236 at narrow width
  and 385.25 instead of 422 at wide width; the unchanged right-edge assertions
  now pass. Narrow/wide captures show complete labels, default reflow and
  right alignment inside the dialog padding. Keyboard/pointer routes and
  explicit custom grouping pass. L04 remains PR until this component/language
  change is delivered; text alignment L07 is a separate ongoing slice.

- 2026-09-08: L04 final integration passes all 365 showcase tests and both
  freshly bundled Tree sibling/state tests. Independent final review found no
  actionable findings. The language/component PR remains the delivery step.
- 2026-09-08: L07 selection slice delivered in
  [PR #1028](https://github.com/byeongsu-hong/ducktape-ui/pull/1028). Existing
  horizontal/vertical text placement passes four native glyph-geometry tests;
  selection now uses that same paragraph anchor. Three runtime drag/copy/
  highlight tests pass, with independent copy and highlight assertion Reds.
  Fixed/fill and explicit multiline shrink cases are covered. Compact button
  defaults and rich-span decoration/link coordinates are separate active fixes;
  soft-wrapped and justified line geometry are not newly claimed by this slice.

- 2026-09-08: L04 delivered by
  [PR #1029](https://github.com/byeongsu-hong/ducktape-ui/pull/1029), merged as
  e023763e. The final source/evidence review had no actionable findings; the
  schema change used the existing head-specific breaking-review label.
- 2026-09-08: L07 rich-span slice delivered by
  [PR #1030](https://github.com/byeongsu-hong/ducktape-ui/pull/1030), merged as
  533e5cb6. Five runtime tests verify horizontal center/right and vertical
  center/bottom decoration placement and painted-link versus empty-corner
  clicks, with a top-left control. Four intended click and underline Reds
  become Green. Compact button fill-label alignment remains under verification.

### L07: compact button labels on both axes

[PR #1031](https://github.com/byeongsu-hong/ducktape-ui/pull/1031), merged as
`f00afe5b`, centers compact labels inside the padded content box for fixed,
fill and fill-portion dimensions in native and Tree rendering. Written-out
child content retains its own layout under fill dimensions; shrink sizing
still hugs content. Six pre-fix assertion failures covered both axes, portions
and unequal padding. After integration, all six native tests and the focused
Tree geometry test passed; the earlier isolated Core/runtime/showcase run
passed 1,843 tests. This joins PR #1028 selection and PR #1030 rich text
alignment evidence; it does not add soft-wrap or justified-text geometry proof.

### L01: Item text shares constrained width

The default Item now uses intrinsic flex bases for primary text and metadata
and prevents compression of leading content. The 280px case no longer gives
a 56.86px title word only 9.94px; the 640px case keeps metadata on one line.
Native tests also preserve short metadata, avatar dimensions and a custom
leading button's actual click route inside custom page insets. Three temporary
behavior mutations produced assertion-level failures and restoration passed
all seven tests. Delivery is recorded in the pull request carrying this change.

The remaining caller-authored input/action composition is covered below.
Native layout fill portions were corrected in [PR #1033](https://github.com/byeongsu-hong/ducktape-ui/pull/1033).

### L01: canonical narrow input/action composition

[The compiling example](../examples/showcase/tests/cases/ui/compact_input.ice)
uses Field for the label and InputGroup for a flexible input plus Apply button.
At 280px, the whole edited value is visible; at 640px, custom Page insets remain
inside the parent. Both tests click the actual button and observe its state
transition. An extra inline label makes the visible value shrink from 44.80px
to 16.42px and fails the paint-width assertion; restoration passes five tests.
The same application component is used in the example view and test mounts.

Together with Item allocation (PR #1032), page insets (PR #1024) and native
layout portions (PR #1033), this closes L01's native acceptance scenario. It
does not promise an arbitrary number of fixed siblings will fit every width;
Tree surfaces and identified-control portions remain separate follow-up work.

### L03: fixed actions and ordinary update preservation

[The bounded Form example](../examples/showcase/tests/cases/ui/scroll_ownership.ice)
keeps heading and Save as siblings of the scroll body. At 320×300, wheel input
reveals the last control and its actual click works; heading and Save keep
their insets. Saving after scrolling 100px preserves that offset. Fixed body
height and an explicit reset each produce the intended assertion-level Red;
restoration passes five native tests. The component guide includes this pattern.

The nested scroll case is covered below. This Form evidence covers ordinary
state updates, not reading-position preservation when content is inserted or
removed.

### L03: nested preview wheel ownership

[The nested preview example](../examples/showcase/tests/cases/ui/nested_scroll.ice)
places a 120px preview inside a bounded document scroller. Native wheel input
moves only the preview, including continued input after its end; its final
control is visible and clickable. Moving the pointer out of the window and
back clears the sequence, allowing edge input to move the document. Tests assert
both offsets so neither double movement nor trapped fresh input can pass.

Reading-position preservation under inserted/removed content is covered below.
These native pointer tests do not cover touch or elapsed-time transaction expiry.

### L03: live transcript updates exposed a state race

The showcase MessageScroller adapter returned deferred state snapshots. One
wheel emits viewport and intent events before those snapshots return; the
intent snapshot could erase the new viewport. Updating the transcript then
restored a stale reading position. The adapter now assigns state synchronously
and routes only follow-up events back through the current state.

A native driver runs [the Ice transcript](../examples/showcase/tests/cases/ui/scroll_reading_anchor.ice)
through the same production adapter. It checks a surviving row's full visible
height and screen coordinate after capped prepend/tail removal and head
removal/tail append. The previous deferred-state path hides the reading row;
the corrected path retains it. L03 remains open for deleting the currently
anchoring row. Similar deferred-state adapters belong to the S02 state audit;
this delivery changes only MessageScroller.

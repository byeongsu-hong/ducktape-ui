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
| L03 | P0 · Done | [scroll guidance](../skills/design-ice-ui/references/design-workflow.md), [MessageScroller contract](../crates/ui-lang-components/docs/parity.md) | A short window can reach the last action; headers/actions that should stay visible do so; nested scroll ownership and reading-position preservation are explicit. C01 covers only a single form scroller. |
| L04 | P0 · Done | [Card.Footer/Dialog.Actions/ButtonGroup](../crates/ui-lang-components/src/ice/components.ice) | Long action labels fit or wrap into an intentional layout at narrow width; button labels remain unclipped and focus order follows the visual order. |
| L05 | P1 · Done | [PR #1043](https://github.com/byeongsu-hong/ducktape-ui/pull/1043), [responsive workspace guide](../crates/ui-lang-components/docs/responsive-workspace.md), [native Ice contract](../examples/showcase/tests/cases/ui/responsive_workspace.ice) | Wide/compact navigation preserves project selection, independent drafts, native focus, selection and caret across 960→360→960 resizing. Exact/custom content breakpoints, readable-width action alignment and reachable Save at 320×240 are verified; 4 authored tests plus 3 generated harness checks pass. |
| L06 | P1 · Done | [PR #1045](https://github.com/byeongsu-hong/ducktape-ui/pull/1045), [native collection contract](../examples/showcase/tests/cases/ui/grid_collection.ice), [rendered evidence](../examples/showcase/screenshots/grid-collection/README.md) | Native minimum-cell grids retain equal columns across odd rows and clamp below the minimum without clipped or zero-width cells. Eight scenarios verify aspect-ratio composition, exact/fractional reflow, custom spacing/insets, natural rows, empty/single updates and real card clicks. Tree equal-column parity remains outside this native acceptance. |
| L07 | P0 · Done | [text/layout emission](../crates/ui-lang-core/src/codegen/view), [text runtime](../crates/ui-lang-runtime/src) | Horizontal and vertical text alignment inside assigned bounds work independently of parent placement, including fixed/fill/shrink sizes, multiline wrapping and padding. Verify painted text geometry, not only the enclosing widget's rectangle. Added from the user's explicit two-axis text-alignment request. |

## Design defaults and customization

| ID | Priority/state | Inspect first | Acceptance scenario |
| --- | --- | --- | --- |
| D01 | P0 · Done | [PR #1047](https://github.com/byeongsu-hong/ducktape-ui/pull/1047), [metric guide](../crates/ui-lang-components/docs/design-metrics.md), [native fixture](../examples/showcase/tests/cases/ui/design_metrics.ice) | Default 640px and compact 360/640px compositions verify page/section/field spacing, exact input/action heights, at least 32px desktop hit areas, centered labels and pointer/keyboard Save. Existing recipe inheritance and typed inputs cover the overrides; no new global density mechanism is needed. |
| D02 | P0 · Done | [PR #1047](https://github.com/byeongsu-hong/ducktape-ui/pull/1047), [typography owner](../crates/ui-lang-components/src/ui/typography.rs), [font guide](../crates/ui-lang-components/docs/design-metrics.md) | All 13 native Rust/Ice text roles agree on size, font, line height, measured height, baseline offset and semantic color. Longer wrapped copy and Korean editing use explicitly loaded IBM Plex Sans KR faces; omission of those assets fails actual glyph-width assertions. |
| D03 | P0 · Done | [PR #1048](https://github.com/byeongsu-hong/ducktape-ui/pull/1048), [default palette](../crates/ui-lang-components/src/ice/default.ice), [theme contract](../SPEC.md) | Changing an application's semantic palette keeps fields, surfaces and control states coherent; light/dark and custom accent examples do not require copying component source. Distinguish existing complete palettes from proposed partial overrides. |
| D04 | P0 · Done | [PR #1048](https://github.com/byeongsu-hong/ducktape-ui/pull/1048), [native state fixture](../examples/showcase/tests/cases/ui/theme_state_defaults.ice), [control/action recipes](../crates/ui-lang-components/src/ice/recipes.ice), [button-status tests](../examples/showcase/tests/cases/ui/button_status_children.ice) | Default, hover, focus, pressed, disabled and invalid states remain legible; overriding one geometric or color property does not reset unrelated states. |
| D05 | P1 · Audit | [component slots and custom content](../crates/ui-lang-components/README.md) | Replace a row/trigger/body visual through the public interface while retaining its selection, dismissal, keyboard and accessibility contract. Identify actual source-copy requirements before adding new extension points. |

## Common compositions

| ID | Priority/state | Inspect first | Acceptance scenario |
| --- | --- | --- | --- |
| C01 | P0 · Done | [PR #1015](https://github.com/byeongsu-hong/ducktape-ui/pull/1015) | Default and customized fields at 360/960px; wrapping errors; no empty-help gap; short-window scroll; binding, Tab/Space/Enter and focus preserved. Local: 8 settings tests, 330 showcase tests pass; five mutation Reds; merged as 607f7ce3. |
| C02 | P1 · Done | [Item/Attachment/Breadcrumb](../crates/ui-lang-components/src/ice/components.ice), [VirtualList](../crates/ui-lang-components/docs/virtual-list.md) | Leading visual, long content and trailing metadata/actions remain aligned; stable-key selection survives reorder/filter; optional content does not leave blank columns. |
| C03 | P1 · Done | [navigation examples](../examples/showcase/src/ui/components/navigation.ice), [parity navigation contracts](../crates/ui-lang-components/docs/parity.md) | A list/detail or settings screen has one obvious selected location, coherent title/actions and predictable back behavior, with a reusable default composition. |
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
the corrected path retains it. Deleting the currently anchoring row is covered
below. Similar deferred-state adapters belong to the S02 state audit;
this delivery changes only MessageScroller.

### L03: deleted anchor and completion

The first surviving previously visible transcript row now anchors the viewport
when its predecessor is deleted. Owner-level Red proves the missing correction;
a native delete click proves the previously clipped next row remains fully
visible at the same y=107.25px. All 48 MessageScroller unit tests and all five
native transcript tests pass. No surviving visible row means native offset and
clamping, not an invented replacement anchor.

This completes L03's native acceptance cases: fixed heading/actions and reachable
body end ([#1036](https://github.com/byeongsu-hong/ducktape-ui/pull/1036)), nested
wheel ownership ([#1037](https://github.com/byeongsu-hong/ducktape-ui/pull/1037)),
current-state transcript updates ([#1039](https://github.com/byeongsu-hong/ducktape-ui/pull/1039))
and deleted-anchor restoration here. Touch and Tree-specific integration remain
outside this native evidence.

### L07: soft wrapping and justification complete the native audit

The wrapped paragraph fixture adds per-line painted ink evidence for plain and
rich text: center/right/justified horizontal alignment and independent
center/bottom vertical alignment inside padded bounds. The renderer now
justifies to the finite available line width instead of first reducing it to
the longest natural line. Single-line, hard-newline-only and unbounded controls
retain natural width; actual justified soft wraps measure their expanded width.

The owner test and both native paragraph tests reject the original width loss
and an overwide hard-newline mutation. Center/right/vertical mutations fail both
native ink tests. After restoration, one owner and eight focused fixture tests
pass. The 576×480 monochrome/Geist capture is inspected.

Together with [#1028](https://github.com/byeongsu-hong/ducktape-ui/pull/1028),
[#1030](https://github.com/byeongsu-hong/ducktape-ui/pull/1030) and
[#1031](https://github.com/byeongsu-hong/ducktape-ui/pull/1031), this completes
L07's native fixed/fill/shrink, multiline, padding, selection, rich decoration
and compact-label acceptance. Tree-host and platform-specific font evidence
are outside this audit.

### S02: controlled catalog adapters commit current state

The [catalog adapters](../examples/showcase/src/adapters.rs) used the same
`Task<State>` snapshot pattern as the transcript. A native typing/navigation
batch lost the typed query; queued menu, calendar, modal and selection events
lost independent state fields. Toast ticks and reduced-motion changes could
remove a notification added before their completion. A navigation completion
could also overwrite a newer route chosen by a different control.

All twelve focus-bearing catalog adapters now return an immediate state and a
once-consumed focus task. The [production handlers](../examples/showcase/src/ui/handlers/app.ice)
assign state before launching that task. Previous-visibility decisions remain
inside the single reducer call. Both toast maintenance reducers return state
directly, without asynchronous completion handlers.

Nine regression tests reach their intended assertions on the prior behavior,
including native Character/ArrowDown events routed through the actual Command
widget and generated app handlers. The [first-class focus tests](../examples/showcase/src/ui/tests/app.ice)
exercise Select keyboard selection and trigger restoration, plus AlertDialog's
safe cancellation and trigger restoration. Discarding the shared focus task
fails both native assertions. A native Driver check additionally distinguishes
the Cancel and Confirm focus IDs and verifies restored trigger focus; its safe
Cancel assertion rejects the same mutation. Restoration passes all 336 tests.
The [component guide](../crates/ui-lang-components/README.md#controlled-state-and-focus)
explains the boundary and the distinction between native replacement events
and field-level updates. S02 remains open for product I/O ownership, failed
saves and stale network completions; those paths are outside this catalog audit.

### L05: retained editing across responsive layouts

2026-09-09: [PR #1043](https://github.com/byeongsu-hong/ducktape-ui/pull/1043)
closes L05 with a verified default-component responsive workspace
composition and agent guidance. An actual selected-word replacement assertion
failed after resizing when optional navigation shifted the native editor's
child position; always-present navigation ancestors restore editing continuity.
Sidebar-width mutation Reds report 208 vs 200 and 168 vs 160; changing Form
from fill to shrink fails the explicit minimum Save-height assertion (the
action collapses to zero height). All mutations are restored. Seven focused
tests pass; captures cover 960×640, 360×640, 320×240 and customized 527×480,
using scale 1, en-US, Linux metadata, reduced motion, native light theme and
the default app palette. Native window-manager and touch behavior are outside
this evidence. Source/API support is unchanged; the delivered scope is a
reusable composition, executable examples, screenshots and guidance.

### L06: equal native collection columns at every tested width

2026-09-09: [PR #1045](https://github.com/byeongsu-hong/ducktape-ui/pull/1045)
closes the native grid acceptance. Before the fix, a 320px minimum overflowed
232px of available width, and incomplete rows stretched to 432/330.5/298px
instead of the preceding 210/216.3333/192px tracks. One column calculation now
keeps track widths consistent and fits a narrow parent. Existing Flex layout
retains padding and natural row heights; inner one-cell grids provide 4:3 cards.

Eight authored scenarios plus three generated harness checks verify 280/419/420/480/721px
widths, custom 640px minimum/gap/insets, empty/single-item updates, natural rows,
painted content containment and the last card's actual pointer route. Five
pre-fix geometry assertions and three production mutations prove the oracles;
restoration passes. All 19 grid/fill-portion checks passed after renderer
integration. The broader earlier core/runtime and showcase runs passed 1,508
and 421 tests respectively. Inspected before/after captures and a 60-frame
production-view measurement are recorded in the
[evidence directory](../examples/showcase/screenshots/grid-collection/README.md).
Tree still sends ordinary growing, non-shrinking Flex items; these tests and
the completed L06 status claim native behavior only.

### D01/D02: compact metrics and complete native text-role parity

2026-09-09: [PR #1047](https://github.com/byeongsu-hong/ducktape-ui/pull/1047).

The shared Rust typography owner now matches established Ice recipe metrics
for all 13 text roles. Eleven line-height assertions and four semantic-color
assertions failed on the previous role table. Two-line native comparisons
verify the corrected size, line box, first baseline, font and color directly.
The default Ice screen metrics remain established; separate inline-code
container presentations are not forced into one wrapper contract.

A reusable workspace form demonstrates 24/16px standard page/section metrics
and 12/12px compact metrics at 640px and 360/640px. It retains 12/8px inner
section/field gaps and 32.25px compact controls. Tests edit Korean content,
activate the button below its label, then save through Tab/Enter. Padding,
font-asset and page-inset mutations fail the intended geometry assertions.
The font guide distinguishes loading bytes from selecting families and records
the actual IBM Plex Sans KR/Geist Mono setup. This completes native D01/D02;
Tree and platform font-fallback evidence are outside the scope.

D03/D04: the default Ice source now supplies the retained Rust light/dark
semantic palettes. The native `theme_state_defaults` fixture selects light,
dark and a complete application palette without copying shared components,
retains edited values, and checks customized geometry across hover, pressed,
disabled and input error/focus states. Pixel assertions cover contrasting
keyboard rings on filled primary/danger/custom actions in all three palettes.
The original low-contrast ring and three independent palette/state mutations
produce intended assertion Reds; seven native fixture checks and the existing
324-test Showcase binary pass after restoration. Typed externs continue to
receive their complete Rust Theme explicitly; no partial palette syntax or
Tree/platform appearance parity is claimed.

### C02/C03: a native list/detail composition

2026-09-09: [PR #1049](https://github.com/byeongsu-hong/ducktape-ui/pull/1049)
completes the native list/detail acceptance.

The [project browser](../examples/showcase/tests/cases/ui/list_detail_navigation.ice)
uses default components with stable-key selection, independent project drafts,
filter/reorder controls, clear selected location, coherent detail title/actions
and Back that restores the selected row's native keyboard focus. The checker
now admits identified buttons as native widget-operation targets, matching the
focus IDs already generated for them. Item and Attachment omit empty text and
its layout gaps; existing leading and long-text sizing remains intact.

Actual mutation Reds cover lost Back focus, discarded drafts, reset selection,
incorrect custom cap, an empty metadata column and overflowing long text. Native
owner and composition tests pass with all mutations restored. Compact captures
cover 320×560 and minimum 320×360; the default/custom readable cap is also checked.
The [guide](../crates/ui-lang-components/docs/list-detail-navigation.md) documents
public customization, button keyboard/accessibility semantics and state ownership.
This completes C02/C03's native composition acceptance. Durable saves, large
virtualized collections and platform-specific accessibility remain outside it.

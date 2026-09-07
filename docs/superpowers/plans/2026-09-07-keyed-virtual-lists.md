# Phase 4a: keyed and virtual module lists

Continue the authorized module-view prerequisites in Ice only. Actual Ducktape
MessageTimeline repeats keyed rows with virtual-row=44.0, while FilesScreen uses
27.0 and 39.0 estimates. Both require host widget state and measured heights to
follow row identity after inserts/reordering. Plain column lowering is insufficient.

Reuse native keyed_column, virtual_keyed_children, virtual_children and
virtual_scroll. The guest supplies copied keys, children and checked dimensions;
the host owns viewport observation, row layout, focus and scroll anchoring. Do
not add guest layout callbacks or a second virtualization engine.

Implementation scope:
- Add bounded keyed-column wire data with a key for each row and existing layout
  properties (padding, spacing, width/height, max-width, alignment and optional
  virtual-row estimate). Preserve native numeric key semantics; virtual keys use
  the existing lossless VirtualKey conversion, not display strings or hashes.
- Support virtual-row on ordinary columns through the same native virtual list
  machinery, with virtual_scroll on the enclosing native scroll container.
- Update wire traversal, sanitization, Props child attachment, retained native
  inputs/pictures, guest testing and generated node support together. Malformed
  key/child cardinality, duplicate identities and numeric limits need explicit
  bounded handling consistent with native identity requirements.
- Actual wasm evidence must cover reorder/prepend/remove and row-local focus or
  selection; scrolled rows preserve identity and layout measurements. Measure
  visible versus offscreen layout work. Check scroll anchoring and keyboard focus
  as well as pixels and routes. Add intended-assertion Red/Green evidence.
- Update public support claims and CI actual-bundle gates; independent review,
  local checks, focused PR, green CI, merge and cleanup.

Lazy remains the next coupled prerequisite, not an eager-render alias. The current
wasm route tables are rebuilt each frame, so caching a Node also caches its route
indices. A correct lazy implementation must retain/rebind callbacks and invalidate
state revisions without dispatching stale rows. Review the native memo dependency
and parking semantics before introducing that guest cache. This list change must
not claim the entire existing chat graph portable until lazy and its other blockers
are implemented and tested.

Status: wire key/list records, keyed emitter, ordinary virtual-column emitter and
host keyed/virtual rendering are implemented. Native VirtualScroll wraps the
relevant scroll containers before scroll anchoring. The copied ListKey preserves
native PartialEq while virtual keys preserve all numeric bits.

Initial evidence before the main advance: existing Tree tests 22 passed, hostile
frame checks 6 passed, explicit keyed/plain virtual codegen test passed, and the
actual app-store-keyed-fixture wasm bundle passed with stateful component rows.
Earlier fixture syntax and unused-handler inference failures were setup errors,
not assertion Red evidence. Interaction, measured offscreen layout work, deliberate
mutation evidence, expanded hostile key payloads, final lint/checks and PR delivery
remain. The fixture currently has three rows for reorder/prepend/remove and needs
an explicit large-list case for virtualization measurements.

Advanced to layered-layouts main 70e611ef after #959 merged with all CI green.
Stash conflicts union all traversal variants and preserve both fixture packages.
Autostash 03575ed1f791848ad63028b70b8f3a71b694cfb5 remains until final delivery.
Workspace-with-tests check passed (39.28s), recorded in .keyed-main-check.log. The pre-advance
wasm must be rebuilt before host interaction tests against the new wire variants.

Priority update from Claude/user: complete Tree text options against node/kit
before resuming this list scope. The keyed worktree is deliberately preserved.
Review fixes implemented: optional row keys distinguish positional virtual rows
(and splice responsive children); wire key equality preserves float bits while
native nonvirtual equality remains numeric; Scroll.virtual_rows is emitted from
the existing static native source traversal so conditional rows cannot toggle its
wrapper. Updated workspace-with-tests check and wasm bundle pass.

The new real host test reaches an intended failure: “focus follows row 2 through
reorder” for [1,2,3] -> [2,3,1]. .keyed-host-tests.log records it. Review traced
this to native VirtualChildren::diff at virtual_children.rs:701: Iced's
 diff_children_custom_with_search preserves position at equal lengths, while our
height/focus metadata moves by key. Replace that native tree reconciliation with
actual old-key -> Tree movement, retaining deferred offscreen diff and handling
duplicates. Ordinary Iced keyed_column uses the same helper, so its nonvirtual
path must also preserve arbitrary permutations. Add native rotation regression
and rerun the actual wasm failure. No success claim for list interactions yet.
All local commands have completed; no test process is running.

Resumed after node input prerequisites. Native arbitrary rotations now move each
row Tree, measured height and focus together by key, matching duplicate occurrences
in FIFO order and retaining deferred offscreen diff. The new native rotation test
failed at its focus assertion before the fix (.keyed-rotation-red.log); all 13
VirtualChildren tests pass after restoration (.keyed-native-green.log). Rebuilt
the actual keyed wasm fixture and its host reorder/prepend/remove input/focus
test passes (.keyed-rotation-host.log). Ordinary nonvirtual keyed-column
reconciliation and large-list evidence still remain.

The clear/refill regression also failed its measured-row assertion, then passed
with stale key metadata cleared; 14 VirtualChildren tests pass. Ordinary keyed
columns now delegate native Column behavior while moving Trees by PartialEq keys.
Native rotation failed [10,20,30,40] versus [40,10,20,30] before the fix; rotations,
duplicate occurrences and numeric zero/NaN semantics pass. Changed ordinary keys
use O(n²) occurrence search; unchanged keys take the direct diff path. The virtual
path retains hash-queue reconciliation. Read-only reconciliation review found no
actionable issue. Workspace check --tests passes. Rebuilt fixture now covers both
ordinary and virtual keyed rows: actual wasm input/focus tests 2/2 pass. Temporary
fixture syntax/duplicate-id setup failures were corrected and are not Red evidence.

Actual nonvirtual wasm mutation evidence: switching the host back to Iced
keyed_column fails the row-2 reorder focus assertion (.keyed-nonvirtual-wasm-red.log).
Restored runtime keyed_column; both actual wasm tests pass again
(.keyed-both-host-restored.log). No temporary mutation remains. Large-list wire
viewport evidence, current-main rebase, final full gates/docs/CI and PR remain.

Large actual-wire evidence now passes: the guest emits all 200 rows, the host
exposes at most 32 mounted input widgets at the head and tail, and a native Snap
operation reaches the last row. Disabling the virtual renderer fails the bound
with 200 mounted inputs (.keyed-large-red.log); restored all three wasm tests pass
(.keyed-all-host.log). This is mounted reachability evidence; existing native
layout counters separately prove bounded layout work. The CI bundle/test gate and
public support docs now cover keyed/virtual columns while explicitly retaining lazy
and selector command limitations. Core keyed tests 14 pass. Current-main rebase,
final lint/full gates and PR remain; input PR #966 waits only for performance CI.

Full root/app-store clippy passed after removing one redundant scroll conversion
and collapsing the fixture route lookup condition. Review found a native parity
gap: utility spacing on ordinary virtual columns was only applied to the outer
Column. A codegen regression fails the inner-spacing assertion before the fix;
the native path now falls back to style.gap after explicit linear.spacing.

Input PR #966 merged as 9d16417f with every CI green; merged rev sent to Claude
through the authorized socket, archived input evidence and removed that worktree.
Keyed branch is still based on 70e611ef and must now rebase to origin/main.
Final pre-rebase core tests: 989 unit tests pass plus integration suites; wire
45 and hostile-frame 6 pass (.keyed-final-core-wire.log). The utility-gap test
passes (.keyed-gap-green.log), and follow-up review has no remaining findings.

Rebased onto input-presentation main 9d16417f. Unioned text/widget and keyed
fixtures, docs, CI and coverage; workspace --tests check passed before rebase
--continue (.keyed-rebase-input-check.log). Post-rebase core 997 unit tests plus
integrations and wire/hostile checks pass; runtime library 337 tests pass. Rebuilt
wasm and all three actual keyed host tests pass. Root and full app-store Clippy
pass. Final integration review found no actionable findings. Lazy remains next.

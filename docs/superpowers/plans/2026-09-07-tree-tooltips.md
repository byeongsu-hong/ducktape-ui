# Tree tooltips

Ducktape shell's StatusPill has a tooltip at components/shell.ice:456 with
position bottom, gap 13.5, zero padding, 90ms delay and transparent style.
This is a known remaining node-kit requirement, after button recipes/wrapping.
Confirm the combined typed root after those PRs merge before claiming it is
the next compiler refusal.

Use the existing native tooltip widget and GuestView overlay forwarding. Carry
copied content/tip nodes, position, gap, padding, delay, viewport snapping and
concrete native container style values/presets. Preserve tooltip-derived
accessible descriptions without overriding an explicit descendant description.
Reject native callbacks and gradients, consistent with the Tree boundary.
Keep two children bounded by existing tree depth/node/text budgets and include
them in sanitize, traversal, diff/patch, host resources and hostile generators.
No guest pixel drawing, pointer positions or native resource pointers cross.

Validation must exercise actual bundled wasm hover before/after delay, hiding
after leaving, nondefault transparent styling and native accessibility
description. Include intended assertion failure when overlay/description
forwarding is removed, restoration, workspace/host checks and complete PR
review. Update module-views phase inventory with delivered presentation/layout
work. Rebase onto current main before delivery and check workspace with tests
before any conflicted rebase continuation. Do not edit Ducktape sources.

Work in progress on 72a90af4: Tooltip wire node and style/preset types, bounded
children/gap/padding/shadows and a 60-second delay cap, native host rendering
and visible-tip description collection, and Tree emission are implemented.
Rust callbacks/gradient backgrounds remain explicit refusals. Core/runtime
check passed. Hostile generation and bounds include Tooltip; its depth helper
initially omitted the new variant and was corrected. Core unit/integration and wire unit/hostile tests pass.
Actual wasm tooltip delay/overlay/accessibility tests and Red/Green evidence,
docs and final review remain. No commit or PR yet.

After #962 merged as c7145f2e, #963 wrapping combined probe found a preceding
intentional Icon Rust style callback refusal at components/icon.ice:7. Claude
was informed that declarative icon colors are needed downstream. SVG button
ink inheritance is another concrete missing capability; tooltip remains a
known shell requirement, not the claimed current first refusal.

Actual text wasm bundle and mounted host tests pass: public AccessKit snapshot
contains tooltip description before hover; raster is hidden before 90ms, shown
after delay, and hidden after pointer exit. Dropping description or forcing a
60-second delay fails the corresponding assertion; restoration passes.
Workspace and host clippy, Rust/Ice formatting and independent review pass.
Rebased onto c7145f2e with only documentation conflicts; workspace check with
tests and real wasm bundle pass again. Combined Tree tests and both mounted host tests pass.

Rebased onto wrapping main f6367837. Workspace check with tests passed before
continuing the rebase; final combined diff review has no findings. Bundling
the combined fixture into its worktree-local output and all three mounted
host tests pass. An initial run read an older local wasm because the bundle
output used the shared target directory; rerunning the bundle with the fixture
output path corrected that setup failure.

# Host-evaluated responsive rules for wasm views

Continue phase 3c without changing Ducktape. Preserve the existing
`responsive size=(width, height)` syntax and lower size-dependent conditions
into bounded copied container rules. The host evaluates these rules while
laying out the container; resizing must not invoke guest code or create a
sensor feedback loop.

Support width/height comparisons, numeric arithmetic and Boolean combinations,
including thresholds computed from guest state. Keep size-independent conditions
and loops in the guest. Bind nested responsive conditions to their own container.
Reject native callbacks and unsupported uses of measured locals explicitly;
do not substitute invented measurements.

The renderer must build only the selected branch and release the old branch's
native view lease on selection changes. Input/editor state, widget identity,
focus, hit routing, nested containers and branch changes during layout need
behavioral evidence. Retain the existing native responsive implementation and
its per-instance same-size reuse. Copy the immutable rendering context needed
for deferred host layout; do not retain a lock on the guest.

- [x] Bounded rule data, decoding, evaluation and hostile-rule tests.
- [x] Wire responsive/conditional nodes and Tree lowering with scoped local IDs.
- [x] Host branch selection and correct native widget lifecycle.
- [x] Actual wasm fixture at multiple container sizes, input routes, nested
      rules and resize without guest ticks; regression Red/Green evidence.
- [x] Batch checks/bundle/tests, support documentation and phase ledger;
      independent review.
- [ ] Delivery: focused PR, green CI, merge and worktree cleanup.

Implementation notes: query dimension operands carry the responsive node key,
so nested conditions can reference the correct ancestor without exporting
window coordinates or guessing a lexical depth. Only ancestor sizes enter the
host lookup. Add a typed measured-local binding owner in codegen; snapshots and
slots preserve that owner, and ordinary expression emission rejects measured
locals outside supported host rules with E190. Avoid code-string markers.

`When` is a structural child list, not a wrapper widget: selected children must
splice into their surrounding row/column/grid so conditionals do not change
layout direction or spacing. Native Ice already requires `if` under a layout.
A responsive node's deferred host builder renders the chosen branches using
an ancestor-keyed size map. Hidden surface providers must not be invoked.

Next code locations: shared flow emission is in `codegen/expr/children.rs`;
Tree row/grid emission delegates there. Intercept measured-local conditions
there while leaving ordinary guest conditions unchanged. `BindingOwner` is in
`codegen/expr/binding.rs`; measured owners must force a hard capture in the
recording environment and report E190 from ordinary local expression emission.
Use a unique generated local for each responsive node's key so nested bindings
cannot accidentally reference a shadowed Rust temporary.

Host rendering currently borrows Inputs/Pictures/Surfaces. Deferred responsive
layout needs owned snapshots; retain shared editor content and make provider
closures shareable so only selected surfaces are instantiated. Do not prebuild
all branch widgets or keep a Guest mutex locked through layout.

# Declarative canvas geometry for wasm views

Continue the module-view prerequisites in phase 3c. Ducktape remains read-only.
Implement a focused geometry boundary using the existing Ice canvas syntax:
the guest evaluates application data into bounded drawing commands, and the
host paints those commands inside the canvas widget. No raster generation or
native canvas Program crosses the wire.

Start with paths (rectangle, rounded rectangle, circle, line and explicit path
segments), solid fill/stroke and transforms, including ordinary guest-side
conditionals and loops where their inputs are owned application data. Audit the
resolved canvas forms before fixing the precise supported set. Reject native
canvas-local state/events, unsupported paints/assets and host-only layout
bindings explicitly rather than substituting incorrect measurements. Responsive
container rules remain a separate change; do not emulate them with guest
callbacks or sensor feedback.

- [x] Audit resolved commands, existing renderer primitives, wire limits and
      all canvas references in the module inventory; document the supported set.
- [x] Add serializable geometry and strict bounded validation, then host-native
      drawing and Tree code generation using existing language syntax.
- [x] Add an actual wasm geometry fixture with pixel assertions and a native
      interaction wrapper; reject unsupported native callback/state forms.
- [x] Demonstrate assertion failures with deliberate geometry/route mutations,
      then batch bundle, focused tests, workspace checks and formatting.
- [ ] Update support tables and the module phase ledger, obtain independent
      review, push a focused PR, inspect green CI, merge and clean the worktree.

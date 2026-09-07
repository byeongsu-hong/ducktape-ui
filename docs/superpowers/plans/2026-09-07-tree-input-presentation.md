# Tree input presentation

Actual Ducktape NodeScreen log filter requires hint, accessible label, padding,
text size, relative line height and @control styling. Preserve native label
and hint semantics; host owns editing and typed state. Copy layout, accessible
metadata and checked utility/status styles without Rust callbacks.

Implemented InputOptions and extended InputStyle. Named fonts and app default
typography follow the existing trusted host registry. Explicit default and
mono fonts carry concrete descriptors. InputStyle is boxed to avoid expanding
every wire node for input-only style data. Styles apply utility, active, focus
border, then explicit status and focused-hovered values, matching native.

Validation: core 995 plus integration and wire 47 plus six hostile tests pass.
Actual bundled widget tests cover a11y metadata, native geometry, default
typography and enabled/disabled typing. Ignoring disabled fails X-vs-Y; removing
focus-border forwarding fails red-vs-green. Repeated X was a weak oracle and
was replaced with Y plus an explicit guest-disabled assertion. Restored runs
and final formatting/lints are in the task logs. Independent review's default
font/size finding was fixed and re-reviewed with no remaining findings.

The actual node-root now passes the kit and input options and stops at
app.ice:97 opaque node_log_timeline. Claude owns its conversion to a data-only
extern component and host registry: source:&str -> unit with retained state
in the host. No additional wire variant is required. Verify existing native
registry routing and the actual surface wasm fixture; no Ducktape edits.

Final base 26b28b83 includes tooltip and SVG inheritance. Rebase conflicts
preserve both fixture tests, with SVG first and input clicks derived from
accessibility bounds. Workspace check with tests passed before continuing;
actual combined wasm bundle and all seven mounted widget tests pass. Tree
tests and the actual node-root probe pass the input boundary again, stopping
at the same opaque log slot. Final complete diff review has no findings.
The data-surface wasm route test and native registry-hit test also pass.

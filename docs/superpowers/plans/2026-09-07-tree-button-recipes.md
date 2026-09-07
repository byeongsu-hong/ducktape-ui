# Tree button recipes

Actual Ducktape kit buttons use ghost/secondary/icon recipes and keyboard focus
rings. Carry resolved recipe values, not recipe names or Rust callbacks. Keep
native precedence: preset → recipe colors/border → typed active face → typed
state face → recipe disabled treatment unless a typed disabled face exists.
Preserve native label size/line-height/font utilities, padding, fixed-size
content centering and focus-visible ring placement on the accessible wrapper.

Add copied ButtonPreset/ButtonRecipe data to ButtonStyle, bound every numeric
and color value, and keep unsupported utility bits explicitly refused. Reuse
native preset functions and existing Face/Border overlays. Do not add a generic
CSS engine or callback transport. Test disabled alpha after active overrides,
explicit disabled precedence, hover/pressed colors and keyboard-only focus ring,
then bundle the real widget fixture. Read the Ducktape typed node root after
rebasing onto merged text and button accessibility support to identify the next
actual refusal. Never modify downstream files.

Implemented on main 72a90af4. Copied presets/recipe faces, shared named-font
resolution, guest default typography, explicit zero padding and native keyboard
focus rings are implemented. Native template selection now respects explicit
zero padding. Workspace check with tests passes after combining accessibility.
Combined verification passes: core 994 unit tests plus integration tests, wire
47 unit/6 hostile tests, 29 native view-tree tests, actual widget wasm bundle
and 5 mounted host tests. Workspace and host clippy, Rust/Ice formatting pass.
Independent review reports no actionable findings. Ring-forwarding mutation
fails the intended raster assertion and restoration passes.

Read-only actual Ducktape node probe passes button presentation and now refuses
`row wrap` at components/node.ice:708. Wrapping layout is the next actual node
blocker, followed by tooltip/opaque host-surface slots as encountered.

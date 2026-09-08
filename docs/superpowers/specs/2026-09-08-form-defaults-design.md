# Composable form defaults

The first delivery of the agreed design direction is an Ice-native settings form:
applications declare labels, bindings and descriptions, and the library supplies
consistent geometry and control styles. This is a component contract, not new
language syntax or an editor/platform project.

Provide Form (bounded, centered, vertically scrolling content), FormSection
(section heading and caller-owned body), and TextField (Field plus a native bound
input). Extend Field with optional description/error text without empty spacer
rows. TextField exposes padding, radius, disabled and secure inputs; Field keeps
its slot for fully custom controls. Existing semantic palette tokens remain the
color source. No backend or dependency additions.

Use a separate settings example to demonstrate defaults and a customized control
in one real form. At 360 and 960 logical pixels, fields and descriptions stay
inside their content column; long labels wrap; errors grow the field naturally.
Form owns vertical scrolling and caps reading width. A custom radius/padding
must preserve native input binding, accessibility and focus. Validation and save
state belong to the example, not the component library.

Evidence: first-class geometry, text, input and focus assertions; a compiling
minimal behavior mutation must fail the intended assertion and pass after
restoration. Capture and inspect wide, narrow and error/customized states. The
scope establishes these form defaults only; it does not claim automatic layout
for arbitrary screens, OS-native styling or a global density override system.

## Resolved interface details

Use the ordinary component name `FormSection`: a `Form.Section` direct child
would participate in Ice's compound-slot resolution, which is unnecessary for
these independently composable wrappers. Form and FormSection each accept the
existing single-root content slot. Multi-field callers provide a column.

The settings example uses required-email validation on Save. Tests exercise a
successful save before invalid input to prove success feedback is cleared.
TextField customization preserves Tab traversal through a caller-supplied
checkbox and Enter activation of the Save button. No app-facing copy describes
implementation details.

# Ice UI design workflow

Use this reference for a new screen, redesign, or product-flow request. Design
within Ice's actual rendering, state, routing, and accessibility model; do not
produce a React-shaped mockup that must be translated later.

## Contents

- [Start with behavior](#start-with-behavior)
- [Map state and boundaries](#map-state-and-boundaries)
- [Choose the view structure](#choose-the-view-structure)
- [Choose component boundaries](#choose-component-boundaries)
- [Build the visual system](#build-the-visual-system)
- [Audit component internals](#audit-component-internals)
- [Run a focused polish pass](#run-a-focused-polish-pass)
- [Design every interaction state](#design-every-interaction-state)
- [Design responsive native layout](#design-responsive-native-layout)
- [Preserve accessibility](#preserve-accessibility)
- [Implement and verify](#implement-and-verify)

## Start with behavior

Write a compact brief before code:

- Name the user's primary task.
- Identify the first useful screen and primary action.
- List secondary actions that deserve visible controls.
- Define success, empty, loading, validation, and failure outcomes.
- Identify destructive or irreversible actions that need confirmation.
- Note keyboard, pointer, window, and platform behavior that materially changes
  the flow.

Prefer one obvious primary action. Do not add navigation, panels, or settings
without a concrete user task.

## Map state and boundaries

Classify each value before declaring it:

| Value | Owner |
| --- | --- |
| editable text, selection, open/closed state, loading flag | Ice state |
| reusable component's private interaction state | component-local Ice state |
| domain entity returned from Rust | typed extern struct |
| authoritative validation or invariant | Rust |
| persistence, networking, authentication | Rust async extern |
| pure missing conversion | Rust `pure` extern |
| immediate effect, environment read, retained identity | Rust `sync` extern in a top-level app state initializer or immediately evaluated handler expression, not an async completion route |
| native task, widget, shader, or subscription | matching typed adapter |

Trace each interaction:

```text
control -> route -> handler -> optional Rust effect
        -> result handler -> state -> recomputed view
```

Design the success and failure routes together. Do not represent failure as an
untyped string if the Rust boundary needs structured recovery.

## Choose the view structure

Sketch the indentation tree, not boxes floating on a canvas:

1. Choose one root for the view.
2. Use `col` for primary vertical reading order.
3. Use `row` for short related action/control groups.
4. Use `flex` only for real distribution, wrapping, or grow/shrink behavior.
5. Use `grid` for repeated two-dimensional content.
6. Use `scroll` around one bounded content tree.
7. Use `stack` for visual overlap and `overlay` for modal interaction.
8. Use `responsive` when layout must react to native limits.

Keep source order equal to semantic reading and keyboard focus order. Avoid
deep nesting that exists only to simulate CSS wrappers.

## Choose component boundaries

Create an Ice component when at least one is true:

- the same meaningful view structure repeats;
- the structure has a stable product name;
- it needs explicit slots;
- it owns small reusable interaction state;
- it emits one typed value that callers route.

Pass all inputs as typed props. Use `slot` or named/compound slots for caller
content. Give repeated stateful instances stable IDs.

Keep a screen-specific row inline when extraction would merely rename five
lines. Do not create wrapper components whose only purpose is forwarding every
property.

## Build the visual system

If the application has the `ducktape-ui` source interface at a stable relative
path, import its `default.ice` and start with the shared components and recipes.
A Cargo dependency alone does not create an Ice import path; otherwise vendor
the complete `src/ice` directory or use the Rust API. Do not copy individual
declarations into the app or rebuild their variants from raw color and geometry
values. Add a local component only for product-specific structure that the
shared layer does not represent. The showcase adapter interface contains fixed
catalog data; do not import it into a product application. Put retained
behavior behind a typed boundary owned by that application.

Start from semantic tokens:

```text
bg, surface, fg, muted, primary, danger, border
```

Keep the required `bg`, `fg`, `primary`, and `danger` tokens. Add a token only
when it has a repeated semantic role; do not name tokens after a single screen
or raw hue.

Establish:

- one base text size and a small type hierarchy;
- consistent spacing increments;
- one normal and one emphasized surface treatment;
- clear primary, secondary, subtle, danger, and disabled control states;
- visible focus and sufficient contrast;
- restrained radius, border, and shadow use.

Use typed geometry properties first. Use checked `@` utilities for semantic
colors, font emphasis, and documented wrapper gaps. When a visual role repeats,
declare one target-specific semantic `recipe` and import it with the theme.
Use a component only when structure or behavior repeats. Use structured native
status blocks only for meaningful state deltas.

## Audit component internals

Treat a control as a contract, not just an outer rectangle. For each affected
repeated control role, inspect the relevant layers:

| Layer | Useful signals |
| --- | --- |
| outer box | width, height, border, radius, and target size |
| inner geometry | horizontal and vertical padding, icon gap, content alignment |
| label | font family, size, weight, line height, clipping, and baseline/center |
| states | normal, hover, pressed, focus, disabled, loading, and selected where relevant |
| semantics | accessible name, role, disabled state, and focus order |

Make the semantic recipe own every repeated value that its target can lower.
For a text-only action, use compact `button "Label" @recipe`; the recipe owns
the generated label metrics. Use child content only when the button truly has
structure such as an icon-and-label row, and then style that child explicitly.
An otherwise redundant label component is evidence that the recipe or compiler
contract is incomplete; fix that source of truth before multiplying wrappers.

For fixed-height controls, calculate the content budget before rendering:

```text
content height = control height - vertical padding - vertical border
```

The label line box must fit that budget. Then verify the rendered `text_y` is
the control's vertical center; equal outer heights alone do not prove correct
internal alignment.

Start with representative instances, including an edge state and content near
the end of a scrollable view. Expand to every instance, breakpoint, or state
when changing a shared layer or when the sample exposes inconsistency. A clean
first viewport is not evidence for content farther down.

## Run a focused polish pass

Use this pass after the feature works and before calling a catalog, showcase,
or long settings surface complete. These are risk areas, not a mandatory test
sequence; select the ones the actual change can affect.

- **Coverage:** inspect representative content and edge states across the real
  scroll extent and relevant responsive limits.
- **Consistency:** compare the outer and inner contracts of repeated roles,
  including their text and accessibility semantics.
- **Containment:** ensure content, focus visuals, overlays, and scroll affordances
  neither obscure content nor capture interaction outside their ownership.
- **Typography:** inspect rendered family, weight, and style across the paths
  involved, and keep application, theme, and adapter choices coherent.
- **Interaction:** exercise the relevant pointer and keyboard paths; visual
  feedback must identify the state or target that will actually receive input.
- **Evidence:** choose captures and assertions proportional to risk. Put a
  focused regression at the layer that owned the defect; add end-to-end
  evidence when behavior crosses boundaries.

## Design every interaction state

For every effectful screen, cover:

- initial or boot state;
- loading without duplicate submission;
- useful empty state;
- populated state;
- recoverable failure with a clear next action;
- disabled state;
- focused and keyboard-active state;
- success feedback when the result is not otherwise obvious.

Use `return if` as a UI guard and `disabled=` for feedback, while retaining
authoritative validation in Rust. Prevent stale search or preview results from
every handler that starts the same logical work with one fully qualified
`run latest` lane when completion filtering is enough, or a named `run replace`
lane when the prior Iced task should be aborted. App and preset handlers split
across files share a root lane only through unaliased imports; aliased component
lanes remain instance-owned. Confirm that the Rust boundary
does not rely on abort to roll back an effect or stop detached work.

Do not hide errors only in logs. Do not use color as the only state signal.

## Design responsive native layout

Treat Ice as a native Iced application:

- Use `fill`, `shrink`, fixed lengths, and fill portions deliberately.
- Set sensible initial and minimum window sizes.
- Let content scroll rather than clipping important controls.
- Use wrapping or responsive branches when horizontal space is constrained.
- Keep pointer targets and spacing usable at the minimum supported size.
- Consider named windows and daemon behavior only when the product flow needs
  them.
- Add target-specific window settings only for an explicit platform need.

Do not assume browser viewport units, DOM measurement, media-query CSS, or
mobile touch conventions.

## Preserve accessibility

Before polishing visuals:

- Give child-content/icon-only buttons an explicit `label=`.
- Give meaningful images a label and optional description.
- Leave decorative images unlabeled.
- Never expose secure input values.
- Keep enabled controls in meaningful source order.
- Preserve visible focus.
- Make disabled state understandable without color alone.
- Prefer visible text labels over placeholder-only inputs.
- Use descriptions for useful extra context, not repeated labels.

Do not invent numeric tab order. Ice derives focus order from the checked view
tree.

## Implement and verify

Implement in this order:

1. Declare the app settings, semantic theme, and repeated visual recipes.
2. Declare typed Rust boundaries.
3. Declare minimal state.
4. Implement handlers and complete effect routes.
5. Build the view tree from recipes and typed local exceptions.
6. Extract only proven structural component boundaries.
7. Add status styling and polish.
8. Audit control internals across roles, states, breakpoints, and scroll extent.
9. Check accessibility labels and source order.

Then:

```bash
cargo ice fmt
cargo ice check
```

Use the live LSP throughout, with the importing app root open. Run the narrow
behavior test for the primary interaction. Use `cargo ice schema` or the
language references whenever syntax or property ownership is uncertain.

---
name: design-ice-ui
description: Design, write, modify, explain, debug, format, and verify Ice UI language applications in `.ice` files and their typed Rust/Iced boundaries. Use for planning or implementing Ice screens, interaction states, app and daemon structure, components, slots, views, widgets, themes, accessibility, responsive layout, subscriptions, tasks, externs, `ui_lang::include_app!`, `cargo ice`, schema, diagnostics, or Ice LSP work, especially when a request might otherwise be approached with React, JSX, hooks, DOM, CSS, or JavaScript assumptions.
---

# Design Ice UI

Design and build against Ice's checked indentation-based language and typed
Rust boundary. Treat the compiler, generated schema, and live LSP as
authorities. Do not invent syntax by analogy with React or Iced's Rust API.

## Keep the mental model straight

Apply these rules before editing:

- Treat a `.ice` source graph as a statically checked program that compiles to
  ordinary Iced Rust. There is no interpreter, DOM, virtual DOM, JSX, or
  JavaScript runtime.
- Let indentation form the view tree. Do not add closing tags, braces, commas
  between nodes, or JSX fragments.
- Keep transient/display state and interaction flow in Ice. Keep validation,
  invariants, persistence, networking, security, and platform-specific work in
  Rust.
- Change state only in `on` handlers. Views and components describe rendering;
  they do not execute effects during render.
- Route events with `->`, bind supported state with `<->`, forward an emitted
  payload with `_`, apply checked utilities with `@`, and assign scoped identity
  with `#`.
- Pass component inputs explicitly. Components do not capture app state.
  Declare slots explicitly; every declared slot is required and receives one
  root.
- Use the small closed Ice expression language. Move missing domain operations
  behind typed `sync` or async extern functions instead of embedding Rust.
- Prefer Core constructs. Use an existing typed Rust adapter for unusual native
  behavior; do not extend the DSL merely to mirror another Iced method.
- When `ducktape-ui` is available, import its `default.ice` interface once and
  reuse its checked components and recipes before declaring local equivalents.
  Use compound variants such as `Alert.Success` and `Badge.Secondary`; do not
  replace them with free-form variant strings. Do not import its showcase
  adapter interface into an application; define a typed Rust boundary for
  product-specific retained widgets.
- Preserve accessibility: label child-content buttons, label meaningful images,
  never expose secure-input values, and keep source order meaningful.

Read [references/language.md](references/language.md) whenever writing or
refactoring `.ice`. Read the other references only when their scope is involved:

- [references/design-workflow.md](references/design-workflow.md) for new
  screens, redesigns, user flows, component boundaries, visual hierarchy, and
  interaction-state planning.
- [references/views-and-style.md](references/views-and-style.md) for layout,
  widgets, control flow, components, IDs, styling, and accessibility.
- [references/rust-boundary.md](references/rust-boundary.md) for project setup,
  extern types/functions/adapters, effects, subscriptions, and Rust tests.
- [references/tooling-and-lsp.md](references/tooling-and-lsp.md) for live editor
  use, diagnostics, formatting, schema inspection, expansion, or verification.
- [references/extended-surface.md](references/extended-surface.md) for advanced
  widgets, canvas, panes, window operations, native values, or escape hatches.

## Inspect before writing

1. Find the Cargo workspace root, `.cargo/config.toml`, app/daemon root, imported
   fragments, Rust `include_app!` call, and corresponding extern modules.
2. Read the complete files being changed and every `use` edge in their source
   graph. Treat declarations as graph-wide even when split across files.
3. Reuse the repository's closest compiling `.ice` example. Prefer the readable
   task app for Core patterns and focused fixtures for advanced features.
4. Run `cargo ice schema` when a construct or property is uncertain. The schema
   is generated from the same Core table as LSP completion.
5. If a live LSP client is available, keep the app root open so imported-buffer
   diagnostics are analyzed in that root graph. Still run the compiler before
   claiming completion.

In this repository, start with:

```text
crates/ui/src/ice/default.ice            canonical design-system import
crates/ui/src/ice/components.ice         shared structural components and variants
crates/ui/src/ice/recipes.ice            shared semantic visual roles
examples/showcase/src/ui/showcase.ice    complete default component catalog
examples/showcase/tests/cases/           generated-app behavior through iced_test
examples/showcase/tests/snapshots/       headless visual regression baselines
examples/iced-app/src/ui/tasks.ice       readable app root
examples/iced-app/src/ui/extern/         production and test extern declarations
examples/iced-app/src/ui/components/     component and slot patterns
examples/iced-app/src/ui/handlers/       state transitions and effects
examples/iced-app/src/ui/showcase.ice    language widget surface
examples/iced-app/src/ui/*.ice           focused native fixtures
examples/apple-music/src/ui/music.ice    complete product-style application
SPEC.md                                  implemented language revision
COVERAGE.md                              exact Iced surface ledger
```

When those upstream files are not present in the working project, use the
public [language specification](https://github.com/byeongsu-hong/ducktape-ui-lang/blob/main/SPEC.md)
and [coverage ledger](https://github.com/byeongsu-hong/ducktape-ui-lang/blob/main/COVERAGE.md).

## Make the smallest valid change

Choose the boundary first:

| Need | Put it in |
| --- | --- |
| layout, display state, styling, event route | `.ice` |
| reusable view with explicit inputs/slots | Ice `component` |
| pure domain conversion missing from expressions | Rust + `sync` extern |
| I/O or ordinary future | Rust async function + bare extern + `run` |
| existing `iced::Task` | Rust `task` extern + `task` statement |
| custom widget, shader, subscription, or style | matching typed extern adapter |
| common missing authoring concept | propose a language revision; do not improvise syntax |

Keep one canonical spelling and follow nearby formatting. Add no compatibility
aliases, wrapper components, state mirrors, or new dependencies unless the
requested behavior requires them.

## Write in compiler order

Organize a source graph canonically:

```text
app | daemon
use
extern
theme / font / qr
state
preset
component
on
subscribe
view
```

Use one `app` or `daemon` and one `view` across the graph. Split files by
concern with relative `use "file.ice"` imports; do not duplicate declarations
across fragments.

For an interaction, trace the complete loop:

```text
widget route -> handler -> optional typed Rust effect
             -> result handler -> state assignment -> view recomputation
```

Declare every payload explicitly at the destination handler and let the checker
infer handler parameter types from incoming routes. Do not hide domain failure:
fallible externs require both success and error routes.

## Validate continuously

After a meaningful edit:

1. Use LSP diagnostics and formatting for immediate feedback.
2. Run `cargo ice fmt` to format Rust and every discovered `.ice` file.
3. Run `cargo ice check` to analyze every app graph and let rustc verify extern
   paths, signatures, generated types, and Cargo features.
4. Run the narrow relevant Rust test or fixture suite.
5. Run `cargo ice compat` only when changing backend versions, runtime
   dependencies, compatibility contracts, accessibility bridges, or the
   reference app integration.

Use `cargo ice expand path/to/app.ice` only to diagnose lowering or Rust errors.
Never edit generated Rust.

For end-to-end generated-app behavior, fix initial state with an Ice `preset`,
put the existing `iced_test` DSL cases under `tests/cases/*.ice`, and call
`iced_test::run` with the generated in-crate program. Keep screenshot regression
in a Rust test using the headless snapshot API; do not invent a second test DSL.

If `cargo ice` is unavailable, inspect `.cargo/config.toml`. In this repository
it is a local Cargo alias for the `cargo-ice` workspace binary; run commands
from the workspace root.

## Respond with evidence

Report:

- the `.ice` and Rust boundaries changed;
- the command or LSP evidence that verified them;
- any current language or platform limit that remains.

Do not describe the result as React-like. Explain it using Ice's state,
handlers, routes, components, slots, externs, and generated-Iced model.

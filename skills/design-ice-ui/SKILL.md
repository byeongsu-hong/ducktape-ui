---
name: design-ice-ui
description: Design, write, modify, explain, debug, format, and verify Ice UI language applications in `.ice` files and their typed Rust/Iced boundaries. Use for planning or implementing Ice screens, interaction states, app and daemon structure, components, slots, views, widgets, themes, accessibility, responsive layout, subscriptions, tasks, externs, `ui_lang::include_app!`, `cargo ice`, schema, diagnostics, or Ice LSP work, especially when a request might otherwise be approached with React, JSX, hooks, DOM, CSS, or JavaScript assumptions.
---

# Design Ice UI

Design and build against Ice's checked indentation-based language and typed
Rust boundary. Together they cover the pinned public, application-facing Iced
surface. Treat the compiler, generated schema, coverage ledger, and live LSP as
authorities. Do not invent syntax by analogy with React or Iced's Rust API.

Use the references as decision aids, not rituals to execute wholesale. Choose
the depth of inspection and evidence from the requested outcome, the affected
surface, and the risk of shared-layer regressions. Preserve intentional
variation. Add a narrow rule only for a recurring, non-obvious invariant; do
not turn a one-off component fix into permanent implementation prescription.

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
- Use the small closed Ice expression language. Move deterministic missing
  domain operations behind typed `pure` externs, immediate effects/environment
  reads/retained identity behind app-initializer-or-immediate-handler-only
  `sync` externs, and asynchronous work behind async externs instead of
  embedding Rust. Explicit `run every`, `run latest`, and `run replace` Future
  completion route expressions, and `task` statement completion route
  expressions, are pure-only owned snapshots of ordinary cloneable Ice data,
  materialized when the statement launches; direct recomputation-unsafe
  builtins are rejected, and `_` is supplied by the delivered completion.
  Other completion route families keep their documented timing.
- Prefer the canonical checked representation recorded for the pinned Iced
  capability: direct Ice syntax for common concepts and a typed Rust adapter
  for higher-order or custom native behavior. Do not add a keyword merely to
  mirror another Iced method, and do not treat a missing application-facing
  capability as outside the language contract.
- When the `ducktape-ui` source interface exists at a stable relative path,
  import its `default.ice` once and reuse its checked components and recipes
  before declaring local equivalents. A Cargo dependency alone does not create
  an Ice import path; otherwise vendor its complete `src/ice` directory or use
  the Rust API. Use compound variants such as `Alert.Success` and
  `Badge.Secondary`; do not replace them with free-form variant strings. Do not
  import its showcase adapter interface into an application; define a typed
  Rust boundary for product-specific retained widgets.
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

1. Find the Cargo workspace root, `.cargo/config.toml`, `build.rs`, app/daemon
   root, imported fragments, Rust `include_app!` call, and corresponding extern
   modules.
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
crates/ui/src/ice/default.ice            workspace design-system entry source
crates/ui/src/ice/components.ice         shared structural components and variants
crates/ui/src/ice/recipes.ice            shared semantic visual roles
examples/showcase/src/ui/app.ice         catalog and first-class behavior test
examples/iced-app/src/ui/tasks.ice       readable app root
examples/iced-app/src/ui/extern/         production and test extern declarations
examples/iced-app/src/ui/components/     component and slot patterns
examples/iced-app/src/ui/handlers/       state transitions and effects
examples/iced-app/src/ui/component_state.ice  layout, paint, and interaction tests
examples/iced-app/src/ui/showcase.ice    language widget surface
examples/iced-app/src/ui/*.ice           focused native fixtures
examples/apple-music/src/ui/app.ice      complete product-style source graph
SPEC.md                                  implemented language revision
COVERAGE.md                              exact Iced surface ledger
```

When those upstream files are not present in the working project, use the
public [language specification](https://github.com/byeongsu-hong/ducktape-ui/blob/main/SPEC.md)
and [coverage ledger](https://github.com/byeongsu-hong/ducktape-ui/blob/main/COVERAGE.md).

## Make the smallest valid change

Choose the boundary first:

| Need | Put it in |
| --- | --- |
| layout, display state, styling, event route | `.ice` |
| reusable view with explicit inputs/slots | Ice `component` |
| pure domain conversion missing from expressions | Rust + `pure` extern |
| immediate effect, environment read, or retained identity | Rust + `sync` extern in a top-level app state initializer or immediately evaluated app/component/preset handler expression; never an async completion route expression |
| I/O future whose every completion matters | Rust async function + bare extern + `run every` |
| existing `iced::Task` | Rust `task` extern + `task` statement |
| custom widget, shader, subscription, or style | matching typed extern adapter |
| missing public application-facing Iced capability | inspect the coverage ledger, then add one checked direct or typed representation through a language revision |

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
test
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
4. Run `cargo ice test` when changing first-class tests or behavior they cover;
   ordinary Cargo discovers the same generated tests.
5. For every visual UI change, run `cargo ice inspect` with an explicit root,
   viewport, theme, and relevant preset. Open the PNG and inspect the JSON
   outer geometry, inner padding, text size/line box/baseline, paint,
   accessibility, and `.ice` source fields. Scroll through the full view and
   inspect its end; do not infer appearance from code or stop after compilation
   succeeds.
6. After a visual correction, inspect again with the same inputs. When a prior
   capture is available, run `cargo ice diff` and resolve every unexplained
   manifest or pixel delta. Keep an intentional delta only when it matches the
   requested design change.
7. Run the narrow relevant Rust test or fixture suite.
8. Run `cargo ice compat` only when changing backend versions, runtime
   dependencies, compatibility contracts, accessibility bridges, or the
   reference app integration.

Use `cargo ice expand path/to/app.ice` only to diagnose lowering or Rust errors.
Never edit generated Rust.

For end-to-end generated-app or component behavior, write a top-level Ice
`test` in the app graph. Use `preset`, `viewport`, or a one-root `mount` for
setup; declare `target` aliases for rendered descendant IDs; then drive widgets
and assert app state, exact text, input values, computed bounds, or structured
paint fields. A component call ID is only an identity scope, so target an
identified rendered descendant such as `#card/root`; later targets may use that
alias as a descendant-path prefix. `cargo ice test` checks
the source graphs and runs the generated tests. For repeated controls, assert
both outer dimensions and internal text metrics/alignment, plus the accessible
name and each affected interaction state. Do not register Rust wrappers,
add a second case format, or mock Rust externs in Ice; deterministic extern
behavior belongs behind `cfg(test)` or a named preset.

If `cargo ice` is unavailable, inspect `.cargo/config.toml`. In this repository
it is a local Cargo alias for the `cargo-ice` workspace binary; run commands
from the workspace root.

Use a deterministic visual loop such as:

```bash
cargo ice inspect path/to/app.ice --viewport 1440x900 --theme light \
  --preset populated --name populated_light
cargo ice diff path/to/baseline/populated_light.json \
  target/ice-inspect/path_to_app/populated_light.json
```

Name and preserve the input tuple in review evidence: app root, preset,
viewport, theme, scale, locale, platform, and reduced-motion setting. Run the
loop for each materially different responsive breakpoint or interaction state;
one convenient viewport is not evidence for all of them. Treat custom-renderer
paint marked unavailable in JSON as a declared inspection limit, not as a
passing visual result.

Use a preset to expose deterministic application states. For a state reached
through clicks, typing, focus, scrolling, or time, author a first-class Ice
test that performs those semantic actions and `capture`s the result; run it
with `cargo ice test <test-name> -- --nocapture`, then inspect and diff those
artifacts by the same rules.

## Respond with evidence

Report:

- the `.ice` and Rust boundaries changed;
- the command or LSP evidence that verified them;
- the inspected input tuple and PNG/JSON paths for visual changes, plus the
  diff report when a baseline existed;
- any current language or platform limit that remains.

Do not describe the result as React-like. Explain it using Ice's state,
handlers, routes, components, slots, externs, and generated-Iced model.

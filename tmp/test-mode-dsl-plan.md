# First-class Ice test mode implementation plan

Status: implemented and validated.

## Product boundary

Ice tests live in normal `.ice` source as top-level `test` declarations. They
exercise the generated Iced program and real Rust handlers, tasks, streams,
subscriptions, layout, interaction, and structured paint. They do not reuse or
preserve the removed external `iced_test` ICE-file DSL, and they do not invent
a DOM, CSS selectors, mocks, snapshots, virtual time, or synthetic component
boxes.

Stable widget identity is the only selector model. A component-call ID creates
a scope; rendered descendants carry their own IDs. Target aliases are local to
one test and are re-resolved after every update.

## Language surface

- Add checked top-level `test snake_case_name` declarations.
- Allow optional `preset`, `viewport`, `timeout`, one-node `mount`, and
  `target alias = #scoped/id` declarations before executable steps.
- Add `click`, `hover`, `press`, `release`, `type`, named `key`, `resize`, and
  checked top-level `dispatch` actions.
- Add boolean, exact equality, approximate numeric, presence, absence, global
  text, and target-scoped text expectations.
- Expose target identity, visibility, layout, clipping, content, transform,
  scroll, surface-paint, and text-paint fields through the existing expression
  checker.
- Permit direct `#id` identity on every rendered built-in that users need to
  inspect or interact with, including text and selection controls.
- Reject statically known paint-field access for custom renderers; retain
  geometry and interaction tests where the renderer contract supports them.

## Compiler and editor work

1. Extend AST, parser, formatter, semantic checking, static ID scopes, imports,
   and source-origin tracking.
2. Lower each declaration to an ordinary generated `#[test]` under
   `#[cfg(test)]`, with a mounted-view program variant only when requested.
3. Preserve the normal application boot, update, theme, subscription, preset,
   and Rust extern paths. Emit source-mapped runtime calls for every statement.
4. Add schema/completion support and test-local definition, reference, and
   rename behavior for target aliases.
5. Add `cargo ice test` as analysis followed by the standard Rust test harness.

## Runtime work

Build the smallest public headless driver over existing Iced test/runtime
primitives:

- fresh state and executor per test;
- persistent UI cache within a test;
- real boot/preset tasks, update loop, recursively emitted finite tasks, and
  subscriptions;
- native widget IDs plus Ice scoped stable IDs;
- pointer, keyboard, resize, and direct-message actions;
- post-layout target geometry and visible text queries;
- structured tiny-skia quad/text inspection for style assertions;
- deterministic ambiguity, missing-data, timeout, and source-mapped failures.

## Migration and documentation

- Replace repository-owned external ICE behavior fixtures and Rust registration
  wrappers with in-source tests; remove their direct dependency where unused.
- Add contracts that prove computed layout/paint, focus and typing, component
  state, sync handlers, tasks, and direct dispatch.
- Update `SPEC.md`, `README.md`, `COVERAGE.md`, component-library docs, and the
  bundled Ice design skill. State all deliberate non-goals explicitly.

## Acceptance gates

- Parser/checker/formatter/codegen fixtures cover every syntax family and key
  rejection path.
- Generated example tests pass through `cargo test` and `cargo ice test`.
- `cargo fmt --all -- --check`, `cargo ice fmt --check`,
  `cargo check --workspace`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --no-deps`, and `cargo ice check`
  pass.
- No external DSL registration, compatibility adapter, stale fixture, or stale
  public documentation remains.

## Result

Implemented the complete first-class test mode across parsing, checking,
formatting, code generation, the persistent headless Iced driver, editor
schema/LSP support, `cargo ice test`, examples, fixtures, and public
documentation. The resulting tests cover computed layout and paint, scoped
component identity, real Rust sync/update/task/subscription paths, keyboard and
pointer interaction, mount-only views, and source-mapped failures.

All acceptance commands pass:

- `cargo fmt --all -- --check`
- `cargo ice fmt --check`
- `cargo check --workspace` (also exercised by `cargo ice check`)
- `cargo test --workspace` (also exercised by `cargo ice test`)
- `cargo clippy --workspace --all-targets --no-deps`
- `cargo ice check`
- `cargo ice test`
- `git diff --check`

The final full test run passed 46 `cargo-ice` tests, 370 component-library
tests, 358 core tests, all 3 fixture suites, and 43 runtime tests; the existing
isolated Linux AT-SPI smoke test remains intentionally ignored. Independent
specification and regression audits found no release blocker.

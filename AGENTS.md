# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 workspace for the Ice UI language. `crates/ui-lang-core/`
contains the parser, AST, semantic checker, formatter, and Rust code generator.
`crates/ui-lang/` provides the proc-macro adapter, `crates/ui-lang-runtime/`
contains runtime widgets and accessibility support, and `crates/cargo-ice/`
implements the `cargo ice` tooling. Runnable applications live in `examples/`;
their `.ice` sources are under `src/ui/` and supporting Rust code under
`src/`. End-to-end language fixtures are in
`crates/ui-lang-core/tests/cases/`. Treat `vendor/iced_wgpu/` as a pinned local
patch and do not edit `target/`.

## Build, Test, and Development Commands

- `cargo check --workspace`: type-check every workspace member.
- `cargo test --workspace`: run the Rust and auto-discovered fixture tests.
- `cargo clippy --workspace --all-targets --no-deps`: lint project targets
  without linting dependencies.
- `cargo fmt --all -- --check`: verify Rust formatting.
- `cargo ice fmt --check`: verify `.ice` formatting.
- `cargo ice check`: analyze Ice sources, then check the workspace.
- `cargo run -p iced-app`: run the main reference application.

Use `cargo run -p apple-music-example` for the larger visual example.
Platform-specific accessibility checks are in `scripts/`.

## Coding Style & Naming Conventions

Let `rustfmt` define Rust layout (four-space indentation). Use `snake_case` for
modules, functions, tests, and source files; `UpperCamelCase` for types and
components; and `SCREAMING_SNAKE_CASE` for constants. The workspace forbids
unsafe Rust. Format `.ice` files with `cargo ice fmt`; indentation defines the
view tree, so never align it manually for appearance.

## No Legacy or Compatibility Policy

Do not preserve deprecated APIs, syntax, compatibility shims, migrations, or
fallback paths. Remove stale code, tests, fixtures, and documentation outright,
then update all callers in the same change. Do not retain unused behavior “just
in case”; Git history is the archive.

## Testing Guidelines

Rust uses the built-in test harness, with focused unit tests beside parser,
checker, or codegen modules. For end-to-end behavior, add
`cases/<format|diagnostic|compile>/<case-name>/as-is.ice` and the matching
`to-be.ice` or `to-be.txt`; no test registration is needed. There is no numeric
coverage target. Changes to supported Ice behavior should satisfy the evidence
rules in `COVERAGE.md`.

## Commit & Pull Request Guidelines

Recent history favors concise imperative subjects, often Conventional Commit
style such as `feat(editor): ...` or `fix(codegen): ...`. Keep each commit
focused. Pull requests should describe user-visible behavior, list commands
run, link relevant issues, and include screenshots for visual example changes.
Update `SPEC.md`, `README.md`, and `COVERAGE.md` when public syntax, tooling, or
support claims change.

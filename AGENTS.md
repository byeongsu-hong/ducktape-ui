# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 workspace for the Ice UI language. `crates/ui-lang-core/`
contains the parser, AST, semantic checker, formatter, and Rust code generator.
`crates/ui-lang/` provides the proc-macro adapter, `crates/ui-lang-runtime/`
contains runtime widgets and accessibility support, `crates/ui-lang-template/`
defines the published view format both the generator and the runtime use,
`crates/ui/` provides the default component library, and `crates/cargo-ice/`
implements the `cargo ice` tooling. Runnable applications live in `examples/`;
their `.ice` sources are under `src/ui/` and supporting Rust code under `src/`.
End-to-end language fixtures are in `crates/ui-lang-core/tests/cases/`. Do not
edit `target/`.

## Build, Test, and Development Commands

- `cargo check --workspace`: type-check every workspace member.
- `cargo test --workspace`: run the Rust and auto-discovered fixture tests.
- `cargo clippy --workspace --all-targets --no-deps`: lint project targets
  without linting dependencies.
- `cargo fmt --all -- --check`: verify Rust formatting.
- `cargo ice fmt --check`: verify `.ice` formatting.
- `cargo ice check`: analyze Ice sources, then check the workspace.
- `cargo run -p showcase`: run the reference application and component catalog.

Use `cargo run -p music-example` for the larger visual example.
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

Use the `test-ice-ui` skill whenever adding, changing, or reviewing first-class
Ice UI tests. A new regression test is incomplete until command evidence shows
that its intended assertion fails against the pre-fix behavior or one minimal
temporary behavior mutation, then passes after restoration. Parse, compile,
timeout, target-resolution, and setup failures do not count as Red evidence.
`capture` alone is not an assertion, and `dispatch` alone does not prove a
widget route or interaction works.

## Commit & Pull Request Guidelines

Recent history favors concise imperative subjects, often Conventional Commit
style such as `feat(editor): ...` or `fix(codegen): ...`. Keep each commit
focused. Pull requests should describe user-visible behavior, list commands
run, link relevant issues, and include screenshots for visual example changes.
Update `SPEC.md`, `README.md`, and `COVERAGE.md` when public syntax, tooling, or
support claims change.

## Required Agent Delivery Workflow

All repository changes must be made in a dedicated Git worktree on a non-default
branch. Never edit the primary checkout or commit directly to `main`/`master`.
Reuse a task worktree only when the new change belongs to the same review scope;
otherwise create a new worktree and branch.

Create every task worktree under `.worktree/<task-name>` inside the primary
checkout. Do not create sibling worktree roots, worktrees under `/tmp`, or any
other external worktree directory. Treat the primary checkout as a read-only
worktree-management surface: inspect and manage worktrees there, but make task
edits only in the matching `.worktree/<task-name>` checkout. Remove a task
worktree promptly after its pull request is merged, closed, or abandoned; never
discard uncommitted changes while cleaning worktrees.

For every completed change, run the relevant local checks, commit the focused
diff, push the branch, and open a pull request. Review the complete PR diff and
available CI/review results before merging; resolve every actionable finding and
rerun affected checks.

Merge without asking again only when confidence is high: the scope is fully
understood, the diff is focused, required checks are green, no review findings
remain unresolved, and the merge target and strategy are unambiguous. Otherwise
leave the PR open and report the exact uncertainty or approval needed. Never
bypass branch protection, required review, or failing checks.

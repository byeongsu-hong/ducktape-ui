# Form Defaults Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan in this session.

**Goal:** Ship consistent, customizable settings forms using existing Ice components.

**Architecture:** Add form composition and a bound native input to the shared Ice library. Exercise them through a small settings application and first-class UI tests.

**Tech Stack:** Rust 2024, Ice, pinned iced 0.14.0.

**Spec:** ../specs/2026-09-08-form-defaults-design.md

## Global Constraints

- Changes only in `.worktree/form-defaults` on `agent/form-defaults`.
- No new dependencies, language syntax or backend changes.
- Preserve custom control slots and native focus/accessibility.
- Red evidence must be an intended assertion failure, not setup/compile failure.

## Task 1: Components and executable contract

Files: `crates/ui-lang-components/src/ice/components.ice`, new `forms.ice`,
`default.ice`, `examples/settings/{Cargo.toml,build.rs,src/main.rs,src/ui/app.ice}`, workspace `Cargo.toml`.

- [x] Write settings interaction and narrow/wide geometry tests before the components.
- [x] Implement Form, FormSection and TextField; make Field help/error optional.
- [x] Run `cargo test -p settings-example` and fix diagnosed implementation errors.
- [x] Temporarily constrain TextField to a fixed width; confirm the narrow geometry assertion fails, restore, rerun.
- [x] Temporarily disconnect the custom input binding; confirm the state assertion fails, restore, rerun.
- [x] Capture and visually inspect default, customized and narrow/error states.

## Task 2: Documentation and delivery

Files: library README, root README, SPEC, COVERAGE, settings README/screenshots.

- [x] Document public component signatures, defaults, customization and layout ownership.
- [x] Run Ice/Rust formatting checks and targeted tests: settings 8 passed; showcase 330 passed, 1 ignored.
- [x] Complete targeted Clippy check: `cargo clippy -p settings-example --all-targets --no-deps`.
- [x] Review focused diff and captures; independent final review found no blockers.
- [ ] Commit, push and open a PR with evidence.
- [ ] Inspect CI and review results; merge only if all required gates allow it.

## Recorded verification

- Five authored settings tests reached their intended failing assertions under
  the documented minimal mutations, then passed after restoration.
- `cargo ice api diff`: 5 additive changes, 0 breaking, 0 behavioral-review.
- `cargo ice inspect examples/settings/src/ui/app.ice --viewport 360x820 --frames 60 --name settings_default`:
  debug view p50/p95 125/144us, layout 3/5us, update 35/53us. These are local
  descriptive measurements, not a release performance guarantee.
- Rust formatting and Ice formatting pass; Ice analyzed 51 root graphs with
  unrelated existing fixture/CEF warnings.
- Independent reviewer reported no blocker; its suggestion to assert mutually
  exclusive error/success feedback was implemented and passes.

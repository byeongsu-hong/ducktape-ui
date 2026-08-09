---
name: test-ice-ui
description: Design, write, repair, and review tests for Ice UI changes, including choosing between first-class `.ice` tests, Core fixtures, Rust tests, render inspection, and platform smoke tests. Use whenever Codex adds tests to an Ice screen or component, tests user interaction or visual behavior, fixes a misleading or vacuous UI test, reviews UI test coverage, or must prove that an Ice regression test can fail. Do not use for implementation-only UI work with no test or evidence task.
---

# Test Ice UI

Write the smallest test that observes the real behavior at its owning boundary,
then prove that its oracle can reject a plausible regression.

## Inspect the behavior

1. Read the changed production source, its complete Ice `use` graph, and the
   nearest existing test file before writing assertions.
2. Read the relevant sections of `docs/testing.md` for driver, target,
   determinism, capture, or performance behavior. Treat `SPEC.md` as the
   grammar of record when syntax is uncertain.
3. State this contract before editing the test:

```text
Claim: the user-visible behavior being protected
Trigger: the production path that causes it
Oracle: the observation that distinguishes correct behavior
Counterexample: a plausible broken implementation the oracle must reject
```

If the counterexample would still pass, choose a stronger oracle before
writing the test.

## Choose the owning layer

Stop at the narrowest layer that observes the claim without duplicating
production logic:

| Claim | Evidence |
| --- | --- |
| Ice syntax, checking, formatting, lowering, or codegen | focused Core unit test or `tests/cases` fixture |
| Product state transition or finite handler flow | first-class Ice test using a deterministic `preset`, `dispatch`, and state/output assertions |
| Widget route, pointer, keyboard, focus, or accessibility behavior | first-class Ice test using a rendered `target` and semantic input such as `click`, `key`, `focus`, or `a11y` |
| Layout, paint, theme, or responsive behavior | scoped geometry/paint/accessibility assertions plus `inspect` or `capture` evidence |
| Domain invariant or typed Rust boundary | focused Rust unit or integration test |
| Native renderer or platform-only integration | the repository's platform smoke path |

When behavior crosses layers, keep the owner test and add at most one focused
end-to-end Ice test for the integration seam.

## Reject weak oracles

- Do not use `dispatch` as proof that a widget route works. Drive the control
  through semantic input when the claim concerns the control.
- Do not count `capture` as an assertion. Pair it with semantic, accessibility,
  geometry, or structured paint assertions and inspect the artifact.
- Do not leave a negative assertion vacuous. Establish that the old value was
  initially present, or assert the state that could have produced it, before
  expecting it to disappear.
- Scope repeated text with `within`; global text is valid only when any visible
  occurrence proves the claim.
- Assert both sides of a distinction when nearby states can look plausible:
  selected/unselected, old/new, enabled/disabled, or present/absent.
- Prefer observable state, output, accessibility, and structured paint over
  implementation details or generated Rust strings.
- Use named presets or `cfg(test)` Rust behavior for deterministic inputs. Do
  not dispatch handlers that start endless streams; preset the reached state.
- Keep one behavior claim per test. Add another test only for a distinct
  counterexample or interaction path.

## Prove Red and Green

A new regression test is incomplete until its intended assertion has failed:

1. Run the focused test against the pre-fix behavior when that behavior still
   builds with the new test.
2. If implementation already exists or the old source cannot host the test,
   make one minimal temporary behavior mutation that preserves compilation and
   realizes the stated counterexample.
3. Run the focused test and confirm it reaches the intended assertion. A parse,
   compile, timeout, missing-target, or setup failure is not Red evidence.
4. Restore the exact production source. Never commit the mutation.
5. Run the same focused test and confirm Green.

Do not weaken an assertion to obtain Green. Fix the test setup or production
behavior when the observed contract is correct.

## Verify proportionally

Run the narrow command first, then the checks affected by the change:

```sh
cargo ice test <test-name> -- --nocapture
cargo ice fmt --check
cargo ice check
```

For visual claims, preserve the root, preset, viewport, theme, scale, locale,
platform, and reduced-motion tuple. Open the PNG and inspect the JSON or review
bundle; a successful capture command does not prove appearance.

Run relevant Rust, platform, workspace, or compatibility checks only when the
changed boundary requires them. Ensure ignored performance tests are explicitly
selected by CI as described in `docs/testing.md`.

## Report evidence

Report the claim, chosen layer, test file/name, Red mutation and intended
failure, Green command, and visual artifact tuple when applicable. State any
renderer or platform limit instead of treating unavailable evidence as a pass.

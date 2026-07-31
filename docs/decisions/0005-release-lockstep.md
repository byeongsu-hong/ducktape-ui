# 0005: Releases are proven from packaged crates

- Status: Accepted
- Date: 2026-08-01

## Context

Workspace checks can accidentally rely on path dependencies, unlisted files,
or repository layout that published consumers do not receive. Building a
multi-platform `cargo-ice` binary is necessary but does not prove that the
published crate graph can compile a real external Ice application.

The repository's No Legacy or Compatibility Policy forbids preserving removed
behavior through legacy shims, so a public contract change must be classified
before release rather than hidden afterwards.

## Decision

CI packages the publishable crates, extracts the resulting `.crate` archives
into a temporary workspace outside the repository, and builds a minimal
downstream Ice application using only those extracted packages. The fixture
must exercise `ui-lang-build` generation in the consumer `OUT_DIR`, the
versioned SHA manifest, `ui_lang::include_app!`, the direct runtime dependency,
an imported `.ice` fragment, a source-mapped extern diagnostic probe, and a
minimal first-class Ice test.

The packaged downstream fixture runs on pull requests that alter publishable
packages and again in the tag release workflow. The fixture rejects dependency
metadata that reaches back into the source workspace, and its external build
fails when an archive omits required source or build inputs.

The public Ice interface must have a deterministic, versioned API fingerprint.
CI diffs the managed baseline and rejects classified breaking changes unless a
maintainer-controlled approval records the intent and the reviewed baseline is
updated in the same change. Before 1.0, an approved breaking pull request does
not independently bump every package; package versions advance together at the
release boundary. Component contracts, recipes, theme tokens, extern surfaces,
language revision, and package version participate in the fingerprint.

The archive consumer and API fingerprint are independent gates. Package smoke
proves archive contents and downstream compilation; it does not classify API
compatibility. Conversely, a stable fingerprint does not prove that published
archives contain enough files to build an external application. This decision
is fully enforced only when both gates run.

Publishable Ice crates and the `cargo-ice` tool are released as one tested
version set. There are no deprecated aliases, legacy-syntax shims, migrations,
or fallback package graphs; callers and documentation are updated in the same
change.

## Rejected alternatives

### Rely on `cargo test --workspace`

It does not prove archive contents, external `OUT_DIR` behavior, or absence of
workspace-only paths.

### Test archives only on tags

Discovering a broken package graph after a release commit is too late for an
ordinary review gate.

### Preserve breaking contracts with shims

That contradicts the repository policy and makes the pre-1.0 surface harder to
reason about. The fingerprint makes the change explicit instead.

## Consequences

The two gates make release CI take longer and require maintaining a small
external consumer fixture and an API baseline. Once both are present, a green
release proves both the packaged graph and the reviewed public contract, not
merely the repository checkout, and reviewers can separate refactors from
public contract changes.

## Revisit trigger

Revisit the fixture when the published package topology or supported consumer
entry point changes. Revisit lockstep versioning only after the crates have
independent compatibility guarantees and separate downstream fixtures.

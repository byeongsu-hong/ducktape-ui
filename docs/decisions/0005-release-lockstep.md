# 0005: Releases are proven from packaged crates

- Status: Accepted
- Date: 2026-08-01

## Context

Workspace checks can accidentally rely on path dependencies, unlisted files,
or repository layout that published consumers do not receive. Building a
multi-platform `cargo-ice` binary is necessary but does not prove that the
published crate graph can compile a real external Ice application.

The workspace intentionally has no legacy or compatibility policy, so a public
contract change must be classified before release rather than hidden behind a
shim afterwards.

## Decision

CI packages the publishable crates, extracts the resulting `.crate` archives
into a temporary workspace outside the repository, and builds a minimal
downstream Ice application using only those extracted packages. The fixture
must exercise `ui-lang-build` generation in the consumer `OUT_DIR`, the
versioned SHA manifest, `ui_lang::include_app!`, the direct runtime dependency,
an imported `.ice` fragment, a source-mapped extern diagnostic probe, and a
minimal first-class Ice test.

The packaged downstream fixture runs on pull requests that alter publishable
packages and again in the tag release workflow. Package metadata is rejected
if it leaks repository paths or undeclared source inputs.

The public Ice interface has a deterministic, versioned API fingerprint. CI
diffs the managed baseline and rejects classified breaking changes unless the
release change deliberately updates the version and reviewed baseline under
the release process. Component contracts, recipes, theme tokens, extern
surfaces, language revision, and package version participate in that record.

Publishable Ice crates and the `cargo-ice` tool are released as one tested
version set. There are no deprecated aliases, compatibility adapters, or
fallback package graphs; callers and documentation are updated in the same
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

Release CI takes longer and maintains a small external consumer fixture and an
API baseline. In return, a green release proves the artifact users receive,
not merely the repository checkout, and reviewers can separate refactors from
public contract changes.

## Revisit trigger

Revisit the fixture when the published package topology or supported consumer
entry point changes. Revisit lockstep versioning only after the crates have
independent compatibility guarantees and separate downstream fixtures.

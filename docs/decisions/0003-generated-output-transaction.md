# 0003: Generated output is a recoverable transaction

- Status: Accepted
- Date: 2026-08-01

## Context

`ui-lang-build` publishes generated Rust and a hash-to-source manifest below
Cargo's `OUT_DIR`. Build scripts, `cargo ice dev`, and separate Cargo commands
may target the same generation directory concurrently. A killed process or a
partially written manifest must not leave a cache that requires a user to run
`cargo clean` manually.

`OUT_DIR` is a disposable cache, not user-authored state. Correctness therefore
depends on never publishing a partial individual file, using the manifest as
the commit record for a completed inventory, and recovering automatically from
incomplete cache data. This is not a claim that a filesystem can atomically
replace the entire multi-file directory for unsynchronized readers.

## Decision

Generation directories use a cross-process lock. A publisher stages every
changed Rust output under a unique temporary name, flushes staged data, then
atomically replaces final output files. It writes and atomically replaces the
versioned manifest last. Readers never treat an uncommitted staged file as a
generated output.

The manifest records the full SHA-256 source-path identity, its hash-to-source
mapping, and each generated-content digest required to validate the cache. A
filename collision with a different normalized source remains a hard error.
Re-emitting identical bytes preserves the existing output and its modification
time. Generated Rust source markers remain the separate diagnostic source-map
mechanism.

On lock acquisition, generation removes abandoned transaction files. A
missing, malformed, unsupported, or digest-inconsistent manifest invalidates
the disposable cache and causes complete regeneration. Recovery must not ask
the user to clean the workspace. Concurrent publishers merge the roots still
owned by their completed generation and publish a valid final inventory.

Tests must cover interrupted publication, corrupt and digest-mismatched
manifests, stale temporary files, collision rejection, unchanged modification
times, and concurrent `cargo ice dev`/Cargo generation.

## Rejected alternatives

### Write final outputs in place

A process can be killed between truncation and completion, exposing invalid
Rust or JSON to another Cargo process.

### Publish the manifest before outputs

That advertises files or digests which are not yet committed. The manifest is
the commit record and is therefore replaced last.

### Fail and require `cargo clean`

This treats recoverable cache corruption as user data loss and makes concurrent
tooling unreliable.

## Consequences

Generation performs extra staging, synchronization, and locking work. In
return, every newly committed manifest describes fully written outputs,
individual output files are never visible partially written, identical builds
avoid needless recompilation, and failed publishers recover without manual
cleanup.

## Revisit trigger

Revisit the storage protocol if Cargo provides an equivalent transactional
artifact API, or measurements show synchronization exceeds the build budget.
Any replacement must retain manifest-last publication and automatic recovery.

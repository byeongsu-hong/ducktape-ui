# Tree wire protocol admission

Status: implemented and reviewed. Rebased onto origin/main c36ad66e after the
editor transaction lane (#1019, f878b948). Initial WIRE_EPOCH = 1 includes that
transaction schema; subsequent serialized shape changes require another epoch.

## Contract

Replace the five-line manifest with exactly six lines:

```
ice.manifest.v2
<name>
<description>
<comma-terminated permission families, or empty>
<preferred size, or none>
<canonical unsigned decimal wire epoch>
```

Authored packages use `ice.test.manifest.v2` with the same fields. Reject old
headers, missing/extra lines, zero, signs, whitespace, leading zeroes, overflow,
and duplicate sections. Keep all existing byte/count bounds. Parsing an
otherwise valid manifest with a different epoch succeeds, so callers can report
an incompatibility instead of calling it malformed.

`ui_lang_wire::WIRE_EPOCH: u32` is the single source of the current epoch;
initial value 1. A const decimal serialization helper derives the emitted line
from that integer, avoiding independent integer/string literals. `Manifest`
gains `wire_epoch: u32` and `check_wire_protocol() -> Result<(), ProtocolMismatch>`.
`ProtocolMismatch { guest: u32, host: u32 }` has stable Display diagnostics.
The check requires equality. This is not a minimum version or version range.

Manifest format version describes how to parse static metadata. Wire epoch
describes the bincode Node/Event/Frame/native transport contract. WIT package
version describes component function signatures. These are independent. This PR
changes the manifest format but leaves WIT signatures unchanged.

`export_app!` and authored/native export paths generate the field automatically;
callers do not select or override an epoch. A forged declaration is not a safety
boundary: bounded decode/sanitization and sandboxing still apply.

## Admission

Wasm: parse and check exactly the bytes whose content hash was validated, before
Component compilation/instantiation and before init, restore, or tick. The
content-hash component cache holds only admitted components; a cache hit cannot
introduce a component skipped by the check.

Native: parse and check the manifest in the already bounded, hash-verified
package before creating the executable child process. Use the same public
Manifest method. Authored native and Wasm loaders apply their own header parser
and the same epoch check.

Catalog metadata parsing remains separate from compatibility admission. A
syntactically valid unsupported epoch may be catalogued, but installation must
fail with a specific mismatch. Never trust only catalog fields: load checks the
manifest associated with the exact bytes being executed.

Reload uses the same Instance admission path, which currently precedes old
instance snapshot and candidate restore. Rejection leaves the current surface,
instance, hash and state in place. Native packages are rejected before any child
side effect. No automatic fallback, reinitialization, or old decoder is added.

## Evidence

1. Wire parser accepts the current and another positive epoch, but rejects the
old header, absent epoch, noncanonical integer, overflow, and duplicate section.
2. Shared gate accepts equal and rejects different epochs with guest/host values.
3. Actual Wasm installation rejects a well-formed incompatible artifact before
its init/tick sentinel; a compatible control runs. An equivalent native package
must be rejected before its executable sentinel runs.
4. Start an actual native and Wasm guest with non-default state, offer a candidate
whose sole metadata change is its epoch, and assert mismatch plus the existing
instance/hash/state still usable. Candidate init/restore/tick must not execute.
5. Mutation remove the gate: the intended mismatch assertions must fail, rather
than compile/setup failures. Restore the gate and rerun the same tests Green.

Use isolated CARGO_TARGET_DIR under this worktree. No live network or downstream
pin changes. Rebase/check against transaction lane before final release.

## Deliberately outside this PR

Existing capability families are permissions, not render features. No optional
negotiation, minimum Ducktape release requirement, or compatibility-range claim
is introduced. Snapshot schema validation remains independent and unchanged.

Until a stable additive encoding is deliberately introduced, any serialized
shape change requires a new epoch. Do not promise unchanged epochs for ordinary
Rust enum/struct edits. A future extension must have explicit stable tags and
length-delimited payloads plus a required-feature gate; unsupported required
features are refused, and optional alternatives must be selected before boot.
That design will replace affected encoding outright, without legacy decoders.

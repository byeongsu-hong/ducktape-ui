# Tree button presentation

The real Ducktape typed node root reaches button checked/expanded after text
support. Preserve optional checked, expanded and description values on wire
buttons and forward them to the existing native accessible wrapper. False must
remain distinct from absence. Bound descriptions with the shared text budget;
verify state transitions and accessible properties, plus actual guest bundling.

Then support existing lowered button recipes/presets using native precedence:
preset, utility face, active typed face, per-state typed face, recipe disabled
pass last unless a typed disabled face exists. Keep focus-visible rings on the
accessible wrapper and fixed-size content centering consistent with native.
Do not silently accept unsupported utility bits or Rust style callbacks.

Implement and verify the accessibility contract first; recipes may be a separate
focused PR. All downstream files remain read-only. Text PR #960 is independent
and currently in CI; rebase this branch onto its merged main before delivery and
run workspace checks and actual wasm bundling after any conflict resolution.

Accessibility implementation status: native AccessKit true/false/absent snapshot
passes. Removing checked forwarding makes it fail (None versus Some(True));
restoration passes. The widget guest bundles and its native click/focus test
checks copied checked/expanded/description values before and after the handler.
Wire unit/hostile tests and workspace checking pass. Workspace and host clippy pass.
Review's accidental composer dispatcher match restriction was removed.
Rebased onto merged text support 9b7fa7b3. Conflict resolution was checked with
cargo check --workspace --tests before rebase --continue. Tree tests, native
AccessKit state tests, actual widget bundle and its mounted host test pass again.
PR #961 awaits CI on this combined revision.

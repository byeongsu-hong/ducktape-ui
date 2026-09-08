# Guest State Snapshots Implementation Plan

> **For agentic workers:** Execute inline with superpowers:executing-plans. Steps use checkboxes to retain the delivery boundary.

**Goal:** Export bounded guest state snapshots and restore a replacement without replaying boot or losing component state.

**Architecture:** Generated code copies owned state into a schema-checked data envelope. A separate `SnapshotApp: App` capability constructs a restored app without calling boot. Driver tasks, callable routes, memoized trees and native pointers are not serialized; fresh routes and subscriptions are constructed from restored state.

**Tech Stack:** Rust, existing serde/bincode wire codec, Ice codegen and wasmtime fixture tests.

**Spec:** `examples/app-store/module-views.md`, “Reload” and “Hot reload after state/lifecycle boundary”.

## Global Constraints

- Ducktape sources remain read-only. All edits use this dedicated worktree.
- Preserve root and retained/mounted component state, including mounted boot markers and component initial state.
- Strict schema equality; no migrations, compatibility decoder or silently dropped fields.
- Reject busy one-shot tasks and deferred boot before snapshot. A failed restore leaves the existing driver unchanged.
- Preserve owned data only. Unsupported native/opaque state returns a precise error for the entire snapshot.
- Host catalog watching and transactional replacement are the next PR; exporting state alone does not complete hot reload.
- Actual wasm tests and behavior mutation Red evidence are required before merging.

## Task 1: Bounded wire data and generated state conversion

**Files:** `crates/ui-lang-wire/src/snapshot.rs`, `crates/ui-lang-wire/src/lib.rs`, `crates/ui-lang-core/src/codegen/snapshot.rs`, shared codegen value conversion, `crates/ui-lang-runtime/src/lib.rs`.

**Interfaces:**
```rust
pub struct Snapshot { pub schema: String, pub state: SnapshotValue }
// SnapshotValue contains Unit/Bool/I64/F64/Str/Bytes/List/Option/Record.
impl Snapshot {
    pub fn encode(&self) -> Result<Vec<u8>, String>;
    pub fn decode(bytes: &[u8]) -> Result<Self, String>;
}
// Generated methods, on each Tree app:
fn __snapshot(&self) -> Result<Vec<u8>, String>;
fn __restore(bytes: &[u8]) -> Result<Self, String>;
```

- [x] Add a decoder bounded to 8 MiB, depth 32 and 65,536 values. Count before allocating child collections; never truncate state. Test round-trip data, malformed lengths, nesting, non-finite numbers and trailing data.
- [x] Generate a SHA-256 schema from reachable state names/types/record fields/enum variants and component storage modes, excluding source offsets and layout. Serialize root states, component maps in sorted scope order, initial component states and mounted boot markers.
- [x] Reuse existing surface value code where representations agree. Support owned scalars, editor text, markdown source, bytes, lists/options/results, declared records/enums, palettes and keyboard modifiers. Generate an explicit whole-snapshot error for unsupported state instead of breaking normal Tree compilation.
- [x] Decode every field before constructing restored state. Reset derived/memo/route/task bookkeeping; never call `__boot` or `__boot_task`. Add bounded mounted-state restoration that retains booted scopes while allowing new render pruning.
- [x] Assert a layout-only source change preserves schema; changed state type and malformed state are refused.

## Task 2: Driver and component exports

**Files:** `crates/ui-lang-guest/src/snapshot.rs`, `lib.rs`, `wit/view.wit`.

**Interfaces:**
```rust
pub trait SnapshotApp: App {
    fn snapshot(&self) -> Result<Vec<u8>, String>;
    fn restore(bytes: &[u8]) -> Result<Self, String>;
}
// Driver<A: SnapshotApp>
fn snapshot(&self) -> Result<Vec<u8>, String>;
fn from_snapshot(bytes: &[u8], macos: bool) -> Result<Self, String>;
```
```wit
export snapshot: func() -> result<list<u8>, string>;
export restore: func(state: list<u8>, macos: bool) -> result<_, string>;
```

- [x] Driver snapshot rejects `busy`, live one-shot tasks and deferred messages. Restore initializes the host platform before state decoding and builds an empty tracker/task queue without boot.
- [x] Export native helpers and WIT methods through `export_app!`; install the panic hook for either initialization path. Build a candidate before assigning the driver so errors preserve the old instance.
- [x] Test boot side-effect counts, host-platform initialization, rejected busy snapshots and failed restoration preserving the existing state.

## Task 3: Actual wasm evidence and delivery

**Files:** a focused app-store snapshot fixture and its host tests, CI, README/SPEC/COVERAGE.

- [x] Native pointer/key events edit a root draft, retained component and mounted component; snapshot and restore into a fresh wasm instance. Assert all values survive and newly generated routes remain callable.
- [x] Observe boot host requests before and after replacement; restoration must emit none. Remove/reinsert a mounted component afterward and assert it boots once normally.
- [x] Keep a one-shot host request pending: snapshot must fail. Complete it, snapshot successfully, then reject a corrupted/type-incompatible snapshot without damaging the old instance.
- [x] Mutate one serialized value and independently remove restored boot markers; intended value/boot-request assertions must fail, then pass after exact restoration.
- [ ] Run focused tests, workspace checks, lint/format, real `cargo ice bundle` and the ignored wasm host tests. Review the complete diff, push one focused PR, inspect exact-head CI, merge only all-green, archive evidence and remove the worktree.

## Follow-up: Host replacement

Retain the host window/native state and consented artifact boundary. Watch catalog changes, stage a candidate without boot or host effects, validate the snapshot, then atomically swap and cancel old requests/subscriptions. Discard stale route/reply queues. Keep the old instance on failure. Prove this with two different wasm builds and native draft/focus/scroll behavior, not just same-instance restore.

## Evidence so far

- Wire limits/trailing bytes: 2 tests green, trailing-byte acceptance mutation Red then restored Green.
- Mounted boot markers, Driver boot side effects and schema shape tests: each intended mutation Red then restored Green.
- Generated Tree regression tests: 36 passed. Initial component fixture Rust check and actual wasm bundle passed.
- Actual wasm snapshot tests: native draft + retained/mounted state + fresh routes; pending task refusal and boot request suppression/remount; saved initial state for untouched retained scopes. All 5 passed after fixing saved-initial fallback/insertion and adding owned type coverage plus initializer side-effect observation. The untouched-scope test first failed the expected 0-vs-55 assertion in wasm.
- Independent review found the untouched initial-state gap; follow-up cleared the fix. Actual owned type coverage is green. Final gates, PR and complete feature review remain.
- Any live Task (including a long-running Task stream) blocks snapshots; Tracker subscriptions restart from restored state. Catalog watching/transactional host replacement remains a separate follow-up.

Latest local verification: related core/guest/runtime/wire Rust suites 1,513 passed, 0 failed (77 ignored), actual bundled snapshot tests 3 passed, root Ice formatting 143 files and app-store Ice formatting 34 files clean. Latest wasm includes saved-initial fixes and qualified standard type paths. This does not replace the pending full-workspace gates and wider codec type evidence.

Final wasm evidence: five tests pass. Replacing byte encoding with an empty vector fails the Bytes([]) versus Bytes([0, 255, 164]) assertion; inserting __boot into restore fails the source initializer counter (1 versus 0). Exact source restoration and a fresh bundle return all five tests to Green. Post-rebase cargo check --workspace --tests passed at 33bbb6f0.

Final Driver review caught Tracker runner futures sharing the Task pool. Task entries now distinguish subscription runners, so settled subscriptions can snapshot while normal tasks still block. Active keyboard subscription regression failed the snapshot assertion before the fix, then all 27 guest tests passed; restored keyboard events update only the replacement.

Delivery gates: workspace tests 3,374 passed (141 ignored); app-store workspace tests 42 passed (53 ignored); reviewed actual component wasm tests 7 passed, including all 5 snapshot cases. Workspace all-target Clippy and app-store workspace/tests Clippy passed with -D warnings. Root/app Ice formatting passed. Guest 27-test rerun covers the later subscription fix. PR/CI/merge remain pending.

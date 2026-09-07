# Guest Widget Requests Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan inline, with independent review before delivery. Steps use checkbox syntax for tracking.

**Goal:** Execute Ice focus, input selection/cursor and scroll commands inside the requesting mounted wasm view, preserving focused-query replies and cancellation.

**Architecture:** Lower the existing checked Ice statements to typed requests on the existing guest host channel. The host executes Iced operations against the requesting GuestView's child tree, never the desktop root. Queue requests until the frame that created them is mounted. Run that prior queue before the next guest tick so a busy timer cannot supersede it; process the new tick's cancellations before its commands. Reject stale requests and include operation work in existing host budgets.

**Tech Stack:** Rust, existing Ice codegen, ui-lang-wire bincode/serde, ui-lang-guest Tasks, Iced widget operations, app-store wasmtime host.

**Spec:** `examples/app-store/module-views.md`, “Effects”, “Mount contract” and Runtime alongside 3–4; existing native widget statement semantics in `crates/ui-lang-core/src/codegen/statement.rs`.

## Global Constraints

- Do not modify ducktape. Its chat/page/agent views supply the requirements only.
- No new dependency packages or syntax (the host directly declares the already-used iced_test crate for its tests); arbitrary operation serialization or process-global widget traversal.
- Existing checked widget paths retain their component/key qualification.
- Widget-local operations require no external-device capability; use the existing reserved `host` request namespace.
- Existing native generation remains unchanged. Unsupported tree selector/virtual-row operations must produce E190 instead of silently dropping Tasks.
- Request count, reply bytes, cancellation, fault and elapsed-time governors apply before native work; unmount drops pending work.
- All code changes live in `.worktree/widget-requests` on `agent/widget-requests`.
- Run `cargo check --workspace --tests` before any `git rebase --continue` and build a real wasm guest before merging.

## One delivery: scoped commands and actual guest execution

Files:
- Add `crates/ui-lang-wire/src/widget.rs`: serialized command and reply contract, checked finite offsets and bounded identifiers/indices.
- Add `crates/ui-lang-guest/src/widget.rs`: `Task` adapters over `host::request("host.widget", ...)`; canceled futures use existing request Drop cancellation.
- Modify `crates/ui-lang-core/src/codegen/statement.rs`: share widget path construction; native Id construction versus owned string for tree requests.
- Add host operation execution beside `examples/app-store/host/src/guest_view.rs`; integrate queue/cancellation/budgets in `host/src/store.rs` and `host/src/capabilities.rs` as appropriate after tracing dispatch.
- Add an actual guest fixture alongside existing app-store test guests and mounted headless host tests.
- Update SPEC.md, README.md, COVERAGE.md and the module-view phase ledger with supported operations and evidence.

- [x] Read the complete statement and target lowering, host request dispatch, cancellation, redraw, quiet/wake, and GuestView rebuild paths before editing.
- [x] Define the wire command variants using the existing native names:

```rust
pub enum WidgetCommand {
    FocusPrevious,
    FocusNext,
    Focus { target: String },
    Focused { target: String },
    CursorFront { target: String },
    CursorEnd { target: String },
    Cursor { target: String, position: u32 },
    SelectAll { target: String },
    Select { target: String, start: u32, end: u32 },
    Snap { target: String, x: f32, y: f32 },
    SnapEnd { target: String },
    ScrollTo { target: String, x: f32, y: f32 },
    ScrollBy { target: String, x: f32, y: f32 },
}
```

Use exact target strings without truncation: over-budget identifiers are rejected, never redirected. Reject nonfinite offsets, clamp relative offsets to [0, 1], and preserve signed absolute scrolling. Convert checked nonnegative cursor values without wasm/native usize disagreement. Focused replies are bool; mutation replies are unit.

- [x] Add a production-path fixture: two inputs, a scroll region taller than its viewport, buttons invoking focus/query/cursor/select/scroll handlers, and an on-mount focus request. Show the focused-query reply in guest text.
- [x] Render two independent guests with identical local widget keys. Click a guest button through the headless host, then type: only its addressed input changes. Assert focused-query true/false for the two views, cursor/selection replacement text, and changed scroll geometry. A no-op host executor must fail these assertions.
- [x] Lower eligible tree statements using the path expression before native `Id` wrapping. Keep native statement branches intact. Reject Find/ScrollToKey on tree until their selector/virtualization contracts are implemented; do not advertise arbitrary native Action::Widget support.
- [x] Implement guest request Tasks and host decoding, preserving query routing through `resolved_route_code`. Missing focus targets return false; missing mutation targets complete without affecting another view.
- [x] Execute bounded operations only on the child element through its native `operate` method. Run native operation finish/chain semantics with a fixed pass cap so focus-next works without an unbounded traversal loop.
- [x] Associate each queued command with the frame revision that produced it. Defer until that frame is mounted; reject superseded commands. Include the deferred queue in wake/quiet logic and cancellation. Execute within the existing redraw cost accounting, checking per-operation reply/count budgets before mutation.
- [x] Add cancellation, stale-frame and budget tests at the host boundary; wire validation covers out-of-range inputs. Fault/unmount lifecycle integration remains in the phase ledger. Verify on-mount focus executes after the first layout, not against an empty old tree.
- [x] Produce Red evidence by replacing native command execution with a unit/false reply: intended focus/text/scroll assertions fail. Restore and rerun Green. Separately remove instance scoping in a controlled two-view test and demonstrate the isolation assertion detects interference without touching external desktop widgets.
- [x] Run focused wire/guest/codegen/host tests, `cargo check --workspace --tests`, relevant clippy and `cargo ice fmt --check`. Bundle and run the actual wasm fixture. Record exact commands and Red/Green observations in COVERAGE.md.
- [ ] Commit one focused Conventional Commit, push, open a PR, review the full diff independently, resolve findings, and merge only with all required CI green. Remove the task worktree after merge.

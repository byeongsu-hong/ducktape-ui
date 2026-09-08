# Catalog-driven host replacement

Goal: apply an approved module rebuild in the existing host window without losing guest drafts, native input focus/scroll state or host-owned surfaces. A failed candidate must leave the old instance usable. Depends on guest snapshot exports in PR #982; implementation starts after rebasing onto its merged main revision.

## Existing boundaries

- `library::Installed` pins the exact module hash. Changed artifacts require the existing consent flow; catalog watching must not silently move a pin.
- `Surface(Arc<Mutex<Guest>>)` identifies the running window's instance handle. Keep this Arc and the existing `Running.window` when replacing its contents.
- `Guest::load` compiles/instantiates and calls init; replacement needs a separate no-init path. Instantiation and snapshot/restore/tick retain existing fuel, memory and epoch limits.
- `GuestView` delivers local and overlay outputs to the handle. A replacement must invalidate outputs captured by the old instance without discarding current native widget state.
- Native surfaces hold host resources. Preserve the approved host state; do not replay boot, reopen a terminal or execute staged host requests before commit.

## Replacement sequence

1. Watch the catalog by periodic asynchronous scanning using existing executor/subscription facilities. Compare entries and refresh the catalog only on change. Reading/scanning must stay off the window update path. A malformed/incomplete write never replaces a running instance.
2. The existing consent/install action handles a running app by preparing a replacement instead of returning early or opening a second window. Pin the candidate hash only after a successful swap. Failure keeps the previous pin and status explains the error.
3. Compile and instantiate the candidate outside the running guest lock, without init or host effects. Capture the current guest only after pending UI events and ordinary tasks settle. Persistent subscriptions may restart.
4. Record old instance identity and tick count with the snapshot. Restore the candidate and obtain its first complete frame without dispatching its requests. Validate/decode/sanitize that frame before touching the live handle. Preserve its cancels alongside requests.
5. Re-lock the running handle and reject a stale candidate if the instance changed, new guest ticks ran, or UI work is pending. Never restore an old draft over edits made while compilation/restoration was running. A retry captures current state. Commit also validates the current Running window, Surface and consent/install request token; close or uninstall invalidates an outstanding replacement.
6. Atomically swap the verified candidate into the same Arc. Transfer current native Inputs and approved host surface resources, reconcile them against the new tree, and advance the frame revision and instance generation. Retire the old subscriptions/request queues. Reconcile transferred resources with the candidate manifest: removed terminal capability removes its provider, and newly approved resources are created only after commit. Apply the staged frame in the existing requests → cancels → clipboard/widgets order after commit.
7. `GuestView` local and overlay output paths capture the instance generation and refuse stale-generation outputs. Guard direct keyboard pending writes and nested keyboard overlays as well as Output delivery. Old replies/widget/clipboard work cannot route into the new guest. Window identity and keyed native widget state remain stable.

## Verification

- [x] Catalog watcher detects add/change/remove, ignores unchanged scans, and does not change consent pins.
- [x] Two separately built wasm artifacts share a compatible state schema but show distinct version labels. Bundle both in CI; never substitute a renamed identical binary.
- [x] In an actual mounted host, type a draft, focus its input, scroll a long region, then replace. Assert new version label, preserved draft/focus/native offset, stable Surface Arc and same window id.
- [x] Restoring does not replay source initializer or boot effects; subscriptions restart once and old request/reply routes are discarded.
- [x] Wrong hash, incompatible schema, pending writes and changed old state during staging each leave the old guest usable and the pin unchanged.
- [x] Close, uninstall or supersede consent while staging; no obsolete swap or pin resurrection occurs. Removed capabilities cannot retain old providers; same-tick subscription creation/cancellation leaves no subscription.
- [x] Drive stale native and captured overlay keyboard events after swap; they must not mutate the candidate. Current-generation interactions still work.
- [x] Verify intended assertion failures with minimal behavior mutations, then restore and rerun actual wasm tests.
- [x] Run app-store workspace tests, CI Rust 1.98 Clippy and Rust/Ice formatting.
- [ ] Root independent full diff review and exact-head CI before merging.

This is host implementation, not a Ducktape source change. Ducktape module owners continue to register host surfaces and route their events into intents. The overall module-porting goal remains open while view/runtime blockers or packaging requirements remain.


## Implementation evidence

Implemented on merged snapshot, shadow and scroll main (`9ba67c16`). The host
uses asynchronous one-second polling and explicit hash consent, stages a separate
instance, and commits on the UI thread. New install/launch completions carry a
serial; startup restoration independently checks the current pin so multiple
remembered apps remain supported without reviving removed consent.

The two versioned fixture artifacts and component fixture were rebuilt on that
base. The seven `bundled_reload_` tests cover mounted state/native preservation,
wrong-hash/schema causes, stale completion/input refusal, terminal provider removal
and staged request/cancel ordering. The overlay test uses a capturing native
overlay with actual wasm instances; the staged frame test supplies valid host
requests at the raw-frame boundary. Neither claims OS window-manager automation.
Catalog scanning, the install serial guard, native/overlay keyboard guards,
provider removal, failure reasons and request/cancel ordering each have intended
assertion failures under temporary production mutations followed by restoration.
The startup guard additionally rejects an uninstalled pin under mutation.

Final delivery requires the app workspace tests, CI-toolchain Clippy, Rust/Ice
formatting and root's independent PR review/CI assessment; root owns merging.

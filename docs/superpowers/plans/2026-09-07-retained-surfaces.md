# Retained host surfaces implementation plan

> Execute inline using superpowers:executing-plans; independent review before delivery.

**Goal:** Give each guest an owned surface registry and prove that a retained native log view has an instance-scoped view lifetime independent from its host session.

**Architecture:** Keep the existing copied SurfaceValue arguments/events. Construct registries per Guest, with closures binding authorized host sessions. The example's `session_log` surface uses the real runtime LogTimelineState and virtual list. A native lazy widget cache retains each mounted view's selection/scroll state; dropping the mount releases that view, while a host-held Arc keeps the session alive. Registry identity participates in the cache key so replacing a guest cannot inherit another guest's retained widget even at identical node keys. No guest-provided pointer or process-global resource lookup.

**Scope:** Phase 3b's session/view ownership boundary, demonstrated through an actual wasm guest. Rich editor semantic actions and terminal providers remain explicit later work; this does not claim their parity. Ducktape is read-only.

- [x] Change app-store registries from a global OnceLock to Guest-owned providers.
- [x] Add a bounded host log session and native retained timeline surface. Host request names provide the running example's log data; native selection/scroll/resume events return a typed notice record.
- [x] Add a wasm fixture with mount/unmount controls and visible notice state.
- [x] Verify real native interaction, independent instances, replacement with identical node keys, view cleanup, host-session survival and updates while unmounted. Mutate retention/isolation to prove assertions fail.
- [x] Batch implementation, then tests, actual wasm bundle, relevant checks and full independent review.
- [ ] Update the module-view phase ledger and support/evidence docs. Commit, push, PR, green CI, squash merge, cleanup.

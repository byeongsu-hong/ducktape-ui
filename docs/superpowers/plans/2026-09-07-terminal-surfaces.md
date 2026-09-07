# Terminal surface and host-owned session

Execute inline and batch implementation before tests. Ducktape remains read-only.

Reuse ui-lang-components' actual alacritty/PTTY engine and native terminal_surface. The application host chooses a resolved program; guest arguments must never choose executable paths or inject terminal input. A configured terminal capability binds that host session to one Guest-owned registry. Native user input/clipboard remain in the terminal; typed title/running notices use the guest's existing capability stream. Unmounting the widget does not cancel the host session or its event processing.

The engine currently exposes only a Tokio-timed Subscription. Add bounded nonblocking event polling for hosts that already own a frame loop, without replacing the host's executor or starting detached async workers. Poll and Subscription are alternative consumers for a session; never run both. Exit is sticky and captures the final frame.

- [x] Add bounded native session polling and real PTY tests, including final output and exit state.
- [x] Bind a host-configured session to the app-store registry and poll it from the guest host loop even when the terminal node is hidden. Keep native session ownership distinct from the widget lease.
- [x] Add an actual wasm terminal fixture and native PTY interaction/lifecycle tests, with bounded waits and observable output/status assertions. No fake terminal or idle placeholder counts as evidence.
- [x] Verify instance isolation, unmount/background output/remount, stream cancellation, exit/cleanup, keyboard and display. Prove regression assertions with deliberate behavior mutations.
- [ ] Update the phase ledger and support claims; batch bundle/tests/checks/Clippy/formatting, independent review, focused PR, green CI, merge and cleanup.

Local evidence: native terminal 36, vendored engine 132, actual wasm 4 pass;
workspace check/tests, host Clippy, Rust/Ice formatting pass. Deliberate
mutations reach final-output, sticky-exit, batch-limit, focus-cleanup and ANSI
pixel assertions. Initial/latest notice ordering has pre-fix assertion evidence.

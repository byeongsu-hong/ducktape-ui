# Hosted environment and own-window effects

Tree's authored `task system theme` and `subscribe system theme` use the host's
OS theme source, preserving native `none`, `light`, and `dark`. This differs
from `host.theme`, which follows the store's selected application palette.
Typed replies use the existing request/stream transport. Before the host has
an OS answer, bounded requests wait; cancellation removes them. A replacement
inherits the current host fact and recreates only live guest subscriptions.
Errors are logged explicitly, never mapped to an invented light-mode success.

Own-window maximize, minimize and resizable commands join the existing
WindowEffects queue. Preparation, installation-generation checks, cancellation
and submission acknowledgement remain the same as focus/resize/close. No native
window identifier crosses the wire. Acknowledgement means native dispatch,
not proof the operating system applied the change. Added WindowCommand variants
advance the strict wire epoch to 6; the Pages epoch-4 pin remains separate.

Other unsupported authored Tree window operations and system information,
font-load and image-allocation effects fail compilation explicitly.
Raw Rust tasks remain subject to the guest runtime's existing explicit dropped-
action diagnostic. No generic callback transport or compatibility layer is added.

Acceptance uses one actual native/Wasm guest fixture for initial OS mode, updates,
unsubscribe and replacement, with application palette deliberately different.
Mounted buttons must produce exactly scoped native window Action variants and
stale requests must not dispatch. Platform application of these actions is not
claimed by an in-process host test. Only these affected checks and meaningful
regression mutations run; no broad CI waiting.

## Verification

`cargo test --manifest-path examples/app-store/Cargo.toml -p app-store-host
--bin app-store-host bundled_environment_ -- --ignored --nocapture` runs two
mounted tests, each against the actual native executable and Wasm component.
They pass OS query, mode changes, unsubscribe, no-init replacement and scoped
maximize/minimize/resizable dispatch. Replacing the None reply with Light fails
the mode assertion; replacing maximize dispatch with minimize fails the native
action assertion. Exact restoration passes both tests again.

The 32-listener boundary has a separate off-by-one mutation regression.
Core lowering refuses an unhosted window move; removing that refusal fails its
E190 assertion. Wire mode decoding retains None and rejects malformed replies.
These checks exercise host routing, not a window manager applying an operation.

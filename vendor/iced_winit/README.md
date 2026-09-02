# iced_winit 0.14.0, patched

The published `iced_winit 0.14.0` source with one line changed in the event
loop's control-flow merge: a `WaitUntil` deadline already in the past is no
longer kept in favour of a later one. Kept, it makes winit wake with a zero
timeout on every iteration until the later deadline passes — a program with
two or more windows that each ask to be redrawn at a time (a daemon with a
clock in one window and an animation in another) spins a whole core. Upstream
master carries the same code as of 2026-09.

Wired in through `[patch.crates-io]` in the workspace `Cargo.toml` and in
`examples/app-store/Cargo.toml`. Delete this directory and both patch entries
once a release carries the fix.

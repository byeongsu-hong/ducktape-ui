# Ice Terminal example

This example embeds a native terminal emulator behind Ice's typed Rust
boundary. It can launch the local shell, OpenSSH, Claude Code, or Codex in a
real PTY with ANSI colors, alternate-screen applications, keyboard and mouse
input, selection, clipboard bindings, scrolling, and resize propagation.

```bash
cargo run -p terminal-example
```

`ssh`, `claude`, and `codex` are discovered on `PATH`. The SSH field accepts
either `user@host` or a quoted command such as
`ssh -p 2222 "user@host"`; arguments are parsed and passed directly without a
shell. Claude Code and Codex inherit the parent process environment, so their
normal authentication and configuration continue to apply.

The launcher uses the same warm graphite palette for native form controls even
when the system theme is light. Once a session starts, the launcher disappears
and the terminal owns the complete surface below the compact session bar:

![Terminal launcher controls](screenshots/launcher_controls_light.png)

The Ice layer owns the launcher, session bar, and session status.
`src/terminal.rs` uses `alacritty_terminal` directly for the PTY, VT parser,
grid, terminal modes, and selection, then exposes a native renderer and an
event subscription through Ice's typed extern boundary. No PTY handles or
terminal protocol bytes cross into Ice state.

The renderer snapshots only the visible grid, batches adjacent cells into text
and color runs, and coalesces PTY wakeups to one update per frame. Resize
notifications are sent only when the computed row, column, or cell dimensions
actually change. This avoids redraw/resize feedback in bursty alternate-screen
clients such as Claude Code while keeping shell, SSH, tmux, and Codex behavior
on the same terminal core.

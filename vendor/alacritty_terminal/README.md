# alacritty_terminal 0.26.0, patched

Published crate source and Apache-2.0 license. The large upstream integration
terminal recordings and their `ref` test target are omitted; source unit tests
remain. Native PTY regressions live in ui-lang-components.

`event_loop::pty_read` flushes already-read bytes before reporting a read error
or EOF. Previously, if the terminal lock was busy, a read could buffer output,
then receive Linux EIO at child exit and return before parsing those bytes.
The final process output was lost even with `drain_on_exit` enabled.

The exit path also flushes a pending synchronized update before marking the
terminal exited, so an unterminated DEC synchronized update preserves its output.

The workspace and app-store workspace select this source with Cargo patches.
Cargo patches do not propagate through published library dependencies; other
embedding workspaces must select this patched source as well.

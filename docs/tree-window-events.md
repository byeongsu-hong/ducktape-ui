# Hosted window and IME observations

The hosted Tree native process and Wasm component share opt-in, copied window
lifecycle and input-method observations. Existing Ice window/input-method and
generic event subscription syntax uses the existing Iced subscription tracker.
These observations never replay native widget input or grant task permissions.
Wire epoch 5 is a separate host/guest pin from editor presentation epoch 4.

Supported first scope: focus/unfocus, close-requested/closed, file-hovered,
file-dropped/files-hovered-left, and input-method opened/preedit/commit/closed.
Payloads retain captured/ignored status. Paths and composition text are bounded,
not silently truncated; preedit ranges must be valid UTF-8 byte boundaries.
File paths identify a user drop and do not grant a guest filesystem access.
Window global position, frame clocks and additional system effects remain gaps.

The host forwards a normal event after the mounted widget handles it. An overlay
forwards only captured events, which never reach the base widget. Forwarding
rechecks the instance token under the guest lock; a replacement cannot receive
an old instance's event. Interest is recomputed with live subscriptions, so a
removed subscription stops host forwarding. Rebuilding a guest does not replay
old observations.

Closed windows cannot redraw. The app-store's existing window-closed handler
therefore retires the instance and offers one final bounded backend tick.
That result is observed but its outgoing requests are not executed: close cannot
reopen a window or become a persistence guarantee. Native close is already
complete and neither guest work nor an error vetoes it. Close-requested is an
observation, not a close-cancellation API.

Evidence uses actual native/Wasm fixtures and real mounted Iced event routes:
focus/loss, dropped path, IME preedit/commit with exactly one editor update and
one observation, captured overlay delivery, unsubscribe, replacement rejection,
and the host's terminal close boundary. These are host integration tests, not
claims about Finder, Windows drag/drop or platform IME dispatch.

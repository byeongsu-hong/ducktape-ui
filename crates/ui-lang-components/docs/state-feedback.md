# State feedback and request ownership

Use the default `Alert` family for a notice alongside the current task and
`EmptyState` for an empty content area. Both accept `description=""`; omitted
or empty descriptions allocate no text row or gap. EmptyState centers each
wrapped line, while Alert keeps its copy aligned beside its icon. Long words
can break at glyph boundaries inside the available width.

Keep the useful action outside a fill-height empty body, inside the same
bounded column. The [native example](../../../examples/showcase/tests/cases/ui/state_feedback.ice)
uses Page's default inset, the four semantic Alert variants and a reachable
Create project button. Activating it opens the new-project name field, restores
keyboard focus there and accepts typed input. Components retain their public geometry overrides;
no optional slot or new state-management syntax is required.

The [music app](../../../examples/apple-music/src/ui/handlers/app.ice) demonstrates
independent home, search and authentication requests:

- The search field stays editable during a request. The submitted query labels
  the pending/result view; typing another draft does not relabel older results.
- `run latest lane=search` rejects an older completion after a newer request.
  It does not stop the old future or undo its side effects.
- Authentication owns its busy flag. An unrelated search does not disable
  sign-in, and signing out invalidates a pending authentication completion.
- Each failure clears its own busy flag. Starting a new search or sign-in
  clears the previous displayed error. Pending and failed searches do not also
  display the empty-result presentation.

The [Markdown editor](../../../examples/markdown-editor/src/ui/handlers/app.ice)
already supplies the save policy. A save snapshots the document and revision;
while it is pending, another submit is guarded and the editor remains usable.
Completion marks only the written revision saved. New edits remain dirty and
are persisted by a later save. A filesystem error retains the document, clears
busy state and supplies error feedback; retry clears the old error.

Its request-owner tests execute the real file-writing future in a scratch
library and defer delivery of its completion until after further edits. They
compare both the edited buffer and actual disk contents. The failure case
removes the scratch parent directory, then restores it for a successful retry.
The native pending-save preset separately verifies actual typing and visible
Saving feedback. It represents an in-flight request and does not itself start
or claim to complete a disk write.

These are native UI and application ownership contracts. The music backend is
a deterministic example API; it is not evidence for production network or
platform authentication. The Markdown checks exercise real local persistence.

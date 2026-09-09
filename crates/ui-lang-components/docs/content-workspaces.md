# Content-heavy workspaces

Choose the scrolling owner from the content. A growing Markdown response has
different requirements from a fixed-height build log.

| Content | Composition | Overflow and retained work |
| --- | --- | --- |
| Prose, Markdown, or a document editor | A centered readable column inside a bounded workspace | Wrap prose, give code its own horizontal scroll, and let the document/editor own vertical scrolling. |
| A table with many fixed-height rows | `DataGrid.Frame` around the typed data-grid boundary | The grid owns both axes. Keep row/column keys, sort order, and cell drafts in the caller's model. |
| A fixed-height hierarchy | `TreeView.Frame` around the typed tree boundary | The tree owns vertical scrolling. Reconcile stable preorder keys; retain expansion, selection, and the rename draft. |
| An append-only fixed-height log | `LogTimeline.Frame` around the typed log boundary | Pause tail following when reading history, count unread appends, and provide an explicit Resume tail action. |

## Readable width and media

Give the outer box `w=fill align-x=center`, then put the content in an inner
`box w=fill max-w=...`. The cap is a caller choice: the AI chat uses 760 logical
pixels, the Markdown editor uses 880, and the media fixture uses 720 with a
480-pixel customization. The surrounding workspace can fill a wide window
without stretching every line of prose across it.

Use word wrapping with glyph fallback for long unbroken prose and link labels.
The AI chat's selectable text uses Iced's `Wrapping::WordOrGlyph`. Its code
blocks instead sit inside a horizontal scrollable, where each code line can
keep its full width. Copying code retains its original line breaks. A clip
alone would hide the end of the line without providing a way to read it.

Size media independently from its intrinsic dimensions. For example:

```ice
image cover
  with
    w=fill
    h=160.0
    fit=cover
    r=12.0
    label="Album artwork"
```

`cover` scales and crops the source into that 160-pixel surface. The software
renderer intersects the image's clip with its parent layer and applies raster
corner radii to the logical image surface. The taller scaled source must not
paint over the heading, editor, or actions around it. An additional clipping
container is unnecessary for this image contract.

Literal relative raster assets in native `image`, `viewer`, and canvas image
commands retain one handle per call site. Rebuilding a view therefore reuses
the renderer's decoded-image cache. For downloaded or generated images, keep
the typed image handle in caller state; repeatedly constructing an encoded
handle creates a new native identity.

Markdown image references and loaded image widgets are separate contracts.
The AI chat's current Markdown viewer renders an image reference as its alt
text; it does not fetch remote images. Use a bounded image widget for media the
application has actually loaded. The Markdown editor edits the source and
keeps its native editor in a bounded page; its existing last-line test verifies
that a long document remains reachable.

## Reading while a response arrives

The AI chat keeps a `following` flag from the native scroll viewport. A reader
at the bottom follows streamed additions; scrolling into history pauses that
behavior. The Latest button resumes it. Background row updates and streamed
text do not unconditionally snap a reader back to the bottom.

Its transcript uses a top-relative offset so content appended below an already
visible history row leaves that row in place. Existing keyed rows and lazy
Markdown ownership keep settled content stable while the live reply grows.
This is an append-at-the-bottom policy. It does not claim that an ordinary
scrollable can restore an arbitrary deleted or prepended variable-height row.
For those operations, use the measured
[MessageScroller](../README.md#reading-position-in-a-changing-transcript)
boundary and stable message IDs; its native reading-anchor tests cover prepend,
capped replacement, and deletion
of the visible anchor.

The chat's `virtual-row` boundary limits layout and paint, not construction of
the whole outer loop. Settled Markdown is parsed once per lazy row; an evicted
row can be parsed again. See the [AI chat performance notes](../../../examples/ai-chat/README.md)
for the measured stream, row-cache, and 500-row history limits.

## Empty, selected, and editing data

The [native workspace fixture](../../../examples/showcase/tests/cases/ui/content_workspace.ice)
shows all three retained data surfaces with the same surrounding navigation,
bounded stage, and caller-selected maximum width. Empty data gets an
`EmptyState` with a concrete Load sample action. The caller updates each frame's
count from the model, rather than leaving a hard-coded count beside an empty
widget. Replacing the empty state mounts the corresponding retained widget.

Reconcile an append into the existing state. Constructing a new grid or tree
state for each update loses its active cell, expansion, selection, and editor
identity. The sample keeps grid drafts and committed cell values separately;
the tree keeps its rename draft in retained tree state. Native tests append
while each editor has focus, type another character, and commit through Enter.
That proves the draft and the active input survive the update together.

The grid's F2/Enter and tree's Rename selected action mount native inputs.
After commit or cancel, restore focus to the owning grid or tree so arrow keys
resume navigation. These are in-memory edits in the sample. A product must
separately handle durable-save success, failure, and retry.

For logs, keep the selected historical row visible while appending, show the
unread count, and let Resume tail clear it. Do not reuse this policy for a
Markdown block that changes height. Conversely, do not mount a 100,000-line
log as one variable-height transcript.

## Virtualization and performance boundaries

The workspace fixture uses 24 rows so the complete interaction is easy to
inspect. The retained owners have separate 100,000-row contracts:

- [DataGrid](data-grid.md) mounts visible rows plus overscan and **all fixed
  columns** for those rows. It does not virtualize columns or support variable
  row heights, resizable columns, ranges, or frozen columns.
- [TreeView](tree-view.md) accepts caller-owned flattened preorder nodes with
  fixed row height. The caller owns hierarchy loading and filesystem work.
- [LogTimeline](log-timeline.md) validates append-only history. Use `replace`
  for an intentional discontinuity; use the documented trimmed reconciliation
  boundary when dropping a known prefix.

Give these owners a bounded height outside another vertical scrollable. The
grid additionally owns horizontal scrolling. Their release contracts measure
unchanged rendering and reconciliation separately; the scalar retained-snapshot
and scroll reducers also enforce allocation budgets. A small fixture's frame
capture is useful for spotting composition regressions, and does not replace
those large-data contracts.

The companion [media fixture](../../../examples/showcase/tests/cases/ui/media_workspace.ice)
checks a fixed-height cover, a bounded editor, width customization, and typing
after a resize. Its captures pair with renderer pixel assertions for raster
and SVG clipping; layout bounds alone cannot prove that paint stays inside an
image.

## Inspected native examples

The [capture gallery](../../../examples/showcase/screenshots/content-workspaces/README.md)
records each root, preset, viewport and input sequence, alongside the native
frame measurements and the separate twelve-cover software raster probe.

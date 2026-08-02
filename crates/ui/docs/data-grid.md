# DataGrid v1

`DataGrid` is a retained runtime primitive for large, fixed-row tabular data.
It builds on the `VirtualList` row engine and is separate from the small,
non-virtualized `Table` and `DataTableState` helpers. It does not add Ice Core
syntax.

The public path has three layers:

1. `ui-lang-runtime` owns keyed row reconciliation, fixed-pixel column
   geometry, the active cell and selected row, keyboard navigation, native
   two-axis scrolling, headless inspection, and AccessKit grid semantics.
2. `ducktape-ui::ui::data_grid` re-exports those typed contracts and adds
   Ducktape header, cell, selection, focus, and editing styles.
3. An Ice application owns its rows, sort policy, edit values, and native cell
   editors behind a typed `extern component`. `DataGrid.Frame` composes a title,
   counts, and the bounded native slot.

```rust
let config = DataGridConfig::new(28.0, 32.0)?.overscan(3);
let columns = [
    DataGridColumn::new(Column::Name, "Name", 184.0)
        .sortable(true)
        .editable(true),
    DataGridColumn::new(Column::Status, "Status", 104.0)
        .sortable(true),
];
let mut state = DataGridState::new(DataGridId::new("repositories"));
state.reconcile(&rows, |row| row.id, &columns, config)?;
```

Row and column keys must implement `Clone + Eq + Hash` and be unique. A
successful `reconcile` atomically publishes both key indexes. Stable row
identity, selection, and the active cell follow their typed keys across caller
reordering; deleting the active row clears selection. `scroll_to_cell` resolves
both axes through constant-time key indexes. It does not scan 100,000 rows.

`DataGridId` is explicit. Use `state.selector()`, `row_selector`,
`header_selector`, and `cell_selector` for exact headless targets; selector
strings are canonical and callers should not reconstruct them. Logical names
must be unique among concurrently mounted grids. State is intentionally not
`Clone`: `update_snapshot` replaces the same value-oriented reducer mount,
while `fork` requires a new logical name and allocates independent native and
semantic namespaces.

Only visible rows plus overscan are materialized. Every fixed-width column is
mounted for each mounted row; v1 horizontally scrolls that complete column set
instead of virtualizing columns. `inspect` reports logical row and column
counts, visible and mounted ranges, mounted row and cell counts, the active and
editing cells, viewport size, and both scroll offsets. The grid owns both
native scroll axes and requires a bounded width and height outside a scrolling
ancestor.

Clicking a cell makes it the single active cell and selects its row. Arrow keys
move by cell, Home/End move within a row, Ctrl/Cmd+Home and Ctrl/Cmd+End move to
the grid bounds, and PageUp/PageDown move by the measured viewport. Navigation
reveals the destination on both axes. Modified arrows, Tab, and unrelated
control chords are not intercepted.

Sorting is caller-owned. Activating a sortable column returns a typed
`SortRequested(column_key)` outcome; the application updates its sort model,
reorders domain rows, and reconciles them. The runtime neither stores a sort
direction nor mutates row data.

Editing is also caller-owned. F2, Enter, or a double click begins editing only
for a column declared editable. The application renders a native editor for
`editing_cell`, owns the draft and committed value, then sends `CommitEdit` or
`CancelEdit`. It focuses the editor after mounting and returns focus to
`DataGridState::focus_task` after removing it. Child controls receive events
first, so a focused text input keeps IME, editing keys, Enter submission, and
control chords; the grid does not implement a second text-input protocol.

AccessKit exposes a named `Grid`, a mounted header `Row` with `ColumnHeader`
children, and only mounted data `Row` and `Cell` nodes. It reports total row and
column counts, one-based row and column indexes, selected rows, header sort
direction supplied by the caller, and an active descendant only while the
active cell is mounted. Semantic cell IDs combine a stable row allocation with
a collision-free per-column namespace, so identity survives reorder without an
`O(rows * columns)` retained map.

Release contracts exercise 100,000 rows by 16 columns. They separately budget
unchanged mounted-window build/diff/layout/draw frames, complete row/column
reconciliation, and the constant-time `update_snapshot` plus scroll and
`scroll_to_cell` reducer path. The scalar-key reducer path allocates zero bytes.
The showcase consumes `DataGrid.Frame` through a typed extern, exercises native
keyboard editing in a first-class Ice test, and contributes a mounted-cell draw
probe to the native WGPU smoke job.

V1 deliberately excludes variable row heights, variable or user-resizable
column widths, column virtualization, multi-cell/range selection, frozen data
columns, drag reordering, clipboard fill, formulas, and new Ice syntax. Those
features need evidence from product use before extending the retained contract.

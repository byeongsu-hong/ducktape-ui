# TreeView v1

`TreeView` is a retained, fixed-row runtime primitive for large hierarchical
product surfaces. It builds on the `VirtualList` collection engine; it is not
Ice Core syntax.

The public path has three layers:

1. `ui-lang-runtime` owns keyed preorder reconciliation, expansion, lazy-load
   requests, hierarchical keyboard movement, selection, rename state, drag
   target calculation, native scrolling, inspection, and AccessKit semantics.
2. `ui_lang_components::ui::tree_view` adds Ducktape selection colors and level-based
   indentation.
3. An Ice application owns its typed data and reducer behind an `extern
   component`. `TreeView.Frame` composes the title and bounded native slot.

```rust
let config = TreeViewConfig::new(28.0)?.overscan(3).indentation(16.0);
let mut state = TreeViewState::new(TreeViewId::new("repository"));
state.reconcile(
    &nodes,
    |node| TreeViewNode {
        key: node.id,
        parent: node.parent,
        has_children: node.has_children,
        children_loaded: node.children_loaded,
    },
    config,
)?;
```

Input is caller-owned preorder data: a parent must appear before its contiguous
child subtree, must be marked `has_children`, and keys must be unique.
Reconciliation is atomic; invalid data leaves the previous tree intact.
Expansion is retained by key, selection follows a key
across reconciliation, and collapsing an ancestor moves hidden selection to
that ancestor. Expanding an unloaded branch returns `load_requested`, leaving
fetch policy and child insertion with the application.

Up, Down, Home, End, PageUp, and PageDown navigate visible rows. Right expands
a branch or enters its first child; Left collapses it or selects its parent.
Rename initiation is caller-owned: an explicit edit action sends `BeginRename`,
then the caller renders and focuses the editor when `TreeViewRow::editing()` is true, sends
`RenameChanged` as text changes, and routes submit/cancel to `CommitRename` or
`CancelRename`. After removing the editor, run `TreeViewState::focus_task` to
resume tree navigation. The tree never intercepts keys owned by another focused
control. Pointer-driven drag and drop remains an application concern;
`drag_target` deterministically classifies a visible row as `Before`, `Inside`,
or `After`.

AccessKit exposes a named `Tree` and only the mounted `TreeItem` nodes. Each
item reports stable semantic identity, one-based level and sibling position,
set size, selection, and expansion where applicable. Headless inspection
reports logical and visible node counts, visible and mounted ranges, mounted
row count, expansion count, selection, and edit target.

V1 deliberately requires finite positive fixed row height and caller-flattened
preorder data. It does not measure variable heights, recursively own data,
perform filesystem operations, or introduce `tree-for` syntax. Its parent must
give it bounded height and must not scroll it vertically.

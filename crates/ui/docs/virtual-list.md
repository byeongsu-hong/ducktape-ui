# VirtualList v1

`VirtualList` is a retained runtime primitive for large product data surfaces.
It is not Ice Core syntax and does not introduce `virtual-for`.

The public path has three layers:

1. `ui-lang-runtime` owns fixed-row range calculation, keyed reconciliation,
   focus, mouse selection, keyboard navigation, scroll commands, and AccessKit
   collection/item semantics.
2. `ducktape-ui::ui::virtual_list` re-exports those exact public state/event
   types and adds Ducktape row theming.
3. An Ice application declares its own typed `extern component`, `task`
   reducer, item type, and data source. `VirtualList.Frame` is the reusable Ice
   composition around that boundary.

```rust
let config = VirtualListConfig::new(32.0, 208.0)?.overscan(3);
state.reconcile(&items, |item| item.id, config)?;

let list = virtual_list(
    &state,
    &items,
    config,
    |item| item.id,
    |item| item.accessible_name.clone(),
    |index, item, selected| row_view(index, item, selected),
    Message::List,
    &theme,
);
```

`reconcile` is the explicit data-mutation boundary. Keys must be unique. A
selected key follows its item across reordering and selection is cleared if
that key is deleted. View/layout/draw calls never scan the complete collection:
the row callback runs only for the visible range plus overscan. The state
exposes `visible_range` and `inspect`; inspection reports the mounted range,
row count, and exact keyed-column child-slot budget.

Mouse selection focuses the list. The focused list supports Up, Down, Home,
End, PageUp, and PageDown. `scroll_to_item`, `scroll_to_key`, and `sync_scroll`
produce native Iced widget tasks. AccessKit exports a `List`, total item count,
and only mounted `ListItem` nodes with stable key-derived identities,
one-based position, set size, and selected state.

V1 intentionally requires a finite positive fixed row height. It does not
measure variable-height content, retain interactive controls inside a row,
support multiple selection, reorder items by drag, or add Ice syntax. TreeView
and DataGrid can build on this runtime contract; variable-height virtualization
needs separate measurement and anchoring evidence before admission.

# VirtualList v1

`VirtualList` is a retained runtime primitive for large product data surfaces.
It is not Ice Core syntax and does not introduce `virtual-for`.

The public path has three layers:

1. `ui-lang-runtime` owns fixed-row range calculation, keyed reconciliation,
   measured viewport geometry, focus, pointer selection, keyboard navigation,
   internal native-scroll synchronization, and AccessKit
   collection/item semantics.
2. `ducktape-ui::ui::virtual_list` re-exports those exact public state/event
   types and adds Ducktape row theming.
3. An Ice application declares its own typed `extern component`, `task`
   reducer, item type, and data source. `VirtualList.Frame` is the reusable Ice
   composition around that boundary.

```rust
let config = VirtualListConfig::new(32.0)?.overscan(3);
let mut state = VirtualListState::new(VirtualListId::new("repository-results"));
state.reconcile(&items, |item| item.id, config)?;

let list = virtual_list(
    &state,
    &items,
    config,
    "Repository results",
    |item| item.id,
    |item| item.accessible_name.clone(),
    |index, item, selected| row_view(index, item, selected),
    Message::List,
    &theme,
);
```

`reconcile` is the explicit data-mutation boundary. Keys must implement
`Clone + Eq + Hash` and be unique; owned identifiers such as `String` and
`PathBuf` do not need application-side interning. A
selected key follows its item across reordering and selection is cleared if
that key is deleted. View/layout/draw calls never scan the complete collection:
the row callback runs only for the visible range plus overscan. The state
exposes pure `visible_range(item_count, config)`, `mounted_range`, and `inspect`
queries; inspection reports both ranges, the mounted row count, and the exact
scroll-content child-slot budget. Queries and rendering therefore cannot drift
when the item count or geometry changes.

The viewport height is never duplicated in application configuration. The
widget measures its native layout and emits `ViewportChanged`; the reducer
applies that event like any other typed list event. `VirtualListId` is explicit
and has no default: its readable logical name is paired with a runtime-unique
namespace so independently mounted lists cannot alias focus, scrolling, or
AccessKit nodes. The caller must keep logical names unique among concurrently
mounted lists so driver and headless selectors resolve exactly one target.
Separate `VirtualListId::new` calls with a duplicate logical name remain safe at
the native and accessibility layers, but violate that selector contract. Neither
identity nor retained state implements `Clone`; `VirtualListState::fork` requires
a new logical name and copies data and selection into a fresh native and semantic
namespace. Value-oriented adapters that must pass state through an update
reducer use `update_snapshot` to replace that same mount; the old snapshot must
not remain mounted beside the replacement. Snapshots and forks share the
immutable per-key semantic map in constant time. A successful `reconcile` is the
only operation that atomically publishes a newly allocated complete map.

Mouse clicks and touch taps focus the named list without stealing input or
cursor semantics from interactive row content or its native scrollbar. Touch
ownership follows `FingerPressed` and `FingerLifted` positions within mounted-row
bounds and descendant capture, even when the mouse cursor is unavailable or
elsewhere. The
focused list supports Up, Down, Home, End, PageUp, and PageDown.
`scroll_to_item` and `scroll_to_key` update retained state; a private revisioned
operation synchronizes the native scrollable on the next layout, including
fresh mounts, remounts, and an absolute offset of zero. AccessKit exports a
focusable named `List`, total
item count, and only mounted `ListItem` nodes. Item node IDs come from a
collision-free retained allocation keyed through `Eq + Hash`, not from a key
hash; they keep identity across reorder and expose one-based position, set size,
and selected state. The focused list points `active-descendant` at its selected
mounted row, so keyboard navigation is announced after the revealed window is
rebuilt.

Screen readers can focus the collection and use the same list keyboard
navigation. V1 does not expose offscreen rows as AccessKit nodes and does not
offer per-row accessibility click/focus actions; row selection remains a list
keyboard or pointer interaction. This mounted-only hierarchy is intentional
until virtual accessibility child requests have a native Iced contract.

V1 intentionally requires a finite positive fixed row height. It does not
measure variable-height content, retain controls after their key leaves the
mounted overscan window, support multiple selection, reorder items by drag, or
add Ice syntax. Interactive state for keys shared by consecutive mounted
windows is retained exactly. TreeView and DataGrid can build on this runtime
contract; variable-height virtualization needs separate measurement and
anchoring evidence before admission.

The `ducktape-ui` feature enables only the renderer-side
`ui-lang-runtime/virtual-list` boundary and therefore compiles for
`wasm32-unknown-unknown`. The full native Ice headless driver remains the
runtime crate's default `test-runtime` feature. Release CI measures unchanged
100,000-row frames, a showcase-equivalent `update_snapshot` plus `Scrolled`
reducer step, and explicit 100,000-key reconciliation separately, with p50/p95
time and allocation budgets for each path. The scalar-key reducer step has a
strict zero-allocation, zero-byte contract, proving its cost does not scale with
collection size.

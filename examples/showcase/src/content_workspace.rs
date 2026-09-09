//! Small deterministic data sets around the catalog's real retained adapters.
//! Only payload reconciliation lives here; rendering, editing, and focus use
//! the same boundary as the runnable showcase.

use super::*;
pub use super::{
    DataGridEvent, DataGridState, LogTimelineEvent, LogTimelineState, TreeViewEvent, TreeViewState,
    data_grid, data_grid_apply, data_grid_focus, log_timeline, log_timeline_append,
    log_timeline_apply, log_timeline_resume, tree_view, tree_view_apply,
    tree_view_begin_selected_rename, tree_view_focus,
};

pub fn grid_initial() -> DataGridState {
    let mut state = DataGridState {
        grid: UiDataGridState::new(DataGridId::new("showcase-data-grid")),
        rows: Arc::from([]),
        columns: data_grid_columns(),
        sort: None,
        edits: Arc::new(HashMap::new()),
        draft: String::new(),
        focus_target: DataGridFocusTarget::None,
    };
    reconcile_grid(&mut state);
    state
}

fn reconcile_grid(state: &mut DataGridState) {
    state
        .grid
        .reconcile(&state.rows, |row| *row, &state.columns, data_grid_config())
        .unwrap();
}

pub fn grid_load(mut state: DataGridState) -> DataGridState {
    state.rows = (0..24).collect::<Vec<_>>().into();
    reconcile_grid(&mut state);
    state
}

pub fn grid_append(mut state: DataGridState) -> DataGridState {
    let mut rows = state.rows.to_vec();
    rows.push(rows.last().map_or(0, |last| last + 1));
    state.rows = rows.into();
    reconcile_grid(&mut state);
    state
}

pub fn grid_count(state: &DataGridState) -> i64 {
    state.rows.len() as i64
}

pub fn tree_initial() -> TreeViewState {
    let mut state = TreeViewState {
        tree: UiTreeViewState::new(TreeViewId::new("showcase-tree-view")),
        items: Arc::from([]),
        renames: Arc::new(HashMap::new()),
        focus_target: TreeFocusTarget::None,
    };
    state
        .tree
        .reconcile(&state.items, tree_node, tree_view_config())
        .unwrap();
    state
}

pub fn tree_load(mut state: TreeViewState) -> TreeViewState {
    state.items = (0..24)
        .map(|key| TreeNode {
            key,
            parent: (key != 0).then_some(0),
            has_children: key == 0,
        })
        .collect::<Vec<_>>()
        .into();
    state
        .tree
        .reconcile(&state.items, tree_node, tree_view_config())
        .unwrap();
    state
        .tree
        .apply(UiTreeViewEvent::Toggle(0), tree_view_config());
    state
}

pub fn tree_append(mut state: TreeViewState) -> TreeViewState {
    let mut items = state.items.to_vec();
    items.push(TreeNode {
        key: items.len() as u64,
        parent: Some(0),
        has_children: false,
    });
    state.items = items.into();
    state
        .tree
        .reconcile(&state.items, tree_node, tree_view_config())
        .unwrap();
    state
}

pub fn tree_count(state: &TreeViewState) -> i64 {
    state.items.len() as i64
}

pub fn log_initial() -> LogTimelineState {
    let mut state = LogTimelineState {
        timeline: UiLogTimelineState::new(VirtualListId::new("showcase-log-timeline")),
        rows: Arc::from([]),
    };
    state
        .timeline
        .replace(&state.rows, |row| *row, log_timeline_config())
        .unwrap();
    state
}

pub fn log_load(mut state: LogTimelineState) -> LogTimelineState {
    state.rows = (0..24).collect::<Vec<_>>().into();
    state
        .timeline
        .reconcile(&state.rows, |row| *row, log_timeline_config())
        .unwrap();
    state
}

pub fn log_count(state: &LogTimelineState) -> i64 {
    state.rows.len() as i64
}

ui_lang::include_app!("tests/cases/ui/content_workspace.ice");

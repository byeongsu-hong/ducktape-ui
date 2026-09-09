app ContentWorkspace
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../../assets/fonts/Geist-Bold.ttf"

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

extern crate::adapters::content_workspace
  DataGridState()
  DataGridEvent()
  TreeViewState()
  TreeViewEvent()
  LogTimelineState()
  LogTimelineEvent()
  sync grid_initial() -> DataGridState
  sync grid_load(state:DataGridState) -> DataGridState
  sync grid_append(state:DataGridState) -> DataGridState
  pure grid_count(state:&DataGridState) -> i64
  sync data_grid_apply(state:DataGridState, event:DataGridEvent) -> DataGridState
  task data_grid_focus(state:DataGridState) -> unit
  component data_grid(state:&DataGridState) -> DataGridEvent
  sync tree_initial() -> TreeViewState
  sync tree_load(state:TreeViewState) -> TreeViewState
  sync tree_append(state:TreeViewState) -> TreeViewState
  pure tree_count(state:&TreeViewState) -> i64
  sync tree_view_apply(state:TreeViewState, event:TreeViewEvent) -> TreeViewState
  sync tree_view_begin_selected_rename(state:TreeViewState) -> TreeViewState
  task tree_view_focus(state:TreeViewState) -> unit
  component tree_view(state:&TreeViewState) -> TreeViewEvent
  sync log_initial() -> LogTimelineState
  sync log_load(state:LogTimelineState) -> LogTimelineState
  pure log_count(state:&LogTimelineState) -> i64
  sync log_timeline_apply(state:LogTimelineState, event:LogTimelineEvent) -> LogTimelineState
  sync log_timeline_append(state:LogTimelineState) -> LogTimelineState
  sync log_timeline_resume(state:LogTimelineState) -> LogTimelineState
  component log_timeline(state:&LogTimelineState) -> LogTimelineEvent

enum Surface
  grid
  tree
  log

state
  surface:Surface = Surface.grid
  grid:DataGridState = grid_initial()
  tree:TreeViewState = tree_initial()
  log:LogTimelineState = log_initial()
  readable_width = 1040.0

preset customized
  state
    readable_width = 640.0

preset populated
  state
    grid = grid_load(grid)
    tree = tree_load(tree)
    log = log_load(log)

on choose_grid
  surface = Surface.grid

on choose_tree
  surface = Surface.tree

on choose_log
  surface = Surface.log

on populate
  grid = grid_load(grid)
  tree = tree_load(tree)
  log = log_load(log)

on append_grid
  grid = grid_append(grid)

on append_tree
  tree = tree_append(tree)

on append_log
  log = log_timeline_append(log)

on resume_log
  log = log_timeline_resume(log)

on grid_event(event)
  grid = data_grid_apply(grid, event)
  task data_grid_focus(grid) -> focused

on tree_event(event)
  tree = tree_view_apply(tree, event)
  task tree_view_focus(tree) -> focused

on rename
  tree = tree_view_begin_selected_rename(tree)
  task tree_view_focus(tree) -> focused

on log_event(event)
  log = log_timeline_apply(log, event)

on focused

view
  box #shell
    with
      w=fill
      h=fill
      p=16.0
      align-x=center
    box #workspace
      with
        w=fill
        h=fill
        max-w=readable_width
      col
        with
          w=fill
          h=fill
          gap=12.0
        text "Build workspace" #title @section_title
        row wrap
          with
            w=fill
            gap=8.0
            wrap-gap=8.0
          button "Data" #grid-tab checked=(surface == Surface.grid) @secondary_action -> choose_grid
          button "Files" #tree-tab -> choose_tree
            with
              checked=(surface == Surface.tree)
              @secondary_action
          button "Logs" #log-tab checked=(surface == Surface.log) @secondary_action -> choose_log
          button "Load sample" #load @outline_action -> populate
        match surface
          Surface.grid
            DataGrid.Frame #grid-frame
              with
                title="Repository data"
                description="Fixed columns scroll horizontally. Edit names with F2."
                rows=grid_count(grid)
                columns=16
              col w=fill gap=8.0
                button "Append row" #append @outline_action -> append_grid
                col #stage w=fill h=280.0
                  if grid_count(grid) == 0
                    EmptyState #empty
                      with
                        title="No repository rows"
                        description="Load sample to explore selection and editing."
                  if grid_count(grid) > 0
                    extern data_grid(grid) #grid -> grid_event _
          Surface.tree
            TreeView.Frame #tree-frame
              with
                title="Repository files"
                description="Expand folders with Right. Rename the selected file."
                count=tree_count(tree)
              col w=fill gap=8.0
                row wrap
                  with
                    w=fill
                    gap=8.0
                    wrap-gap=8.0
                  button "Append file" #append -> append_tree
                    with
                      disabled=(tree_count(tree) == 0)
                      @outline_action
                  button "Rename selected" #rename -> rename
                    with
                      disabled=(tree_count(tree) == 0)
                      @outline_action
                col #stage w=fill h=280.0
                  if tree_count(tree) == 0
                    EmptyState #empty
                      with
                        title="No repository files"
                        description="Load sample to explore hierarchy and renaming."
                  if tree_count(tree) > 0
                    extern tree_view(tree) #tree -> tree_event _
          Surface.log
            LogTimeline.Frame #log-frame
              with
                title="Build output"
                description="Pause by reading history. Resume tail explicitly."
              col w=fill gap=8.0
                row wrap
                  with
                    w=fill
                    gap=8.0
                    wrap-gap=8.0
                  button "Append log" #append @outline_action -> append_log
                  button "Resume tail" #resume @outline_action -> resume_log
                col #stage w=fill h=280.0
                  if log_count(log) == 0
                    EmptyState #empty
                      with
                        title="No build output"
                        description="Load sample or append the first log line."
                  if log_count(log) > 0
                    extern log_timeline(log) #log -> log_event _

test empty_workspace_explains_each_surface
  viewport 760 600
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target files = #shell/workspace/tree-tab
  target logs = #shell/workspace/log-tab
  target load = #shell/workspace/load
  target append_file = #shell/workspace/tree-frame/root/append
  target rename_file = #shell/workspace/tree-frame/root/rename
  expect text "No repository rows"
  click files
  expect text "No repository files"
  expect a11y append_file disabled true
  expect a11y rename_file disabled true
  click logs
  expect text "No build output"
  click load
  expect no text "No build output"
  expect text "000023"
  capture populated_log_workspace

test appending_keeps_the_native_cell_draft
  viewport 760 600
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target load = #shell/workspace/load
  target stage = #shell/workspace/grid-frame/root/stage
  click load
  click stage
  key home
  key f2
  replace "Draft survives append"
  dispatch append_grid
  expect grid_count(grid) == 25
  type "!"
  key enter
  expect text "Draft survives append!"
  capture grid_after_append_and_commit

test appending_keeps_the_native_tree_rename
  viewport 760 600
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target load = #shell/workspace/load
  target files = #shell/workspace/tree-tab
  target stage = #shell/workspace/tree-frame/root/stage
  target rename = #shell/workspace/tree-frame/root/rename
  click load
  click files
  click stage
  key home
  key arrow-right
  click rename
  replace "Renamed during append"
  dispatch append_tree
  expect tree_count(tree) == 25
  type "!"
  key enter
  expect text "Renamed during append!"
  key arrow-left
  expect text "Selected 0"
  capture tree_after_append_and_rename

test paused_logs_keep_selection_and_count_new_rows
  viewport 760 600
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target load = #shell/workspace/load
  target logs = #shell/workspace/log-tab
  target stage = #shell/workspace/log-frame/root/stage
  target append = #shell/workspace/log-frame/root/append
  target resume = #shell/workspace/log-frame/root/resume
  click load
  click logs
  click stage
  key home
  expect text "000000"
  expect text "paused · 0 unread"
  click append
  expect text "000000"
  expect text "selected"
  expect text "paused · 1 unread"
  click resume
  expect text "000024"
  expect text "following · 0 unread"
  capture logs_after_explicit_resume

test a_custom_readable_width_keeps_data_controls_reachable
  preset customized
  viewport 560 600
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target workspace = #shell/workspace
  target load = #shell/workspace/load
  target stage = #shell/workspace/grid-frame/root/stage
  expect workspace.width ~= 528.0
  click load
  expect stage.height ~= 280.0
  expect stage.visible
  click stage
  key home
  key f2
  replace "Custom-width draft"
  window resize 1120 600
  expect workspace.width ~= 640.0
  type "!"
  key enter
  expect text "Custom-width draft!"
  capture customized_data_width

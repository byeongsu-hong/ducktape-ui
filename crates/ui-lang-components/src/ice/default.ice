use "recipes.ice"
use "components.ice"
use "forms.ice"
use "data-grid.ice"
use "log-timeline.ice"
use "tree-view.ice"
use "virtual-list.ice"

theme contract AppTheme
  bg
  surface
  fg
  muted
  muted_bg
  primary
  primary_hover
  primary_fg
  secondary
  secondary_fg
  accent
  accent_fg
  brand
  brand_fg
  brand_bg
  brand_line
  danger
  danger_fg
  danger_bg
  danger_line
  danger_dot
  success
  success_fg
  success_bg
  success_line
  success_dot
  warning
  warning_fg
  warning_bg
  warning_line
  warning_dot
  avatar_bg
  avatar_fg
  border
  control_line
  input
  ring
  disabled
  disabled_fg
  glass_thin
  glass_regular
  glass_sheet
  shadow_popover
  shadow_modal
  shadow_window
  shadow_window_secondary
palette app for AppTheme
  bg         #fdfdfb
  surface    #ffffff
  fg         #2c2b27
  muted      #6b6962
  muted_bg   #f6f5f2
  primary    #26251f
  primary_hover #322f28
  primary_fg #ffffff
  secondary  #ffffff
  secondary_fg #5e5c55
  accent     #f3f2ef
  accent_fg  #3f3e39
  brand      #a05a3c
  brand_fg   #ffffff
  brand_bg   #f9f1ea
  brand_line #e7d2c4
  danger     #b8544c
  danger_fg  #ffffff
  danger_bg  #fdf4f3
  danger_line #efd6d3
  danger_dot #e0655c
  success    #5f9e74
  success_fg #151410
  success_bg #eef5f0
  success_line #cfe3d7
  success_dot #5cb45f
  warning    #a07b32
  warning_fg #151410
  warning_bg #fbf4e6
  warning_line #ecdcae
  warning_dot #e3b443
  avatar_bg  #d2d0c7
  avatar_fg  #4f4d47
  border     #e7e6e2
  control_line #e0dfd7
  input      #8a8983
  ring       #26251f
  disabled   #ecebe6
  disabled_fg #b3b1a8
  glass_thin #fdfcfa80
  glass_regular #fdfcfa9e
  glass_sheet #fdfcfadb
  shadow_popover #28262221
  shadow_modal #2826224d
  shadow_window #28262238
  shadow_window_secondary #2826221a

// Same complete semantic roles as the retained Rust DARK theme.
palette dark for AppTheme
  bg #1b1a17
  surface #1b1a17
  fg #eceae4
  muted #9f9c95
  muted_bg #151410
  primary #ecebe5
  primary_hover #dad9d2
  primary_fg #1b1a17
  secondary #26251f
  secondary_fg #eceae4
  accent #2b2a25
  accent_fg #eceae4
  brand #c87552
  brand_fg #1b1a17
  brand_bg #35231c
  brand_line #68402f
  danger #d4655a
  danger_fg #1b1a17
  danger_bg #351d1b
  danger_line #713b36
  danger_dot #d4655a
  success #6cc06f
  success_fg #1b1a17
  success_bg #182a1d
  success_line #345b3b
  success_dot #6cc06f
  warning #d3a25c
  warning_fg #1b1a17
  warning_bg #302617
  warning_line #68512c
  warning_dot #d3a25c
  avatar_bg #4d4b45
  avatar_fg #eceae4
  border #2e2d27
  control_line #4d4b45
  input #6b6a63
  ring #ecebe5
  disabled #2b2a25
  disabled_fg #6b6a63
  glass_thin #1b1a1680
  glass_regular #1b1a169e
  glass_sheet #1b1a16db
  shadow_popover #28262221
  shadow_modal #2826224d
  shadow_window #28262238
  shadow_window_secondary #2826221a

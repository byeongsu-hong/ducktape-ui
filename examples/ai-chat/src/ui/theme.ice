// The night palette for the design system's own contract.
//
// `crates/ui-lang-components/src/ice/default.ice` ships one palette, `app`, which is warm paper
// — so this is warm ink rather than the usual blue-grey: the same hues held at
// low light, so a chat that is open all evening does not change character when
// the lamp goes off. Terracotta lifts to clay, which is the one hue that has to
// stay legible against ink as well as paper, because links are drawn in it.
//
// `src/render.rs` mirrors the link and inline-code colours here, because the
// Markdown adapter builds its own style and cannot read a palette.
palette night for AppTheme
  bg         #161513
  surface    #1d1c1a
  fg         #e8e6e0
  muted      #9a978e
  muted_bg   #232220
  primary    #e8e6e0
  primary_hover #d5d2ca
  primary_fg #171614
  secondary  #232220
  secondary_fg #cfccc4
  accent     #262523
  accent_fg  #d8d5cd
  brand      #deaa80
  brand_fg   #1a1512
  brand_bg   #2a201a
  brand_line #453026
  danger     #d97b72
  danger_fg  #1a1210
  danger_bg  #2c1e1c
  danger_line #4a2f2b
  danger_dot #e08b81
  success    #79b48d
  success_fg #10160f
  success_bg #1c2620
  success_line #2f4436
  success_dot #6fc077
  warning    #d4a851
  warning_fg #191408
  warning_bg #2a2313
  warning_line #4a3d1f
  warning_dot #e3b443
  avatar_bg  #35332e
  avatar_fg  #d8d5cd
  border     #302e2b
  control_line #3a3733
  input      #77746c
  ring       #c98a63
  disabled   #24231f
  disabled_fg #5c5952
  glass_thin #16151380
  glass_regular #1615139e
  glass_sheet #161513db
  shadow_popover #00000066
  shadow_modal #00000099
  shadow_window #00000080
  shadow_window_secondary #00000040

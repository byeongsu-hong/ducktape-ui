on adapter_focused

on clicked
  clicks = clicks + 1

on accepted_changed(next)
  accepted = next

on notifications_changed(next)
  notifications = next

on volume_changed(next)
  volume = next

on density_changed(next)
  let transition = radio_apply(next)
  density = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on framework_changed(next)
  native_select_framework = some(next)

on scratch_submitted(text)
  scratch_note = text
  // The app keeps the route; the pad the key names gets its share.
  slice ScratchPad.noted(text) at "notes"

on searched_framework_changed(next)
  searched_framework = some(next)

on framework_registered(name)
  combo combobox_frameworks push name

on otp_changed(next)
  otp = next

on calendar_changed(event)
  let transition = calendar_apply(calendar, event)
  calendar = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on date_picker_changed(event)
  let transition = date_picker_apply(date_picker, event)
  date_picker = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on chart_hovered(next)
  chart_hover = next

on hover_card_changed(next)
  hover_card_open = next

on card_cancel
  card_action = "cancelled"

on card_apply
  card_action = "applied"

on navigate_home
  navigation_route = "Home"

on navigate_library
  navigation_route = "Library"

on show_components
  showcase_page = "components"

on show_retained_data
  showcase_page = "retained"

on command_changed(event)
  let transition = command_apply(command, event)
  command = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on select_changed(event)
  let transition = select_apply(select, event)
  select = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on dropdown_changed(event)
  let transition = dropdown_menu_apply(dropdown, event)
  dropdown = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on context_menu_changed(event)
  let transition = context_menu_apply(context_menu, event)
  context_menu = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on alert_dialog_changed(event)
  let transition = alert_dialog_apply(alert_dialog, event)
  alert_dialog = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on sidebar_changed(event)
  sidebar = sidebar_apply(sidebar, event)

on sonner_changed(event)
  sonner = sonner_apply(sonner, event)

on sonner_tick
  sonner = sonner_tick(sonner)

on reduced_motion_changed(next)
  reduced_motion = next
  sonner = sonner_set_reduced_motion(sonner, next)

on drawer_changed(event)
  let transition = drawer_apply(drawer, event)
  drawer = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on navigation_menu_changed(event)
  let transition = navigation_menu_apply(event)
  navigation_menu = transition.state
  navigation_route = navigation_menu_route(navigation_menu)
  task apply_focus(transition.focus) -> adapter_focused

on menubar_changed(event)
  let transition = menubar_apply(menubar, event)
  menubar = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on native_resized(next)
  native_sizes = next

on native_range_changed(next)
  native_range = next

on catalog_sort_changed
  catalog_sort = data_table_next_sort(catalog_sort)
  catalog_page = 0

on catalog_previous
  return if catalog_at_start
  catalog_page = catalog_page - 1

on catalog_next
  return if !data_table_can_next(catalog_query, catalog_page)
  catalog_page = catalog_page + 1

on catalog_page_changed(page)
  catalog_page = data_table_page(catalog_query, page - 1)

on demo_page_previous
  return if demo_page <= 1
  demo_page = demo_page - 1

on demo_page_next
  return if demo_page >= demo_page_max
  demo_page = demo_page + 1

on message_scroller_changed(event)
  let transition = message_scroller_apply(message_scroller, event)
  message_scroller = transition.state
  task message_scroller_effects(transition) -> message_scroller_changed _

on virtual_list_changed(event)
  virtual_list = virtual_list_apply(virtual_list, event)

on log_timeline_changed(event)
  log_timeline = log_timeline_apply(log_timeline, event)

on append_log
  log_timeline = log_timeline_append(log_timeline)

on resume_log_tail
  log_timeline = log_timeline_resume(log_timeline)

on tree_view_changed(event)
  tree_view = tree_view_apply(tree_view, event)
  task tree_view_focus(tree_view) -> tree_view_focused

on tree_view_focused

on data_grid_changed(event)
  data_grid = data_grid_apply(data_grid, event)
  task data_grid_focus(data_grid) -> data_grid_focused

on data_grid_focused

on begin_tree_rename
  tree_view = tree_view_begin_selected_rename(tree_view)
  task tree_view_focus(tree_view) -> tree_view_focused

on cancel_tree_rename
  tree_view = tree_view_cancel_rename(tree_view)
  task tree_view_focus(tree_view) -> tree_view_focused

on native_popover_changed(event)
  let transition = popover_apply(event)
  native_popover = transition.state
  task apply_focus(transition.focus) -> adapter_focused

on open_dialog
  dialog_open = true

on cancel_dialog
  dialog_result = "cancelled"
  dialog_open = false

on continue_dialog
  dialog_result = "continued"
  dialog_open = false

on close_dialog
  dialog_open = false

on mount
  let transcript_boot = message_scroller_bootstrap(message_scroller)
  message_scroller = transcript_boot.state
  task message_scroller_effects(transcript_boot) -> message_scroller_changed _

subscribe
  every 1s -> sonner_tick

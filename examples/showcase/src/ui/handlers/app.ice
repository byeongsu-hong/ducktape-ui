on clicked
  clicks = clicks + 1

on accepted_changed(next)
  accepted = next

on notifications_changed(next)
  notifications = next

on volume_changed(next)
  volume = next

on density_changed(next)
  task radio_apply(next) -> density_applied _

on density_applied(next)
  density = next

on framework_changed(next)
  native_select_framework = some(next)

on searched_framework_changed(next)
  searched_framework = some(next)

on otp_changed(next)
  otp = next

on calendar_changed(event)
  task calendar_apply(calendar, event) -> calendar_applied _

on calendar_applied(next)
  calendar = next

on date_picker_changed(event)
  task date_picker_apply(date_picker, event) -> date_picker_applied _

on date_picker_applied(next)
  date_picker = next

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

on command_changed(event)
  task command_apply(command, event) -> command_applied _

on command_applied(next)
  command = next

on select_changed(event)
  task select_apply(select, event) -> select_applied _

on select_applied(next)
  select = next

on dropdown_changed(event)
  task dropdown_menu_apply(dropdown, event) -> dropdown_applied _

on dropdown_applied(next)
  dropdown = next

on context_menu_changed(event)
  task context_menu_apply(context_menu, event) -> context_menu_applied _

on context_menu_applied(next)
  context_menu = next

on alert_dialog_changed(event)
  task alert_dialog_apply(alert_dialog, event) -> alert_dialog_applied _

on alert_dialog_applied(next)
  alert_dialog = next

on sidebar_changed(event)
  sidebar = sidebar_apply(sidebar, event)

on sonner_changed(event)
  sonner = sonner_apply(sonner, event)

on sonner_tick
  task sonner_tick(sonner) -> sonner_ticked _

on sonner_ticked(next)
  sonner = next

on reduced_motion_changed(next)
  reduced_motion = next
  task sonner_set_reduced_motion(sonner, next) -> sonner_reduced_motion_applied _

on sonner_reduced_motion_applied(next)
  sonner = next

on drawer_changed(event)
  task drawer_apply(drawer, event) -> drawer_applied _

on drawer_applied(next)
  drawer = next

on navigation_menu_changed(event)
  task navigation_menu_apply(event) -> navigation_menu_applied _

on navigation_menu_applied(next)
  navigation_menu = next
  navigation_route = navigation_menu_route(navigation_menu)

on menubar_changed(event)
  task menubar_apply(menubar, event) -> menubar_applied _

on menubar_applied(next)
  menubar = next

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

on message_scroller_bootstrapped(next)
  message_scroller = next

on message_scroller_changed(event)
  task message_scroller_apply(message_scroller, event) -> message_scroller_applied _

on message_scroller_applied(next)
  message_scroller = next

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
  task popover_apply(event) -> native_popover_applied _

on native_popover_applied(next)
  native_popover = next

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
  task message_scroller_bootstrap(message_scroller) -> message_scroller_bootstrapped _

subscribe
  every 1s -> sonner_tick

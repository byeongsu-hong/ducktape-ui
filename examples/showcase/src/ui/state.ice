state
  email = ""
  project_slug = ""
  clicks = 0
  accepted = false
  notifications = true
  volume = 58.0
  density = "comfortable"
  native_select_frameworks = ["Ice", "iced", "Rust"]
  native_select_framework:str? = none
  combobox_frameworks:combo[str] = ["Ice", "iced", "Rust", "wgpu"]
  searched_framework:str? = none
  textarea_notes:editor = "Default multiline editor"
  catalog_query = ""
  catalog_sort = "none"
  catalog_page = 0
  demo_page = 1
  demo_page_max = 5
  otp = ""
  reduced_motion = false
  calendar:CalendarState = calendar_state()
  date_picker:DatePickerState = date_picker_state()
  chart_hover:ChartHit? = none
  command:CommandState = command_state()
  select:SelectState = select_state()
  dropdown:DropdownMenuState = dropdown_menu_state()
  context_menu:ContextMenuState = context_menu_state()
  alert_dialog:AlertDialogState = alert_dialog_state()
  sidebar:SidebarState = sidebar_state()
  sonner:SonnerState = sonner_state()
  drawer:DrawerState = drawer_state()
  navigation_menu:NavigationMenuState = navigation_menu_state()
  menubar:MenubarState = menubar_state()
  native_sizes = [0.25, 0.5, 0.25]
  native_range = [25.0, 75.0]
  message_scroller:MessageScrollerState = message_scroller_state()
  log_timeline:LogTimelineState = log_timeline_state()
  virtual_list:VirtualListState = virtual_list_state()
  tree_view:TreeViewState = tree_view_state()
  native_popover = false
  dialog_open = false
  dialog_result = "none"
  hover_card_open = false
  card_action = ""
  navigation_route = "Components"
  toast_visible = true

derived
  catalog_at_start = catalog_page <= 0
  catalog_page_number = catalog_page + 1

preset test
  state
    email = ""
    project_slug = ""
    clicks = 0
    accepted = false
    notifications = true
    volume = 58.0
    density = "comfortable"
    catalog_query = ""
    catalog_sort = "none"
    catalog_page = 0
    demo_page = 1
    demo_page_max = 5
    reduced_motion = false
    dialog_open = false
    dialog_result = "none"
    hover_card_open = false
    card_action = ""
    navigation_route = "Components"
    toast_visible = false

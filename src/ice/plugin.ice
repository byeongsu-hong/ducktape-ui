extern ducktape_ui::ice
  AlertDialogState()
  AlertDialogEvent()
  CalendarState()
  CalendarEvent()
  ChartHit()
  CommandState()
  CommandEvent()
  ContextMenuState()
  ContextMenuEvent()
  DatePickerState()
  DatePickerEvent()
  DrawerState()
  DrawerEvent()
  NavigationMenuState()
  NavigationMenuEvent()
  MenubarState()
  MenubarEvent()
  MessageScrollerState()
  MessageScrollerEvent()
  MessageScrollerResult()
  DropdownMenuState()
  DropdownMenuEvent()
  PopoverEvent()
  SelectState()
  SelectEvent()
  SidebarState()
  SidebarEvent()
  SonnerState()
  SonnerEvent()
  button-style button_style(variant:str, accent:color)
  checkbox-style checkbox_style(accent:color)
  input-style input_style(invalid:bool, accent:color)
  progress-style progress_style(variant:str, accent:color)
  component switch(id:&str, checked:bool, disabled:bool, accent:color) -> bool
  component input_otp(id:&str, value:&str, invalid:bool, disabled:bool, accent:color) -> str
  component spinner(frame:i64, reduced_motion:bool, accent:color) -> unit
  sync calendar_state() -> CalendarState
  task calendar_apply(state:CalendarState, event:CalendarEvent) -> CalendarState
  component calendar(state:&CalendarState, accent:color) -> CalendarEvent
  sync date_picker_state() -> DatePickerState
  task date_picker_apply(state:DatePickerState, event:DatePickerEvent) -> DatePickerState
  component date_picker(state:&DatePickerState, accent:color) -> DatePickerEvent
  component chart(hovered:ChartHit?, accent:color) -> ChartHit?
  sync command_state() -> CommandState
  task command_apply(state:CommandState, event:CommandEvent) -> CommandState
  component command(state:&CommandState, accent:color) -> CommandEvent
  sync select_state() -> SelectState
  task select_apply(state:SelectState, event:SelectEvent) -> SelectState
  component select(state:&SelectState, accent:color) -> SelectEvent
  sync dropdown_menu_state() -> DropdownMenuState
  task dropdown_menu_apply(state:DropdownMenuState, event:DropdownMenuEvent) -> DropdownMenuState
  component dropdown_menu(state:&DropdownMenuState, accent:color) -> DropdownMenuEvent
  sync context_menu_state() -> ContextMenuState
  task context_menu_apply(state:ContextMenuState, event:ContextMenuEvent) -> ContextMenuState
  component context_menu(state:&ContextMenuState, accent:color) -> ContextMenuEvent
  sync alert_dialog_state() -> AlertDialogState
  task alert_dialog_apply(state:AlertDialogState, event:AlertDialogEvent) -> AlertDialogState
  component alert_dialog(state:&AlertDialogState, accent:color) -> AlertDialogEvent
  sync sidebar_state() -> SidebarState
  sync sidebar_apply(state:SidebarState, event:SidebarEvent) -> SidebarState
  component sidebar(state:&SidebarState, accent:color) -> SidebarEvent
  sync sonner_state() -> SonnerState
  sync sonner_apply(state:SonnerState, event:SonnerEvent) -> SonnerState
  component sonner(state:&SonnerState, accent:color) -> SonnerEvent
  sync drawer_state() -> DrawerState
  task drawer_apply(state:DrawerState, event:DrawerEvent) -> DrawerState
  component drawer(state:&DrawerState, accent:color) -> DrawerEvent
  sync navigation_menu_state() -> NavigationMenuState
  task navigation_menu_apply(event:NavigationMenuEvent) -> NavigationMenuState
  component navigation_menu(state:&NavigationMenuState, accent:color) -> NavigationMenuEvent
  sync menubar_state() -> MenubarState
  task menubar_apply(state:MenubarState, event:MenubarEvent) -> MenubarState
  component menubar(state:&MenubarState, accent:color) -> MenubarEvent
  component hover_card(accent:color) -> unit
  component slider(values:&[f64], accent:color) -> [f64]
  component radio_group(selected:&str, accent:color) -> str
  task radio_apply(next:str) -> str
  sync message_scroller_state() -> MessageScrollerState
  task message_scroller_apply(state:MessageScrollerState, event:MessageScrollerEvent) -> MessageScrollerResult
  sync message_scroller_result() -> MessageScrollerResult
  sync message_scroller_result_state(result:MessageScrollerResult) -> MessageScrollerState
  task message_scroller_continue(state:MessageScrollerState, result:MessageScrollerResult) -> MessageScrollerResult
  component message_scroller(state:&MessageScrollerState, accent:color) -> MessageScrollerEvent
  sync data_table_rows(query:str, sort:str, page:i64) -> [str]
  sync data_table_next_sort(sort:str) -> str
  sync data_table_can_next(query:str, page:i64) -> bool
  component resizable_demo(sizes:&[f64], accent:color) -> [f64]
  task popover_apply(event:PopoverEvent) -> bool
  component popover_demo(open:bool, accent:color) -> PopoverEvent

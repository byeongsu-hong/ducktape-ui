// Catalog-only retained adapters. Applications define their own typed extern boundary.

extern crate::adapters
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
  checkbox-style checkbox_style()
  progress-style progress_style()
  progress-style progress_success_style()
  progress-style progress_warning_style()
  progress-style progress_destructive_style()
  component switch(id:&str, checked:bool, disabled:bool) -> bool
  component input_otp(id:&str, value:&str, invalid:bool, disabled:bool) -> str
  component spinner(frame:i64, reduced_motion:bool) -> unit
  sync calendar_state() -> CalendarState
  task calendar_apply(state:CalendarState, event:CalendarEvent) -> CalendarState
  component calendar(state:&CalendarState) -> CalendarEvent
  sync date_picker_state() -> DatePickerState
  task date_picker_apply(state:DatePickerState, event:DatePickerEvent) -> DatePickerState
  component date_picker(state:&DatePickerState) -> DatePickerEvent
  component chart(hovered:ChartHit?) -> ChartHit?
  sync command_state() -> CommandState
  task command_apply(state:CommandState, event:CommandEvent) -> CommandState
  component command(state:&CommandState) -> CommandEvent
  sync select_state() -> SelectState
  task select_apply(state:SelectState, event:SelectEvent) -> SelectState
  component select(state:&SelectState) -> SelectEvent
  sync dropdown_menu_state() -> DropdownMenuState
  sync dropdown_menu_is_open(state:DropdownMenuState) -> bool
  task dropdown_menu_apply(state:DropdownMenuState, event:DropdownMenuEvent) -> DropdownMenuState
  component dropdown_menu(state:&DropdownMenuState) -> DropdownMenuEvent
  sync context_menu_state() -> ContextMenuState
  task context_menu_apply(state:ContextMenuState, event:ContextMenuEvent) -> ContextMenuState
  component context_menu(state:&ContextMenuState) -> ContextMenuEvent
  sync alert_dialog_state() -> AlertDialogState
  task alert_dialog_apply(state:AlertDialogState, event:AlertDialogEvent) -> AlertDialogState
  sync alert_dialog_is_open(state:AlertDialogState) -> bool
  component alert_dialog(state:&AlertDialogState) -> AlertDialogEvent
  sync sidebar_state() -> SidebarState
  sync sidebar_apply(state:SidebarState, event:SidebarEvent) -> SidebarState
  component sidebar(state:&SidebarState) -> SidebarEvent
  sync sonner_state() -> SonnerState
  sync sonner_apply(state:SonnerState, event:SonnerEvent) -> SonnerState
  component sonner(state:&SonnerState) -> SonnerEvent
  sync drawer_state() -> DrawerState
  task drawer_apply(state:DrawerState, event:DrawerEvent) -> DrawerState
  component drawer(state:&DrawerState) -> DrawerEvent
  sync navigation_menu_state() -> NavigationMenuState
  sync navigation_menu_is_open(state:NavigationMenuState) -> bool
  task navigation_menu_apply(event:NavigationMenuEvent) -> NavigationMenuState
  component navigation_menu(state:&NavigationMenuState) -> NavigationMenuEvent
  sync menubar_state() -> MenubarState
  task menubar_apply(state:MenubarState, event:MenubarEvent) -> MenubarState
  component menubar(state:&MenubarState) -> MenubarEvent
  component hover_card() -> unit
  component slider(values:&[f64]) -> [f64]
  component radio_group(selected:&str) -> str
  task radio_apply(next:str) -> str
  sync message_scroller_state() -> MessageScrollerState
  task message_scroller_apply(state:MessageScrollerState, event:MessageScrollerEvent) -> MessageScrollerResult
  sync message_scroller_result() -> MessageScrollerResult
  sync message_scroller_result_state(result:MessageScrollerResult) -> MessageScrollerState
  task message_scroller_continue(state:MessageScrollerState, result:MessageScrollerResult) -> MessageScrollerResult
  component message_scroller(state:&MessageScrollerState) -> MessageScrollerEvent
  sync data_table_rows(query:str, sort:str, page:i64) -> [str]
  sync data_table_next_sort(sort:str) -> str
  sync data_table_can_next(query:str, page:i64) -> bool
  sync data_table_page_count(query:str) -> i64
  sync data_table_result_count(query:str) -> i64
  sync data_table_page_range(query:str, page:i64) -> [i64]
  sync data_table_page_label(page:i64, current:bool) -> str
  component resizable_demo(sizes:&[f64]) -> [f64]
  task popover_apply(event:PopoverEvent) -> bool
  component popover_demo(open:bool) -> PopoverEvent

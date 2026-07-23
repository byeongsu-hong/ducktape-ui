app Showcase
  title "ducktape-ui · Ice"
  id "dev.ducktape.ui.showcase"
  default-text-size 14
  antialiasing true
  window
    size 1120 820
    min-size 720 560
    position centered

use "ice/components.ice"

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

theme
  bg         #f8fafc
  background #f8fafc
  surface    #ffffff
  foreground #0f172a
  fg         #0f172a
  muted      #64748b
  primary    #2563eb
  accent     #eff6ff
  danger     #dc2626
  success    #16a34a
  warning    #d97706
  border     #e2e8f0

state
  email = ""
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
  otp = ""
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
  message_scroller_update:MessageScrollerResult = message_scroller_result()
  native_popover = false
  dialog_open = false
  toast_visible = true
  accent:color = color.rgb8(37, 99, 235)

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

on drawer_changed(event)
  task drawer_apply(drawer, event) -> drawer_applied _

on drawer_applied(next)
  drawer = next

on navigation_menu_changed(event)
  task navigation_menu_apply(event) -> navigation_menu_applied _

on navigation_menu_applied(next)
  navigation_menu = next

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
  return if catalog_page <= 0
  catalog_page = catalog_page - 1

on catalog_next
  return if !data_table_can_next(catalog_query, catalog_page)
  catalog_page = catalog_page + 1

on message_scroller_changed(event)
  task message_scroller_apply(message_scroller, event) -> message_scroller_applied _

on message_scroller_applied(result)
  message_scroller_update = result
  message_scroller = message_scroller_result_state(message_scroller_update)
  task message_scroller_continue(message_scroller, message_scroller_update) -> message_scroller_applied _

on native_popover_changed(event)
  task popover_apply(event) -> native_popover_applied _

on native_popover_applied(next)
  native_popover = next

on open_dialog
  dialog_open = true

on close_dialog
  dialog_open = false

on dismiss_toast
  toast_visible = false

on show_toast
  toast_visible = true

view
  overlay when=dialog_open dismiss=close_dialog backdrop=black/45 padding=24.0 align-x=center align-y=center
    content
      box width=fill height=fill bg=background
        scroll direction=vertical width=fill height=fill
          col width=fill padding=32.0 spacing=24.0
            row width=fill align=center
              PageHeader title="ducktape-ui" description="Default iced components, composed and checked by Ice."
              space width=fill height=1.0
              Badge label="ui-lang" variant="default"

            Alert title="Ice is the source of truth" description="Layout, state, routes, styles, and accessibility are generated from .ice files." destructive=false

            row width=fill spacing=20.0 wrap wrap-spacing=20.0
              Panel title="Buttons" description="Clear defaults with native focus and disabled behavior."
                col width=500.0 spacing=14.0
                  row spacing=8.0 wrap wrap-spacing=8.0
                    button "Primary" height=36.0 padding=8.0 style=button_style("default", accent) -> clicked
                    button "Secondary" height=36.0 padding=8.0 style=button_style("secondary", accent) -> clicked
                    button "Outline" height=36.0 padding=8.0 style=button_style("outline", accent) -> clicked
                    button "Ghost" height=36.0 padding=8.0 style=button_style("ghost", accent) -> clicked
                    button "Destructive" height=36.0 padding=8.0 style=button_style("destructive", accent) -> clicked
                    button "Disabled" disabled=true height=36.0 padding=8.0 style=button_style("secondary", accent) -> clicked
                  row spacing=8.0 align=center
                    text "Activated" size=12.0 @text-muted
                    text clicks size=13.0 @font-bold text-primary
                    Badge label="events" variant="secondary"

              Panel title="Badges & keyboard" description="Compact status and shortcut primitives."
                col width=500.0 spacing=14.0
                  row spacing=8.0 wrap wrap-spacing=8.0
                    Badge label="Default" variant="default"
                    Badge label="Secondary" variant="secondary"
                    Badge label="Outline" variant="outline"
                    Badge label="Danger" variant="destructive"
                  row spacing=6.0 align=center
                    text "Command palette" size=13.0 @text-fg
                    space width=fill height=1.0
                    Kbd label="⌘"
                    Kbd label="K"

              Panel title="Fields" description="Labels, help copy, validation, and native editing."
                col width=500.0 spacing=14.0
                  col width=fill spacing=6.0
                    input "Work email" description="We only use this address for product updates." <-> email hint="you@example.com" width=fill padding=10.0 style=input_style(false, accent)
                    text "We only use this address for product updates." size=12.0 @text-muted
                  Field label="Framework" description="Pick the runtime you want to build on."
                    pick native_select_frameworks native_select_framework placeholder="Choose a framework" width=fill -> framework_changed _
                  if email != ""
                    Alert title="Controlled input" description="The value is owned by Ice application state." destructive=false

              Panel title="Selection" description="Controlled values stay in the Ice state block."
                col width=500.0 spacing=14.0
                  checkbox "Accept the component contract" checked=accepted size=16.0 spacing=8.0 text-size=14.0 style=checkbox_style(accent) -> accepted_changed _
                  row spacing=10.0 align=center
                    extern switch("showcase-notifications", notifications, false, accent) -> notifications_changed _
                    text "Product notifications" size=13.0 @text-fg
                  extern radio_group(density, accent) -> density_changed _
                  slider volume min=0.0 max=100.0 step=1.0 width=fill -> volume_changed _
                  extern slider(native_range, accent) -> native_range_changed _
                  row spacing=8.0 align=center
                    box width=fill
                      progress volume length=fill girth=5.0 style=progress_style("success", accent)
                    text volume size=12.0 @text-muted

              Panel title="Composition" description="Slots keep caller state and handlers in their original scope."
                col width=500.0 spacing=12.0
                  Item title="Default components" description="One visual language across the application." meta="Ready"
                  Attachment name="component-contract.ice" meta="4.2 KB · Ice source"
                  Alert title="Composable by default" description="Content remains owned by the caller." destructive=false

              Panel title="Foundations" description="The small pieces compose into application-specific surfaces."
                col width=500.0 spacing=14.0
                  Breadcrumb current="Components"
                    row spacing=8.0
                      text "Home" size=12.0 @text-primary
                      text "/" size=12.0 @text-muted
                      text "Library" size=12.0 @text-primary
                  Card
                    Card.Header
                      col spacing=3.0
                        Typography content="Default card" role="heading"
                        Typography content="Compound slots keep structure readable." role="muted"
                    Card.Body
                      Surface
                        Message author="ducktape-ui" copy="Everything visible here is composed from Ice declarations." outgoing=false
                    Card.Footer
                      row width=fill spacing=8.0 align=center
                        Marker label="stable" active=true
                        Marker label="native" active=false
                        space width=fill height=1.0
                        ButtonGroup
                          row
                            button "Cancel" height=36.0 padding=8.0 style=button_style("ghost", accent) -> clicked
                            button "Apply" height=36.0 padding=8.0 style=button_style("default", accent) -> clicked
                  Separator
                  Bubble copy="Incoming and outgoing content keep explicit alignment." outgoing=false
                  Bubble copy="Caller state still owns the conversation." outgoing=true

              Panel title="Disclosure" description="Reusable components may own instance-scoped UI state."
                col width=500.0
                  AccordionItem question="Where does state live?" answer="Application state stays with the app. Small interaction state may live inside a reusable Ice component." #state
                  AccordionItem question="What stays in Rust?" answer="Domain rules, I/O, and advanced native widget escape hatches." #rust
                  AccordionItem question="Is accessibility optional?" answer="No. Ice emits the semantic tree and keyboard focus contract with the view." #accessibility

              Panel title="Stateful primitives" description="Disclosure, toggles, segments, and carousel state stay inside reusable Ice components."
                col width=500.0 spacing=14.0
                  CollapsibleDemo #collapsible
                  Separator
                  row width=fill spacing=16.0 align=center
                    ToggleDemo #toggle
                    space width=fill height=1.0
                    ToggleGroupDemo #segments
                  CarouselDemo #carousel

              Panel title="Tabs" description="A self-contained default interaction."
                box width=500.0
                  TabsDemo #tabs

              Panel title="Pagination" description="Small local state, bounded at both ends."
                box width=500.0
                  PaginationDemo #pagination

              Panel title="Native authoring" description="Search, rich editing, and tooltips use ui-lang primitives directly."
                col width=500.0 spacing=14.0
                  combo combobox_frameworks searched_framework "Search frameworks" width=fill padding=9.0 -> searched_framework_changed _
                  editor #default-editor <-> textarea_notes placeholder="Write notes" width=500.0 height=108.0 min-height=80.0 max-height=180.0 size=13.0 padding=10.0 wrapping=word
                    active bg=surface border=border border-w=1.0 r=8.0 placeholder=muted value=fg selection=primary
                    hovered border=primary
                    focused border=primary border-w=2.0
                  row spacing=8.0 align=center
                    text "Keyboard help" size=13.0 @text-fg
                    Tooltip label="Open the command palette"
                      button label="Command palette shortcut" height=32.0 padding=5.0 style=button_style("ghost", accent) -> clicked
                        row spacing=4.0
                          Kbd label="⌘"
                          Kbd label="K"

              Panel title="Command palette" description="Ice owns query and active state; Rust retains native editing, navigation, and focus."
                box width=500.0
                  extern command(command, accent) -> command_changed _

              Panel title="Advanced selection" description="Grouped options, typeahead, overlay collision, and focus remain controlled through Ice."
                col width=500.0 spacing=12.0
                  extern menubar(menubar, accent) -> menubar_changed _
                  row spacing=12.0 align=center
                    extern select(select, accent) -> select_changed _
                    extern dropdown_menu(dropdown, accent) -> dropdown_changed _
                  extern context_menu(context_menu, accent) -> context_menu_changed _
                  extern hover_card(accent)

              Panel title="Layout & data" description="Aspect ratio, scrolling, and table layout compile from Ice."
                col width=500.0 spacing=14.0
                  AspectRatio
                    text "16 / 9" size=20.0 @font-bold text-primary
                  ScrollArea
                    col width=fill spacing=4.0
                      Item title="Button" description="Actions and focus" meta="Core"
                      Item title="Input" description="Controlled text" meta="Core"
                      Item title="Dialog" description="Overlay composition" meta="UI"
                  row width=fill spacing=8.0 align=center
                    input "Filter components" <-> catalog_query hint="Filter components" width=fill padding=8.0 style=input_style(false, accent)
                    button "Sort" height=36.0 padding=8.0 style=button_style("outline", accent) -> catalog_sort_changed
                    text catalog_sort size=11.0 @text-muted
                  table item in data_table_rows(catalog_query, catalog_sort, catalog_page) width=fill padding=8.0 separator-y=1.0
                    column width=fill align-x=left align-y=center
                      header
                        text "Component" size=12.0 @font-bold text-fg
                      cell
                        text item size=12.0 @text-fg
                    column width=100.0 align-x=center align-y=center
                      header
                        text "Source" size=12.0 @font-bold text-fg
                      cell
                        text "Ice" size=12.0 @text-primary
                  row width=fill spacing=8.0 align=center
                    button "Previous" disabled=(catalog_page <= 0) height=32.0 padding=7.0 style=button_style("secondary", accent) -> catalog_previous
                    text (catalog_page + 1) size=12.0 @text-muted
                    button "Next" disabled=(!data_table_can_next(catalog_query, catalog_page)) height=32.0 padding=7.0 style=button_style("secondary", accent) -> catalog_next

              Panel title="Identity & calendar" description="Simple native state and opaque Rust state both stay controlled by Ice handlers."
                col width=500.0 spacing=14.0
                  Field label="Verification code" description="Paste and keyboard editing stay inside one native focus target."
                    extern input_otp("showcase-otp", otp, false, false, accent) -> otp_changed _
                  row width=fill spacing=10.0 align=center
                    extern spinner(clicks, false, accent)
                    text "Spinner frame follows the Ice click counter." size=12.0 @text-muted
                  extern date_picker(date_picker, accent) -> date_picker_changed _
                  extern calendar(calendar, accent) -> calendar_changed _

              Panel title="Chart" description="Ice owns hover state; Rust retains Canvas geometry and visible companion data."
                box width=500.0
                  extern chart(chart_hover, accent) -> chart_hovered _

              Panel title="Modal contracts" description="Alert dismissal, safe initial focus, focus trapping, and restoration cross the typed task boundary."
                box width=500.0
                  extern alert_dialog(alert_dialog, accent) -> alert_dialog_changed _

              Panel title="Navigation" description="Sidebar collapse and active route remain controlled by Ice."
                col width=500.0 spacing=16.0
                  extern navigation_menu(navigation_menu, accent) -> navigation_menu_changed _
                  box width=500.0 height=240.0 clip=true
                    extern sidebar(sidebar, accent) -> sidebar_changed _

              Panel title="Notifications" description="Ice owns the Sonner queue; native interaction reports reducer events back through one boundary."
                box width=500.0 height=220.0 clip=true
                  extern sonner(sonner, accent) -> sonner_changed _

              Panel title="Messages" description="Ice owns transcript anchors and unread state while native measurement tasks loop back through handlers."
                box width=500.0 height=220.0 clip=true
                  extern message_scroller(message_scroller, accent) -> message_scroller_changed _

              Panel title="Edge panels" description="Drawer composes Sheet geometry, drag dismissal, modal focus, and Ice-owned state."
                box width=500.0 height=160.0 clip=true
                  extern drawer(drawer, accent) -> drawer_changed _

              Panel title="Loading" description="Static placeholders remain reduced-motion safe."
                box width=500.0
                  Skeleton

              Panel title="Native escape hatches" description="Ice owns composition and state; advanced widgets cross one typed boundary."
                col width=500.0 spacing=14.0
                  extern resizable_demo(native_sizes, accent) -> native_resized _
                  extern popover_demo(native_popover, accent) -> native_popover_changed _

              Panel title="Empty state" description="A useful default before application-specific actions."
                box width=500.0
                  Empty title="No components found" description="Try a different filter or create the first component."

            row width=fill spacing=12.0 align=center
              button "Open dialog" height=36.0 padding=8.0 style=button_style("default", accent) -> open_dialog
              if !toast_visible
                button "Show toast" height=36.0 padding=8.0 style=button_style("secondary", accent) -> show_toast
              space width=fill height=1.0
              if toast_visible
                Toast title="Migration active" description="This screen is generated by ui-lang."
                  button "×" label="Dismiss toast" height=32.0 padding=6.0 style=button_style("ghost", accent) -> dismiss_toast
    layer
      Dialog
        Dialog.Header
          col spacing=4.0
            text "Default dialog" size=20.0 @font-bold text-fg
            text "The overlay, dismissal route, and focusable controls are declared in Ice." size=13.0 @text-muted
        Dialog.Body
          Alert title="No Rust view code" description="The proc macro emits ordinary iced code at compile time." destructive=false
        Dialog.Actions
          row width=fill spacing=8.0 align=end
            space width=fill height=1.0
            button "Cancel" height=36.0 padding=8.0 style=button_style("secondary", accent) -> close_dialog
            button "Continue" height=36.0 padding=8.0 style=button_style("default", accent) -> close_dialog

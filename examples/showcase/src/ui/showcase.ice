app Showcase
  title "ducktape-ui · Ice"
  id "dev.ducktape.ui.showcase"
  text-size 14
  antialiasing true
  window
    size 1120 820
    min-size 720 560
    position centered

use "components.ice"
use "../../../../crates/ui/src/ice/default.ice"
use "adapters.ice"

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

preset test
  state
    email = ""
    clicks = 0
    accepted = false
    notifications = true
    volume = 58.0
    density = "comfortable"
    catalog_query = ""
    catalog_sort = "none"
    catalog_page = 0
    dialog_open = false
    toast_visible = false

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

test app_behavior
  preset test
  viewport 1120 820
  target primary = #buttons/primary
  target email_input = #fields/work-email
  expect text "ducktape-ui"
  expect clicks == 0
  click primary
  expect clicks == 1
  click email_input
  type "ai@example.com"
  expect email_input.value == "ai@example.com"
  expect text "Controlled input"
  expect text "ai@example.com"
  key backspace
  expect email_input.value == "ai@example.co"
  dispatch catalog_sort_changed
  expect catalog_sort == "ascending"
  dispatch density_changed("compact")
  expect density == "compact"

view
  overlay when=dialog_open dismiss=close_dialog backdrop=black/45 p=24.0 align-x=center align-y=center
    content
      box w=fill h=fill bg=bg
        scroll dir=vertical w=fill h=fill
          col @page
            row w=fill align=center
              PageHeader title="ducktape-ui" description="Default iced components, composed and checked by Ice."
              space w=fill h=1.0
              Badge label="ui-lang"

            Alert title="Ice is the source of truth" description="Layout, state, routes, styles, and accessibility are generated from .ice files."

            grid gap=20.0 min-cell=500.0 @w-full
              Panel title="Buttons" description="Clear defaults with native focus and disabled behavior." #buttons
                col w=fill gap=14.0
                  row gap=8.0 wrap wrap-gap=8.0
                    button "Primary" #primary @primary_action -> clicked
                    button "Secondary" @secondary_action -> clicked
                    button "Outline" @outline_action -> clicked
                    button "Ghost" @ghost_action -> clicked
                    button "Destructive" @danger_action -> clicked
                    button "Disabled" disabled=true @secondary_action -> clicked
                  row gap=8.0 align=center
                    text "Activated" size=12.0 @text-muted
                    text clicks size=13.0 @font-bold text-primary
                    Badge.Secondary label="events"

              Panel title="Badges & keyboard" description="Compact status and shortcut primitives."
                col w=fill gap=14.0
                  row gap=8.0 wrap wrap-gap=8.0
                    Badge label="Default"
                    Badge.Secondary label="Secondary"
                    Badge.Outline label="Outline"
                    Badge.Destructive label="Danger"
                    Badge.Success label="Success"
                    Badge.Warning label="Warning"
                  row gap=6.0 align=center
                    text "Command palette" size=13.0 @text-fg
                    space w=fill h=1.0
                    Kbd label="⌘"
                    Kbd label="K"

              Panel title="Fields" description="Labels, help copy, validation, and native editing." #fields
                col w=fill gap=14.0
                  col w=fill gap=6.0
                    input "Work email" #work-email description="We only use this address for product updates." <-> email hint="you@example.com" @control
                    text "We only use this address for product updates." size=12.0 @text-muted
                  Field label="Framework" description="Pick the runtime you want to build on."
                    pick native_select_frameworks native_select_framework hint="Choose a framework" w=fill -> framework_changed _
                  if email != ""
                    text email size=12.0 @text-muted
                    Alert.Success title="Controlled input" description="The value is owned by Ice application state."

              Panel title="Selection" description="Controlled values stay in the Ice state block."
                col w=fill gap=14.0
                  checkbox "Accept the component contract" checked=accepted size=16.0 gap=8.0 text-size=14.0 style=checkbox_style() -> accepted_changed _
                  row gap=10.0 align=center
                    extern switch("showcase-notifications", notifications, false) -> notifications_changed _
                    text "Product notifications" size=13.0 @text-fg
                  extern radio_group(density) -> density_changed _
                  slider volume min=0.0 max=100.0 step=1.0 w=fill -> volume_changed _
                  extern slider(native_range) -> native_range_changed _
                  row gap=8.0 align=center
                    box w=fill
                      progress volume length=fill girth=5.0 style=progress_success_style()
                    text volume size=12.0 @text-muted

              Panel title="Composition" description="Slots keep caller state and handlers in their original scope."
                col w=fill gap=12.0
                  Item title="Default components" description="One visual language across the application." meta="Ready"
                    Avatar initials="UI"
                  Attachment name="component-contract.ice" meta="4.2 KB · Ice source"
                  Alert.Warning title="Caller ownership stays explicit" description="Pass slots and events instead of hiding application behavior in a component."

              Panel title="Foundations" description="The small pieces compose into application-specific surfaces."
                col w=fill gap=14.0
                  Breadcrumb current="Components"
                    row gap=8.0
                      text "Home" size=12.0 @text-primary
                      text "/" size=12.0 @text-muted
                      text "Library" size=12.0 @text-primary
                  Card
                    Card.Header
                      col gap=3.0
                        Typography.Heading content="Default card"
                        Typography.Muted content="Compound slots keep structure readable."
                    Card.Body
                      Surface
                        Message author="ducktape-ui" copy="Everything visible here is composed from Ice declarations." initials="UI" outgoing=false
                    Card.Footer
                      row w=fill gap=8.0 align=center
                        Marker label="stable" active=true
                        Marker label="native" active=false
                        space w=fill h=1.0
                        ButtonGroup
                          row
                            button "Cancel" @ghost_action -> clicked
                            button "Apply" @primary_action -> clicked
                  Separator
                  Bubble copy="Incoming and outgoing content keep explicit alignment." outgoing=false
                  Bubble copy="Caller state still owns the conversation." outgoing=true

              Panel title="Disclosure" description="Reusable components may own instance-scoped UI state."
                col w=fill
                  AccordionItem question="Where does state live?" answer="Application state stays with the app. Small interaction state may live inside a reusable Ice component." #state
                  AccordionItem question="What stays in Rust?" answer="Domain rules, I/O, and advanced native widget escape hatches." #rust
                  AccordionItem question="Is accessibility optional?" answer="No. Ice emits the semantic tree and keyboard focus contract with the view." #accessibility

              Panel title="Stateful primitives" description="Disclosure, toggles, segments, and carousel state stay inside reusable Ice components."
                col w=fill gap=14.0
                  CollapsibleDemo #collapsible
                  Separator
                  row w=fill gap=16.0 align=center
                    ToggleDemo #toggle
                    space w=fill h=1.0
                    ToggleGroupDemo #segments
                  CarouselDemo #carousel

              Panel title="Tabs" description="A self-contained default interaction."
                box w=fill
                  TabsDemo #tabs

              Panel title="Pagination" description="Small local state, bounded at both ends."
                box w=fill
                  PaginationDemo #pagination

              Panel title="Native authoring" description="Search, rich editing, and tooltips use ui-lang primitives directly."
                col w=fill gap=14.0
                  combo combobox_frameworks searched_framework "Search frameworks" w=fill p=9.0 -> searched_framework_changed _
                  editor #default-editor <-> textarea_notes hint="Write notes" h=108.0 min-h=80.0 max-h=180.0 size=13.0 p=10.0 wrap=word
                    active bg=surface border=border border-w=1.0 r=8.0 placeholder=muted value=fg selection=primary
                    hovered border=primary
                    focused border=primary border-w=2.0
                  row gap=8.0 align=center
                    text "Keyboard help" size=13.0 @text-fg
                    Tooltip label="Open the command palette"
                      button label="Command palette shortcut" @ghost_action -> clicked
                        row gap=4.0
                          Kbd label="⌘"
                          Kbd label="K"

              Panel title="Command palette" description="Ice owns query and active state; Rust retains native editing, navigation, and focus."
                box w=fill
                  extern command(command) -> command_changed _

              Panel title="Advanced selection" description="Grouped options, typeahead, overlay collision, and focus remain controlled through Ice."
                col w=fill gap=12.0
                  extern menubar(menubar) -> menubar_changed _
                  row gap=12.0 align=center
                    extern select(select) -> select_changed _
                    extern dropdown_menu(dropdown) -> dropdown_changed _
                  extern context_menu(context_menu) -> context_menu_changed _
                  extern hover_card()

              Panel title="Layout & data" description="Aspect ratio, scrolling, and table layout compile from Ice."
                col w=fill gap=14.0
                  AspectRatioDemo
                    text "16 / 9" size=20.0 @font-bold text-primary
                  ScrollAreaDemo
                    col w=fill gap=4.0
                      Item title="Button" description="Actions and focus" meta="Core"
                        Avatar initials="B"
                      Item title="Input" description="Controlled text" meta="Core"
                        Avatar initials="I"
                      Item title="Dialog" description="Overlay composition" meta="UI"
                        Avatar initials="D"
                  row w=fill gap=8.0 align=center
                    input "Filter components" <-> catalog_query hint="Filter components" @control
                    button "Sort" @outline_action -> catalog_sort_changed
                    text catalog_sort size=11.0 @text-muted
                  table item in data_table_rows(catalog_query, catalog_sort, catalog_page) w=fill p=8.0 sep-y=1.0
                    col w=fill align-x=left align-y=center
                      header
                        text "Component" size=12.0 @font-bold text-fg
                      cell
                        text item size=12.0 @text-fg
                    col w=100.0 align-x=center align-y=center
                      header
                        text "Source" size=12.0 @font-bold text-fg
                      cell
                        text "Ice" size=12.0 @text-primary
                  row w=fill gap=8.0 align=center
                    button "Previous" disabled=(catalog_page <= 0) @secondary_action -> catalog_previous
                    text (catalog_page + 1) size=12.0 @text-muted
                    button "Next" disabled=(!data_table_can_next(catalog_query, catalog_page)) @secondary_action -> catalog_next

              Panel title="Identity & calendar" description="Simple native state and opaque Rust state both stay controlled by Ice handlers."
                col w=fill gap=14.0
                  Field label="Verification code" description="Paste and keyboard editing stay inside one native focus target."
                    extern input_otp("showcase-otp", otp, false, false) -> otp_changed _
                  row w=fill gap=10.0 align=center
                    extern spinner(clicks, false)
                    text "Spinner frame follows the Ice click counter." size=12.0 @text-muted
                  extern date_picker(date_picker) -> date_picker_changed _
                  extern calendar(calendar) -> calendar_changed _

              Panel title="Chart" description="Ice owns hover state; Rust retains Canvas geometry and visible companion data."
                box w=fill
                  extern chart(chart_hover) -> chart_hovered _

              Panel title="Modal contracts" description="Alert dismissal, safe initial focus, focus trapping, and restoration cross the typed task boundary."
                box w=fill
                  extern alert_dialog(alert_dialog) -> alert_dialog_changed _

              Panel title="Navigation" description="Sidebar collapse and active route remain controlled by Ice."
                col w=fill gap=16.0
                  extern navigation_menu(navigation_menu) -> navigation_menu_changed _
                  box w=fill h=240.0 clip=true
                    extern sidebar(sidebar) -> sidebar_changed _

              Panel title="Notifications" description="Ice owns the Sonner queue; native interaction reports reducer events back through one boundary."
                box w=fill h=220.0 clip=true
                  extern sonner(sonner) -> sonner_changed _

              Panel title="Messages" description="Ice owns transcript anchors and unread state while native measurement tasks loop back through handlers."
                box w=fill h=220.0 clip=true
                  extern message_scroller(message_scroller) -> message_scroller_changed _

              Panel title="Edge panels" description="Drawer composes Sheet geometry, drag dismissal, modal focus, and Ice-owned state."
                box w=fill h=160.0 clip=true
                  extern drawer(drawer) -> drawer_changed _

              Panel title="Loading" description="Static placeholders remain reduced-motion safe."
                box w=fill
                  SkeletonDemo

              Panel title="Native escape hatches" description="Ice owns composition and state; advanced widgets cross one typed boundary."
                col w=fill gap=14.0
                  extern resizable_demo(native_sizes) -> native_resized _
                  extern popover_demo(native_popover) -> native_popover_changed _

              Panel title="Empty state" description="A useful default before application-specific actions."
                box w=fill
                  EmptyState title="No components found" description="Try a different filter or create the first component."

            row w=fill gap=12.0 align=center
              button "Open dialog" @primary_action -> open_dialog
              if !toast_visible
                button "Show toast" @secondary_action -> show_toast
              space w=fill h=1.0
              if toast_visible
                Toast title="Migration active" description="This screen is generated by ui-lang."
                  button "×" label="Dismiss toast" @ghost_action -> dismiss_toast
    layer
      Dialog
        Dialog.Header
          col gap=4.0
            text "Default dialog" size=20.0 @font-bold text-fg
            text "The overlay, dismissal route, and focusable controls are declared in Ice." size=13.0 @text-muted
        Dialog.Body
          Alert title="No Rust view code" description="The proc macro emits ordinary iced code at compile time."
        Dialog.Actions
          row w=fill gap=8.0 align=end
            space w=fill h=1.0
            button "Cancel" @secondary_action -> close_dialog
            button "Continue" @primary_action -> close_dialog

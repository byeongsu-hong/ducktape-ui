component Catalog(bind email:str, bind project_slug:str, bind textarea_notes:editor, bind catalog_query:str, clicks:i64, accepted:bool, notifications:bool, volume:f64, density:str, native_select_frameworks:[str], native_select_framework:str?, combobox_frameworks:combo[str], searched_framework:str?, catalog_sort:str, catalog_page:i64, catalog_at_start:bool, catalog_page_number:i64, demo_page:i64, otp:str, calendar:CalendarState, date_picker:DatePickerState, chart_hover:ChartHit?, command:CommandState, select:SelectState, dropdown:DropdownMenuState, context_menu:ContextMenuState, alert_dialog:AlertDialogState, sidebar:SidebarState, sonner:SonnerState, drawer:DrawerState, navigation_menu:NavigationMenuState, menubar:MenubarState, native_sizes:[f64], native_range:[f64], message_scroller:MessageScrollerState, native_popover:bool)
  emits
    clicked
    accepted_changed(bool)
    notifications_changed(bool)
    volume_changed(f64)
    density_changed(str)
    framework_changed(str)
    searched_framework_changed(str)
    otp_changed(str)
    calendar_changed(CalendarEvent)
    date_picker_changed(DatePickerEvent)
    chart_hovered(ChartHit?)
    command_changed(CommandEvent)
    select_changed(SelectEvent)
    dropdown_changed(DropdownMenuEvent)
    context_menu_changed(ContextMenuEvent)
    alert_dialog_changed(AlertDialogEvent)
    sidebar_changed(SidebarEvent)
    sonner_changed(SonnerEvent)
    drawer_changed(DrawerEvent)
    navigation_menu_changed(NavigationMenuEvent)
    menubar_changed(MenubarEvent)
    native_resized([f64])
    native_range_changed([f64])
    catalog_sort_changed
    catalog_previous
    catalog_next
    demo_page_previous
    demo_page_next
    message_scroller_changed(MessageScrollerEvent)
    native_popover_changed(PopoverEvent)
  grid #root
    with
      gap=20.0
      min-cell=500.0
      @w-full
    Panel #buttons
      with
        title="Buttons"
        description="Clear defaults with native focus and disabled behavior."
      col w=fill gap=14.0
        row gap=8.0 wrap wrap-gap=8.0
          button #primary -> emit(clicked)
            with
              label="Primary"
              w=128.0
              h=36.0
              @primary_action
              @py-8px
            ActionLabel content="Primary"
          button #secondary -> emit(clicked)
            with
              label="Secondary"
              w=128.0
              h=36.0
              @secondary_action
              @py-8px
            ActionLabel content="Secondary"
          button #outline -> emit(clicked)
            with
              label="Outline"
              w=128.0
              h=36.0
              @outline_action
              @py-8px
            ActionLabel content="Outline"
          button #ghost -> emit(clicked)
            with
              label="Ghost"
              w=128.0
              h=36.0
              @ghost_action
              @py-8px
            ActionLabel content="Ghost"
          button #destructive -> emit(clicked)
            with
              label="Destructive"
              w=128.0
              h=36.0
              @danger_action
              @py-8px
            ActionLabel content="Destructive"
          button #disabled -> emit(clicked)
            with
              label="Disabled"
              w=128.0
              h=36.0
              disabled=true
              @secondary_action
              @py-8px
            ActionLabel content="Disabled"
        row gap=8.0 align=center
          text "Activated" size=12.0 @text-muted
          text clicks
            with
              size=13.0
              @font-bold
              @text-primary
          Badge.Secondary label="events"

    Panel #badges title="Badges & keyboard" description="Compact status and shortcut primitives."
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
        row gap=6.0 align=center
          text "Quick search" size=13.0 @text-fg
          space w=fill h=1.0
          Kbd label="/"
        row gap=6.0 align=center
          text "Move focus" size=13.0 @text-fg
          space w=fill h=1.0
          Kbd label="⇧"
          Kbd label="Tab"

    Panel #fields title="Fields" description="Labels, help copy, validation, and native editing."
      col w=fill gap=14.0
        col w=fill gap=6.0
          input "Work email" #work-email <-> email
            with
              description="We only use this address for product updates."
              hint="you@example.com"
              p=9.0
              @control
          text "We only use this address for product updates." size=12.0 @text-muted
        Field #framework-field
          with
            label="Framework"
            description="Pick the runtime you want to build on."
          pick native_select_frameworks native_select_framework #framework-picker -> emit(framework_changed, _)
            with
              hint="Choose a framework"
              w=fill
              p=9.0
            active bg=surface border=border border-w=1.0 r=9.0 text=fg placeholder=muted handle=muted
            hovered bg=surface border=control_line border-w=1.0 r=9.0 text=fg placeholder=muted handle=fg
            opened bg=surface border=primary border-w=2.0 r=9.0 text=fg placeholder=muted handle=fg
            opened-hovered bg=surface border=primary border-w=2.0 r=9.0 text=fg placeholder=muted handle=fg
            menu bg=surface border=border border-w=1.0 r=9.0 text=fg selected-text=fg selected-bg=accent
        if email != ""
          text email size=12.0 @text-muted
          Alert.Success
            with
              title="Controlled input"
              description="The value is owned by Ice application state."
        Field #project-url-field
          with
            label="Project URL"
            description="The fixed origin and editable slug share one control boundary."
          InputGroup #project-url
            ProjectSlugInput value<->project_slug #slug-editor
        Alert.Destructive
          with
            title="Validation stays adjacent"
            description="Errors remain next to the control that needs attention."

    Panel title="Selection" description="Controlled values stay in the Ice state block."
      col w=fill gap=16.0
        Field label="Consent" description="Checkbox state is explicit and reversible."
          checkbox "Accept the component contract" -> emit(accepted_changed, _)
            with
              checked=accepted
              size=16.0
              gap=8.0
              text-size=14.0
              style=checkbox_style()
        Field label="Notifications" description="The switch updates one controlled Boolean value."
          row gap=10.0 align=center
            extern switch("showcase-notifications", notifications, false) -> emit(notifications_changed, _)
            text "Product notifications" size=13.0 @text-fg
        Field label="Interface density" description="Arrow keys move through a single-choice group."
          extern radio_group(density) -> emit(density_changed, _)
        Field label="Volume" description="The progress bar mirrors the slider value."
          col w=fill gap=10.0
            slider volume -> emit(volume_changed, _)
              with
                min=0.0
                max=100.0
                step=1.0
                w=fill
            row gap=8.0 align=center
              box w=fill
                progress volume
                  with
                    length=fill
                    girth=5.0
                    style=progress_success_style()
              text volume size=12.0 @text-muted
        Field label="Active range" description="Two handles define the selected interval."
          extern slider(native_range) -> emit(native_range_changed, _)

    Panel
      with
        title="Composition"
        description="Slots keep caller state and handlers in their original scope."
      col w=fill gap=12.0
        Item
          with
            title="Default components"
            description="One visual language across the application."
            meta="Ready"
          Avatar initials="UI"
        Attachment name="component-contract.ice" meta="4.2 KB · Ice source"
        Alert.Warning
          with
            title="Caller ownership stays explicit"
            description="Pass slots and events instead of hiding application behavior in a component."
        Surface
          col w=fill
            Item
              with
                title="Header slot"
                description="The caller supplies hierarchy and context."
                meta="slot"
              Avatar initials="H"
            Item
              with
                title="Body slot"
                description="Domain content stays outside the shell."
                meta="slot"
              Avatar initials="B"
            Item
              with
                title="Action slot"
                description="Events route back to the caller."
                meta="emit"
              Avatar initials="A"

    Panel
      with
        title="Foundations"
        description="The small pieces compose into application-specific surfaces."
      col w=fill gap=14.0
        Breadcrumb current="Components"
          row gap=8.0
            text "Home" size=12.0 @text-primary
            text "/" size=12.0 @text-muted
            text "Library" size=12.0 @text-primary
        Card
          Card.Header
            col gap=3.0
              Typography.SectionTitle content="Default card"
              Typography.Caption content="Compound slots keep structure readable."
          Card.Body
            Surface
              Message
                with
                  author="ducktape-ui"
                  copy="Everything visible here is composed from Ice declarations."
                  initials="UI"
                  outgoing=false
          Card.Footer
            row
              with
                w=fill
                gap=8.0
                align=center
              Marker label="stable" active=true
              Marker label="native" active=false
              space w=fill h=1.0
              ButtonGroup
                row
                  button -> emit(clicked)
                    with
                      label="Cancel"
                      h=36.0
                      @ghost_action
                      @py-8px
                    ActionLabel content="Cancel"
                  button -> emit(clicked)
                    with
                      label="Apply"
                      h=36.0
                      @primary_action
                      @py-8px
                    ActionLabel content="Apply"
        Separator
        Bubble copy="Incoming and outgoing content keep explicit alignment." outgoing=false
        Bubble copy="Caller state still owns the conversation." outgoing=true

    Panel #disclosure-panel
      with
        title="Disclosure"
        description="Reusable components may own instance-scoped UI state."
      col w=fill gap=12.0
        AccordionItem #state
          with
            question="Where does state live?"
            answer="Application state stays with the app. Small interaction state may live inside a reusable Ice component."
        AccordionItem #rust
          with
            question="What stays in Rust?"
            answer="Domain rules, I/O, and advanced native widget escape hatches."
        AccordionItem #accessibility
          with
            question="Is accessibility optional?"
            answer="No. Ice emits the semantic tree and keyboard focus contract with the view."
        Alert
          with
            title="Keep ownership narrow"
            description="Components own disclosure state; the application keeps domain state and side effects."

    Panel #stateful-panel
      with
        title="Stateful primitives"
        description="Local disclosure, toggle, segment, and carousel state."
      col w=fill gap=14.0
        CollapsibleDemo #collapsible
        Separator
        row
          with
            w=fill
            gap=16.0
            align=center
          ToggleDemo #toggle
          space w=fill h=1.0
          ToggleGroupDemo #segments
        CarouselDemo #carousel

    Panel #tabs-panel title="Tabs" description="A self-contained default interaction."
      box w=fill
        TabsDemo #tabs

    Panel #pagination-panel
      with
        title="Pagination"
        description="Small local state, bounded at both ends."
      col w=fill gap=14.0
        PaginationDemo page=demo_page #pagination
          events
            previous -> emit(demo_page_previous)
            next -> emit(demo_page_next)
        Surface
          col w=fill
            Item
              with
                title="Controlled page"
                description="The application owns the current page."
                meta="1 / 5"
              Avatar initials="P"
            Item
              with
                title="Bounded actions"
                description="Previous and next disable at the edges."
                meta="safe"
              Avatar initials="B"

    Panel
      with
        title="Native authoring"
        description="Search, rich editing, and tooltips use ui-lang primitives directly."
      col w=fill gap=14.0
        FrameworkCombo -> emit(searched_framework_changed, _)
          with
            options=combobox_frameworks
            selected=searched_framework
        editor #default-editor <-> textarea_notes
          with
            hint="Write notes"
            h=108.0
            min-h=80.0
            max-h=180.0
            size=13.0
            p=10.0
            wrap=word
          active bg=surface border=border border-w=1.0 r=8.0 placeholder=muted value=fg selection=primary
          hovered border=primary
          focused border=primary border-w=2.0
        row gap=8.0 align=center
          text "Keyboard help" size=13.0 @text-fg
          Tooltip label="Open the command palette"
            button -> emit(clicked)
              with
                label="Command palette shortcut"
                h=32.0
                p=4.0
                @ghost_action
              row gap=4.0
                Kbd label="⌘"
                Kbd label="K"

    Panel
      with
        title="Command palette"
        description="Ice owns query and active state; Rust retains native editing, navigation, and focus."
      box w=fill
        extern command(command) -> emit(command_changed, _)

    Panel #advanced-selection-panel
      with
        title="Advanced selection"
        description="Grouped options, typeahead, overlays, and focus stay controlled."
      col #advanced-selection-content w=fill gap=14.0
        extern menubar(menubar) -> emit(menubar_changed, _)
        row gap=12.0 align=center
          extern select(select) -> emit(select_changed, _)
          extern dropdown_menu(dropdown) -> emit(dropdown_changed, _)
        extern context_menu(context_menu) -> emit(context_menu_changed, _)
        extern hover_card()
        Surface
          col w=fill
            Item
              with
                title="Keyboard navigation"
                description="Arrow keys move without losing focus."
                meta="↑ ↓"
              Avatar initials="K"
            Item
              with
                title="Overlay placement"
                description="Menus stay inside the visible window."
                meta="auto"
              Avatar initials="O"
            Item
              with
                title="Dismissal"
                description="Escape and outside presses share one route."
                meta="esc"
              Avatar initials="D"

    Panel #layout-primitives-panel
      with
        title="Layout primitives"
        description="Aspect ratio and constrained scrolling compile from Ice."
      col #layout-primitives-content w=fill gap=14.0
        box w=fill align-x=center
          AspectRatioDemo
        ScrollAreaDemo
          col w=fill gap=4.0
            Item
              with
                title="Button"
                description="Actions and focus"
                meta="Core"
              Avatar initials="B"
            Item
              with
                title="Input"
                description="Controlled text"
                meta="Core"
              Avatar initials="I"
            Item
              with
                title="Dialog"
                description="Overlay composition"
                meta="UI"
              Avatar initials="D"
            Item
              with
                title="Table"
                description="Rows and paging"
                meta="Data"
              Avatar initials="T"

    Panel #identity-calendar-panel
      with
        title="Identity & calendar"
        description="Simple native state and opaque Rust state both stay controlled by Ice handlers."
      col #identity-calendar-content w=fill gap=14.0
        Field
          with
            label="Verification code"
            description="Paste and keyboard editing stay inside one native focus target."
          extern input_otp("showcase-otp", otp, false, false) -> emit(otp_changed, _)
        row
          with
            w=fill
            gap=10.0
            align=center
          extern spinner(clicks, false)
          text "Spinner frame follows the Ice click counter." size=12.0 @text-muted
        extern date_picker(date_picker) -> emit(date_picker_changed, _)
        extern calendar(calendar) -> emit(calendar_changed, _)

    Panel #chart-panel
      with
        title="Chart"
        description="Ice owns hover state; Rust retains Canvas geometry and visible companion data."
      box #chart-content w=fill
        extern chart(chart_hover) -> emit(chart_hovered, _)

    Panel #modal-contracts-panel
      with
        title="Modal contracts"
        description="Safe focus, focus trapping, dismissal, and restoration stay typed."
      col #modal-contracts-content w=fill gap=14.0
        DemoStage height=190.0 padding=8.0
          extern alert_dialog(alert_dialog) -> emit(alert_dialog_changed, _)
        Surface
          col w=fill
            Item
              with
                title="Initial focus"
                description="The safe cancel action receives focus first."
                meta="1"
              Avatar initials="F"
            Item
              with
                title="Focus trap"
                description="Tab stays within the destructive decision."
                meta="2"
              Avatar initials="T"
            Item
              with
                title="Restoration"
                description="Dismissal returns focus to the trigger."
                meta="3"
              Avatar initials="R"
            Item
              with
                title="Typed events"
                description="Open, dismiss, and focus changes share one route."
                meta="4"
              Avatar initials="E"

    Panel #data-table-panel
      with
        title="Data table"
        description="Filtering, sorting, rows, and paging remain application-owned."
      col #data-table-content w=fill gap=14.0
        row
          with
            w=fill
            gap=8.0
            align=center
          input "Filter components" <-> catalog_query
            with
              hint="Filter components"
              p=9.0
              @control
          button -> emit(catalog_sort_changed)
            with
              label="Sort"
              h=36.0
              @outline_action
              @py-8px
            ActionLabel content="Sort"
          text catalog_sort size=11.0 @text-muted
        table item in data_table_rows(catalog_query, catalog_sort, catalog_page)
          with
            w=fill
            p=8.0
            sep-y=1.0
          col w=fill align-x=left align-y=center
            header
              text "Component"
                with
                  size=12.0
                  @font-bold
                  @text-fg
            cell
              text item size=12.0 @text-fg
          col w=100.0 align-x=center align-y=center
            header
              text "Source"
                with
                  size=12.0
                  @font-bold
                  @text-fg
            cell
              text "Ice" size=12.0 @text-primary
        row
          with
            w=fill
            gap=8.0
            align=center
          button -> emit(catalog_previous)
            with
              label="Previous"
              h=32.0
              disabled=catalog_at_start
              @secondary_action
              @py-6px
            ActionLabel content="Previous"
          text catalog_page_number size=12.0 @text-muted
          button -> emit(catalog_next)
            with
              label="Next"
              h=32.0
              disabled=(!data_table_can_next(catalog_query, catalog_page))
              @secondary_action
              @py-6px
            ActionLabel content="Next"
        Surface
          col w=fill
            Item
              with
                title="Filtering"
                description="The query stays in application state."
                meta="live"
              Avatar initials="F"
            Item
              with
                title="Sorting"
                description="One action cycles through three explicit modes."
                meta="3"
              Avatar initials="S"
            Item
              with
                title="Paging"
                description="Boundary actions disable instead of wrapping."
                meta="safe"
              Avatar initials="P"

    Panel
      with
        title="Notifications"
        description="Ice owns the Sonner queue; native interaction reports reducer events back through one boundary."
      DemoStage height=208.0
        extern sonner(sonner) -> emit(sonner_changed, _)

    Panel
      with
        title="Messages"
        description="Ice owns transcript anchors and unread state while native measurement tasks loop back through handlers."
      DemoStage height=208.0 padding=8.0
        extern message_scroller(message_scroller) -> emit(message_scroller_changed, _)

    Panel #edge-panels
      with
        title="Edge panels"
        description="Sheet geometry, drag dismissal, modal focus, and state stay composed."
      DemoStage height=190.0 #edge-stage
        extern drawer(drawer) -> emit(drawer_changed, _)

    Panel #native-escape-hatches
      with
        title="Native escape hatches"
        description="Ice owns composition and state; advanced widgets cross one typed boundary."
      DemoStage height=190.0 padding=10.0 #native-stage
        col w=fill gap=10.0
          extern resizable_demo(native_sizes) -> emit(native_resized, _)
          extern popover_demo(native_popover) -> emit(native_popover_changed, _)

    Panel title="Loading" description="Static placeholders remain reduced-motion safe."
      DemoStage height=154.0 padding=16.0
        SkeletonDemo

    Panel title="Empty state" description="A useful default before application-specific actions."
      box w=fill h=160.0
        EmptyState
          with
            title="No components found"
            description="Try a different filter or create the first component."

    Panel
      with
        title="Navigation shell"
        description="The full-width shell keeps collapse, active route, and workspace context controlled by Ice."
      col w=fill gap=16.0
        extern navigation_menu(navigation_menu) -> emit(navigation_menu_changed, _)
        DemoStage height=308.0 padding=10.0
          extern sidebar(sidebar) -> emit(sidebar_changed, _)

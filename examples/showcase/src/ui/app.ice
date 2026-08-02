app Showcase
  title "ducktape-ui · Ice"
  id "dev.ducktape.ui.showcase"
  font "../../assets/fonts/Geist-Regular.ttf"
  font "../../assets/fonts/Geist-Bold.ttf"
  font "../../assets/fonts/Geist-Italic.ttf"
  font "../../assets/fonts/GeistMono-Regular.ttf"
  font "../../assets/fonts/GeistMono-Bold.ttf"
  font "../../assets/fonts/GeistMono-Italic.ttf"
  text-size 14
  // Keep deterministic headless paint and interaction captures.
  antialiasing false
  window
    size 1120 820
    min-size 720 560
    position centered

font geist family="Geist" default=true

use "extern/adapters.ice"
use "state.ice"
use "components/controls.ice"
use "components/navigation.ice"
use "components/content.ice"
use "components/catalog.ice"
use "handlers/app.ice"
use "tests/app.ice"
use "../../../../crates/ui/src/ice/default.ice"

view
  overlay
    with
      when=dialog_open
      dismiss=close_dialog
      backdrop=black/45
      p=24.0
      align-x=center
      align-y=center
    content
      box #app
        with
          w=fill
          h=fill
          bg=bg
        col w=fill h=fill
          col
            with
              w=fill
              gap=16.0
              p=24.0
            row w=fill align=center
              PageHeader
                with
                  title="ducktape-ui"
                  description="Default iced components, composed and checked by Ice."
              space w=fill h=1.0
              Badge label="ui-lang"

            Alert
              with
                title="Ice is the source of truth"
                description="Layout, state, routes, styles, and accessibility are generated from .ice files."

            row w=fill gap=12.0
              box w=fill
                VirtualList.Frame #virtual-list-panel
                  with
                    title="Virtual list"
                    description="Only visible fixed-height keyed rows cross the typed boundary."
                    count=100000
                  box w=fill h=96.0
                    extern virtual_list(virtual_list) #virtual-list -> virtual_list_changed _
              box w=fill
                TreeView.Frame #tree-view-panel
                  with
                    title="Tree view"
                    description="Hierarchy, expansion, rename, and tree semantics stay retained."
                    count=100000
                  box w=fill h=96.0
                    extern tree_view(tree_view) #tree-view -> tree_view_changed _
              box w=fill
                DataGrid.Frame #data-grid-panel
                  with
                    title="Data grid"
                    description="Fixed keyed rows and typed cells."
                    rows=100000
                    columns=16
                  box w=fill h=96.0
                    extern data_grid(data_grid) #data-grid -> data_grid_changed _

          scroll #catalog-scroll
            with
              dir=vertical
              w=fill
              h=fill
              bar-w=8.0
              scroller-w=6.0
              bar-gap=8.0
            col #page @page
              Catalog #catalog-grid email<->email project_slug<->project_slug textarea_notes<->textarea_notes catalog_query<->catalog_query
                with
                  clicks=clicks
                  accepted=accepted
                  notifications=notifications
                  volume=volume
                  density=density
                  native_select_frameworks=native_select_frameworks
                  native_select_framework=native_select_framework
                  combobox_frameworks=combobox_frameworks
                  searched_framework=searched_framework
                  catalog_sort=catalog_sort
                  catalog_page=catalog_page
                  catalog_at_start=catalog_at_start
                  catalog_page_number=catalog_page_number
                  demo_page=demo_page
                  demo_page_max=demo_page_max
                  reduced_motion=reduced_motion
                  otp=otp
                  calendar=calendar
                  date_picker=date_picker
                  chart_hover=chart_hover
                  hover_card_open=hover_card_open
                  navigation_route=navigation_route
                  card_action=card_action
                  command=command
                  select=select
                  dropdown=dropdown
                  context_menu=context_menu
                  alert_dialog=alert_dialog
                  sidebar=sidebar
                  sonner=sonner
                  drawer=drawer
                  navigation_menu=navigation_menu
                  menubar=menubar
                  native_sizes=native_sizes
                  native_range=native_range
                  message_scroller=message_scroller
                  native_popover=native_popover
                events
                  clicked -> clicked
                  accepted_changed -> accepted_changed _
                  notifications_changed -> notifications_changed _
                  volume_changed -> volume_changed _
                  density_changed -> density_changed _
                  framework_changed -> framework_changed _
                  searched_framework_changed -> searched_framework_changed _
                  otp_changed -> otp_changed _
                  calendar_changed -> calendar_changed _
                  date_picker_changed -> date_picker_changed _
                  chart_hovered -> chart_hovered _
                  hover_card_changed -> hover_card_changed _
                  command_changed -> command_changed _
                  select_changed -> select_changed _
                  dropdown_changed -> dropdown_changed _
                  context_menu_changed -> context_menu_changed _
                  alert_dialog_changed -> alert_dialog_changed _
                  sidebar_changed -> sidebar_changed _
                  sonner_changed -> sonner_changed _
                  reduced_motion_changed -> reduced_motion_changed _
                  drawer_changed -> drawer_changed _
                  navigation_menu_changed -> navigation_menu_changed _
                  menubar_changed -> menubar_changed _
                  native_resized -> native_resized _
                  native_range_changed -> native_range_changed _
                  catalog_sort_changed -> catalog_sort_changed
                  catalog_previous -> catalog_previous
                  catalog_next -> catalog_next
                  catalog_page_changed -> catalog_page_changed _
                  demo_page_previous -> demo_page_previous
                  demo_page_next -> demo_page_next
                  card_cancel -> card_cancel
                  card_apply -> card_apply
                  navigate_home -> navigate_home
                  navigate_library -> navigate_library
                  message_scroller_changed -> message_scroller_changed _
                  native_popover_changed -> native_popover_changed _
              row
                with
                  w=fill
                  gap=12.0
                  align=center
                button "Open dialog" #open-dialog -> open_dialog
                  with
                    h=36.0
                    @primary_action
                    @py-8px
                if dialog_result != "none"
                  text dialog_result size=12.0 @text-muted
                if !toast_visible
                  button "Show toast" -> show_toast
                    with
                      h=36.0
                      @secondary_action
                      @py-8px
                space w=fill h=1.0
                if toast_visible
                  Toast #migration-toast
                    with
                      title="Migration active"
                      description="This screen is generated by ui-lang."
                    button "×" #dismiss-toast -> dismiss_toast
                      with
                        label="Dismiss toast"
                        w=32.0
                        h=32.0
                        p=0.0
                      active bg=transparent text=toast_fg r=8.0
                      hovered bg=white/12 text=toast_fg r=8.0
                      pressed bg=white/18 text=toast_fg r=8.0
    layer
      Dialog
        Dialog.Header
          col gap=4.0
            text "Default dialog"
              with
                size=20.0
                @font-bold
                @text-fg
            text "The overlay, dismissal route, and focusable controls are declared in Ice."
              with
                size=13.0
                @text-muted
        Dialog.Body
          Alert
            with
              title="No Rust view code"
              description="The build script emits ordinary iced code at compile time."
        Dialog.Actions
          row
            with
              w=fill
              gap=8.0
              align=end
            space w=fill h=1.0
            button "Cancel" -> cancel_dialog
              with
                h=36.0
                @secondary_action
                @py-8px
            button "Continue" -> continue_dialog
              with
                h=36.0
                @primary_action
                @py-8px

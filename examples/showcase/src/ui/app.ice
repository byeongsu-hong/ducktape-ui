app Showcase
  title "ducktape-ui · Ice"
  id "dev.ducktape.ui.showcase"
  font "../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../assets/fonts/Geist-Bold.ttf"
  font "../../../../assets/fonts/Geist-Italic.ttf"
  font "../../../../assets/fonts/GeistMono-Regular.ttf"
  font "../../../../assets/fonts/GeistMono-Bold.ttf"
  font "../../../../assets/fonts/GeistMono-Italic.ttf"
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
use "../../../../crates/ui-lang-components/src/ice/default.ice"

component CompactFeature(title:str)
  box #root r=11.0 @panel
    col w=fill gap=12.0
      text title @section_title
      slot

component RetainedFeatures(panel_width:f64, stage_height:f64, compact:bool, virtual_list:VirtualListState, tree_view:TreeViewState, data_grid:DataGridState)
  emits
    virtual_list_changed(VirtualListEvent)
    tree_view_changed(TreeViewEvent)
    data_grid_changed(DataGridEvent)
  row
    with
      w=fill
      h=fill
      gap=12.0
    box w=panel_width h=fill
      col w=fill h=fill
        if compact
          CompactFeature #virtual-list-panel title="Virtual list"
            box w=fill h=stage_height
              extern virtual_list(virtual_list) #virtual-list -> emit(virtual_list_changed, _)
        if !compact
          VirtualList.Frame #virtual-list-panel-wide
            with
              title="Virtual list"
              description="Only visible fixed-height keyed rows cross the typed boundary."
              count=100000
            box w=fill h=stage_height
              extern virtual_list(virtual_list) #virtual-list -> emit(virtual_list_changed, _)
    box w=panel_width h=fill
      col w=fill h=fill
        if compact
          CompactFeature #tree-view-panel title="Tree view"
            box w=fill h=stage_height
              extern tree_view(tree_view) #tree-view -> emit(tree_view_changed, _)
        if !compact
          TreeView.Frame #tree-view-panel-wide
            with
              title="Tree view"
              description="Hierarchy, expansion, rename, and tree semantics stay retained."
              count=100000
            box w=fill h=stage_height
              extern tree_view(tree_view) #tree-view -> emit(tree_view_changed, _)
    box w=panel_width h=fill
      col w=fill h=fill
        if compact
          CompactFeature #data-grid-panel title="Data grid"
            box w=fill h=stage_height
              extern data_grid(data_grid) #data-grid -> emit(data_grid_changed, _)
        if !compact
          DataGrid.Frame #data-grid-panel-wide
            with
              title="Data grid"
              description="Fixed keyed rows and typed cells."
              rows=100000
              columns=16
            box w=fill h=stage_height
              extern data_grid(data_grid) #data-grid -> emit(data_grid_changed, _)

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

            if showcase_page == "components"
              row #view-switcher-components
                with
                  gap=4.0
                  p=4.0
                  @bg-accent
                  @rounded-lg
                button "Components" #show-components-selected -> show_components
                  with
                    checked=true
                    h=32.0
                    @secondary_action
                    @py-6px
                button "Retained data" #show-retained-data -> show_retained_data
                  with
                    checked=false
                    h=32.0
                    @ghost_action
                    @py-6px
            if showcase_page == "retained"
              row #view-switcher-retained
                with
                  gap=4.0
                  p=4.0
                  @bg-accent
                  @rounded-lg
                button "Components" #show-components -> show_components
                  with
                    checked=false
                    h=32.0
                    @ghost_action
                    @py-6px
                button "Retained data" #show-retained-data-selected -> show_retained_data
                  with
                    checked=true
                    h=32.0
                    @secondary_action
                    @py-6px

          if showcase_page == "components"
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
                    scratch_note=scratch_note
                    clicks=clicks
                    accepted=accepted
                    notifications=notifications
                    volume=volume
                    density=density
                    native_select_frameworks=["Ice", "iced", "Rust"]
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
                    scratch_submitted -> scratch_submitted _
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
                  space w=fill h=1.0

          if showcase_page == "retained"
            box #retained-screen
              with
                w=fill
                h=fill
                px=24.0
                pb=24.0
                align-x=start
                align-y=start
              responsive
                with
                  w=fill
                  h=fill
                  size=(feature_width, feature_height)
                col w=fill h=fill
                  if feature_width < 900.0
                    scroll
                      with
                        w=fill
                        h=fill
                        dir=horizontal
                        bar-w=8.0
                        scroller-w=6.0
                      RetainedFeatures #compact-feature-strip
                        with
                          panel_width=300.0
                          stage_height=(feature_height - 132.0)
                          compact=true
                          virtual_list=virtual_list
                          tree_view=tree_view
                          data_grid=data_grid
                        events
                          virtual_list_changed -> virtual_list_changed _
                          tree_view_changed -> tree_view_changed _
                          data_grid_changed -> data_grid_changed _
                  if feature_width >= 900.0
                    RetainedFeatures #wide-feature-strip
                      with
                        panel_width=((feature_width - 24.0) / 3.0)
                        stage_height=(feature_height - 132.0)
                        compact=false
                        virtual_list=virtual_list
                        tree_view=tree_view
                        data_grid=data_grid
                      events
                        virtual_list_changed -> virtual_list_changed _
                        tree_view_changed -> tree_view_changed _
                        data_grid_changed -> data_grid_changed _
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

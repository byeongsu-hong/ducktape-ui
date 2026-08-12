test app_behavior
  preset test
  viewport 1120 820
  target app = #app
  target scroller = app/catalog-scroll
  target grid = app/catalog-scroll/page/catalog-grid/root
  target buttons = grid/buttons/root
  target fields = grid/fields/root
  target primary = buttons/primary
  target email_input = fields/work-email
  target project_input = fields/project-url-field/root/project-url/root/slug-editor/project-slug
  expect text "ducktape-ui"
  expect clicks == 0
  click primary
  expect clicks == 1
  scroll-to scroller 0.0 400.0
  click project_input
  type "catalog"
  expect project_input.value == "catalog"
  click email_input
  type "tester@example"
  expect email_input.value == "tester@example"
  expect text "Controlled input"
  expect text "tester@example"
  key backspace
  expect email_input.value == "tester@exampl"
  dispatch catalog_sort_changed
  expect catalog_sort == "ascending"
  dispatch catalog_page_changed(5)
  expect catalog_page == 4
  dispatch density_changed("compact")
  expect density == "compact"

test language_feature_contracts
  preset test
  viewport 640 480
  mount
    col gap=12.0
      TabsDemo #tabs
      PaginationDemo page=demo_page #pagination
        events
          previous -> demo_page_previous
          next -> demo_page_next
      extern aspect_ratio_demo() #aspect
  target show_code = #tabs/show-code
  target show_preview = #tabs/show-preview
  target show_preview_selected = #tabs/show-preview-selected
  target show_code_selected = #tabs/show-code-selected
  target pagination_next = #pagination/next
  expect show_code.height ~= 32.0
  expect show_code.text_size ~= 12.5
  expect show_code.text_y ~= show_code.y + (show_code.height - show_code.text_height) / 2.0
  expect show_preview_selected.height ~= 32.0
  expect show_preview_selected.text_y ~= show_preview_selected.y + (show_preview_selected.height - show_preview_selected.text_height) / 2.0
  expect a11y show_code name "Code"
  expect a11y show_code checked false
  expect a11y show_preview_selected checked true
  expect pagination_next.height ~= 32.0
  expect pagination_next.text_size ~= 12.5
  expect pagination_next.text_y ~= pagination_next.y + (pagination_next.height - pagination_next.text_height) / 2.0
  expect a11y pagination_next name "Next"
  click show_code
  expect text "button \"Save\" @primary_action -> save"
  expect demo_page == 1
  expect a11y show_code_selected checked true
  expect a11y show_preview checked false
  expect show_code_selected.height ~= 32.0
  expect show_code_selected.text_y ~= show_code_selected.y + (show_code_selected.height - show_code_selected.text_height) / 2.0
  click pagination_next
  expect demo_page == 2
  expect text "16 / 9"

test collapsible_button_exposes_expanded_state
  preset test
  viewport 520 420
  mount
    CollapsibleDemo #collapsible
  target deployment_toggle = #collapsible/deployment-toggle
  expect a11y deployment_toggle expanded false
  click deployment_toggle
  expect a11y deployment_toggle expanded true

test generated_control_accessibility
  preset test
  viewport 520 420
  mount
    col p=20.0 gap=12.0
      toggler "Feature" #toggle -> accepted_changed _
        with
          label="Feature toggle"
          description="Controls feature state"
          checked=accepted
      slider volume #slider min=0.0 max=100.0 -> volume_changed _
      progress volume #progress
      pick ["Ice", "iced", "Rust"] native_select_framework #pick -> framework_changed _
        with
          hint="Framework"
      combo combobox_frameworks searched_framework "Framework search" #combo -> searched_framework_changed _
      editor #editor <-> textarea_notes
        with
          hint="Notes"
          disabled=true
          h=72.0
  target toggle = #toggle
  target slider = #slider
  target progress = #progress
  target pick = #pick
  target combo = #combo
  target editor = #editor
  expect a11y toggle role "switch"
  expect a11y toggle name "Feature toggle"
  expect toggle.accessibility_description == "Controls feature state"
  expect a11y toggle checked false
  expect a11y toggle action click
  expect a11y toggle action focus
  a11y activate toggle
  expect accepted
  expect a11y slider role "slider"
  expect a11y slider name "Slider"
  expect a11y slider value "58"
  expect a11y slider action focus
  expect a11y progress role "progress-indicator"
  expect a11y progress name "Progress"
  expect a11y progress value "58"
  expect a11y progress action focus false
  dispatch framework_changed("iced")
  expect a11y pick role "combo-box"
  expect a11y pick name "Framework"
  expect a11y pick value "iced"
  expect a11y pick action focus
  dispatch searched_framework_changed("Rust")
  dispatch framework_registered("wgpu-next")
  expect a11y combo role "combo-box"
  expect a11y combo name "Framework search"
  expect a11y combo value "Rust"
  expect a11y combo action focus
  expect a11y editor role "multiline-text-input"
  expect a11y editor name "Notes"
  expect a11y editor value "Default multiline editor"
  expect a11y editor disabled true
  expect a11y editor action focus false
  capture accessibility_contract

test catalog_layout
  preset test
  viewport 1120 1200
  target app = #app
  target virtual_panel = app/compact-feature-strip/virtual-list-panel/root
  target virtual_widget = virtual_panel/virtual-list
  target tree_panel = app/compact-feature-strip/tree-view-panel/root
  target tree_widget = tree_panel/tree-view
  target data_grid_panel = app/compact-feature-strip/data-grid-panel/root
  target data_grid_widget = data_grid_panel/data-grid
  target scroller = app/catalog-scroll
  target page = scroller/page
  target grid = page/catalog-grid/root
  target buttons = grid/buttons/root
  target badges = grid/badges/root
  target fields = grid/fields/root
  target email_input = fields/work-email
  target framework_picker = fields/framework-field/root/framework-picker
  target project_group = fields/project-url-field/root/project-url/root
  target primary = buttons/primary
  target secondary = buttons/secondary
  target outline = buttons/outline
  target ghost = buttons/ghost
  target destructive = buttons/destructive
  target disabled = buttons/disabled
  target advanced_panel = grid/advanced-selection-panel/root
  target advanced_content = advanced_panel/advanced-selection-content
  target layout_panel = grid/layout-primitives-panel/root
  target layout_content = layout_panel/layout-primitives-content
  target identity_panel = grid/identity-calendar-panel/root
  target identity_content = identity_panel/identity-calendar-content
  target chart_panel = grid/chart-panel/root
  target chart_content = chart_panel/chart-content
  target modal_panel = grid/modal-contracts-panel/root
  target modal_content = modal_panel/modal-contracts-content
  target data_panel = grid/data-table-panel/root
  target data_content = data_panel/data-table-content
  target edge_panel = grid/edge-panels/root
  target edge_stage = edge_panel/edge-stage/root
  target native_panel = grid/native-escape-hatches/root
  target native_stage = native_panel/native-stage/root
  expect app.width ~= 1120.0
  expect app.height ~= 1200.0
  expect page.x ~= app.x
  expect page.width ~= app.width - 16.0
  expect grid.x ~= page.x + 24.0
  expect grid.width ~= page.width - 48.0
  expect buttons.y ~= badges.y
  expect badges.x ~= buttons.right + 20.0
  expect primary.height ~= 36.0
  expect primary.width ~= 128.0
  expect secondary.height ~= primary.height
  expect secondary.width ~= primary.width
  expect outline.height ~= primary.height
  expect outline.width ~= primary.width
  expect ghost.height ~= primary.height
  expect ghost.width ~= primary.width
  expect destructive.height ~= primary.height
  expect destructive.width ~= primary.width
  expect disabled.height ~= primary.height
  expect disabled.width ~= primary.width
  expect primary.text_size ~= 12.5
  expect secondary.text_size ~= primary.text_size
  expect outline.text_size ~= primary.text_size
  expect ghost.text_size ~= primary.text_size
  expect destructive.text_size ~= primary.text_size
  expect disabled.text_size ~= primary.text_size
  expect primary.font.family == family.named("Geist")
  expect email_input.font.family == family.named("Geist")
  expect primary.font.weight == weight.semibold()
  expect secondary.font == primary.font
  expect outline.font == primary.font
  expect ghost.font == primary.font
  expect destructive.font == primary.font
  expect disabled.font == primary.font
  expect primary.background == background.color(color.rgb8(38, 37, 31))
  expect primary.text_y ~= primary.y + (primary.height - primary.text_height) / 2.0
  expect secondary.text_y ~= secondary.y + (secondary.height - secondary.text_height) / 2.0
  expect outline.text_y ~= outline.y + (outline.height - outline.text_height) / 2.0
  expect ghost.text_y ~= ghost.y + (ghost.height - ghost.text_height) / 2.0
  expect destructive.text_y ~= destructive.y + (destructive.height - destructive.text_height) / 2.0
  expect disabled.text_y ~= disabled.y + (disabled.height - disabled.text_height) / 2.0
  expect a11y primary name "Primary"
  expect a11y secondary name "Secondary"
  expect a11y outline name "Outline"
  expect a11y ghost name "Ghost"
  expect a11y destructive name "Destructive"
  expect a11y disabled name "Disabled"
  expect a11y disabled disabled true
  move primary
  expect primary.background == background.color(color.rgb8(50, 47, 40))
  press primary
  expect primary.background == background.color(color.rgba8(38, 37, 31, 0.8))
  release
  expect email_input.height ~= 36.2
  expect framework_picker.height ~= email_input.height
  expect project_group.height ~= email_input.height
  expect advanced_content.y ~= layout_content.y
  expect layout_content.height > advanced_content.height - 1.0
  expect layout_content.height < advanced_content.height + 1.0
  expect identity_content.y ~= chart_content.y
  expect modal_content.y ~= data_content.y
  expect data_content.height > modal_content.height - 12.0
  expect edge_stage.y ~= native_stage.y
  expect edge_stage.height ~= native_stage.height
  snap-end scroller
  expect fields.x ~= buttons.x
  expect fields.y > buttons.bottom
  resize 720 560
  expect app.width ~= 720.0
  expect app.height ~= 560.0
  expect virtual_panel.width > 280.0
  expect virtual_panel.height < 210.0
  expect tree_panel.height ~= virtual_panel.height
  expect data_grid_panel.height ~= virtual_panel.height
  expect virtual_widget.bottom < virtual_panel.bottom
  expect tree_widget.bottom < tree_panel.bottom
  expect data_grid_widget.bottom < data_grid_panel.bottom
  expect virtual_widget.height ~= 96.0
  expect tree_widget.height ~= virtual_widget.height
  expect data_grid_widget.height ~= virtual_widget.height
  expect scroller.height > 140.0
  expect page.x ~= app.x
  expect page.width ~= app.width - 16.0
  expect grid.x ~= page.x + 24.0
  expect grid.width ~= page.width - 48.0
  expect badges.x ~= buttons.x
  expect badges.y > buttons.bottom
  expect fields.x ~= buttons.x
  expect fields.y > badges.bottom

test dialog_preserves_catalog_position
  preset test
  viewport 720 560
  target app = #app
  target scroller = app/catalog-scroll
  target page = scroller/page
  target open_dialog = page/open-dialog
  snap-end scroller
  expect scroller.scroll_y > 9000.0
  click open_dialog
  expect dialog_open
  expect scroller.scroll_y > 9000.0
  dispatch close_dialog
  expect !dialog_open
  expect scroller.scroll_y > 9000.0

// A modal is the part of the screen the reader is looking at, so what it says
// has to be answerable. Its layer draws through iced's overlay rather than in
// place, and a question that never reaches the overlay answers "missing" for
// ink that is plainly there — harmless-looking in the positive form, which
// merely cannot be written, and dangerous in the negative one, which passes.
//
// Both sides, and in this order: the heading is genuinely absent while the
// layer is unbuilt, and the press is what makes the absence worth asserting.
test dialog_text_is_visible_while_the_layer_is_open
  preset test
  viewport 720 560
  target app = #app
  target scroller = app/catalog-scroll
  target page = scroller/page
  target open_dialog = page/open-dialog
  snap-end scroller
  expect no text "Default dialog"
  click open_dialog
  expect dialog_open
  expect text "Default dialog"
  // Nested inside a component on the layer, not just its outermost text.
  expect text "No Rust view code"
  dispatch close_dialog
  expect no text "Default dialog"

test focused_component_feedback
  preset test
  viewport 560 280
  mount
    col gap=20.0
      box #otp-group
        with
          w=fill
          h=48.0
          p=4.0
        extern input_otp("showcase-test-otp", otp, false, false) -> otp_changed _
      DemoStage height=188.0 padding=8.0 #message-stage
        extern message_scroller(message_scroller) -> message_scroller_changed _
  target otp_group = #otp-group
  target message_stage = #message-stage/root
  click-at 40.0 28.0
  type "1"
  capture otp_after_first_digit
  type "2"
  capture otp_after_second_digit
  expect otp == "12"
  click-at 66.0 28.0
  capture otp_second_slot_caret
  type "9"
  expect otp == "19"
  click-at 66.0 28.0
  type "2"
  expect otp == "12"
  click-at 220.0 150.0
  capture message_pointer_focus
  expect otp_group.width > 240.0
  expect message_stage.height ~= 188.0

test virtual_list_native_boundary
  preset test
  viewport 620 360
  mount
    VirtualList.Frame #virtual-list-frame
      with
        title="Virtual list"
        description="Only the mounted fixed-height rows cross the typed boundary."
        count=100000
      box #virtual-list-stage w=fill h=252.0
        extern virtual_list(virtual_list) -> virtual_list_changed _
  target list_stage = #virtual-list-frame/root/virtual-list-stage
  expect list_stage.height ~= 252.0
  click list_stage
  idle
  key end
  idle
  expect text "mounted 99989..100000 · Selected #99999"
  expect text "#99999"
  capture virtual_list_end_selection

test log_timeline_native_boundary
  preset test
  viewport 620 360
  mount
    LogTimeline.Frame #log-timeline-frame
      with
        title="Log timeline"
        description="Append-only fixed-height logs reuse VirtualList selection, keyboard, inspection, and accessibility."
      col w=fill gap=8.0
        row gap=8.0
          button "Append log" #append-log -> append_log
          button "Resume tail" #resume-log-tail -> resume_log_tail
        box #log-timeline-stage w=fill h=224.0
          extern log_timeline(log_timeline) -> log_timeline_changed _
  target log_stage = #log-timeline-frame/root/log-timeline-stage
  target append_log_button = #log-timeline-frame/root/append-log
  target resume_log_button = #log-timeline-frame/root/resume-log-tail
  expect log_stage.height ~= 224.0
  click log_stage
  idle
  key home
  idle
  expect text "paused · 0 unread"
  expect text "000000"
  click append_log_button
  idle
  expect text "paused · 1 unread"
  click resume_log_button
  idle
  expect text "following · 0 unread"
  expect text "100000"
  capture log_timeline_tail_resume

test tree_view_native_boundary
  preset test
  viewport 620 420
  mount
    TreeView.Frame #tree-view-frame
      with
        title="Tree view"
        description="Only visible preorder nodes cross the typed boundary."
        count=100000
      col gap=4.0
        row gap=4.0
          button "Rename selected" #rename-tree-node -> begin_tree_rename
          button "Cancel rename" #cancel-tree-rename -> cancel_tree_rename
        box #tree-view-stage w=fill h=252.0
          extern tree_view(tree_view) -> tree_view_changed _
  target tree_stage = #tree-view-frame/root/tree-view-stage
  target rename_one = #tree-view-frame/root/rename-tree-node
  target cancel_rename_one = #tree-view-frame/root/cancel-tree-rename
  expect tree_stage.height ~= 252.0
  click tree_stage
  idle
  key home
  key arrow-left
  idle
  expect text "100 visible / 100000 logical"
  expect text "Selected 0"
  key arrow-right
  idle
  expect text "1099 visible / 100000 logical"
  key arrow-right
  idle
  expect text "Selected 1"
  click rename_one
  idle
  replace "Renamed file"
  key enter
  idle
  expect text "Renamed file"
  key arrow-left
  idle
  expect text "Selected 0"
  key arrow-right
  idle
  click rename_one
  idle
  replace "Discarded rename"
  click cancel_rename_one
  idle
  expect text "Renamed file"
  key arrow-left
  idle
  expect text "Selected 0"
  capture tree_view_hierarchical_navigation

test data_grid_native_boundary
  preset test
  viewport 760 460
  mount
    DataGrid.Frame #data-grid-frame
      with
        title="Data grid"
        description="Only mounted fixed rows cross the typed extern boundary."
        rows=100000
        columns=16
      box #data-grid-stage w=fill h=300.0
        extern data_grid(data_grid) -> data_grid_changed _
  target grid_stage = #data-grid-frame/root/data-grid-stage
  expect grid_stage.height ~= 300.0
  expect text "Repository item 00000"
  click grid_stage
  idle
  key home
  key f2
  idle
  replace "Renamed grid cell"
  key enter
  idle
  expect text "Renamed grid cell"
  key end
  key arrow-down
  idle
  capture data_grid_keyboard_editing

test modal_trigger_is_only_the_action
  preset test
  viewport 560 240
  mount
    DemoStage height=190.0 padding=8.0 #modal-stage
      extern alert_dialog(alert_dialog) -> alert_dialog_changed _
  click-at 90.0 92.0
  idle
  expect !alert_dialog_is_open(alert_dialog)
  click-at 455.0 92.0
  idle
  expect alert_dialog_is_open(alert_dialog)
  expect text "Delete this component?"
  capture modal_open_from_action

test catalog_full_scroll_visuals
  preset test
  viewport 1120 820
  target app = #app
  target scroller = app/catalog-scroll
  target dropdown_trigger = app/catalog-scroll/page/catalog-grid/root/advanced-selection-panel/root/advanced-selection-content/dropdown-trigger
  target navigation_trigger = app/catalog-scroll/page/catalog-grid/root/navigation-shell/root/navigation-trigger
  scroll-to scroller 0.0 2500.0
  click dropdown_trigger
  idle
  capture dropdown_categories
  expect dropdown_menu_is_open(dropdown)
  key escape
  scroll-to scroller 0.0 2950.0
  capture identity_and_smooth_chart
  scroll-to scroller 0.0 3700.0
  capture modal_and_data_table
  scroll-to scroller 0.0 4400.0
  capture messages_and_edge_panels
  scroll-to scroller 0.0 5000.0
  click navigation_trigger
  idle
  expect navigation_menu_is_open(navigation_menu)
  capture navigation_menu_and_shell
  key escape
  resize 720 560
  snap-end scroller
  capture narrow_navigation_shell

test shared_component_font_contracts
  preset test
  viewport 520 240
  mount
    col p=24.0 gap=16.0
      Attachment name="component-contract.ice" meta="4.2 KB · Ice source" #attachment
      Breadcrumb current="Components" #breadcrumb
        text "Home"
      Tooltip label="Open the command palette" #tooltip
        button "Hover me" #tooltip-trigger -> clicked
  target attachment_menu = #attachment/root/menu
  target breadcrumb_separator = #breadcrumb/root/separator
  target breadcrumb_current = #breadcrumb/root/current
  expect attachment_menu.font.family == family.named("Geist")
  expect breadcrumb_separator.font.family == family.named("Geist")
  expect breadcrumb_current.font.family == family.named("Geist")

test dropdown_categories_open_from_pointer
  preset test
  viewport 420 360
  mount
    box p=24.0
      extern dropdown_menu(dropdown) -> dropdown_changed _
  click-at 70.0 42.0
  idle
  expect dropdown_menu_is_open(dropdown)
  capture dropdown_pointer_open

test smooth_chart_surface
  preset test
  viewport 560 620
  mount
    box p=24.0
      extern chart(none) -> chart_hovered _
  capture smooth_chart

test embedded_component_scrollbars
  preset test
  viewport 420 260
  mount
    box p=24.0
      extern command(command) -> command_changed _
  capture command_embedded_scrollbar

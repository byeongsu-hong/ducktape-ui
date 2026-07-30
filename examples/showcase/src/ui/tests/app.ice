test app_behavior
  preset test
  viewport 1120 820
  target app = #app
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
      AspectRatioDemo #aspect
  target show_code = #tabs/show-code
  target pagination_next = #pagination/next
  expect show_code.height ~= 32.0
  expect show_code.text_size ~= 12.5
  expect show_code.text_y ~= show_code.y + (show_code.height - show_code.text_height) / 2.0
  expect a11y show_code name "Code"
  expect pagination_next.height ~= 32.0
  expect pagination_next.text_size ~= 12.5
  expect pagination_next.text_y ~= pagination_next.y + (pagination_next.height - pagination_next.text_height) / 2.0
  expect a11y pagination_next name "Next"
  click show_code
  expect text "button \"Save\" @primary_action -> save"
  expect demo_page == 1
  click pagination_next
  expect demo_page == 2
  expect text "16 / 9"

test catalog_layout
  preset test
  viewport 1120 820
  target app = #app
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
  target toast = page/migration-toast/root
  target dismiss_toast = toast/dismiss-toast
  expect app.width ~= 1120.0
  expect app.height ~= 820.0
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
  dispatch show_toast
  expect toast_visible
  snap-end scroller
  expect dismiss_toast.width ~= 32.0
  expect dismiss_toast.height ~= 32.0
  expect a11y dismiss_toast role "button"
  expect a11y dismiss_toast name "Dismiss toast"
  expect fields.x ~= buttons.x
  expect fields.y > buttons.bottom
  resize 720 560
  expect app.width ~= 720.0
  expect app.height ~= 560.0
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
  scroll-to scroller 0.0 2500.0
  click-at 370.0 220.0
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
  scroll-to scroller 0.0 5320.0
  click-at 160.0 266.0
  idle
  expect navigation_menu_is_open(navigation_menu)
  capture navigation_menu_and_shell
  key escape
  resize 720 560
  snap-end scroller
  capture narrow_navigation_shell

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

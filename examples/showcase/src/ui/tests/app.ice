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
  expect pagination_next.height ~= 32.0
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
  expect page.width ~= app.width
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
  expect page.width ~= app.width
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

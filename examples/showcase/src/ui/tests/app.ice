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
  target page = app/catalog-scroll/page
  target grid = page/catalog-grid/root
  target buttons = grid/buttons/root
  target badges = grid/badges/root
  target fields = grid/fields/root
  expect app.width ~= 1120.0
  expect app.height ~= 820.0
  expect page.x ~= app.x
  expect page.width ~= app.width
  expect grid.x ~= page.x + 24.0
  expect grid.width ~= page.width - 48.0
  expect buttons.y ~= badges.y
  expect badges.x ~= buttons.right + 20.0
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

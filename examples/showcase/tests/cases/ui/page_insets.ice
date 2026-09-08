app PageInsets
  text-size 14

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

view
  Page
    text "Page insets"

test page_custom_insets_preserve_panel_padding
  viewport 360 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page padding=12.0
      col #content w=fill h=fill
        Panel #panel title="Profile"
          text "Your profile information" #body w=fill wrap=word @body
  target page = #page/root
  target content = page/content
  target panel = content/panel/root
  target title = panel/title
  target body = panel/body
  expect page.width ~= 360.0
  expect page.height ~= 300.0
  expect content.left ~= 12.0
  expect content.top ~= 12.0
  expect content.right ~= 348.0
  expect content.bottom ~= 288.0
  expect panel.left ~= 12.0
  expect panel.top ~= 12.0
  expect panel.right ~= 348.0
  expect panel.bottom <= 288.0
  expect title.left ~= panel.left + 20.0
  expect title.top ~= panel.top + 20.0
  expect body.left ~= panel.left + 20.0
  expect body.right <= panel.right - 20.0
  expect body.bottom ~= panel.bottom - 20.0
  capture custom_insets

test page_allows_explicit_edge_to_edge
  viewport 360 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page padding=0.0
      box #content w=fill h=fill bg=surface
        text "Full bleed" @body
  target page = #page/root
  target content = page/content
  expect content.left ~= 0.0
  expect content.top ~= 0.0
  expect content.right ~= 360.0
  expect content.bottom ~= 300.0
  capture full_bleed

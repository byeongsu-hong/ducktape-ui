app HeaderDefaults
  text-size 14

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

view
  PageHeader title="Settings" description=""

test page_header_empty_description_has_no_row
  viewport 360 300
  mount
    col w=fill
      PageHeader #header title="Settings"
      text "Settings" #reference @display
  target header = #header/root
  target reference = #reference
  expect header.height ~= reference.height
  capture header_empty

test panel_empty_description_has_no_row
  viewport 360 300
  mount
    col w=fill
      Panel #panel title="Profile"
        text "Body" #body @body
      text "Profile" #reference @section_title
  target panel = #panel/root
  target body = panel/body
  target reference = #reference
  expect panel.height ~= reference.height + body.height + 52.0
  capture panel_empty

test headers_wrap_long_copy
  viewport 280 800
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #layout w=fill p=16.0 gap=16.0
      PageHeader #header title="Settings for your personal and shared workspaces" description="Choose how your account appears to the people you collaborate with across all your workspaces."
      Panel #panel title="Profile and notification preferences for shared projects" description="These preferences apply to activity in all your shared projects and can be changed at any time."
        text "Your settings stay within the available space." #body w=fill wrap=word @body
  target layout = #layout
  target header = layout/header/root
  target title = header/title
  target description = header/description
  target panel = layout/panel/root
  target panel_title = panel/title
  target panel_description = panel/description
  target body = panel/body
  expect header.right <= layout.right - 16.0
  expect title.text_height > 30.0
  expect title.text_y + title.text_height <= title.bottom
  expect description.top > title.bottom
  expect description.text_y + description.text_height <= header.bottom
  expect panel.top >= header.bottom + 16.0
  expect panel_title.text_height > 30.0
  expect panel_title.right <= panel.right - 20.0
  expect panel_description.top > panel_title.bottom
  expect panel_description.text_y + panel_description.text_height <= panel_description.bottom
  expect body.top >= panel_description.bottom + 12.0
  expect body.bottom <= panel.bottom - 20.0
  capture long_copy

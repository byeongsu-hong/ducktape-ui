app Settings
  title "Workspace preferences"
  text-size 14
  font "../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../assets/fonts/Geist-Bold.ttf"
  window
    size 960 820
    min-size 360 320
    position centered

font geist family="Geist" default=true

use "../../../../crates/ui-lang-components/src/ice/default.ice"

state
  explanation = "Manage your profile and workspace."
  name = "Alex Morgan"
  email = "alex@example.com"
  workspace = "Personal workspace"
  notifications = true
  error = ""
  saved = false

preset empty_explanation
  state
    explanation = ""

on save
  saved = false
  error = "Enter an address your team can use to reach you about shared projects and account recovery."
  return if empty(trim(email))
  error = ""
  saved = true

on notifications_changed(next)
  notifications = next

view
  col #screen w=fill h=fill
    box w=fill align-x=center
      box
        with
          w=fill
          max-w=640.0
          p=16.0
        PageHeader #heading title="Workspace preferences" description=explanation
    Form #settings padding=16.0
      col w=fill gap=16.0
        FormSection #profile title="Profile" padding=16.0
          col w=fill gap=12.0
            TextField #name value<->name label="Display name"
            TextField #email value<->email label="Email address" error=error
        FormSection #preferences title="Workspace" padding=16.0
          col w=fill gap=12.0
            TextField #workspace value<->workspace
              with
                label="Display name for your personal and shared workspaces"
                radius=4.0
                padding=14.0
                description="Shown in the workspace menu."
            Field #notifications
              with
                label="Keep me up to date"
                description="Receive updates about shared projects."
              checkbox "Project notifications" #toggle -> notifications_changed _
                with
                  checked=notifications
    box w=fill align-x=center
      box
        with
          w=fill
          max-w=640.0
          p=16.0
        row
          with
            w=fill
            gap=12.0
            align=center
          button "Save changes" #save @primary_action -> save
          if saved
            text "Changes saved" #saved live=polite @body

test settings_narrow_layout
  viewport 360 820
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #screen/settings/root/content
  target profile = content/profile/root
  target name_field = profile/name/field/root
  target input = name_field/input
  target preferences = content/preferences/root
  target workspace_field = preferences/workspace/field/root
  target label = workspace_field/label
  expect content.width <= 360.0
  expect input.width ~= name_field.width
  expect input.right <= content.right
  expect label.text_height > 30.0
  expect label.text_y + label.text_height <= label.bottom
  expect workspace_field.right <= preferences.right
  expect workspace_field.bottom <= preferences.bottom
  capture narrow

test settings_custom_input
  viewport 960 820
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #screen/settings/root/content
  target input = content/preferences/root/workspace/field/root/input
  target default_input = content/profile/root/name/field/root/input
  target toggle = content/preferences/root/notifications/root/toggle
  target save = #screen/save
  expect save.visible
  expect save.bottom ~= 804.0
  expect content.width ~= 640.0
  expect input.border.radius == radius(4.0)
  expect input.height ~= default_input.height + 6.0
  expect a11y input name "Display name for your personal and shared workspaces"
  click input
  replace "Studio"
  key end
  type " team"
  expect workspace == "Studio team"
  expect input.focused
  expect input.border.width ~= 2.0
  key tab
  expect toggle.focused
  key space
  expect !notifications
  key tab
  expect save.focused
  key enter
  expect saved
  expect text "Changes saved"
  capture customized

test settings_error_layout
  viewport 360 820
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #screen/settings/root/content
  target email_field = content/profile/root/email/field/root
  target email_input = email_field/input
  target save = #screen/save
  target preferences = content/preferences/root
  target error_text = email_field/error
  target form = #screen/settings/root
  target name_input = content/profile/root/name/field/root/input
  target workspace_input = preferences/workspace/field/root/input
  target toggle = preferences/notifications/root/toggle
  click name_input
  replace "Sam"
  click workspace_input
  replace "Research"
  click toggle
  click save
  expect saved
  click email_input
  replace ""
  click save
  expect !saved
  expect error_text.text_height > 30.0
  expect error_text.text_y + error_text.text_height <= error_text.bottom
  expect error_text.right <= email_field.right
  expect error_text.bottom <= email_field.bottom
  expect preferences.top > error_text.bottom
  capture error
  window resize 360 320
  scroll-to form 0.0 160.0
  expect save.visible
  expect error_text.top - form.scroll_y >= form.top
  expect error_text.bottom - form.scroll_y <= form.bottom
  capture error_short
  click email_input
  replace "team@example.com"
  key tab
  expect workspace_input.focused
  key tab
  key tab
  expect save.focused
  key enter
  expect saved
  expect error == ""
  expect email == "team@example.com"
  expect name == "Sam"
  expect workspace == "Research"
  expect !notifications
  expect text "Changes saved"
  capture corrected

test settings_short_window_scroll
  viewport 360 320
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target form = #screen/settings/root
  target save = #screen/save
  target last = form/content/preferences/root/notifications/root/toggle
  expect save.visible
  expect save.bottom ~= 304.0
  expect form.bottom <= save.top - 16.0
  expect !last.visible
  capture initial
  move form
  wheel 0.0 -10000.0
  expect form.scroll_y > 0.0
  expect last.visible
  expect last.top - form.scroll_y >= form.top
  expect last.bottom - form.scroll_y <= form.bottom - 16.0
  expect save.visible
  expect save.bottom ~= 304.0
  click last
  expect !notifications
  key tab
  expect save.focused
  key enter
  expect saved
  expect text "Changes saved"
  expect save.bottom ~= 304.0
  expect last.visible
  expect last.bottom - form.scroll_y <= form.bottom - 16.0
  capture scrolled

test settings_empty_explanation
  preset empty_explanation
  viewport 360 320
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target header = #screen/heading/root
  target title = header/title
  target save = #screen/save
  expect header.bottom ~= title.bottom
  expect save.visible
  expect save.bottom ~= 304.0
  capture empty_explanation

test settings_empty_help_has_no_gap
  viewport 360 400
  mount
    Form #form
      FormSection #section title="Profile"
        TextField #name label="Name" value<->name
  target field = #form/root/content/section/root/name/field/root
  target label = field/label
  target input = field/input
  expect input.top ~= label.bottom + 8.0
  expect field.bottom ~= input.bottom
  expect a11y input name "Name"
  click input
  replace "Sam"
  expect name == "Sam"

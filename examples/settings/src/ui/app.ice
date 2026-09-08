app Settings
  title "Account settings"
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
  name = "Alex Morgan"
  email = "alex@example.com"
  workspace = "Personal workspace"
  notifications = true
  error = ""
  saved = false

on save
  saved = false
  error = "Enter an address your team can use to reach you about shared projects and account recovery."
  return if empty(trim(email))
  error = ""
  saved = true

on notifications_changed(next)
  notifications = next

view
  Form #settings
    col w=fill gap=24.0
      PageHeader title="Account settings" description="Your profile and preferences, in one place."
      FormSection #profile
        with
          title="Profile"
          description="Choose how you appear to the people you work with."
        col w=fill gap=20.0
          TextField #name value<->name
            with
              label="Display name"
              description="Use the name your teammates know you by."
          TextField #email value<->email
            with
              label="Email address"
              description="Used for account notifications."
              error=error
      FormSection #preferences
        with
          title="Preferences"
          description="Make this workspace feel like yours."
        col w=fill gap=20.0
          TextField #workspace value<->workspace
            with
              label="Display name for your personal and shared workspaces"
              radius=4.0
              padding=14.0
              description="Shown in the workspace menu. You can change it at any time."
          Field #notifications
            with
              label="Keep me up to date"
              description="Receive updates about activity in shared projects."
            checkbox "Project notifications" #toggle -> notifications_changed _
              with
                checked=notifications
      col w=fill gap=10.0
        button "Save changes" #save @primary_action -> save
        if saved
          text "Changes saved" #saved @body

test settings_narrow_layout
  viewport 360 1000
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #settings/root/content
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
  viewport 960 1000
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #settings/root/content
  target input = content/preferences/root/workspace/field/root/input
  target default_input = content/profile/root/name/field/root/input
  target toggle = content/preferences/root/notifications/root/toggle
  target save = content/save
  expect content.width ~= 640.0
  expect input.border.radius == radius(4.0)
  expect input.height ~= default_input.height + 6.0
  expect a11y input name "Display name for your personal and shared workspaces"
  click input
  replace "Studio"
  expect workspace == "Studio"
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
  viewport 360 1000
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #settings/root/content
  target email_field = content/profile/root/email/field/root
  target email_input = email_field/input
  target save = content/save
  target preferences = content/preferences/root
  target error_text = email_field/error
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

test settings_short_window_scroll
  viewport 360 400
  target form = #settings/root
  target save = #settings/root/content/save
  expect form.height ~= 400.0
  expect !save.visible
  scroll-to form 0.0 10000.0
  expect save.visible
  click save
  expect saved
  capture scrolled

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

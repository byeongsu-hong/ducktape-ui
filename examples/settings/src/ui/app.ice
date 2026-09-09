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
  name = "Alex Morgan"
  email = "alex@example.com"
  workspace = "Personal workspace"
  notifications = true
  explanation = "Profile, workspace and notifications."
  error = ""
  saved = false

preset no_explanation
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
          px=16.0
          py=16.0
        PageHeader #heading title="Workspace preferences" description=explanation
    Form #settings padding=16.0
      col w=fill gap=16.0
        FormSection #profile title="Profile" padding=16.0
          col w=fill gap=16.0
            TextField #name value<->name
              with
                label="Display name"
                description="The name your teammates see."
            TextField #email value<->email
              with
                label="Email address"
                description="Used for account notifications."
                error=error
        FormSection #preferences title="Preferences" padding=16.0
          col w=fill gap=16.0
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
          px=16.0
          py=12.0
        row #actions
          with
            w=fill
            gap=12.0
            align=center
          button "Save changes" #save @primary_action -> save
          if saved
            text "Changes saved" #saved @body
          if !empty(error)
            text "Check email above" #invalid @caption text-danger

test settings_wide_fixed_actions
  viewport 960 820
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target screen = #screen
  target heading = screen/heading/root
  target form = screen/settings/root
  target content = form/content
  target save = screen/actions/save
  target last = content/preferences/root/notifications/root/toggle
  expect content.width ~= 640.0
  expect save.visible
  expect save.visible_height ~= save.height
  expect save.bottom <= 808.0
  expect form.top >= heading.bottom
  expect form.bottom <= save.top
  move form
  wheel 0.0 -10000.0
  expect last.visible
  expect last.bottom <= form.bottom
  expect save.visible
  expect save.visible_height ~= save.height
  click save
  expect saved
  expect text "Changes saved"
  capture wide

test settings_short_window_scroll
  viewport 360 320
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target screen = #screen
  target heading = screen/heading/root
  target form = screen/settings/root
  target save = screen/actions/save
  target last = form/content/preferences/root/notifications/root/toggle
  expect screen.height ~= 320.0
  expect save.visible
  expect save.visible_height ~= save.height
  expect save.bottom <= 308.0
  expect form.top >= heading.bottom
  expect form.bottom <= save.top
  expect !last.visible
  capture short_initial
  move form
  wheel 0.0 -10000.0
  expect form.scroll_y > 0.0
  expect last.visible
  expect last.visible_height ~= last.height
  expect save.visible
  expect save.visible_height ~= save.height
  expect save.bottom <= 308.0
  click last
  expect !notifications
  click save
  expect saved
  expect !notifications
  expect text "Changes saved"
  capture short_end

test settings_custom_input
  viewport 960 820
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target form = #screen/settings/root
  target content = form/content
  target input = content/preferences/root/workspace/field/root/input
  target default_input = content/profile/root/name/field/root/input
  target toggle = content/preferences/root/notifications/root/toggle
  target save = #screen/actions/save
  expect input.border.radius == radius(4.0)
  expect input.height ~= default_input.height + 6.0
  expect a11y input name "Display name for your personal and shared workspaces"
  move form
  wheel 0.0 -10000.0
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
  expect workspace == "Studio"
  expect !notifications
  expect text "Changes saved"
  capture customized

test settings_error_recovery
  viewport 360 320
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target form = #screen/settings/root
  target content = form/content
  target name_input = content/profile/root/name/field/root/input
  target email_field = content/profile/root/email/field/root
  target email_input = email_field/input
  target error_text = email_field/error
  target workspace_input = content/preferences/root/workspace/field/root/input
  target save = #screen/actions/save
  focus name_input
  replace "Sam Rivera"
  scroll-to form 0.0 360.0
  expect workspace_input.visible_height ~= workspace_input.height
  focus workspace_input
  replace "Design studio"
  scroll-to form 0.0 180.0
  expect email_input.visible_height ~= email_input.height
  focus email_input
  replace ""
  click save
  expect !saved
  expect save.visible
  expect save.visible_height ~= save.height
  expect text "Check email above"
  scroll-to form 0.0 220.0
  expect error_text.visible
  expect error_text.height > 30.0
  expect error_text.visible_height ~= error_text.height
  expect error_text.right <= email_field.right
  capture error
  scroll-to form 0.0 180.0
  expect email_input.visible_height ~= email_input.height
  focus email_input
  type "sam@example.com"
  expect email == "sam@example.com"
  click save
  expect saved
  expect error == ""
  expect name == "Sam Rivera"
  expect workspace == "Design studio"
  expect email == "sam@example.com"
  expect notifications
  expect save.visible
  expect save.visible_height ~= save.height
  expect text "Changes saved"
  capture recovered

test settings_empty_explanation_and_long_label
  preset no_explanation
  viewport 360 320
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target heading = #screen/heading/root
  target title = heading/title
  target form = #screen/settings/root
  target workspace_field = form/content/preferences/root/workspace/field/root
  target label = workspace_field/label
  target input = workspace_field/input
  target save = #screen/actions/save
  expect heading.bottom ~= title.bottom
  scroll-to form 0.0 360.0
  expect label.visible
  expect label.height > 30.0
  expect label.visible_height ~= label.height
  expect label.right <= workspace_field.right
  capture long_label
  expect save.visible
  expect save.visible_height ~= save.height
  move form
  wheel 0.0 -10000.0
  focus input
  replace "Shared studio"
  expect workspace == "Shared studio"
  key tab
  key tab
  expect save.focused
  key enter
  expect saved
  expect workspace == "Shared studio"
  capture no_explanation

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

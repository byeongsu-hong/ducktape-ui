app StateFeedback
  font "../../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../../assets/fonts/Geist-Bold.ttf"

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

font geist family="Geist" default=true

state
  created = false
  project_name = ""
  empty_title = "No projects yet"
  empty_description = ""

preset long_copy
  state
    empty_title = "Your next shared workspace starts here"
    empty_description = "Create a project to collect notes and share your next idea with the team."

on create
  created = true
  task widget focus #page/root/content/project/name/field/root/input

view
  Page #page
    col #content
      with
        w=fill
        h=fill
        gap=12.0
      Alert title="Loading projects" description="" #loading
      Alert.Success title="Project saved" description="" #success
      Alert.Warning title="Unsaved changes" description="" #warning
      Alert.Destructive title="Could not save" description="" #error
      if !created
        EmptyState title=empty_title description=empty_description #empty
      if created
        col #project
          with
            w=fill
            h=fill
            gap=12.0
          PageHeader title="New project" description="Give your project a name."
          TextField #name label="Project name" value<->project_name
      if !created
        button "Create project" #create @primary_action -> create

test notices_omit_empty_description_rows
  viewport 360 640
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target loading = #page/root/content/loading/root
  target success = #page/root/content/success/root
  target warning = #page/root/content/warning/root
  target error = #page/root/content/error/root
  expect loading.height ~= 50.0
  expect success.height ~= 50.0
  expect warning.height ~= 50.0
  expect error.height ~= 50.0
  capture compact_notices

test empty_state_keeps_a_reachable_action_without_a_blank_line
  viewport 360 640
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #page/root/content/empty/root/content
  target icon = content/icon
  target title = content/title
  target action = #page/root/content/create
  target project = #page/root/content/project
  target name = project/name/field/root/input
  expect content.height ~= icon.height + 7.0 + title.height
  expect action.bottom <= 616.0
  expect action.height >= 32.0
  click action
  expect created
  expect exists project
  expect name.focused
  type "Research"
  expect name.value == "Research"
  capture created_project

test long_notice_copy_stays_inside_its_surface
  viewport 320 360
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page
      Alert #notice
        with
          title="Projektzusammenarbeitseinstellungen"
          description="Wiederherstellungsbenachrichtigungseinstellungen"
  target root = #page/root/notice/root
  target title = root/content/title
  target description = root/content/description
  expect title.text_x + title.text_width <= root.right - 13.0
  expect description.text_x + description.text_width <= root.right - 13.0
  expect title.height > 20.0
  expect description.height > 20.0
  capture long_notice

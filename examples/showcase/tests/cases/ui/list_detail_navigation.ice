app ListDetailNavigation
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../../assets/fonts/Geist-Bold.ttf"
  window
    size 720 640
    min-size 320 360

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

extern crate::projects
  Project(id:i64, title:str, description:str, metadata:str, initials:str, draft:str)
  pure initial() -> [Project]
  pure visible(projects:&[Project], query:&str, reversed:bool) -> [Project]
  pure selected(projects:&[Project], id:i64) -> Project
  pure edit(projects:[Project], id:i64, draft:str) -> [Project]

state
  projects:[Project] = initial()
  selected_id = 10
  showing_detail = false
  query = ""
  reversed = false
  draft = ""
  saved = ""
  page_padding = 24.0
  reading_width = 640.0

derived
  current = selected(projects, selected_id)
  rows = visible(projects, query, reversed)

preset customized
  state
    page_padding = 12.0
    reading_width = 420.0

on open_project(id)
  selected_id = id
  let opened = selected(projects, id)
  draft = opened.draft
  showing_detail = true
  saved = ""
  task widget focus #page/root/content/detail/body/notes/field/root/input

on back
  projects = edit(projects, selected_id, draft)
  showing_detail = false
  task widget focus #page/root/content/list/rows/key(selected_id)/project(selected_id)/open

on reverse_order
  reversed = !reversed

on save
  projects = edit(projects, selected_id, draft)
  saved = draft

component ProjectRow(project:Project, active:bool) -> i64
  button #open -> emit(project.id)
    with
      w=fill
      p=0.0
      checked=active
      label=project.title
      @secondary_action
    col w=fill
      Item #item
        with
          title=project.title
          description=project.description
          meta=project.metadata
        Avatar #avatar initials=project.initials
      if active
        box px=9.0
          text "Selected" #selection @caption

view
  Page #page padding=page_padding
    box
      with
        w=fill
        h=fill
        align-x=center
      box
        with
          w=fill
          h=fill
          max-w=reading_width
        col #content
          with
            w=fill
            h=fill
            gap=12.0
          if !showing_detail
            col #list
              with
                w=fill
                h=fill
                gap=12.0
              PageHeader title="Projects" description="Open a project to edit its own notes."
              row w=fill gap=8.0
                text "Selected:" @caption
                text current.title #selected-location
                  with
                    w=fill
                    wrap=word
                    @caption
              TextField #search label="Filter projects" value<->query
              button "Reverse order" #reverse @secondary_action -> reverse_order
              scroll #rows w=fill h=fill
                keyed project in rows by=project.id w=fill gap=8.0
                  lazy project, selected_id as entry
                    ProjectRow #project(entry.id) -> open_project _
                      with
                        project=entry
                        active=(entry.id == selected_id)
              if empty(rows)
                text "No projects match. Clear the filter to see your selection." #empty
                  with
                    w=fill
                    wrap=word
                    @caption
          if showing_detail
            col #detail
              with
                w=fill
                h=fill
                gap=12.0
              Breadcrumb #location current=current.title
                button "Back to projects" #back @ghost_action -> back
              scroll #body w=fill h=fill
                col w=fill gap=12.0
                  PageHeader #heading title=current.title description=current.description
                  TextField #notes label="Project notes" value<->draft
                  Attachment #attachment name="Workspace review notes.pdf" meta="Reference document"
                  text "Drafts stay with their project when you go back, filter the list, or change its order."
                    with
                      w=fill
                      wrap=word
                      @caption
              row wrap
                with
                  w=fill
                  gap=8.0
                  wrap-gap=8.0
                  align=center
                button "Save notes" #save @primary_action -> save
                if !empty(saved) && saved == draft
                  text "Notes saved" #saved @caption

test project_selection_survives_filter_and_reorder
  viewport 720 640
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #page/root/content
  target list = content/list
  target search = list/search/field/root/input
  target reverse = list/reverse
  target first = list/rows/key(10)/project(10)/open
  target chosen = list/rows/key(20)/project(20)/open
  target last = list/rows/key(30)/project(30)/open
  target detail = content/detail
  target back_button = detail/location/root/back
  target notes = detail/body/notes/field/root/input
  target heading = detail/body/heading/root
  target save_button = detail/save
  expect a11y first checked true
  expect a11y chosen checked false
  expect a11y chosen name "Quarterly workspace migration and accessibility review"
  expect a11y chosen role "button"
  expect a11y chosen action click
  click chosen
  expect selected_id == 20
  expect showing_detail
  expect text "Quarterly workspace migration and accessibility review" within heading
  focus notes
  replace "Migration draft"
  click back_button
  expect !showing_detail
  expect chosen.focused
  expect a11y chosen checked true
  expect a11y first checked false
  click reverse
  expect last.top < chosen.top
  expect chosen.top < first.top
  expect selected_id == 20
  focus search
  replace "Research"
  expect selected_id == 20
  expect len(rows) == 1
  expect a11y last checked false
  replace "no results"
  expect len(rows) == 0
  expect text "No projects match. Clear the filter to see your selection." within list
  replace ""
  expect a11y chosen checked true
  expect text "Quarterly workspace migration and accessibility review" within list
  click first
  focus notes
  replace "Independent design draft"
  click back_button
  click chosen
  expect notes.value == "Migration draft"
  click save_button
  expect saved == "Migration draft"
  capture project_detail
  click back_button
  click first
  expect notes.value == "Independent design draft"
  click back_button
  capture project_list

test project_keyboard_open_and_back_restore_focus
  viewport 480 640
  target content = #page/root/content
  target search = content/list/search/field/root/input
  target reverse = content/list/reverse
  target first = content/list/rows/key(10)/project(10)/open
  target chosen = content/list/rows/key(20)/project(20)/open
  target detail = content/detail
  target notes = detail/body/notes/field/root/input
  target back_button = detail/location/root/back
  key tab
  expect search.focused
  key tab
  expect reverse.focused
  key tab
  expect first.focused
  key tab
  expect chosen.focused
  key enter
  expect selected_id == 20
  expect notes.focused
  replace "Keyboard draft"
  a11y focus back_button
  expect a11y back_button name "Back to projects"
  expect a11y back_button action click
  key enter
  expect !showing_detail
  expect chosen.focused
  key enter
  expect notes.focused
  expect notes.value == "Keyboard draft"

test project_customization_and_long_content_fit
  preset customized
  viewport 720 640
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target content = #page/root/content
  target list = content/list
  target rows_view = list/rows
  target chosen = rows_view/key(20)/project(20)/open
  target item = chosen/item/root
  target title = item/content/title
  target description = item/content/description
  target metadata = item/meta
  target avatar = item/avatar/root
  target research = rows_view/key(30)/project(30)/open
  target research_content = research/item/root/content
  target research_title = research_content/title
  target detail = content/detail
  target heading = detail/body/heading/root/title
  target location = detail/location/root
  target current_location = location/current
  target save_button = detail/save
  target detail_body = detail/body
  expect content.width ~= 420.0
  expect content.left ~= 150.0
  capture project_customized
  window resize 320 560
  expect content.left ~= 12.0
  expect content.width ~= 296.0
  expect avatar.width ~= 30.0
  expect title.text_x + title.text_width <= metadata.left
  expect description.text_x + description.text_width <= metadata.left
  expect metadata.text_x + metadata.text_width <= item.right
  expect title.text_height > 20.0
  expect research_content.height ~= research_title.height
  move rows_view
  wheel 0.0 -1000.0
  expect research.bottom - rows_view.scroll_y <= rows_view.bottom + 0.5
  capture project_compact_list
  move rows_view
  wheel 0.0 1000.0
  click chosen
  expect selected_id == 20
  expect heading.text_x + heading.text_width <= content.right
  expect current_location.text_x + current_location.text_width <= location.right
  expect save_button.height >= 32.0
  expect save_button.bottom <= 548.0
  expect a11y save_button name "Save notes"
  click save_button
  expect saved == current.draft
  capture project_compact_detail
  window resize 320 360
  expect detail_body.height > 0.0
  expect save_button.height >= 32.0
  expect save_button.bottom <= 348.0
  move detail_body
  wheel 0.0 -1000.0
  expect detail_body.scroll_y > 0.0
  click save_button
  expect saved == current.draft
  capture project_short_detail

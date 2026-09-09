app ResponsiveWorkspace
  text-size 14
  window
    size 960 640
    min-size 320 240

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

enum Project
  design
  engineering

state
  selected:Project = Project.design
  design_notes = "Design notes"
  engineering_notes = "Engineering notes"
  saved = ""
  compact_below = 640.0
  navigation_width = 200.0
  page_padding = 16.0
  reading_width = 640.0

preset customized
  state
    compact_below = 480.0
    navigation_width = 160.0
    page_padding = 24.0
    reading_width = 400.0

component Navigation(selected:Project)
  emits
    design
    engineering
  col #root w=fill gap=8.0
    text "Projects" @section_title
    row wrap
      with
        w=fill
        gap=8.0
        wrap-gap=8.0
      button "Design" #design checked=(selected == Project.design) @secondary_action -> emit(design)
      button "Engineering" #engineering -> emit(engineering)
        with
          checked=(selected == Project.engineering)
          @secondary_action
    match selected
      Project.design
        text "Selected: Design"
          with
            w=fill
            wrap=word
            @caption
      Project.engineering
        text "Selected: Engineering"
          with
            w=fill
            wrap=word
            @caption

on select_design
  selected = Project.design

on select_engineering
  selected = Project.engineering

on save
  match selected
    Project.design
      saved = design_notes
    Project.engineering
      saved = engineering_notes

view
  Page #page padding=page_padding
    responsive
      with
        size=(available_width, available_height)
        w=fill
        h=fill
      row #workspace w=fill h=fill
        // Keep both hosts present: native child state is positional.
        row h=fill
          if available_width >= compact_below
            box #sidebar w=navigation_width h=fill
              Navigation #wide-navigation selected=selected
                events
                  design -> select_design
                  engineering -> select_engineering
            space w=16.0
        col #detail w=fill h=fill
          col w=fill
            if available_width < compact_below
              Navigation #compact-navigation selected=selected
                events
                  design -> select_design
                  engineering -> select_engineering
              space h=12.0
          Form #body padding=0.0 max_width=reading_width
            col w=fill gap=12.0
              match selected
                Project.design
                  PageHeader
                    with
                      title="Design"
                      description="Project notes remain available when the window changes size."
                Project.engineering
                  PageHeader
                    with
                      title="Engineering"
                      description="Project notes remain available when the window changes size."
              if selected == Project.design
                TextField #design-notes label="Design notes" value<->design_notes
              if selected == Project.engineering
                TextField #engineering-notes label="Engineering notes" value<->engineering_notes
              text "Resize this window to switch between a sidebar and compact navigation. Select another project to return to its own draft. Save stays outside the scrolling body so it remains reachable in a short window." #help
                with
                  w=fill
                  wrap=word
                  @body
          space h=12.0
          box w=fill align-x=center
            box #actions w=fill max-w=reading_width
              row
                with
                  w=fill
                  gap=8.0
                  align=center
                button "Save notes" #save @primary_action -> save
                if !empty(saved)
                  text "Notes saved" #saved
                    with
                      w=fill
                      wrap=word
                      @caption

test responsive_workspace_preserves_selection_and_drafts
  viewport 960 640
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target workspace = #page/root/workspace
  target sidebar = workspace/sidebar
  target wide_engineering = sidebar/wide-navigation/root/engineering
  target detail = workspace/detail
  target compact_design = detail/compact-navigation/root/design
  target compact_engineering = detail/compact-navigation/root/engineering
  target body = detail/body/root
  target content = body/content
  target engineering = body/content/engineering-notes/field/root/input
  target design = body/content/design-notes/field/root/input
  target save = detail/actions/save
  expect sidebar.width ~= 200.0
  expect detail.width ~= 712.0
  expect content.width ~= 640.0
  click wide_engineering
  expect selected == Project.engineering
  focus engineering
  replace "Draft survives resizing"
  // Replace the selected word after resize: a surviving bound string alone
  // would miss reconstructed native focus, selection, or caret state.
  select 0 5
  window resize 360 640
  type "Updated"
  expect selected == Project.engineering
  expect engineering_notes == "Updated survives resizing"
  expect engineering.value == "Updated survives resizing"
  expect detail.width ~= 328.0
  expect content.width ~= 328.0
  expect a11y compact_design name "Design"
  expect a11y compact_engineering name "Engineering"
  expect a11y compact_engineering checked true
  expect a11y compact_design checked false
  click compact_design
  expect design.value == "Design notes"
  focus design
  replace "Separate design draft"
  click compact_engineering
  expect engineering.value == "Updated survives resizing"
  click save
  expect saved == "Updated survives resizing"
  capture responsive_compact
  focus engineering
  key end
  window resize 960 640
  type "?"
  expect sidebar.width ~= 200.0
  expect detail.width ~= 712.0
  expect content.width ~= 640.0
  expect selected == Project.engineering
  expect engineering.value == "Updated survives resizing?"
  expect design_notes == "Separate design draft"
  click save
  expect saved == "Updated survives resizing?"
  capture responsive_wide

test responsive_workspace_short_window_keeps_actions
  viewport 320 240
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target workspace = #page/root/workspace
  target detail = workspace/detail
  target navigation = detail/compact-navigation/root
  target choose_engineering = navigation/engineering
  target body = detail/body/root
  target help = body/content/help
  target actions = detail/actions
  target save = actions/save
  expect navigation.width ~= 288.0
  expect choose_engineering.right <= navigation.right
  expect choose_engineering.bottom <= navigation.bottom
  click choose_engineering
  expect selected == Project.engineering
  expect body.height > 0.0
  expect actions.top >= body.bottom
  expect save.bottom <= 224.0
  expect save.right <= 304.0
  expect save.height >= 32.0
  expect save.visible
  move body
  wheel 0.0 -1000.0
  expect body.scroll_y > 0.0
  // Iced rounds scroll translation to logical pixels. Check the content end
  // within that half-pixel rounding, then translate its content-space bounds.
  expect body.scroll_y >= body.content_height - body.height - 0.5
  expect body.scroll_y <= body.content_height - body.height + 0.5
  expect help.bottom - body.scroll_y <= body.bottom + 0.5
  click save
  expect saved == "Engineering notes"
  capture responsive_short

test responsive_workspace_exact_breakpoint
  viewport 672 640
  target workspace = #page/root/workspace
  target sidebar = workspace/sidebar
  target detail = workspace/detail
  target compact = detail/compact-navigation/root
  expect sidebar.width ~= 200.0
  expect detail.left ~= 232.0
  expect detail.width ~= 424.0
  window resize 671 640
  expect compact.width ~= 639.0
  expect detail.left ~= 16.0
  window resize 672 640
  expect sidebar.width ~= 200.0
  expect detail.left ~= 232.0
  expect detail.width ~= 424.0

test responsive_workspace_customized_breakpoint
  preset customized
  viewport 528 480
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target workspace = #page/root/workspace
  target sidebar = workspace/sidebar
  target detail = workspace/detail
  target compact = detail/compact-navigation/root
  target engineering = compact/engineering
  target actions = detail/actions
  target content = detail/body/root/content
  target save = actions/save
  expect workspace.width ~= 480.0
  expect sidebar.width ~= 160.0
  expect detail.left ~= 200.0
  expect detail.width ~= 304.0
  window resize 527 480
  expect compact.width ~= 479.0
  expect detail.left ~= 24.0
  expect content.width ~= 400.0
  expect actions.width ~= 400.0
  expect actions.left ~= content.left
  click engineering
  expect selected == Project.engineering
  click save
  expect saved == "Engineering notes"
  capture responsive_customized

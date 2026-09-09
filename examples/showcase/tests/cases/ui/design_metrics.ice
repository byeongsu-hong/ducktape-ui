app DesignMetrics
  text-size 14
  font "../../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../../assets/fonts/IBMPlexSansKR-Bold.ttf"
  font "../../../../../assets/fonts/GeistMono-Regular.ttf"

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

extern crate::backend
  component native_role(content:str, role:i64) -> unit

font korean family="IBM Plex Sans KR" default=true

recipe compact_action for button extends secondary_action
  @px-12px py-8px

recipe compact_control for input extends control
  @px-12px py-8px

state
  draft = "Design metrics"
  saved = ""

component MetricsScreen(bind value:str, compact:bool=false, inset:f64=24.0, gap:f64=16.0) -> str
  Page #page padding=inset
    col #content w=fill gap=gap
      PageHeader #header
        with
          title="Workspace appearance"
          description="Spacing and typography stay readable in a compact desktop workspace."
      col #roles @section
        Typography.SectionTitle #heading content="프로젝트 작업 공간"
        Typography #body
          with
            content="긴 설명도 읽기 쉽게 표시합니다. Keep longer project descriptions readable when the workspace gets narrow."
        Typography.Caption #caption content="변경 사항은 이 작업 공간에만 적용됩니다."
        Typography.Machine #machine content="workspace.id = 2048"
      Field #field label="Workspace name"
        col #editor w=fill
          if compact
            input "" #compact-name <-> value
              with
                label="Workspace name"
                text-size=12.5
                @compact_control
          if !compact
            input "" #name <-> value label="Workspace name" @control
      if compact
        button "Save workspace" #compact-save @compact_action -> emit(value)
      if !compact
        button "Save workspace" #save @secondary_action -> emit(value)

on save(value)
  saved = value

view
  MetricsScreen value<->draft -> save _

test native_and_ice_display_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @display
      box #native
        extern native_role("Project 2048\nProject 4096", 0)
  target ice = #root/ice
  target native = #root/native
  capture metrics_display_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_display

test native_and_ice_section_title_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @section_title
      box #native
        extern native_role("Project 2048\nProject 4096", 1)
  target ice = #root/ice
  target native = #root/native
  capture metrics_section_title_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_section_title

test native_and_ice_body_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @body
      box #native
        extern native_role("Project 2048\nProject 4096", 2)
  target ice = #root/ice
  target native = #root/native
  capture metrics_body_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_body

test native_and_ice_caption_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @caption
      box #native
        extern native_role("Project 2048\nProject 4096", 3)
  target ice = #root/ice
  target native = #root/native
  capture metrics_caption_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_caption

test native_and_ice_machine_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @machine
      box #native
        extern native_role("Project 2048\nProject 4096", 4)
  target ice = #root/ice
  target native = #root/native
  capture metrics_machine_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_machine

test design_metrics_standard
  viewport 640 720
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    MetricsScreen #screen value<->draft -> save _
      with
        compact=false
        inset=24.0
        gap=16.0
  target page = #screen/page/root
  target content = page/content
  target header = content/header/root
  target title = header/title
  target roles = content/roles
  target heading = roles/heading/root
  target body = roles/body/root
  target caption = roles/caption/root
  target machine = roles/machine/root
  target field = content/field/root
  target label = field/label
  target name = field/editor/name
  target save = content/save
  capture metrics_standard_geometry
  expect content.left ~= page.left + 24.0
  expect content.right ~= page.right - 24.0
  expect roles.top ~= header.bottom + 16.0
  expect field.top ~= roles.bottom + 16.0
  expect save.top ~= field.bottom + 16.0
  expect body.top ~= heading.bottom + 12.0
  expect caption.top ~= body.bottom + 12.0
  expect machine.top ~= caption.bottom + 12.0
  expect name.top ~= label.bottom + 8.0
  expect heading.text_size ~= 16.0
  expect heading.text_width ~= 121.72801208496094
  expect body.text_size ~= 13.5
  expect caption.text_size ~= 12.5
  expect machine.text_size ~= 12.0
  expect body.text_height > 20.0
  expect body.text_x + body.text_width <= content.right
  expect caption.text_x + caption.text_width <= content.right
  expect body.font.family.name == some("IBM Plex Sans KR")
  expect name.height ~= 40.2
  expect name.height >= 32.0
  expect save.height ~= 38.25
  expect save.height >= 32.0
  expect save.bottom <= page.bottom - 24.0
  expect save.text_y ~= save.top + (save.height - save.text_height) / 2.0
  expect save.text_x ~= save.left + (save.width - save.text_width) / 2.0
  expect name.text_y ~= name.top + (name.height - name.text_height) / 2.0
  expect name.accessibility_name == "Workspace name"
  expect save.accessibility_name == "Save workspace"
  click name
  select-all
  type "작업 공간"
  expect name.value == "작업 공간"
  expect saved == ""
  click-at (save.center_x) (save.bottom - 2.0)
  expect saved == "작업 공간"
  capture metrics_standard_korean
  click name
  select-all
  type "Keyboard workspace"
  key tab
  expect save.focused == true
  key enter
  expect saved == "Keyboard workspace"
  capture metrics_standard

test design_metrics_compact
  viewport 360 720
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    MetricsScreen #screen value<->draft -> save _
      with
        compact=true
        inset=12.0
        gap=12.0
  target page = #screen/page/root
  target content = page/content
  target header = content/header/root
  target title = header/title
  target roles = content/roles
  target heading = roles/heading/root
  target body = roles/body/root
  target caption = roles/caption/root
  target machine = roles/machine/root
  target field = content/field/root
  target label = field/label
  target name = field/editor/compact-name
  target save = content/compact-save
  capture metrics_compact_geometry
  expect content.left ~= page.left + 12.0
  expect content.right ~= page.right - 12.0
  expect roles.top ~= header.bottom + 12.0
  expect field.top ~= roles.bottom + 12.0
  expect save.top ~= field.bottom + 12.0
  expect body.top ~= heading.bottom + 12.0
  expect caption.top ~= body.bottom + 12.0
  expect machine.top ~= caption.bottom + 12.0
  expect name.top ~= label.bottom + 8.0
  expect heading.text_size ~= 16.0
  expect heading.text_width ~= 121.72801208496094
  expect body.text_size ~= 13.5
  expect caption.text_size ~= 12.5
  expect machine.text_size ~= 12.0
  expect body.text_height > 20.0
  expect body.text_x + body.text_width <= content.right
  expect caption.text_x + caption.text_width <= content.right
  expect body.font.family.name == some("IBM Plex Sans KR")
  expect name.height ~= 32.25
  expect name.height >= 32.0
  expect save.height ~= 32.25
  expect save.height >= 32.0
  expect save.bottom <= page.bottom - 12.0
  expect save.text_y ~= save.top + (save.height - save.text_height) / 2.0
  expect save.text_x ~= save.left + (save.width - save.text_width) / 2.0
  expect name.text_y ~= name.top + (name.height - name.text_height) / 2.0
  expect name.accessibility_name == "Workspace name"
  expect save.accessibility_name == "Save workspace"
  click name
  select-all
  type "작업 공간"
  expect name.value == "작업 공간"
  expect saved == ""
  click-at (save.center_x) (save.bottom - 2.0)
  expect saved == "작업 공간"
  capture metrics_compact_korean
  click name
  select-all
  type "Keyboard workspace"
  key tab
  expect save.focused == true
  key enter
  expect saved == "Keyboard workspace"
  capture metrics_compact

test design_metrics_compact_wide
  viewport 640 720
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    MetricsScreen #screen value<->draft -> save _
      with
        compact=true
        inset=12.0
        gap=12.0
  target page = #screen/page/root
  target content = page/content
  target header = content/header/root
  target title = header/title
  target roles = content/roles
  target heading = roles/heading/root
  target body = roles/body/root
  target caption = roles/caption/root
  target machine = roles/machine/root
  target field = content/field/root
  target label = field/label
  target name = field/editor/compact-name
  target save = content/compact-save
  capture metrics_compact_wide_geometry
  expect content.left ~= page.left + 12.0
  expect content.right ~= page.right - 12.0
  expect roles.top ~= header.bottom + 12.0
  expect field.top ~= roles.bottom + 12.0
  expect save.top ~= field.bottom + 12.0
  expect body.top ~= heading.bottom + 12.0
  expect caption.top ~= body.bottom + 12.0
  expect machine.top ~= caption.bottom + 12.0
  expect name.top ~= label.bottom + 8.0
  expect heading.text_size ~= 16.0
  expect heading.text_width ~= 121.72801208496094
  expect body.text_size ~= 13.5
  expect caption.text_size ~= 12.5
  expect machine.text_size ~= 12.0
  expect body.text_height > 20.0
  expect body.text_x + body.text_width <= content.right
  expect caption.text_x + caption.text_width <= content.right
  expect body.font.family.name == some("IBM Plex Sans KR")
  expect name.height ~= 32.25
  expect name.height >= 32.0
  expect save.height ~= 32.25
  expect save.height >= 32.0
  expect save.bottom <= page.bottom - 12.0
  expect save.text_y ~= save.top + (save.height - save.text_height) / 2.0
  expect save.text_x ~= save.left + (save.width - save.text_width) / 2.0
  expect name.text_y ~= name.top + (name.height - name.text_height) / 2.0
  expect name.accessibility_name == "Workspace name"
  expect save.accessibility_name == "Save workspace"
  click name
  select-all
  type "작업 공간"
  expect name.value == "작업 공간"
  expect saved == ""
  click-at (save.center_x) (save.bottom - 2.0)
  expect saved == "작업 공간"
  capture metrics_compact_wide_korean
  click name
  select-all
  type "Keyboard workspace"
  key tab
  expect save.focused == true
  key enter
  expect saved == "Keyboard workspace"
  capture metrics_compact_wide

test native_and_ice_screen_title_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @screen_title
      box #native
        extern native_role("Project 2048\nProject 4096", 5)
  target ice = #root/ice
  target native = #root/native
  capture metrics_screen_title_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_screen_title

test native_and_ice_pane_header_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @pane_header
      box #native
        extern native_role("Project 2048\nProject 4096", 6)
  target ice = #root/ice
  target native = #root/native
  capture metrics_pane_header_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_pane_header

test native_and_ice_list_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @list
      box #native
        extern native_role("Project 2048\nProject 4096", 7)
  target ice = #root/ice
  target native = #root/native
  capture metrics_list_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_list

test native_and_ice_meta_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @meta
      box #native
        extern native_role("Project 2048\nProject 4096", 8)
  target ice = #root/ice
  target native = #root/native
  capture metrics_meta_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_meta

test native_and_ice_meta_compact_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @meta_compact
      box #native
        extern native_role("Project 2048\nProject 4096", 9)
  target ice = #root/ice
  target native = #root/native
  capture metrics_meta_compact_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_meta_compact

test native_and_ice_field_label_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @field_label
      box #native
        extern native_role("Project 2048\nProject 4096", 10)
  target ice = #root/ice
  target native = #root/native
  capture metrics_field_label_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_field_label

test native_and_ice_nav_label_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @nav_label
      box #native
        extern native_role("Project 2048\nProject 4096", 11)
  target ice = #root/ice
  target native = #root/native
  capture metrics_nav_label_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_nav_label

test native_and_ice_badge_label_metrics
  viewport 520 260
  scale 1.0
  locale "ko-KR"
  platform linux
  reduced-motion true
  mount
    col #root
      with
        w=fill
        p=24.0
        gap=16.0
      text "Project 2048\nProject 4096" #ice @badge_label
      box #native
        extern native_role("Project 2048\nProject 4096", 12)
  target ice = #root/ice
  target native = #root/native
  capture metrics_badge_label_before
  expect ice.text_count == 1
  expect native.text_count == 1
  expect native.text_size ~= ice.text_size
  expect native.line_height == ice.line_height
  expect native.height ~= ice.height
  expect native.text_baseline - native.top ~= ice.text_baseline - ice.top
  expect native.font == ice.font
  expect native.text_color == ice.text_color
  capture metrics_badge_label

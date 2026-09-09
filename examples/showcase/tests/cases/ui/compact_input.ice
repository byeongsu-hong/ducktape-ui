app CompactInput
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../../assets/fonts/Geist-Bold.ttf"

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

state
  slug = ""
  applied = ""

on apply(value)
  applied = value

component SlugField(bind value:str) -> str
  Field #root label="Project slug"
    InputGroup #group
      row #line w=fill gap=6.0 align=center
        input "" #input <-> value
          with
            label="Project slug"
            hint="ui-lang"
            w=fill
            p=6.0
        button "Apply" #apply @secondary_action -> emit(value)

view
  Page
    Panel title="Project"
      col w=fill gap=12.0
        SlugField value<->slug -> apply _
        if !empty(applied)
          text "Applied: {applied}" @caption

test narrow_input_and_action_remain_usable
  viewport 280 320
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page padding=24.0
      Panel #panel title="Project"
        col #content w=fill gap=12.0
          SlugField #field value<->slug -> apply _
          text "ui-lang" #ruler size=14.0 shape=advanced
  target page = #page/root
  target panel = page/panel/root
  target content = panel/content
  target field = content/field/root/root
  target label = field/label
  target group = field/group/root
  target input = group/line/input
  target action = group/line/apply
  target ruler = content/ruler
  click input
  type "ui-lang"
  expect input.value == "ui-lang"
  expect input.text_count == 1
  expect input.text_width ~= ruler.text_width
  expect input.top >= label.bottom
  expect action.left >= input.right
  expect action.right <= group.right
  expect panel.left >= page.left + 24.0
  expect panel.right <= page.right - 24.0
  expect applied == ""
  click action
  expect applied == "ui-lang"
  capture narrow_input_and_action_remain_usable

test wide_input_respects_custom_page_padding
  viewport 640 320
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page padding=12.0
      Panel #panel title="Project"
        col #content w=fill gap=12.0
          SlugField #field value<->slug -> apply _
          text "ui-lang" #ruler size=14.0 shape=advanced
  target page = #page/root
  target panel = page/panel/root
  target content = panel/content
  target field = content/field/root/root
  target label = field/label
  target group = field/group/root
  target input = group/line/input
  target action = group/line/apply
  target ruler = content/ruler
  click input
  type "ui-lang"
  expect input.value == "ui-lang"
  expect input.text_count == 1
  expect input.text_width ~= ruler.text_width
  expect input.top >= label.bottom
  expect action.left >= input.right
  expect action.right <= group.right
  expect panel.left >= page.left + 12.0
  expect panel.right <= page.right - 12.0
  expect applied == ""
  click action
  expect applied == "ui-lang"
  capture wide_input_respects_custom_page_padding


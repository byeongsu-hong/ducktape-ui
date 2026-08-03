app RenderSurface

use "extern/render_surface.ice"
use "theme.ice"

enum SurfaceState
  idle
  ready(str)

state
  checked = false
  toggled = true
  level = 40.0
  selected = 0
  choice:str? = none
  search:combo[str] = ["One", "Two"]
  draft = "Draft value"
  notes:editor = "Rendered editor"
  docs:markdown = "# Rendered markdown"
  memory_image = rgba(1, 1, bytes(ff 00 ff ff))
  overlay_open = true
  external_active = false
  outcome:result[str,str] = ok("Result child")
  surface_state:SurfaceState = SurfaceState.idle

component Slotted()
  box #frame
    with
      w=240.0
      h=32.0
      @bg-surface
    slot

on clicked
  markdown docs append draft
  combo search push draft
  outcome = ok(draft)
  surface_state = SurfaceState.ready(draft)

on overlay_dismissed
  overlay_open = false

on checked_changed(next)
  checked = next

on toggled_changed(next)
  toggled = next

on level_changed(next)
  level = next

on selected_changed(next)
  selected = next

on choice_changed(next)
  choice = some(next)

on search_changed(next)
  choice = some(next)

on external_changed(next)
  external_active = next

on shader_changed(next)
  external_active = next

on link_opened(_url)

on mouse_pressed

on resized(_dx, _dy)

on measured(_width, _height)

on hidden

on canvas_pressed

view
  col #root
    with
      w=720.0
      gap=8.0
      p=8.0
    text "Render surface" #text size=16.0
    rich-text #rich-text
      span "Rendered rich text"
    input "Draft" #input <-> draft
    button "Button" #button -> clicked
    checkbox "Checkbox" #checkbox checked=checked -> checked_changed _
    toggler "Toggler" #toggler checked=toggled -> toggled_changed _
    slider level #slider min=0.0 max=100.0 -> level_changed _
    progress level #progress
    radio "Radio" #radio value=1 selected=(selected == 1) -> selected_changed _
    pick ["One", "Two"] choice #pick -> choice_changed _
    combo search choice "Search" #combo -> search_changed _
    rule horizontal #rule
    qr "https://example.com/render" #qr
    space #space w=24.0 h=8.0
    row #row w=fill gap=4.0
      text "Row child"
    flex #flex
      with
        w=fill
        gap=4.0
        justify=space-between
      text "Flex start"
      text "Flex end"
    grid #grid
      with
        cols=2
        w=704.0
        gap=4.0
      text "Grid one"
      text "Grid two"
    stack #stack w=fill h=24.0
      text "Stack base"
      text "Stack layer"
    box #box
      with
        w=fill
        h=24.0
        @bg-surface
      text "Box child"
    scroll #scroll w=fill h=40.0
      col h=80.0
        text "Scroll child"
    overlay #overlay when=overlay_open dismiss=overlay_dismissed
      content
        text "Overlay content"
      layer
        text "Overlay layer"
    keyed item in [1, 2] by=(item + 0) #keyed w=fill gap=2.0
      text item
    lazy draft as cached #lazy
      text cached
    markdown docs #markdown -> link_opened _
    editor #editor <-> notes h=48.0
    table item in ["One", "Two"] #table w=fill
      col w=fill
        header
          text "Table header"
        cell
          text item
    Slotted #component
      text "Slotted child" #slot-text
    extern native_help(external_active) #extern -> external_changed _
    themer alternate_panel(true) #themer
    shader status_shader(1.0) #shader w=fill h=24.0 -> shader_changed _
    image memory_image #image w=32.0 h=32.0
    svg "../../assets/ice.svg" #svg w=32.0 h=32.0
    viewer memory_image #viewer w=64.0 h=48.0
    tooltip #tooltip delay=0
      text "Tooltip content"
      text "Tooltip tip"
    mouse #mouse press=mouse_pressed
      text "Mouse area"
    resize-handle #resize drag=resized
      box w=24.0 h=12.0
        text "Resize"
    canvas #canvas w=64.0 h=32.0
      event mouse pressed
        emit canvas_pressed
        capture
      rect x=0.0 y=0.0 w=canvas_width h=canvas_height fill=primary
    theme #theme dark
      text "Nested theme"
    float #float x=2.0 y=3.0
      text "Float child"
    pin #pin
      with
        w=64.0
        h=24.0
        x=2.0
        y=3.0
      text "Pin child"
    sensor #sensor
      with
        show=measured
        resize=measured
        hide=hidden
      text "Sensor child"
    responsive #responsive
      with
        size=(available_width, available_height)
        w=fill
        h=24.0
      text available_width
    panes #panes w=fill h=80.0
      pane first
        text "Pane child"
    if overlay_open
      text "If child" #if-child
    match choice
      some(value)
        text value
      none
        text "Match child" #match-child
    match outcome
      ok(value)
        text value
      err(error)
        text error
    match surface_state
      SurfaceState.idle
        text "Enum child"
      SurfaceState.ready(value)
        text value
    for item in ["One", "Two"]
      text item #for-item(item)
      Slotted #repeated(item)
        text item #repeated-text

test renders_every_node
  viewport 900 2400
  target root = #root
  target text_node = #root/text
  target rich_text = #root/rich-text
  target input = #root/input
  target button = #root/button
  target checkbox = #root/checkbox
  target toggler = #root/toggler
  target slider = #root/slider
  target progress = #root/progress
  target radio = #root/radio
  target pick = #root/pick
  target combo = #root/combo
  target rule = #root/rule
  target qr = #root/qr
  target space = #root/space
  target row = #root/row
  target flex = #root/flex
  target grid = #root/grid
  target stack = #root/stack
  target box_node = #root/box
  target scroll = #root/scroll
  target overlay = #root/overlay
  target keyed = #root/keyed
  target lazy = #root/lazy
  target markdown = #root/markdown
  target editor = #root/editor
  target table = #root/table
  target component_frame = #root/component/frame
  target slot_text = #root/component/frame/slot-text
  target external = #root/extern
  target themer = #root/themer
  target shader = #root/shader
  target image = #root/image
  target svg = #root/svg
  target viewer = #root/viewer
  target tooltip = #root/tooltip
  target mouse = #root/mouse
  target resize = #root/resize
  target canvas = #root/canvas
  target theme = #root/theme
  target float = #root/float
  target pin = #root/pin
  target sensor = #root/sensor
  target responsive = #root/responsive
  target panes = #root/panes
  target if_child = #root/if-child
  target match_child = #root/match-child
  target for_item = #root/for-item("One")
  target repeated_text = #root/repeated("One")/frame/repeated-text
  expect root.visible
  expect text_node.visible
  expect rich_text.visible
  expect input.visible
  expect button.visible
  expect checkbox.visible
  expect toggler.visible
  expect slider.visible
  expect progress.visible
  expect radio.visible
  expect pick.visible
  expect combo.visible
  expect rule.visible
  expect qr.visible
  expect space.visible
  expect row.visible
  expect flex.visible
  expect grid.visible
  expect stack.visible
  expect box_node.visible
  expect scroll.visible
  expect overlay.visible
  expect keyed.visible
  expect lazy.visible
  expect markdown.visible
  expect editor.visible
  expect table.visible
  expect component_frame.visible
  expect slot_text.visible
  expect external.visible
  expect themer.visible
  expect shader.visible
  expect image.visible
  expect svg.visible
  expect viewer.visible
  expect tooltip.visible
  expect mouse.visible
  expect resize.visible
  expect canvas.visible
  expect theme.visible
  expect float.visible
  expect pin.visible
  expect sensor.visible
  expect responsive.visible
  expect panes.visible
  expect if_child.visible
  expect match_child.visible
  expect for_item.visible
  expect repeated_text.visible
  expect text "Render surface" within root
  expect text "1" within keyed
  expect text "Table header" within table
  expect text "Slotted child" within component_frame
  expect text "If child" within root
  expect text "Match child" within root
  expect text "Two" within root
  click button
  expect outcome == ok("Draft value")

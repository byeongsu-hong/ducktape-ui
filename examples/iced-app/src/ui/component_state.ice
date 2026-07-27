app ComponentState

use "extern/component_state.ice"

theme
  bg #111111
  fg #eeeeee
  primary #3366ff
  danger #cc3333

component Flag(value:str)
  state
    checked = false
  on changed(next)
    checked = next
  col
    text value
    checkbox "Nested" checked=checked -> changed _

component Counter(label:str)
  state
    count = 0
    draft = ""
    enabled = false
  on increment
    count = count + 1
  on changed(next)
    enabled = next
  col
    text label
    text count
    input "Draft" <-> draft
    checkbox "Enabled" checked=enabled -> changed _
    checkbox "Mirror" checked=enabled -> changed _
    Flag value=draft #flag
    button "Increment" -> increment
    match count
      0
        text "zero"
      _
        text draft

component Loader()
  state
    query = ""
    loading = false
    tasks:[Task] = []
  on load
    loading = true
    run latest create_task(query) -> loaded _ | failed _
  on loaded(next)
    tasks = next
    loading = false
  on failed(error)
    loading = false
  col
    input "Task" <-> query
    button "Load" disabled=loading -> load
    text len(tasks)

component EditableTitle()
  state
    editing = false
    draft = ""
  on edit
    editing = true
    task widget focus #draft
  col
    button "Edit title" -> edit
    if editing
      input "Title" #draft <-> draft

component RenderContract()
  state
    draft = ""
    count = 0
  on increment
    count = count + 1
  box #root w=240.0 p=16.0 shadow=black/50 shadow-x=2.0 shadow-y=3.0 shadow-blur=4.0 @bg-bg border border-primary rounded-lg
    col gap=12.0
      box #heading
        text "Render contract" size=18.0 @text-primary
      box #field
        input "Draft" #draft <-> draft @bg-bg border border-primary rounded-md
      button "Increment" #increment p=8.0 @bg-primary text-fg rounded-md -> increment
      box #result
        text count size=14.0 @text-fg

component LayoutContract()
  col #root w=320.0 gap=10.0 p=10.0 @bg-bg border border-primary rounded-lg
    row #row w=300.0 gap=8.0 p=4.0
      box #left w=80.0 h=20.0 @bg-primary
        text "Left" size=10.0 @text-fg
      box #right w=fill h=20.0 @bg-primary
        text "Right" size=10.0 @text-fg
    grid #grid cols=2 w=300.0 gap=6.0
      box #first w=fill h=20.0 @bg-primary
        text "First" size=10.0 @text-fg
      box #second w=fill h=20.0 @bg-primary
        text "Second" size=10.0 @text-fg
    flex #flex w=300.0 h=20.0 justify=space-between
      box #start w=60.0 h=20.0 @bg-primary
        text "Start" size=10.0 @text-fg
      box #end w=60.0 h=20.0 @bg-primary
        text "End" size=10.0 @text-fg
    stack #stack w=300.0 h=40.0
      box #base w=100.0 h=20.0 @bg-primary
        text "Base" size=10.0 @text-fg
      box #layer w=100.0 h=20.0 @bg-danger
        text "Layer" size=10.0 @text-fg
    scroll #scroll dir=vertical w=300.0 h=50.0
      col #content w=300.0 h=120.0
        text "Scrollable" size=10.0 @text-fg
    responsive size=(available_width, available_height) w=300.0 h=30.0
      box #responsive-content w=available_width h=available_height @bg-primary
        text "Responsive" size=10.0 @text-fg
    space #space w=300.0 h=8.0

component InteractionContract()
  state
    clicks = 0
    checked = false
    toggled = false
    level = 25.0
    mode = 0
    choices = ["Alpha option", "Beta option"]
    choice:str? = none
    mouse_presses = 0
    resize_x = 0.0
    canvas_presses = 0
    dialog_open = false
    status = "idle"
  on clicked
    clicks = clicks + 1
    status = "button routed"
  on checked_changed(next)
    checked = next
    status = "checkbox routed"
  on toggled_changed(next)
    toggled = next
    status = "toggler routed"
  on level_changed(next)
    level = next
    status = "slider routed"
  on mode_changed(next)
    mode = next
    status = "radio routed"
  on choice_changed(next)
    choice = some(next)
    status = "pick routed"
  on pick_opened
    status = "pick opened"
  on pick_closed
    status = "pick closed"
  on mouse_pressed
    mouse_presses = mouse_presses + 1
    status = "mouse routed"
  on resized(dx, _dy)
    resize_x = resize_x + dx
    status = "resize routed"
  on canvas_pressed
    canvas_presses = canvas_presses + 1
    status = "canvas routed"
  on open_dialog
    dialog_open = true
    status = "overlay opened"
  on close_dialog
    dialog_open = false
    status = "overlay closed"
  col #root w=320.0 gap=8.0 p=12.0 @bg-bg border border-primary rounded-lg
    button "Click" #button p=6.0 @bg-primary text-fg rounded-md -> clicked
    checkbox "Checkbox" #checkbox checked=checked -> checked_changed _
    toggler "Toggler" #toggler checked=toggled -> toggled_changed _
    slider level #slider min=0.0 max=100.0 step=1.0 w=200.0 h=24.0 -> level_changed _
    radio "Radio" #radio value=1 selected=(mode == 1) -> mode_changed _
    pick choices choice #pick hint="Pick" w=fill open=pick_opened close=pick_closed -> choice_changed _
    mouse press=mouse_pressed cursor=pointer
      box #mouse w=200.0 h=32.0 @bg-bg rounded-md
        text mouse_presses size=14.0 @text-fg
    resize-handle drag=resized cursor=resize-horizontal
      box #resize w=200.0 h=12.0 @bg-primary rounded-md
        text resize_x size=1.0 @text-primary
    box #canvas w=200.0 h=40.0
      canvas w=200.0 h=40.0
        event mouse pressed
          emit canvas_pressed
          capture
        rect x=0.0 y=0.0 w=canvas_width h=canvas_height fill=primary
        text canvas_presses x=4.0 y=16.0 color=fg size=12.0
    button "Open dialog" #open-dialog -> open_dialog
    box #status
      text status size=12.0 @text-fg
    overlay when=dialog_open dismiss=close_dialog backdrop=black/25 p=4.0 align-x=end align-y=end
      content
        text "Overlay page" size=12.0 @text-fg
      layer
        button "Close dialog" #close-dialog -> close_dialog

test render_contract
  viewport 320 360
  mount
    RenderContract #render
  target root = #render/root
  target heading = root/heading
  target field = root/field
  target draft = field/draft
  target increment = root/increment
  target result = root/result
  expect root.kind == "semantic"
  expect root.visible
  expect root.width ~= 240.0
  expect root.height > 0.0
  expect root.left ~= root.x
  expect root.top ~= root.y
  expect root.right ~= root.x + root.width
  expect root.bottom ~= root.y + root.height
  expect root.center_x ~= root.x + root.width / 2.0
  expect root.center_y ~= root.y + root.height / 2.0
  expect root.visible_x ~= root.x
  expect root.visible_y ~= root.y
  expect root.visible_width ~= root.width
  expect root.visible_height ~= root.height
  expect heading.x ~= root.x + 16.0
  expect heading.y ~= root.y + 16.0
  expect field.x ~= root.x + 16.0
  expect field.right ~= root.right - 16.0
  expect field.y ~= heading.bottom + 12.0
  expect draft.x ~= field.x
  expect draft.width ~= field.width
  expect draft.y > field.y
  expect increment.y ~= field.bottom + 12.0
  expect result.y ~= increment.bottom + 12.0
  expect result.bottom ~= root.bottom - 16.0
  expect root.background == background.color(color.rgb8(17, 17, 17))
  expect root.border.color == color.rgb8(51, 102, 255)
  expect root.border.width ~= 1.0
  expect root.border.radius == radius(10.0)
  expect root.shadow.color == color.rgba(0.0, 0.0, 0.0, 0.5)
  expect root.shadow.offset.x ~= 2.0
  expect root.shadow.offset.y ~= 3.0
  expect root.shadow.blur ~= 4.0
  expect heading.text_color == color.rgb8(51, 102, 255)
  expect heading.text_size ~= 18.0
  expect draft.background == background.color(color.rgb8(17, 17, 17))
  expect draft.border.color == color.rgb8(51, 102, 255)
  expect draft.border.width ~= 1.0
  expect draft.border.radius == radius(6.0)
  expect increment.background == background.color(color.rgb8(51, 102, 255))
  expect increment.border.radius == radius(6.0)
  expect increment.text_color == color.rgb8(238, 238, 238)
  expect increment.text_size ~= 16.0
  expect result.text_color == color.rgb8(238, 238, 238)
  expect result.text_size ~= 14.0
  expect result.font == font.default()
  expect result.line_height.kind == "absolute"
  click draft
  type "local"
  expect draft.value == "local"
  key tab
  key enter
  expect text "1" within result

test interaction_contract
  viewport 360 700
  mount
    InteractionContract #interactions
  target root = #interactions/root
  target button = #interactions/root/button
  target checkbox = #interactions/root/checkbox
  target toggler = #interactions/root/toggler
  target slider = #interactions/root/slider
  target radio = #interactions/root/radio
  target pick = #interactions/root/pick
  target mouse = #interactions/root/mouse
  target resize = #interactions/root/resize
  target canvas = #interactions/root/canvas
  target open_dialog = #interactions/root/open-dialog
  target status = #interactions/root/status
  target close_dialog = #interactions/root/close-dialog
  expect root.width ~= 320.0
  click button
  expect text "button routed" within status
  click checkbox
  expect text "checkbox routed" within status
  click toggler
  expect text "toggler routed" within status
  click slider
  expect text "slider routed" within status
  click radio
  expect text "radio routed" within status
  click pick
  expect text "pick opened" within status
  click pick
  expect text "pick closed" within status
  click mouse
  expect text "mouse routed" within status
  press resize
  hover open_dialog
  release
  expect text "resize routed" within status
  click canvas
  expect text "canvas routed" within status
  expect missing close_dialog
  click open_dialog
  expect text "overlay opened" within status
  expect exists close_dialog
  click close_dialog
  expect text "overlay closed" within status
  expect missing close_dialog

test layout_contract
  viewport 360 420
  mount
    LayoutContract #layout
  target root = #layout/root
  target row = #layout/root/row
  target left = #layout/root/row/left
  target right = #layout/root/row/right
  target grid = #layout/root/grid
  target first = #layout/root/grid/first
  target second = #layout/root/grid/second
  target flex = #layout/root/flex
  target start = #layout/root/flex/start
  target end = #layout/root/flex/end
  target stack = #layout/root/stack
  target base = #layout/root/stack/base
  target layer = #layout/root/stack/layer
  target scroll = #layout/root/scroll
  target content = #layout/root/scroll/content
  target responsive_content = #layout/root/responsive-content
  target space = #layout/root/space
  expect root.width ~= 320.0
  expect row.x ~= root.x + 10.0
  expect row.y ~= root.y + 10.0
  expect left.x ~= row.x + 4.0
  expect right.x ~= left.right + 8.0
  expect right.right ~= row.right - 4.0
  expect first.width ~= second.width
  expect second.x ~= first.right + 6.0
  expect start.x ~= flex.x
  expect end.right ~= flex.right
  expect base.x ~= stack.x
  expect base.y ~= stack.y
  expect layer.x ~= stack.x
  expect layer.y ~= stack.y
  expect scroll.kind == "semantic"
  expect scroll.content_x ~= content.x
  expect scroll.content_y ~= content.y
  expect scroll.content_width ~= 300.0
  expect scroll.content_height ~= 120.0
  expect scroll.translation_x ~= 0.0
  expect scroll.translation_y ~= 0.0
  expect scroll.scroll_x ~= 0.0
  expect scroll.scroll_y ~= 0.0
  expect content.height ~= 120.0
  expect responsive_content.width ~= 300.0
  expect responsive_content.height ~= 30.0
  expect space.width ~= 300.0
  expect space.height ~= 8.0
  resize 500 500
  expect root.width ~= 320.0

view
  row
    RenderContract #render
    LayoutContract #layout
    InteractionContract #interactions
    Counter label="First" #first
    Counter label="Second" #second
    Loader #loader
    EditableTitle #title

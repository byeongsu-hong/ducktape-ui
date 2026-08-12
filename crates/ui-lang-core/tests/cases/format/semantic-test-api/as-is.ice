app SemanticTestApi
theme contract AppTheme
    bg
    fg
    primary
    danger
palette app for AppTheme
    bg #ffffff
    fg #111111
    primary #3366ff
    danger #cc3344
state
    message = "idle"
    value = ""
on select(next)
    message = next
test semantic_driver
    viewport 800 600
    timeout 5s
    theme dark
    scale 2.0
    locale "ko-KR"
    platform linux
    reduced-motion true
    mount
        col #surface
            input "Value" #field <-> value
            button "Action" #control -> select("clicked")
            scroll #scroller
                col #content
                    text "Scrollable"
    target surface = #surface
    target field = surface/field
    target control = surface/control
    target scroller = surface/scroller
    hover control
    enter control
    leave
    move control
    move 12.0 18.0
    click control
    click control right
    double-click control left
    click-at 12.0 18.0 middle
    press control left
    release left
    wheel 0.0 -40.0
    wheel lines 0.0 -3.0
    scroll-to scroller 0.0 80.0
    scroll-by scroller 0.0 16.0
    snap scroller 0.0 0.5
    snap-end scroller
    drag control field
    press control
    drop field
    focus field
    focus-next
    focus-previous
    blur
    window focus
    window blur
    type "typed"
    clear
    replace "replacement"
    select 0 4
    select-all
    cursor 2
    cursor front
    cursor end
    composition start
    composition update "조"
    composition update "조합" 0 3
    composition commit "조"
    composition cancel
    key enter
    key "x"
    key TVInputHDMI1
    key-down escape modified=escape location=standard physical=IntlBackslash text="x" repeat=false
    key-up escape modified=escape location=standard physical=IntlBackslash
    modifiers shift control
    modifiers
    chord control shift "p"
    repeat backspace 2
    tap control
    tap control 2
    touch down 1 20.0 30.0
    touch move 1 24.0 36.0
    touch up 1 24.0 36.0
    touch down 2 40.0 50.0
    touch cancel 2 40.0 50.0
    window move 40.0 60.0
    window resize 1024.0 768.0
    resize 800.0 600.0
    window rescale 1.5
    window redraw
    window opened
    window closed
    system-theme light
    file-hover "/tmp/example.txt"
    file-drop "/tmp/example.txt"
    file-leave
    wait 20ms
    advance 16ms
    idle
    capture semantic_state
    a11y focus field
    a11y activate control
    expect a11y control role "button"
    expect a11y control name "Action"
    expect a11y field value "replacement"
    expect a11y control checked false
    expect a11y control disabled false
    expect a11y field focused true
    expect a11y control action click
    expect a11y control action focus false
    expect control.width > 0.0
    expect control.background == background.color(primary)
    expect control.surface_count >= 0
    expect control.text_count >= 0
    expect control.image_count >= 0
    expect control.text_x >= 0.0
    expect control.text_y >= 0.0
    expect control.text_width >= 0.0
    expect control.text_height >= 0.0
    expect control.text_baseline >= 0.0
    expect control.image_x >= 0.0
    expect control.image_y >= 0.0
    expect control.image_width >= 0.0
    expect control.image_height >= 0.0
    expect control.pixel_aligned || !control.pixel_aligned
    expect field.focused || !field.focused
    expect control.accessibility_role == "button"
    expect control.accessibility_name == "Action"
    expect control.accessibility_description == "action"
    expect field.accessibility_value == "replacement"
    expect control.accessibility_checked || !control.accessibility_checked
    expect control.accessibility_expanded || !control.accessibility_expanded
    expect !control.accessibility_disabled
    expect control.accessibility_supports_activate
    expect control.accessibility_supports_focus
    dispatch select("done")
    window close-request
view
    text message

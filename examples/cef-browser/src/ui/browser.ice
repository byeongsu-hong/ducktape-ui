app CefBrowser
  title "CEF in Ice"
  id "dev.ducktape.ice.cef-browser"
  text-size 14
  antialiasing true
  window
    size 1100 760
    min-size 1100 760
    max-size 1100 760
    resizable false
    position centered

extern crate::cef_runtime
  AttachResult(attached:bool, status:str)
  task attach(url:str) -> AttachResult
  sync pump() -> bool
  sync load(url:str) -> bool
  sync go_back() -> bool
  sync go_forward() -> bool
  sync reload() -> bool
  sync can_go_back() -> bool
  sync can_go_forward() -> bool

theme contract BrowserTheme
  bg
  chrome
  surface
  address
  fg
  muted
  border
  primary
  primary_hover
  on_primary
  disabled
  danger
  success

palette browser for BrowserTheme
  bg            #e8ebf1
  chrome        #f5f6f9
  surface       #ffffff
  address       #ffffff
  fg            #20232b
  muted         #747b89
  border        #d9dde5
  primary       #5368f5
  primary_hover #4559e8
  on_primary    #ffffff
  disabled      #b6bcc7
  danger        #d34b4b
  success       #28a66a

state
  address = "ice://welcome"
  attached = false
  runtime_active = false
  can_back = false
  can_forward = false
  status = "Starting Chromium Embedded Framework…"

derived
  can_navigate = attached && !empty(trim(address))

on mount
  task attach(address) -> attached_result _

on attached_result(result)
  attached = result.attached
  status = result.status

on tick(_now)
  runtime_active = pump()
  can_back = can_go_back()
  can_forward = can_go_forward()

on navigate
  return if !can_navigate
  runtime_active = load(address)
  status = "Navigating…"

on back
  runtime_active = go_back()
  status = "Back"

on forward
  runtime_active = go_forward()
  status = "Forward"

on refresh
  runtime_active = reload()
  status = "Reloading…"

subscribe
  every 16ms -> tick _

view
  col #root
    with
      w=fill
      h=fill
      gap=0.0
      @bg-bg
    box #toolbar
      with
        w=fill
        h=68.0
        px=18.0
        border=border
        border-w=1.0
        @bg-chrome
      row
        with
          w=fill
          h=fill
          align=center
          gap=10.0
        button #back -> back
          with
            label="Back"
            w=40.0
            h=40.0
            p=0.0
            disabled=!can_back
          text "←"
            with
              size=18.0
              align-x=center
              align-y=center
          active bg=transparent text=fg border=transparent border-w=0.0 r=12.0
          hovered bg=surface text=fg shadow=black/8 shadow-y=1.0 shadow-blur=5.0
          pressed bg=border text=fg
          disabled bg=transparent text=disabled border=transparent border-w=0.0 r=12.0
        button #forward -> forward
          with
            label="Forward"
            w=40.0
            h=40.0
            p=0.0
            disabled=!can_forward
          text "→"
            with
              size=18.0
              align-x=center
              align-y=center
          active bg=transparent text=fg border=transparent border-w=0.0 r=12.0
          hovered bg=surface text=fg shadow=black/8 shadow-y=1.0 shadow-blur=5.0
          pressed bg=border text=fg
          disabled bg=transparent text=disabled border=transparent border-w=0.0 r=12.0
        button #refresh -> refresh
          with
            label="Reload"
            w=40.0
            h=40.0
            p=0.0
            disabled=(!attached || !runtime_active)
          text "↻"
            with
              size=19.0
              align-x=center
              align-y=center
          active bg=transparent text=fg border=transparent border-w=0.0 r=12.0
          hovered bg=surface text=fg shadow=black/8 shadow-y=1.0 shadow-blur=5.0
          pressed bg=border text=fg
          disabled bg=transparent text=disabled border=transparent border-w=0.0 r=12.0
        box #address-shell
          with
            w=fill
            h=42.0
            px=14.0
            align-y=center
            bg=address
            border=border
            border-w=1.0
            r=14.0
            shadow=black/7
            shadow-y=2.0
            shadow-blur=8.0
          row
            with
              w=fill
              h=fill
              align=center
              gap=11.0
            text "◉" size=14.0 @text-success
            input "" #address <-> address
              with
                label="Address"
                hint="Search or enter an address"
                w=fill
                p=6.0
                submit=navigate
              active bg=transparent border=transparent border-w=0.0 value=fg placeholder=muted selection=primary
              hovered bg=transparent border=transparent border-w=0.0 value=fg placeholder=muted
              focused bg=transparent border=transparent border-w=0.0 value=fg placeholder=muted selection=primary
        button #go -> navigate
          with
            label="Go to address"
            w=74.0
            h=40.0
            p=0.0
            disabled=!can_navigate
          row align=center gap=7.0
            text "Go" size=13.0 @font-bold
            text "→" size=17.0
          active bg=primary text=on_primary r=12.0 shadow=primary/18 shadow-y=3.0 shadow-blur=8.0
          hovered bg=primary_hover text=on_primary
          pressed bg=primary_hover text=on_primary shadow=transparent
          disabled bg=disabled text=on_primary r=12.0 shadow=transparent
    box #browser-surface
      with
        w=fill
        h=fill
        align-x=center
        align-y=center
        @bg-surface
      col align=center gap=10.0
        if !attached
          text "Waiting for the CEF child window"
            with
              size=18.0
              @font-bold
              @text-fg
          text status size=13.0 @text-muted
        if attached
          text "CEF owns this native child region" size=14.0 @text-muted

test renders_ice_chrome_without_cef
  viewport 1100 760
  target toolbar = #root/toolbar
  target address_input = #root/toolbar/address-shell/address
  target browser = #root/browser-surface
  expect toolbar.width == 1100.0
  expect address_input.value == "ice://welcome"
  expect browser.height == 692.0

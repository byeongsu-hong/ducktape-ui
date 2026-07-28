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
  surface
  raised
  fg
  muted
  border
  primary
  primary_hover
  on_primary
  danger
  success

palette browser for BrowserTheme
  bg       #eef2f7
  surface  #ffffff
  raised   #f8fafc
  fg       #172033
  muted    #64748b
  border   #d8e0eb
  primary  #2563eb
  primary_hover #1d4ed8
  on_primary #ffffff
  danger   #dc2626
  success  #16a34a

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
    box #status-bar
      with
        w=fill
        h=36.0
        px=16.0
        border=border
        border-w=1.0
        @bg-raised
      row
        with
          w=fill
          h=fill
          align=center
          gap=10.0
        text "ICE + CEF"
          with
            size=12.0
            @font-bold
            @text-fg
        box
          with
            w=8.0
            h=8.0
            r=4.0
            @bg-success
          space w=8.0 h=8.0
        text status
          with
            w=fill
            size=12.0
            @text-muted
        text "native child surface" size=11.0 @text-muted
    box #toolbar
      with
        w=fill
        h=60.0
        px=12.0
        border=border
        border-w=1.0
        @bg-surface
      row
        with
          w=fill
          h=fill
          align=center
          gap=8.0
        button "←" #back -> back
          with
            label="Back"
            w=40.0
            h=36.0
            disabled=!can_back
          active bg=raised text=fg border=border border-w=1.0 r=8.0
          hovered bg=bg
          disabled bg=raised text=muted border=border border-w=1.0 r=8.0
        button "→" #forward -> forward
          with
            label="Forward"
            w=40.0
            h=36.0
            disabled=!can_forward
          active bg=raised text=fg border=border border-w=1.0 r=8.0
          hovered bg=bg
          disabled bg=raised text=muted border=border border-w=1.0 r=8.0
        button "↻" #refresh -> refresh
          with
            label="Reload"
            w=40.0
            h=36.0
            disabled=!attached
          active bg=raised text=fg border=border border-w=1.0 r=8.0
          hovered bg=bg
          disabled bg=raised text=muted border=border border-w=1.0 r=8.0
        input "Enter a URL" #address <-> address
          with
            w=fill
            p=10.0
            submit=navigate
            @bg-raised
          active border=border border-w=1.0 r=8.0
          focused border=primary border-w=2.0 r=8.0
        button "Go" #go -> navigate
          with
            w=58.0
            h=36.0
            disabled=!can_navigate
          active bg=primary text=on_primary r=8.0
          hovered bg=primary_hover
          disabled bg=muted text=on_primary r=8.0
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
  target address_input = #root/toolbar/address
  target browser = #root/browser-surface
  expect toolbar.width == 1100.0
  expect address_input.value == "ice://welcome"
  expect browser.height == 664.0

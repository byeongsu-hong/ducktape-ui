app TerminalWorkspace
  title "Terminal"
  palette AppTheme.terminal
  id "dev.ducktape.ice.terminal"
  text-size 14
  antialiasing true
  window
    size 1100 720
    min-size 820 460
    position centered

use "theme.ice"
use "extern/terminal.ice"
use "tests/app.ice"

font body family=sans default=true
font strong family=sans weight=semibold
font code family=mono

state
  environment:Environment = detect_environment()
  session:Session = idle_session()
  kind = "shell"
  target = ""
  directory = ""
  running = false
  busy = false
  title = ""
  error = ""

derived
  ssh_ready = kind != "ssh" || !empty(trim(target))
  tool_available = (kind == "shell") || (kind == "ssh" && environment.ssh_available) || (kind == "claude" && environment.claude_available) || (kind == "codex" && environment.codex_available)
  can_start = !busy && ssh_ready && tool_available
  has_error = !empty(error)

preset test
  state
    directory = "/workspace/ice-terminal"

preset active_test
  state
    directory = "/workspace/ice-terminal"
    title = "Claude Code"
    running = true

on mount
  environment = detect_environment()
  directory = environment.directory

on kind_changed(next)
  kind = next
  error = ""

on start
  return if !can_start
  busy = true
  error = ""
  run every start_session(kind, target, directory) -> started _ | failed _

on started(result)
  session = result.session
  title = result.title
  running = true
  busy = false
  task focus_terminal(session) -> terminal_focused

on terminal_focused

on stop
  session = idle_session()
  running = false
  busy = false
  title = ""

on terminal_notice(notice)
  running = notice.running
  return if empty(notice.title)
  title = notice.title

on failed(cause)
  busy = false
  error = cause.message

subscribe
  terminal_events(session) when running -> terminal_notice _

component SessionChoice(label:str, value:str, selected:bool) -> str
  radio label #root -> emit(_)
    with
      value=value
      selected=selected
      w=fill
      size=15.0
      gap=8.0
      text-size=12.5
    active selected bg=accent_soft dot=accent border=accent border-w=2.0 text=fg
    active unselected bg=surface dot=muted border=border border-w=1.0 text=fg
    hovered selected bg=accent_soft dot=accent_hover border=accent_hover border-w=2.0 text=fg
    hovered unselected bg=raised dot=accent border=muted border-w=1.0 text=fg

view
  stack #app-root w=fill h=fill
    box
      with
        w=fill
        h=fill
        bg=bg
      space w=fill h=fill
    if running
      col
        with
          w=fill
          h=fill
          gap=0.0
        box #terminal-toolbar
          with
            w=fill
            h=42.0
            bg=surface
            border=border
            border-w=1.0
            px=12.0
          row
            with
              w=fill
              h=fill
              gap=10.0
              align=center
            row gap=7.0 align=center
              box
                with
                  w=7.0
                  h=7.0
                  r=4.0
                  bg=success
                space w=fill h=fill
              text "LIVE"
                with
                  size=10.0
                  font=code
                  @text-success
            text title
              with
                size=12.5
                font=strong
                @text-fg
            space w=fill
            text "Ctrl+Shift+C / V"
              with
                size=10.5
                font=code
                @text-subtle
            button "End" @stop_action -> stop
        box #terminal-panel
          with
            w=fill
            h=fill
            bg=terminal
            clip=true
          extern terminal_surface(session) #terminal-surface
    if !running
      box
        with
          w=fill
          h=fill
          align-x=center
          align-y=center
          p=28.0
        col w=760.0 gap=18.0
          col gap=6.0
            row
              with
                w=fill
                gap=8.0
                align=center
              text "DUCKTAPE"
                with
                  size=10.5
                  font=code
                  @text-accent
              text "/"
                with
                  size=10.5
                  font=code
                  @text-subtle
              text "TERMINAL"
                with
                  size=10.5
                  font=code
                  @text-muted
            text "New terminal"
              with
                size=29.0
                font=strong
                @text-fg
            text "Pick a runtime and start in the directory you are already working in."
              with
                size=12.5
                @text-muted
          box
            with
              w=fill
              bg=surface
              border=border
              border-w=1.0
              r=14.0
              p=20.0
              shadow=black/24
              shadow-y=8.0
              shadow-blur=24.0
            col w=fill gap=18.0
              col w=fill gap=9.0
                text "SESSION"
                  with
                    size=10.5
                    font=code
                    @text-subtle
                row w=fill gap=12.0
                  SessionChoice #shell-choice -> kind_changed _
                    with
                      label="Shell"
                      value="shell"
                      selected=(kind == "shell")
                  SessionChoice #ssh-choice -> kind_changed _
                    with
                      label="SSH"
                      value="ssh"
                      selected=(kind == "ssh")
                  SessionChoice #claude-choice -> kind_changed _
                    with
                      label="Claude"
                      value="claude"
                      selected=(kind == "claude")
                  SessionChoice #codex-choice -> kind_changed _
                    with
                      label="Codex"
                      value="codex"
                      selected=(kind == "codex")
              col w=fill gap=7.0
                text "WORKING DIRECTORY"
                  with
                    size=10.5
                    font=code
                    @text-subtle
                input "" #directory-field <-> directory
                  with
                    label="Local working directory"
                    hint="Directory path or ~"
                    w=fill
                    p=10.0
                    text-size=13.0
                  active bg=raised border=border border-w=1.0 r=8.0 placeholder=muted value=fg selection=accent
                  hovered bg=raised border=muted border-w=1.0 r=8.0 placeholder=muted value=fg selection=accent
                  focused bg=raised border=accent border-w=2.0 r=8.0 placeholder=muted value=fg selection=accent
                  focused-hovered bg=raised border=accent_hover border-w=2.0 r=8.0 placeholder=muted value=fg selection=accent
                  disabled bg=surface border=border border-w=1.0 r=8.0 placeholder=subtle value=subtle selection=accent
              if kind == "ssh"
                col w=fill gap=7.0
                  text "SSH DESTINATION"
                    with
                      size=10.5
                      font=code
                      @text-subtle
                  input "" #target-field <-> target
                    with
                      label="SSH destination"
                      hint="user@host or -p 2222 user@host"
                      w=fill
                      p=10.0
                      text-size=13.0
                    active bg=raised border=border border-w=1.0 r=8.0 placeholder=muted value=fg selection=accent
                    hovered bg=raised border=muted border-w=1.0 r=8.0 placeholder=muted value=fg selection=accent
                    focused bg=raised border=accent border-w=2.0 r=8.0 placeholder=muted value=fg selection=accent
                    focused-hovered bg=raised border=accent_hover border-w=2.0 r=8.0 placeholder=muted value=fg selection=accent
                    disabled bg=surface border=border border-w=1.0 r=8.0 placeholder=subtle value=subtle selection=accent
              if !tool_available
                text "The selected command is not available on PATH." size=12.0 @text-danger
              if has_error
                box
                  with
                    w=fill
                    bg=danger/10
                    border=danger/35
                    border-w=1.0
                    r=8.0
                    p=10.0
                  text error
                    with
                      size=12.0
                      wrap=word
                      @text-danger
              row
                with
                  w=fill
                  gap=12.0
                  align=center
                button "Start terminal" disabled=!can_start @primary_action -> start
                text environment.shell
                  with
                    size=10.5
                    font=code
                    wrap=glyph
                    @text-subtle

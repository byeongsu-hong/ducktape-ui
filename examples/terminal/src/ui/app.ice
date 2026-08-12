app TerminalWorkspace
  title "Ice Terminal"
  palette AppTheme.terminal
  id "dev.ducktape.ice.terminal"
  text-size 14
  antialiasing true
  window
    size 1180 760
    min-size 760 520
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
      size=16.0
      gap=8.0
      text-size=13.0
    active selected bg=raised dot=primary border=primary border-w=2.0 text=fg
    active unselected bg=surface dot=muted border=border border-w=1.0 text=fg
    hovered selected bg=hover dot=primary border=primary_hover border-w=2.0 text=fg
    hovered unselected bg=raised dot=primary border=muted border-w=1.0 text=fg

view
  box
    with
      w=fill
      h=fill
      bg=bg
    col
      with
        w=fill
        h=fill
        gap=0.0
      box
        with
          w=fill
          bg=surface
          border=border
          border-w=1.0
          px=24.0
          py=16.0
        row
          with
            w=fill
            gap=12.0
            align=center
          col w=fill gap=3.0
            text "Ice Terminal"
              with
                size=20.0
                font=strong
                @text-fg
            text "Local shell, SSH, Claude Code, and Codex in a native PTY" size=12.0 @text-muted
          if running
            text "● Running" size=12.0 @text-success
          if !running
            text "● Idle" size=12.0 @text-subtle
      row
        with
          w=fill
          h=fill
          gap=16.0
          p=16.0
        box
          with
            w=300.0
            h=fill
            bg=surface
            border=border
            border-w=1.0
            r=12.0
            p=18.0
          col w=fill gap=18.0
            col w=fill gap=9.0
              text "Session"
                with
                  size=15.0
                  font=strong
                  @text-fg
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
                  label="Claude Code"
                  value="claude"
                  selected=(kind == "claude")
              SessionChoice #codex-choice -> kind_changed _
                with
                  label="Codex"
                  value="codex"
                  selected=(kind == "codex")
            col w=fill gap=6.0
              text "Availability" size=12.0 @text-muted
              row gap=10.0 wrap
                if environment.ssh_available
                  text "SSH ready" size=11.0 @text-success
                if !environment.ssh_available
                  text "SSH missing" size=11.0 @text-danger
                if environment.claude_available
                  text "Claude ready" size=11.0 @text-success
                if !environment.claude_available
                  text "Claude missing" size=11.0 @text-subtle
                if environment.codex_available
                  text "Codex ready" size=11.0 @text-success
                if !environment.codex_available
                  text "Codex missing" size=11.0 @text-subtle
            if kind == "ssh"
              col w=fill gap=6.0
                text "SSH destination" size=12.0 @text-muted
                input "" #target-field <-> target
                  with
                    label="SSH destination"
                    hint="ssh user@host or ssh -p 2222 user@host"
                    w=fill
                    p=9.0
                    text-size=13.0
                  active bg=raised border=border border-w=1.0 r=8.0 placeholder=muted value=fg selection=primary
                  hovered bg=raised border=muted border-w=1.0 r=8.0 placeholder=muted value=fg selection=primary
                  focused bg=raised border=primary border-w=2.0 r=8.0 placeholder=muted value=fg selection=primary
                  focused-hovered bg=raised border=primary_hover border-w=2.0 r=8.0 placeholder=muted value=fg selection=primary
                  disabled bg=surface border=border border-w=1.0 r=8.0 placeholder=subtle value=subtle selection=primary
            col w=fill gap=6.0
              text "Local working directory" size=12.0 @text-muted
              input "" #directory-field <-> directory
                with
                  label="Local working directory"
                  hint="Directory path or ~"
                  w=fill
                  p=9.0
                  text-size=13.0
                active bg=raised border=border border-w=1.0 r=8.0 placeholder=muted value=fg selection=primary
                hovered bg=raised border=muted border-w=1.0 r=8.0 placeholder=muted value=fg selection=primary
                focused bg=raised border=primary border-w=2.0 r=8.0 placeholder=muted value=fg selection=primary
                focused-hovered bg=raised border=primary_hover border-w=2.0 r=8.0 placeholder=muted value=fg selection=primary
                disabled bg=surface border=border border-w=1.0 r=8.0 placeholder=subtle value=subtle selection=primary
            if !tool_available
              text "The selected command is not available on PATH."
                with
                  size=12.0
                  wrap=word
                  @text-danger
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
            row w=fill gap=8.0
              button "Start session" disabled=!can_start @primary_action -> start
              button "Stop" disabled=!running @secondary_action -> stop
            col w=fill gap=3.0
              text "Shell" size=11.0 @text-subtle
              text environment.shell
                with
                  size=11.0
                  font=code
                  wrap=glyph
                  @text-muted
        col
          with
            w=fill
            h=fill
            gap=8.0
          row
            with
              w=fill
              gap=8.0
              align=center
            if empty(title)
              text "No active session"
                with
                  size=13.0
                  font=strong
                  @text-fg
            if !empty(title)
              text title
                with
                  size=13.0
                  font=strong
                  @text-fg
            space w=fill
            if running
              text "Ready for input · use your platform copy/paste shortcuts" size=11.0 @text-subtle
          box
            with
              w=fill
              h=fill
              bg=terminal
              border=terminal_border
              border-w=1.0
              r=12.0
              clip=true
            stack w=fill h=fill
              if running
                extern terminal_surface(session)
              if !running
                box
                  with
                    w=fill
                    h=fill
                    align-x=center
                    align-y=center
                    p=24.0
                  col gap=8.0 align=center
                    text ">_"
                      with
                        size=32.0
                        font=code
                        @text-subtle
                    text "Choose a session and start it." @text-muted

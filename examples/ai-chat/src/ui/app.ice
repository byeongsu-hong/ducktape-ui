app AiChat
  title "Codex"
  palette active_palette
  id "dev.ducktape.ice.ai-chat"
  font "../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../assets/fonts/Geist-Bold.ttf"
  font "../../../../assets/fonts/Geist-Italic.ttf"
  font "../../../../assets/fonts/GeistMono-Regular.ttf"
  font "../../../../assets/fonts/GeistMono-Bold.ttf"
  text-size 14
  window
    size 920 800
    min-size 560 480
    position centered

use "../../../../crates/ui/src/ice/default.ice"
use "theme.ice"
use "extern/codex.ice"
use "entries.ice"
use "handlers.ice"
use "tests/app.ice"

font geist family="Geist" default=true
font code family="Geist Mono"

state
  session:Session = codex_session()
  account:str = codex_account()
  model:str = codex_model()
  entries:[Entry] = []
  live:markdown = ""
  live_thinking:markdown = ""
  status = ""
  busy = false
  error = ""
  draft = ""
  dark = false
  active_palette:palette[AppTheme] = AppTheme.app

derived
  can_send = !busy && !empty(trim(draft))

// Deterministic states for capture and test. Each names its own account, so a
// real address never reaches a screenshot committed to this repository.
preset conversation
  state
    account = "you@example.com"
    model = "gpt-5.6-sol"
    session = sample_session(false)
    entries = sample_entries(false)

preset conversation_night
  state
    account = "you@example.com"
    model = "gpt-5.6-sol"
    session = sample_session(true)
    entries = sample_entries(true)
    dark = true
    active_palette = AppTheme.night

preset streaming
  state
    account = "you@example.com"
    model = "gpt-5.6-sol"
    session = sample_session(false)
    entries = sample_entries(false)
    busy = true
    status = "Responding"
    live_thinking = markdown("**Checking the append path**\n\nThe question is whether the tail alone is reparsed.")
    live = markdown("Yes — hold the parsed document and extend it:\n\n```rust\ncontent.push_str(&delta);\n```")

preset signed_out
  state
    account = ""
    model = "gpt-5.6-sol"
    error = "No Codex login at ~/.codex/auth.json. Run `codex login` first."

component Welcome() -> str
  col #root
    with
      w=fill
      py=46.0
      gap=18.0
      align=center
    Avatar.Agent initials="C"
    text "What are we working on?" @display
    text "Signed in through the Codex CLI. Reasoning, searches and the answer all land here."
      with
        @caption
    row gap=9.0
      button "Explain a codebase" #ask-explain -> emit("Explain how an indentation-based UI language compiles to Rust.")
        with
          @outline_action
      button "Check a version" #ask-version -> emit("What is the latest released version of the iced crate?")
        with
          @outline_action

view
  col #app
    with
      w=fill
      h=fill
      @bg-bg
    box #header
      with
        w=fill
        px=22.0
        py=12.0
        bg=surface
      row
        with
          w=fill
          gap=12.0
          align=center
        Avatar.Agent initials="C"
        col gap=2.0
          text "Codex" @pane_header
          text model @meta
        space w=fill
        if !empty(account)
          Typography.Machine content=account
        if dark
          button "Day" #theme-day @ghost_action -> use_day
        if !dark
          button "Night" #theme-night @ghost_action -> use_night
        button "New chat" #new-chat @ghost_action -> reset
    Separator
    if !empty(error)
      box
        with
          w=fill
          px=22.0
          pt=14.0
        Alert.Destructive title="That turn did not finish" description=error
    scroll #transcript w=fill h=fill
      box
        with
          w=fill
          px=22.0
          py=28.0
          align-x=center
        box w=fill max-w=760.0
          col w=fill gap=18.0
            if empty(entries) && !busy
              Welcome #welcome -> suggest _
            // `keyed` gives every row a stable identity and `lazy` keys its
            // rebuild on the row itself, so a token landing in the live reply
            // below rebuilds one row's widgets and nothing else.
            keyed entry in entries by=entry.id #rows w=fill gap=18.0
              lazy entry as settled
                col w=fill
                  // One `if` per kind rather than a `match`: the kinds are
                  // disjoint, so first-match ordering buys nothing here.
                  if settled.kind == "prompt"
                    Prompt #prompt(settled.id) body=settled.body
                  if settled.kind == "reasoning"
                    Reasoning #reasoning(settled.id) -> toggle_row _
                      with
                        row_id=settled.id
                        title=settled.title
                        body=settled.body
                        open=settled.open
                  if settled.kind == "tool"
                    ToolCall #tool(settled.id)
                      with
                        title=settled.title
                        detail=settled.detail
                        status=settled.status
                  if settled.kind == "answer"
                    Answer #answer(settled.id) -> copy_link _
                      with
                        row_id=settled.id
                        body=settled.body
                        dark=settled.dark
                  if settled.kind == "usage"
                    Usage #usage(settled.id) detail=settled.detail
            if busy
              col #live w=fill gap=18.0
                box #live-work
                  with
                    w=fill
                    px=15.0
                    py=12.0
                    bg=muted_bg
                    border=border
                    border-w=1.0
                    r=10.0
                  col w=fill gap=7.0
                    row gap=8.0 align=center
                      text "◌" size=12.0 @text-warning
                      text status @field_label
                    markdown live_thinking #live-thinking gap=6.0 text-size=12.5 -> copy_link _
                markdown live #live-body -> copy_link _
                  with
                    gap=10.0
                    text-size=13.5
                    code-size=12.5
    Separator
    box #composer
      with
        w=fill
        px=22.0
        py=16.0
        bg=surface
      box w=fill align-x=center
        box w=fill max-w=760.0
          col w=fill gap=9.0
            row
              with
                w=fill
                gap=9.0
                align=center
              input "" #draft <-> draft
                with
                  hint="Message Codex…"
                  description="Message Codex"
                  submit=send
                  @control
              button "Send" #send disabled=!can_send @primary_action -> send
            row
              with
                w=fill
                gap=7.0
                align=center
              Kbd label="Enter"
              text "send" @meta_compact
              space w=fill
              if busy
                text status @meta_compact

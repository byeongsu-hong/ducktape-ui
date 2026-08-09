app AiChat
  title "Codex"
  palette active_palette
  id "dev.ducktape.ice.ai-chat"
  font "../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-Bold.ttf"
  font "../../../../assets/fonts/IBMPlexSans-Italic.ttf"
  font "../../../../assets/fonts/MonoplexKR-Regular.ttf"
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

font body family="IBM Plex Sans KR" default=true
font code family="Monoplex KR"

state
  session:Session = codex_session()
  account:str = codex_account()
  model:str? = some(codex_model())
  models:[str] = codex_models()
  effort:str? = some(codex_effort())
  efforts:[str] = codex_efforts(codex_model())
  entries:[Entry] = []
  live:markdown = ""
  live_thinking:markdown = ""
  status = ""
  busy = false
  error = ""
  draft = ""
  signed:bool = signed_in()
  code = ""
  code_url = ""
  signing_in = false
  dark = false
  active_palette:palette[AppTheme] = AppTheme.app

derived
  can_send = !busy && !empty(trim(draft))

// Deterministic states for capture and test. Each names its own account, so a
// real address never reaches a screenshot committed to this repository.
preset conversation
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol", "gpt-5.5", "gpt-5.4-mini"]
    effort = some("xhigh")
    efforts = ["low", "medium", "high", "xhigh"]
    session = sample_session(false)
    entries = sample_entries(false)

preset conversation_night
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol", "gpt-5.5", "gpt-5.4-mini"]
    effort = some("xhigh")
    efforts = ["low", "medium", "high", "xhigh"]
    session = sample_session(true)
    entries = sample_entries(true)
    dark = true
    active_palette = AppTheme.night

preset streaming
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol", "gpt-5.5", "gpt-5.4-mini"]
    effort = some("xhigh")
    efforts = ["low", "medium", "high", "xhigh"]
    session = sample_session(false)
    entries = sample_running(false)
    busy = true
    status = "Responding"
    live_thinking = markdown("**Checking the append path**\n\nThe question is whether the tail alone is reparsed.")
    live = markdown("Yes — hold the parsed document and extend it:\n\n```rust\ncontent.push_str(&delta);\n```")

// An empty chat with nobody's address in it, for a test that starts from the
// welcome screen and captures what it reaches.
preset signed_in
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol", "gpt-5.5", "gpt-5.4-mini"]
    effort = some("xhigh")
    efforts = ["low", "medium", "high", "xhigh"]

preset signed_out
  state
    account = ""
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol", "gpt-5.5", "gpt-5.4-mini"]
    effort = some("xhigh")
    efforts = ["low", "medium", "high", "xhigh"]
    signed = false

preset signing_in
  state
    account = ""
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol", "gpt-5.5", "gpt-5.4-mini"]
    effort = some("xhigh")
    efforts = ["low", "medium", "high", "xhigh"]
    signed = false
    signing_in = true
    code = "I7DK-7UOAM"
    code_url = "https://auth.openai.com/deviceauth/usercode"

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
        py=9.0
        bg=surface
      row
        with
          w=fill
          gap=12.0
          align=center
        Avatar.Agent initials="C"
        text "Codex" @pane_header
        space w=fill
        if !empty(account)
          Typography.Machine content=account
        if dark
          button "Day" #theme-day @ghost_action -> use_day
        if !dark
          button "Night" #theme-night @ghost_action -> use_night
        if signed
          button "New chat" #new-chat @outline_action -> reset
        if signed
          button "Sign out" #sign-out @ghost_action -> forget
    Separator
    if !empty(error)
      box
        with
          w=fill
          px=22.0
          pt=14.0
        Alert.Destructive title="That turn did not finish" description=error
    if !signed
      box #signin
        with
          w=fill
          h=fill
          px=22.0
          align-x=center
        box w=fill max-w=460.0
          col
            with
              w=fill
              py=64.0
              gap=18.0
              align=center
            Avatar.Agent initials="C"
            text "Sign in to Codex" @display
            if empty(code)
              text "This window signs in on its own. It never sees a password, and it keeps what it is granted in its own file rather than the CLI's."
                with
                  @caption
              button "Sign in" #start-sign-in disabled=signing_in @primary_action -> sign_in
            if !empty(code)
              text "Open this page and enter the code." @caption
              Typography.Machine content=code_url
              box
                with
                  px=22.0
                  py=15.0
                  bg=muted_bg
                  border=border
                  border-w=1.0
                  r=12.0
                text code
                  with
                    size=27.0
                    font=code
                    @text-fg
              row gap=8.0
                button "Copy code" #copy-code @outline_action -> copy_code
                button "Copy link" #copy-link @outline_action -> copy_url
              text "Waiting for it to be approved…" @meta
    if signed
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
                    if settled.kind == "work"
                      Work #work(settled.id) -> toggle_row _
                        with
                          row_id=settled.id
                          title=settled.title
                          open=settled.open
                    if settled.kind == "tool"
                      ToolCall #tool(settled.id) -> toggle_row _
                        with
                          row_id=settled.id
                          title=settled.title
                          detail=settled.detail
                          status=settled.status
                          open=settled.open
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
                        style inline-code-bg=accent inline-code-fg=accent_fg inline-code-font=code inline-code-px=4.0 inline-code-py=1.0 inline-code-r=4.0 link=brand
                  markdown live #live-body -> copy_link _
                    with
                      gap=10.0
                      text-size=13.5
                      code-size=12.5
                    style inline-code-bg=accent inline-code-fg=accent_fg inline-code-font=code inline-code-px=4.0 inline-code-py=1.0 inline-code-r=4.0 link=brand
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
              box #field
                with
                  w=fill
                  px=6.0
                  py=6.0
                  bg=muted_bg
                  border=border
                  border-w=1.0
                  r=17.0
                row
                  with
                    w=fill
                    gap=6.0
                    align=center
                  input "" #draft <-> draft
                    with
                      submit=send
                      hint="Message Codex…"
                      description="Message Codex"
                      w=fill
                      p=8.0
                      text-size=13.5
                    active bg=muted_bg border=muted_bg placeholder=muted value=fg selection=primary
                    focused-hovered bg=muted_bg border=muted_bg
                    disabled bg=muted_bg border=muted_bg value=muted
                  button "↑" #send -> send
                    with
                      disabled=!can_send
                      label="Send"
                      w=30.0
                      h=30.0
                    active bg=primary text=primary_fg r=15.0
                    hovered bg=primary_hover text=primary_fg r=15.0
                    disabled bg=disabled text=disabled_fg r=15.0
              // What will answer sits with the message about to be sent, not
              // up in the chrome: it is a property of this turn, not the app.
              row
                with
                  w=fill
                  gap=6.0
                  align=center
                Chip #model options=models selected=model -> choose_model _
                Chip #effort options=efforts selected=effort -> choose_effort _
                space w=fill
                if busy
                  text status @meta_compact
                if !busy
                  Kbd label="Enter"
                if !busy
                  text "send" @meta_compact

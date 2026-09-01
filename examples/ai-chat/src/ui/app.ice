app AiChat
  title "Codex"
  palette active_palette
  id "dev.ducktape.ice.ai-chat"
  font "../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-Bold.ttf"
  font "../../../../assets/fonts/IBMPlexSans-Italic.ttf"
  font "../../../../crates/ui-lang-components/assets/fonts/JetBrainsMono-Regular.ttf"
  font "../../../../crates/ui-lang-components/assets/fonts/JetBrainsMono-Bold.ttf"
  font "../../../../crates/ui-lang-components/assets/fonts/JetBrainsMono-Italic.ttf"
  text-size 14
  window
    // Wide enough that the sidebar is beside the conversation rather than
    // taking from it: 252 for the list, and the rest leaves the transcript its
    // full 760-wide column with margins instead of squeezing it to 620.
    size 1180 800
    min-size 760 480
    position centered

use "../../../../crates/ui-lang-components/src/ice/default.ice"
use "theme.ice"
use "extern/codex.ice"
use "entries.ice"
use "handlers.ice"
use "tests/app.ice"

// One face for the whole window. JetBrains Mono has no Hangul, so IBM Plex
// Sans KR stays loaded underneath it: a Korean word falls back to the face
// that has the glyphs rather than to a box.
font body family="JetBrains Mono" default=true
font code family="JetBrains Mono"

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
  draft:editor = ""
  copied = ""
  chats:[Chat] = []
  open_path = ""
  scan_ratio = 0.0
  scan_total = 0
  loading_chat = false
  signed:bool = signed_in()
  code = ""
  code_url = ""
  signing_in = false
  dark = false
  active_palette:palette[AppTheme] = AppTheme.app

derived
  scanning = scan_total > 0 && scan_ratio < 1.0
  typed = trim(editor_text(draft))
  can_send = !busy && !empty(typed)
  can_steer = busy && !empty(typed)

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

// One answer on its own, for the drag a selection is made with.
preset one_answer
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol"]
    effort = some("xhigh")
    efforts = ["xhigh"]
    entries = sample_answer()

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

// Mid-turn with something already typed: the three things that can be done
// with a running turn are all on screen.
preset steering
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol", "gpt-5.5", "gpt-5.4-mini"]
    effort = some("xhigh")
    efforts = ["low", "medium", "high", "xhigh"]
    entries = sample_running(false)
    busy = true
    status = "Responding"
    draft = editor("Actually, check the changelog instead")
    live_thinking = markdown("**Checking the append path**")
    live = markdown("Yes — hold the parsed document and extend it.")

// Part way through reading the store: some chats are already listed and the
// bar says how far it has got.
preset scanning
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol"]
    effort = some("xhigh")
    efforts = ["xhigh"]
    entries = sample_entries(false)
    chats = sample_chats()
    scan_ratio = 0.38
    scan_total = 420

// The chats this window has had, as the panel that offers them.
preset history
  state
    open_path = "/sessions/4.jsonl"
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol", "gpt-5.5", "gpt-5.4-mini"]
    effort = some("xhigh")
    efforts = ["low", "medium", "high", "xhigh"]
    entries = sample_entries(false)
    chats = sample_chats()

// A transcript the size an opened chat can reach, for measuring what drawing
// one costs end to end.
preset opened_chat
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol"]
    effort = some("xhigh")
    efforts = ["xhigh"]
    chats = sample_chats()
    entries = sample_transcript(500)

preset small_chat
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol"]
    effort = some("xhigh")
    efforts = ["xhigh"]
    chats = sample_chats()
    entries = sample_transcript(8)

// A chat being read off disk. The read happens on another thread, so this is
// the only thing that says it is happening at all.
preset opening
  state
    signed = true
    account = "you@example.com"
    model = some("gpt-5.6-sol")
    models = ["gpt-5.6-sol"]
    effort = some("xhigh")
    efforts = ["xhigh"]
    chats = sample_chats()
    open_path = "/sessions/2.jsonl"
    entries = sample_entries(false)
    loading_chat = true

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
  row #shell
    with
      w=fill
      h=fill
      @bg-bg
    box #sidebar
      with
        w=252.0
        h=fill
        bg=surface
      col
        with
          w=fill
          h=fill
          p=10.0
          gap=8.0
        button "New chat" #new-chat -> reset
          with
            w=fill
            disabled=(busy || loading_chat)
            @outline_action
        row
          with
            w=fill
            gap=8.0
            align=center
          text "Recent" #recent-label @meta
          space w=fill
          if scanning
            text "reading…" @meta_compact
        if scanning
          progress scan_ratio #scan
            with
              min=0.0
              max=1.0
              girth=3.0
              r=2.0
              bg=muted_bg
              bar=primary
        scroll #chat-list w=fill h=fill
          col w=fill gap=2.0
            if empty(chats) && !scanning
              col
                with
                  w=fill
                  py=24.0
                  gap=6.0
                  align=center
                text "No recent chats" @field_label
                text "Start a new chat to see it here." @caption
            for chat in chats
              lazy chat, open_path as past
                PastChat #chat(past.path) chat=past open=(past.path == open_path) -> pick_chat _
    rule vertical thickness=1.0 color=border
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
            if signed
            button "Sign out" #sign-out disabled=(busy || loading_chat) @ghost_action -> forget
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
        // End-anchored: the offset is a distance from the bottom, so history
        // measured for real above the viewport — or a beat appending below —
        // carries the reader's rows with it instead of shifting them, and the
        // chat rests on its newest row, where `snap-end` already points it.
        // End-anchored: the offset is a distance from the bottom, so history
        // measured for real above the viewport carries the reader's rows with
        // it instead of shifting them, and the chat rests on its newest row,
        // where `snap-end` already points it.
        scroll #transcript
          with
            w=fill
            h=fill
            anchor-y=end
          box
            with
              w=fill
              px=22.0
              py=28.0
              align-x=center
            box w=fill max-w=760.0
              col w=fill gap=18.0
                // Reading a chat off disk takes anything from milliseconds to seconds,
                // and it happens on another thread. Without this the window simply sits
                // there showing the previous chat until the new one appears, which is
                // indistinguishable from being frozen.
                if loading_chat
                  col #opening
                    with
                      w=fill
                      py=64.0
                      gap=10.0
                      align=center
                    text "◌" size=20.0 @text-muted
                    text "Opening that chat…" @caption
                if !loading_chat
                  if empty(entries) && !busy
                    Welcome #welcome -> suggest _
                  // `keyed` gives every row a stable identity and `lazy` keys its
                  // rebuild on the row itself, so a token landing in the live reply
                  // below rebuilds one row's widgets and nothing else.
                  // `virtual-row` lays out only the rows the viewport can reach;
                  // a long chat's settled history costs nothing per frame until
                  // it is scrolled back into view. The estimate is a collapsed
                  // row; measured heights follow the key once a row is seen.
                  keyed entry in entries by=entry.id #rows
                    with
                      w=fill
                      gap=18.0
                      virtual-row=60.0
                    lazy entry as settled
                      col w=fill
                        // One `if` per kind rather than a `match`: the kinds are
                        // disjoint, so first-match ordering buys nothing here.
                        if settled.kind == "prompt"
                          Prompt #prompt(settled.id) -> copy_text _
                            with
                              body=settled.body
                              dark=settled.dark
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
                          Answer #answer(settled.id) -> copy_text _
                            with
                              body=settled.body
                              dark=settled.dark
                        if settled.kind == "usage"
                          Usage #usage(settled.id) detail=settled.detail
                        if settled.kind == "note"
                          Note #note(settled.id) title=settled.title
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
                            text "◌" size=12.5 @text-warning
                            text status @field_label
                          markdown live_thinking #live-thinking -> copy_text _
                            with
                              gap=6.0
                              text-size=12.5
                              viewer=answer_viewer(dark)
                            style inline-code-bg=accent inline-code-fg=accent_fg inline-code-font=code inline-code-px=1.0 inline-code-py=0.0 inline-code-r=4.0 link=brand
                      markdown live #live-body -> copy_text _
                        with
                          gap=10.0
                          text-size=13.5
                          code-size=12.5
                          viewer=answer_viewer(dark)
                        style inline-code-bg=accent inline-code-fg=accent_fg inline-code-font=code inline-code-px=1.0 inline-code-py=0.0 inline-code-r=4.0 link=brand
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
                    editor #draft <-> draft -> send
                      with
                        key-binding=composer_keys()
                        hint="Message Codex…"
                        min-h=22.0
                        max-h=150.0
                        p=8.0
                        size=13.5
                        line-h=1.4
                      active bg=muted_bg border=muted_bg placeholder=muted value=fg selection=primary
                      focused-hovered bg=muted_bg border=muted_bg
                      disabled bg=muted_bg border=muted_bg value=muted
                    if !busy
                      button "↑" #send -> send
                        with
                          disabled=!can_send
                          label="Send"
                          w=30.0
                          h=30.0
                        active bg=primary text=primary_fg r=15.0
                        hovered bg=primary_hover text=primary_fg r=15.0
                        disabled bg=disabled text=disabled_fg r=15.0
                    if busy
                      button "■" #stop -> stop
                        with
                          label="Stop"
                          w=30.0
                          h=30.0
                        active bg=muted text=surface r=15.0
                        hovered bg=fg text=surface r=15.0
                // What will answer sits with the message about to be sent, not
                // up in the chrome: it is a property of this turn, not the app.
                row
                  with
                    w=fill
                    gap=6.0
                    align=center
                  Chip #model -> choose_model _
                    with
                      options=models
                      selected=model
                      hint="Model"
                  Chip #effort -> choose_effort _
                    with
                      options=efforts
                      selected=effort
                      hint="Reasoning effort"
                  space w=fill
                  if can_steer
                    button "Steer" #steer @outline_action -> steer
                  if can_steer
                    button "Send after" #queue @outline_action -> queue
                  if busy && !can_steer
                    text status @meta_compact
                  if !busy && !empty(copied)
                    text "Copied" @meta_compact
                  if !busy
                    Kbd label="Enter"
                  if !busy
                    text "send" @meta_compact

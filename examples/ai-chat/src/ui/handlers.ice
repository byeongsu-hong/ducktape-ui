// One turn, start to finish.
//
// The prompt is drawn before the socket is opened, so the transcript never
// waits on the network to acknowledge what was typed. After that the turn
// arrives on two routes: `streamed` for text as it is written, `rows` for the
// transcript whenever a block settles or a tool starts or stops.

on send
  let prompt = trim(draft)
  return if busy || empty(prompt)
  entries = push_user(session, prompt)
  draft = ""
  error = ""
  busy = true
  status = "Thinking"
  live = markdown("")
  live_thinking = markdown("")
  parallel
    sip codex_turn(session)
      progress -> streamed _
      done -> settled _
      error -> failed _
    stream codex_entries(session) -> rows _

// Two appends and a scroll, once per token. Nothing above is rebuilt: settled
// rows sit behind `lazy`, and Markdown is appended into the parsed document
// rather than reparsed from the top.
on streamed(part)
  status = part.status
  markdown live append part.answer
  markdown live_thinking append part.thinking
  task widget snap-end #app/transcript

// A block settled, or a tool started or finished. The live surfaces are left
// alone: they are cleared once, when the turn ends, so a late chunk can never
// land in a surface that has already been reset.
on rows(next)
  entries = next
  task widget snap-end #app/transcript

on settled(complete)
  entries = complete
  busy = false
  status = ""
  live = markdown("")
  live_thinking = markdown("")
  task widget snap-end #app/transcript

on failed(cause)
  busy = false
  status = ""
  error = cause.message

on reset
  session = codex_session()
  entries = []
  live = markdown("")
  live_thinking = markdown("")
  status = ""
  busy = false
  error = ""
  draft = ""
  task widget focus #app/composer/draft

on toggle_row(id)
  entries = toggle_row(session, id)

on suggest(text)
  draft = text
  task widget focus #app/composer/draft

on use_night
  dark = true
  active_palette = AppTheme.night
  entries = set_palette(session, true)

on use_day
  dark = false
  active_palette = AppTheme.app
  entries = set_palette(session, false)

// A link in an answer is copied rather than opened: which browser a developer
// wanted is not this window's call to make.
on copy_link(url)
  task clipboard write url

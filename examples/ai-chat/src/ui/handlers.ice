// One turn, start to finish.
//
// The prompt is drawn before the socket is opened, so the transcript never
// waits on the network to acknowledge what was typed. After that the turn
// arrives on two routes: `streamed` for text as it is written, `rows` for the
// transcript whenever a block settles or a tool starts or stops.

on send
  let prompt = typed
  return if busy || empty(prompt)
  copied = ""
  entries = push_user(session, prompt)
  draft = editor("")
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
  live_thinking = markdown(part.thinking)
  task widget snap-end #app/transcript

// A block settled, or a tool started or finished. The live surfaces are left
// alone: they are cleared once, when the turn ends, so a late chunk can never
// land in a surface that has already been reset.
on rows(next)
  entries = next
  task widget snap-end #app/transcript

// A turn ends, and a message typed while it ran goes out on its own.
on settled(complete)
  entries = complete
  live = markdown("")
  live_thinking = markdown("")
  let next = take_pending(session)
  busy = !empty(next)
  status = ""
  return if empty(next)
  entries = push_user(session, next)
  status = "Thinking"
  parallel
    sip codex_turn(session)
      progress -> streamed _
      done -> settled _
      error -> failed _
    stream codex_entries(session) -> rows _

on failed(cause)
  busy = false
  status = ""
  error = cause.message

on reset
  session = new_chat(session)
  entries = []
  live = markdown("")
  live_thinking = markdown("")
  status = ""
  busy = false
  error = ""
  draft = editor("")
  task widget focus #app/composer/field/draft

on toggle_row(id)
  entries = toggle_row(session, id)

on choose_model(name)
  model = some(set_model(session, name))
  efforts = codex_efforts(name)
  effort = some(session_effort(session))

on choose_effort(level)
  effort = some(set_effort(session, level))

// Three things to do with a turn already running: end it, cut it short and
// say something else instead, or hold what was typed until it is done.
on stop
  return if !busy
  status = stop_turn(session)

on steer
  return if !busy || empty(typed)
  status = steer_turn(session, typed)
  draft = editor("")

on queue
  return if !busy || empty(typed)
  status = queue_message(session, typed)
  draft = editor("")

on suggest(text)
  draft = editor(text)
  task widget focus #app/composer/field/draft

on use_night
  dark = true
  active_palette = AppTheme.night
  entries = set_palette(session, true)

on use_day
  dark = false
  active_palette = AppTheme.app
  entries = set_palette(session, false)

// Everything that leaves this window leaves the same way. A clicked link is
// copied rather than opened — which browser a developer wanted is not this
// window's call — and so is a message, because iced draws non-editable text
// without selection and there is no dragging over it to copy.
on copy_text(text)
  copied = text
  task clipboard write text

// Signing in without leaving the window. The host mints a short code, the
// person types it into a browser wherever they have one, and this waits.
on sign_in
  error = ""
  signing_in = true
  run begin_sign_in() -> code_ready _ | sign_in_failed _

on code_ready(next)
  code = next.user_code
  code_url = next.verification_uri
  run finish_sign_in(next) -> signed_in_as _ | sign_in_failed _

on signed_in_as(email)
  account = email
  model = some(codex_model())
  signed = true
  signing_in = false
  code = ""
  code_url = ""
  task widget focus #app/composer/field/draft

on sign_in_failed(cause)
  signing_in = false
  code = ""
  code_url = ""
  error = cause.message

on copy_code
  task clipboard write code

on copy_url
  task clipboard write code_url

// Forgetting this app's login. The CLI's is left alone, so staying signed in
// through `codex login` afterwards is the expected outcome.
on forget
  signed = sign_out()
  account = codex_account()
  session = codex_session()
  entries = []
  error = ""

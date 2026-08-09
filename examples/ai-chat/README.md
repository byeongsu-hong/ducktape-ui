# AI chat

A desktop chat client for Codex, built to answer one question: can the kind of
chat screen the web has settled on — streaming Markdown, folded reasoning, tool
calls, a composer — be built as-is on this stack, and be quicker at it?

It talks to the ChatGPT backend the Codex CLI talks to, using the OAuth tokens
the CLI's own login already wrote. There is no second login, and no token is
stored, printed, or copied anywhere by this app.

```sh
cargo run -p ai-chat-example
```

It signs in on its own — the device flow `codex login --device-auth` runs, with
the code shown in the window — and it also accepts a login `codex login` already
made, so an already-signed-in machine needs nothing.

Tokens go in this app's own file (`~/.config/ducktape-ai-chat/auth.json`, owner
only) and never in the CLI's. A refresh rotates the token it is given, and
rotating the CLI's would break `codex` for a login this window only borrowed —
so a borrowed login is read but never refreshed, and says to run `codex login`
when it expires.

![A settled turn: the prompt, a folded reasoning summary, two searches, the answer, and what it cost](screenshots/chat.png)

## What it draws

A turn is not one answer, and the screen does not pretend otherwise. The
transcript is a flat, ordered list of everything the turn produced:

| Row | What it is |
| --- | --- |
| prompt | what was asked, in a bubble on its own side |
| work | one line — `Worked for 12s · 4 steps` — holding everything below it |
| reasoning | a summary, folded under the subject it names |
| tool | a search or page open; its arguments are behind its own fold |
| answer | Markdown — headings, lists, code blocks, links |
| usage | what the turn cost |

A turn is watched while it happens and read once it is done, and it folds
accordingly. The step running now stays open; each step collapses to its own
title as it finishes; and when there is an answer to read instead, the whole
turn's working-out gathers under one line. Nothing is thrown away — every fold
opens — but a finished transcript reads as questions and answers.

Folded rows are left out of what the screen is handed rather than drawn and
hidden, so a folded turn costs no widgets at all.

An item this build does not model still becomes a row rather than being
dropped, because a chat window that silently swallows part of a turn is
misreporting it.

The model and how hard it thinks are both picked in the header, from the
catalogue the CLI keeps — so the lists are the ones `codex` would offer rather
than ones this app invented, and what is in force is always in them. Levels
differ by model, so changing the model carries the effort with it: a level the
new model does not offer would be rejected on the next turn, and is replaced by
that model's own default.

Both belong to the chat, not the machine. They apply from the next turn, they
survive `New chat`, and `~/.codex/config.toml` is only ever read.

The message box is a real editor. Enter sends; Enter held with shift, command
or both writes a line, so a paragraph can be typed without fighting the box.
Backspace erases a word with alt and back to the start of the line with
command — iced maps every Backspace to one character whatever is held with it,
so both of those had to be said explicitly or they did nothing.

A turn already running can be answered in three ways rather than only waited
out. **Stop** ends it and keeps what was already said — a stopped answer is
still an answer. **Steer** cuts it short and sends what has been typed instead.
**Send after** holds the message until the turn finishes and then sends it on
its own. Steer and Send after appear only once something has been typed to use
them with.

Clicking a link copies it; which browser you wanted is not this window's call.
`Night`/`Day` switches palettes, and settled rows follow.

| Mid-turn | Steering one |
| --- | --- |
| ![A reply being written, under the work still running](screenshots/streaming.png) | ![Stop, steer, or send after](screenshots/steering.png) |

| The model menu | Waiting for a code |
| --- | --- |
| ![The catalogue, open](screenshots/menu.png) | ![A code to type, and the page to type it into](screenshots/sign-in-code.png) |

| Signing in | Night |
| --- | --- |
| ![The sign-in screen](screenshots/sign-in.png) | ![The same transcript on the night palette](screenshots/night.png) |

Every one of these is generated, not staged: the first by the test that types
into the composer and presses Send, the rest by `cargo ice inspect` against a
named preset.

## How a turn arrives

Two channels, on purpose:

- `codex_turn` is a `sip`: text token by token, plus what the composer should
  say while it runs.
- `codex_entries` is a `stream`: the row list, published only when it changes —
  a tool starting, a block settling. Putting the list on every token would copy
  the whole transcript once per token.

Ice appends streamed text into a parsed Markdown document rather than reparsing
it, so a long answer costs the same per token as a short one.

## Why some of it looks the way it does

Three constraints shaped the design, and each is worth knowing before changing
it:

- **A parsed Markdown document cannot live in Ice component state**, which
  holds cloneable values only. `src/render.rs` is the typed adapter that holds
  it instead, parsed once per row and kept.
- **A `lazy` body cannot see anything but its dependency.** So a row carries
  everything that decides how it is drawn — including whether it is folded and
  which palette it was stamped for. That is what makes a fold, or a theme
  switch, actually reach a settled row.
- **`lazy` hashes its dependency every frame.** A derived hash would walk every
  row's full answer text, which costs about as much as rebuilding the row and
  defeats the boundary. `Entry` hashes its identity instead — sound because a
  row's prose never changes after it settles.

## Checks

```sh
cargo test -p ai-chat-example          # unit tests + first-class Ice tests
cargo test -p ai-chat-example --release perf -- --ignored --nocapture
cargo ice inspect examples/ai-chat/src/ui/app.ice \
  --viewport 920x800 --preset conversation --name conversation
```

`sending_a_message_runs_a_whole_turn` chats: it types into the real composer,
presses the real Send button, and asserts that the reasoning, both searches, the
answer and the cost all reach the transcript. The turn it drives is played from
fixed events through the same parser and channels the wire uses, so the suite
needs no login and spends no tokens.

To chat with the model instead, open the wire — one flag for the process, so
only a run of the live tests may set it:

```sh
AI_CHAT_LIVE=1 AI_CHAT_ASK="your question" \
  cargo test -p ai-chat-example -- --ignored --nocapture a_live_turn
```

Every preset names its own account — `conversation`, `conversation_night`,
`streaming`, `signed_in` and `signed_out` — so no real address reaches a
committed artifact.

### Measured

One full Elm cycle — `update` with a token, then a whole view rebuild — on a
release build, 200 tokens per sample.

Against transcript length, with the reply being written on screen:

| Transcript | Per token | Same cycle, every row changed |
| --- | --- | --- |
| 1 turn (4 rows) | 25 µs | 23 µs |
| 8 turns (32 rows) | 39 µs | 41 µs |
| 24 turns (96 rows) | 69 µs | 80 µs |

Against the length of the reply itself, split into the append and the redraw.
`Content::push_str` re-reads the block the reply is still inside, so what
decides the cost of a token is the reply's *shape*, not its length:

| Reply so far | Prose (a paragraph every 40) | One unbroken code block |
| --- | --- | --- |
| 0 tokens | 51 µs | 51 µs |
| 1500 tokens | 51 µs | 98 µs |
| 3000 tokens | 56 µs | 135 µs |

Prose is flat — the parser is re-reading the paragraph in hand, not the reply.
A reply that is one long code block has no boundary to close, so it grows.

The worst case this screen can be handed — 24 settled turns behind a
3000-token unbroken code block — is **171 µs**. A model writes 50–100 tokens a
second, so the budget for one is 10–20 ms: that worst case is under 2% of it.
Further tuning is not warranted, and none was kept.

## Chats already had

The sidebar lists the rollouts the Codex CLI wrote under `~/.codex/sessions`,
newest first, and opening one draws it as a transcript. It fills as it is read
rather than when it is done, with a bar saying how far it has got: a thousand
rollouts take long enough that nothing happening would read as nothing
working. Neither the bar nor the list waits on the other, and neither holds up
typing. The session becomes that chat — its own
`input` is loaded too — so carrying on from it continues that conversation
rather than starting beside it.

Two streams run through a rollout: the raw `response_item` records, which are
what was resent to the API, and the `event_msg` records, which are what the CLI
drew. Both are read — the drawn one for what to show, the raw one for what a
next turn would resend, and either for the question, which different sessions
record in different places.

The drawn stream is what brings back the things this window cannot itself
produce: shell commands with their output, file changes, MCP calls. A chat is
being read back, not re-run.

These files are large: a median of 2MB and a tail past 90MB. The listing reads
only each file's head, and opening one streams it a line at a time and keeps a
bounded number of rows, saying how many it left out. Measured on a machine with
1,036 of them: the list took 210ms, and a 26MB chat opened in 247ms.

| Chats already had | Still reading them |
| --- | --- |
| ![The sidebar](screenshots/history.png) | ![Filling as it reads](screenshots/scanning.png) |

## Limits

- An overlay's contents are outside the tree the test harness scans, so a
  menu's appearance is reviewed from a capture rather than asserted. The test
  that produces it says so.
- **Past reasoning does not come back at all.** The raw record is encrypted,
  and the drawn record's `summary_text` and `raw_content` are both empty —
  in all 14,319 reasoning items across the 1,037 rollouts on this machine.
  The parser reads either field if a future version fills one, and the audit
  in `src/qa.rs` watches for exactly that; today a chat read back shows its
  questions, answers and tools, and nothing of what the model was thinking.
- There is no blur on a widget in this toolkit — only a window compositing
  flag and shadow blur — so the list being read is shown filling rather than
  frosted over. Hiding it would hide the part that is worth watching.
- A chat whose question sits past the first 256KB of its rollout is listed as
  `Untitled chat` rather than read through for a name.
- **The sign-in is not fully verified.** Minting a code and telling waiting from
  refused are both checked against the live host; the half past approval — what
  the host returns once a code is typed, and the token exchange — is read off
  the CLI's own strings. `a_real_sign_in_completes` settles it and needs a
  person to approve one code.
- Codex's own shell and patch tools would need this window to execute them,
  which is a different program. The hosted `web_search` tool is served by the
  backend, so a real tool call still reaches the screen.
- A turn cannot be stopped once it starts.
- The Markdown cache is never emptied: clearing a chat orphans its entries for
  the life of the process.

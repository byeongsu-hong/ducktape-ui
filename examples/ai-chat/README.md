# AI chat

A desktop chat client for Codex, built to answer one question: can the kind of
chat screen the web has settled on — streaming Markdown, folded reasoning, tool
calls, a composer — be built as-is on this stack, and be quicker at it?

It talks to the ChatGPT backend the Codex CLI talks to, using the OAuth tokens
the CLI's own login already wrote. There is no second login, and no token is
stored, printed, or copied anywhere by this app.

```sh
codex login          # once, if you have not already
cargo run -p ai-chat-example
```

## What it draws

A turn is not one answer, and the screen does not pretend otherwise. The
transcript is a flat, ordered list of everything the turn produced:

| Row | What it is |
| --- | --- |
| prompt | what was asked, in a bubble on its own side |
| reasoning | a summary, folded away under the subject it names |
| tool | a web search or page open, drawn while it runs and again when it lands |
| answer | Markdown — headings, lists, code blocks, links |
| usage | what the turn cost |

An item this build does not model still becomes a row rather than being
dropped, because a chat window that silently swallows part of a turn is
misreporting it.

Clicking a link copies it; which browser you wanted is not this window's call.
`Night`/`Day` switches palettes, and settled rows follow.

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
release build, 200 tokens per sample:

| Transcript | Per token | Same cycle, every row changed |
| --- | --- | --- |
| 1 turn (6 rows) | 24 µs | 20 µs |
| 8 turns (48 rows) | 44 µs | 51 µs |
| 24 turns (144 rows) | 92 µs | 121 µs |

24× the transcript for 3.8× the per-token cost. What remains is the keyed
column building one wrapper per row whether or not the row is reused;
virtualising the transcript is the next thing that would flatten it.

## Limits

- Codex's own shell and patch tools would need this window to execute them,
  which is a different program. The hosted `web_search` tool is served by the
  backend, so a real tool call still reaches the screen.
- A turn cannot be stopped once it starts.
- The Markdown cache is never emptied: clearing a chat orphans its entries for
  the life of the process.

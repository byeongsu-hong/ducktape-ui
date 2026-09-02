# Ice UI performance

Use this reference for lists, tables, streams, timers, and any screen that will
hold real data, and whenever `cargo ice check` prints a `W016`–`W021` line.

## Contents

- [What a frame costs](#what-a-frame-costs)
- [The checker's performance warnings](#the-checkers-performance-warnings)
- [Canonical boundaries](#canonical-boundaries)
- [Measure, then decide](#measure-then-decide)
- [Do not](#do-not)

## What a frame costs

iced rebuilds the whole view on every message and lays the whole tree out on
every frame, with no dirty region. A frame costs what the view *contains*, not
what it shows: showcase at 480x320 holds 8.4x less area than at full size and
still costs ~3.1ms against ~3.3ms. Everything below the fold is laid out too.

Three kinds of jank reach a user, with different mechanisms:

- **Stutter** — per-frame work is proportional to content: rows built per pass,
  widgets rebuilt from state, text reshaped, keys formatted.
- **Freeze** — the loop thread blocks in `update()`: a `sync` extern doing I/O, a
  `stream`/`task` constructor opening a socket, an `editor` clone, a re-parse.
- **Rehydration** — a write to one field rebuilds and re-lays-out subtrees that
  never read it.

## The checker's performance warnings

| Code | Meaning |
| --- | --- |
| `W016` | an extern component rebuilds its native widget from string or list content on every view pass, outside any `lazy` boundary |
| `W017` | a plain `lazy` inside a repetition clones and hashes a row-local list, or a record owning one, on every view pass |
| `W018` | a `str`, `bytes`, `[T]`, `editor`, or list-owning record state field is cloned into a by-value `pure`/`sync` parameter on every view pass or subscription check |
| `W019` | a `for` or keyed column over a state-rooted list instantiates a component, an extern component, or a nested repetition per row on every view pass with no per-row `lazy` and no `virtual-row` column |
| `W020` | a plain `lazy` inside a repetition evaluates a call or operator over the row on every view pass only to compute the key it is compared by |
| `W021` | a `sync` extern is called from a handler that a sub-second `every`, a stream, the raw event feed, pointer or window motion, or a slider drag routes to |

`cargo ice check` must print no `W0xx` line for the graph you changed. Each
message names the fix in canonical form; apply that spelling.

## Canonical boundaries

### A repeated component row (`W019`)

Wrap the row body in a `lazy`, and derive the row id from the `lazy` alias, not
the loop one, or the rows are indistinguishable to targets and captures. The
`W019` message names the keyed `lazy <item> by <cheap keys> as <alias>`, the
form that holds for any row; the plain form below is enough when the row is a
record of scalars and strings, the whole-row memo idiom `W017` never reports:

```ice
for print in tape_prints
  lazy print as printed
    TradeRow print=printed #print(printed.tid)
```

`lazy row, <state extras> as alias` — `lazy row, locale as market` — hashes each
extra right after the row and exposes it in the subtree under its own name. An
extra is a bare identifier of a cheap type: a state field, a component prop, or
another row local.

### The keyed form when the row owns a list (`W017`, `W020`)

`lazy value by <cheap keys> as alias` captures the value by reference and hashes
only the keys, so an unchanged frame deep-clones nothing. Use it when the row is
a list or owns one, or when the plain form would evaluate a call over the row
only to compare it:

```ice
for note in notes
  lazy note by note.path, note.stamp as row
    NoteRow #note(row.title) note=row selected=true
```

The keys are the author's contract that they move whenever the row's rendered
content does. Keys do not combine with extras.

### A `virtual-row` column for fixed-height rows

A virtualized column accepts every child but lays out only those the viewport
can reach, sizing the rest from the estimate. Mount it inside a `scroll`; a long
list of *identified* rows takes the same estimate on a `keyed` column instead, so
per-row state follows the key rather than the index:

```ice
scroll #position-list h=fill
  col w=fill virtual-row=44.0
    for held in positions
      PositionRow held=held locale=locale #position(held.coin)
```

`wrap` and `align=` are rejected on such a column (`E197`), and a screen reader
— and therefore an `.ice` test — sees only the visible slice.

### Borrowed `&type` parameters (`W018`)

A `pure` or `sync` parameter declared `&str`, `&bytes`, `&[T]`, `&T`, or
`&editor` receives a reference to the state field, local, `for` row, or lazy
alias instead of a clone. The call site is unchanged:

```ice
extern crate::portfolio
  pure range_heading(locale:Locale, range:&str) -> str
  pure portfolio_assets(positions:&[Position]) -> [PortfolioAsset]
```

### `derived` for values computed from state

A derived value is computed on its first read and kept across frames until a
write to a field it reads. Declare one instead of recomputing in the view:

```ice
derived
  typed = trim(editor_text(draft))
  can_send = !busy && !empty(typed)
```

### `run` instead of `sync` in a hot handler (`W021`)

Handlers run on the loop thread, so the app freezes for as long as the Rust body
takes, once per turn. Drop `sync` from the extern and route its result:

```ice
on sign_in
  error = ""
  signing_in = true
  run every begin_sign_in() -> code_ready _ | sign_in_failed _
```

`sync` is for an immediate effect or environment read; anything that can wait
belongs in an async extern. A source firing once per user action is silent.

### `markdown … append` for growing markdown

Markdown is parsed into owned iced state. Append into the parsed document rather
than reparsing it from the top on every token:

```ice
on streamed(part)
  status = part.status
  markdown live append part.answer
```

### What a component use already gets

The compiler inserts a revision-keyed layout memo at a component use when every
read below it is revision-keyed and every widget below it lays out from its
element and `Limits` alone; a hit skips iced's diff and layout walk below the
use, while the element is still built every pass. A row-local, secret, or clock
read anywhere in that union refuses the memo, and so does a body holding a
`lazy`, which needs its diff every pass.

## Measure, then decide

Take a frame reading from the inspection you already run for visuals:

```bash
cargo ice inspect path/to/app.ice --viewport 1440x900 --frames 60
```

`N` is a positive integer. The run takes 8 warmup redraws, discards the memo
counters, then times `N` `redraw_phases` frames; the counters are read once
afterwards, as totals over the `N` frames. `--frames` is rejected together with
`--trace`, `--fuzz`, or `--replay`. `--release` is a flag, accepted only with
`--frames`, and makes the inspection's `cargo test` run with `--release`.

The manifest then carries a top-level `frames` object:

```json
"frames": {
  "count": 60,
  "warmup": 8,
  "build_profile": "debug",
  "view_us":   { "p50": 0, "p95": 0 },
  "layout_us": { "p50": 0, "p95": 0 },
  "update_us": { "p50": 0, "p95": 0 },
  "rev_memo":  { "hits": 0, "misses": 0 },
  "memo_lazy": { "hits": 0, "misses": 0 }
}
```

`build_profile` is `"debug"` or `"release"`; microsecond values are integers.
After its result JSON the command prints exactly one extra line:

```text
frames: 60 @ debug | view p50 650us p95 720us | layout p50 1100us p95 1300us | update p50 100us p95 130us | rev_memo 81/0 | memo_lazy 12/0
```

`81/0` is `hits/misses` over the `N` frames. `cargo ice diff` ignores every
difference whose path is `/frames` or starts with `/frames/` — timings are never
a visual delta — and still lists that rule in the report's `ignored_paths`.

Read the numbers this way:

- A debug run is for ratios and memo misses only. `-O0` numbers measure rustc,
  not the app; add `--release` before quoting an absolute figure.
- Memo misses per idle frame above zero name a boundary that is not holding. Fix
  the boundary, not the widget it wraps.
- `layout_us` several times `view_us` means the answer is a boundary, not tighter
  view code; the `view` share is the ceiling on emitted-code work.
- Showcase is the worst case, not a bug: its catalog is threaded with `bind`
  parameters and ~45 pieces of state, so almost nothing in it holds still long
  enough to cache.

To prove a whole flow smooth, trace it instead —
`cargo ice inspect src/ui/app.ice --test checkout_flow --trace --warmup 2 --repeat 20`
— an opt-in release-mode path over the same program and driver, repeating the
actions `--repeat` times, so it costs far more than one inspection.

For a stall the extern boundary hides, run the app under `cargo ice dev`. A debug
build times every generated handler arm, and a turn over the 16ms frame budget
prints on the app's stderr:

```text
ice: handler `name` took Nms, over the 16ms frame budget, at path.ice:line
```

## Do not

- Do not wrap everything in `lazy`. It changes iced widget construction, carries
  hashability constraints, and only pays around an expensive subtree.
- Do not assert absolute microsecond budgets in tests. They do not survive a
  shared runner; assert a ratio between two measurements taken in the same run,
  or a metric count the widget records.
- Do not use `sync` for I/O. It blocks the loop thread for the whole Rust body.
- Do not guess from reading code. Measure first; both documented loops in this
  repository were dominated by a phase that was not the obvious suspect.

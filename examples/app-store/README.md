# app-store — an OS-shaped host for Ice apps compiled to wasm

A native Ice daemon that reads a catalog of `ice:view` components, installs
one on Get inside a fuel and memory budget, gives it a native window of its
own, and drops the instance when that window closes. Every app in the
catalog is an ordinary Ice application compiled for the `tree` target: its
view builds a widget tree the host renders with its own toolkit, and nothing
in the language changes to make it installable. What the host adds is
everything an app cannot do alone — time, storage, other apps, the colour
mode — and it adds them as capabilities the app's manifest has to declare.

![The store in light mode, with Clock, Activity and Counter open in windows of their own; three presses of Counter's + have just reached Activity over the bus](screenshots/store-light.png)

![The same in dark mode: the guests follow the store's colour mode](screenshots/store-dark.png)

```
crates/ui-lang-wire/   (workspace crate) the wire: a Frame carrying the app's
                widget tree out, meaning-level events (message 3, input 0 now
                reads "abc") in, plus Request / Response for everything else
crates/ui-lang-guest/  (workspace crate) what an app needs to run in wasm: a
                Driver (task executor, per-frame message tables),
                `host::request` / `host::subscribe` / `host::theme`, and
                `export_app!`, which adds the `ice:view` component exports and
                the manifest
crates/ui-lang-runtime/view_tree   (workspace crate) the host's half: the tree
                rendered with iced's widgets, every input's text kept host-side
apps/counter/   three buttons; Auto is a chain of host timers, every change goes
                on the bus and into the store's log
apps/todo/      a list kept in the host's storage — it survives uninstall/reinstall
apps/clock/     host uptime from a subscription, UTC from one `clock.now` plus
                arithmetic; the module has no clock
apps/activity/  a live feed of what the other apps publish, and who published it
apps/chaos/     spins, eats memory, panics, floods the host, asks for an
                undeclared capability
host/           the store: catalog, library, windows, capabilities (clock,
                storage, bus, theme), the fuel/memory sandbox, the component
                cache, and the widget that shows a guest
componentize.sh turns the built app modules into components in the catalog
```

## Run it

```
cd examples/app-store
cargo build -p app-store-todo -p app-store-counter -p app-store-clock \
            -p app-store-activity -p app-store-chaos \
            --release --target wasm32-unknown-unknown
./componentize.sh
cargo run -p app-store-host --release
```

`componentize.sh` needs `wasm-tools` (`cargo install wasm-tools`). An app
builds as a core module whose `ice:view` exports are already in place;
the script wraps it as a component and satisfies the imports iced's wasm
target leaves behind (wasm-bindgen's placeholders, from `web-time` and
`web-sys`) with stub adapters that trap if called — nothing on a guest's
frame path calls them. The catalog lists components only: a module that
was built but not componentized is skipped.

| variable | default | what it names |
|---|---|---|
| `APP_STORE_CATALOG` | `target/app-store-catalog` | the directory `componentize.sh` writes and the store scans |
| `APP_STORE_DATA` | `target/app-store-data` | app storage (`<app>/<key>`), the store's `installed`, `running` and `windows` lists, and wasmtime's artifact `cache` |

The windowing backends the host asks iced for (`x11`, `wayland`) are
requested only on Linux, so a macOS or Windows build resolves without them —
only Linux is exercised here.

The gates are `cargo fmt --all -- --check`, `cargo clippy --workspace --tests
--no-deps` and `cargo test --workspace`. `--tests`, not `--all-targets`:
Ice generates a `#[cfg(test)]` harness that needs the runtime's test driver,
which pulls wgpu into a workspace whose apps link no renderer at all, so
every crate here sets `test = false` and keeps its tests in `tests/`.
`--all-targets` builds a test target anyway and fails on the missing driver.

An app links iced for its types only. iced's null renderer — what it falls
back to with no renderer feature — exists only under `debug_assertions`, so
the workspace keeps that cfg on for `iced_core`, `iced_graphics` and
`iced_renderer` in release builds; nothing in a guest ever calls it.

## The store

One daemon, two kinds of window. The store window has three pages and a
segmented Auto / Light / Dark switch; every guest gets a native window of
its own, titled with the app's name, resizable and movable like any other.

- **Discover** lists every module the catalog directory holds, from its
  manifest: name, description, and a chip per capability, coloured by what
  it reaches. Get loads the module, adds it to the library and opens its
  window. While an app runs its card carries a fuel bar — the last tick's
  fuel against the 100M budget — and the figures under it. A strip at the
  top shows what is running now; Show raises its window, Quit closes it.
  Clicking a card opens the app's page: what each capability lets it do, the
  box it runs in, and its live figures.
- **Library** is what is installed, running or not. Open gives an installed
  app a window again; Uninstall removes it from the library and closes its
  window. What the app wrote to storage stays.
- **Monitor** is the dogfooding page: one row per running guest with fuel
  per tick, tick time, ticks per second, frame bytes and how many of the
  last frames crossed as "unchanged", ticks run against redraws skipped, and
  whether the module came from the cache or through cranelift.

Closing a guest's window quits the app — the wasmtime store, its memory and
its compiled code go with the last handle. Closing the store window ends the
store. What had a window at exit reopens at the next start, one load at a
time; the library comes back as it was.

The keyboard reaches everything in the store window. `Tab` and `Shift+Tab`
move through its controls — the tabs, the search box, the colour switch,
every card's head and button — and `Enter` presses the one wearing the
ring. Two keys are the store's own: `Ctrl+F` (`⌘F` on macOS) puts the
cursor in the search box, and `Escape` steps back one layer — a search in
progress is cleared first, then an app's page returns to Discover. A
guest's window keeps its keyboard; what it does with Tab or Escape is its
own.

The colour mode is the store's: Auto follows the system, Light and Dark are
the user's word. Every guest subscribes to `host.theme` in its `on mount`
and switches its own palette on each answer, so the windows change together.

## Writing an app

```rust
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(Clock, __ClockMessage, "Clock", "Host uptime.", ["clock"]);
```

That is the whole crate, plus a `build.rs` that compiles the Ice sources
for the `tree` target (`ui_lang_build::compile_dir_for("src/ui",
Target::Tree)`): the generated view builds `ui_lang_wire` nodes instead of
iced widgets, and a construct the wire does not carry fails the build at
its `.ice` line. `export_app!` implements the guest crate's `App` trait over
the generated `__boot` / `__view` / `__update`, emits the `ice:view`
component exports (`init`, `tick`, and the `panicked` import the guest's
panic hook calls before it aborts), and writes name,
description and capabilities into an `ice.manifest` custom section, so the
catalog lists the app — and shows what it will touch — by reading the
file: no compilation, no instantiation.

An app talks to the host from ordinary Ice tasks. A one-shot ask is
`host::request`; something that keeps coming is `host::subscribe`:

```ice
extern crate::host
  stream ticks(every_ms:i64) -> i64 ! ClockError
  stream theme_changes() -> str ! ClockError

on mount
  parallel
    stream every ticks(1000) -> ticked _ | clock_failed _
    stream every theme_changes() -> themed _ | theme_failed _

on themed(mode)
  dark = mode == "dark"
  active_palette = ClockTheme.light
  return if !dark
  active_palette = ClockTheme.dark
```

```rust
pub fn ticks(every_ms: i64) -> impl Stream<Item = Result<i64, ClockError>> + Send + 'static {
    host::subscribe("clock.ticks", &every_ms.to_le_bytes()).map(|answer| /* bytes → ms */)
}

pub fn theme_changes() -> impl Stream<Item = Result<String, ClockError>> + Send + 'static {
    host::theme().map(|answer| /* bytes → "light" | "dark" */)
}
```

## Capabilities

A request's kind is `<capability>.<operation>`. The host answers only what
the manifest declares; a request for anything else comes back as the `Err`
of the task, which the app's handler routes like any other error (Chaos's
"Use the clock" shows the refusal text, and its "Flood" the refusal past the
per-tick request cap).

| capability | operations | answer |
|---|---|---|
| `host` (always) | `echo` · `log` · `random` (`u32` LE count) · `theme` (stream) | the text back · nothing, the line goes to the store's stderr as `[<app>] …` · that many bytes · `light` or `dark` at once and on every change |
| `clock` | `sleep` (ms) · `ticks` (every ms, stream) · `now` | nothing at the deadline · host uptime per tick · the unix millisecond and the host uptime it was read at, two `u64` LE |
| `storage` | `get` (key) · `set` (`key\nvalue`) · `delete` (key) · `list` | the value or empty · nothing · nothing · every key, newline-separated; one file per key per app |
| `bus` | `publish` (`topic\ntext`) · `subscribe` (topic or `*`, stream) | how many heard it · every matching message, as `from\ntopic\ntext` |

Publishing does not need the topic: any app with `bus` may publish under
any name, and `from` — the publisher's app id, which the host fills in — is
what says who did. A real host would route `query` / `submit` / `page`
through the same table and answer from its own subscriptions.

## How a frame crosses

1. The guest driver delivers the host's events — a message index the user
   activated, an input's whole new text, a response — runs the handlers
   they name, polls the app's tasks (re-polling one that wakes itself,
   which every `Task::stream` does once), and builds the view.
2. The view is a `ui_lang_wire::Node` tree with every value inlined: text,
   colours resolved from the app's own palette, sizes, paddings, button and
   input faces per state. A button carries the index of the message the
   guest queued for it this tick; an input carries its handler's index and
   the guest's copy of the value. The guest never learns where anything
   landed. The frame also carries the requests the tasks made and the
   requests they dropped.
3. A tree identical to the last one crosses as `unchanged` with no tree:
   a few bytes instead of every node again. The host keeps the tree it
   already has and takes only the requests and the cancels.
4. The host sanitizes the tree, renders it with iced's own widgets
   (`ui_lang_runtime::view_tree`) — layout, fonts, IME, caret, selection,
   scrolling and focus are all the host's — and keeps every input's live
   text itself, adopting the guest's value only when the guest moved it
   (its handler cleared the field). What the user does comes back as
   meaning: a press is `Event::Message(i)`, typing is
   `Event::Input { handler, text }`. The widget that wraps the rendered
   tree ticks the guest once per redraw with a fresh fuel budget, answers
   its requests, and rebuilds the window's view when the tree changed.
   Answers are delivered as events: an echo on the next redraw, a timer at
   its deadline via `request_redraw_at`, a bus message when another guest
   publishes.
5. Not every redraw of a window is a tick of its guest. A guest with no
   event pending, no answer due and nothing in its inbox is left alone:
   the tree the host holds is the tree it would send. It still says when
   it next wants to run, and the widget schedules that wake-up.

There is no executor thread and no clock inside a module. `clock.now`
answers the wall clock together with the host's uptime it was read at, and
`clock.ticks` streams that uptime — measured from the moment the store
starts, not from the first guest — which is what lets an app anchor one to
the other instead of drifting by however long the store had been running
when it was installed.

## What it costs

Loading a module is cranelift's compile the first time and a file read the
next: wasmtime's artifact cache lives under the data directory, and a module
loaded once in a run is kept in memory, so Restart, Quit-then-Open and
reinstall are an instantiation — under a millisecond. The compile itself
runs across every core (wasmtime's `parallel-compilation`), so a cold
restore of five apps takes about a second on a quiet machine rather than
their sum, and a warm one about a fifth of that. The Monitor's Load column
says which it was.

A guest is ticked only when something is due for it, so Counter sits at
0/s and Clock at 1/s; a press or a keystroke reaches the window it
happened in, so one guest ticks instead of five; a tree that changed
nothing crosses as a flag. Pointer movement never reaches a guest at all —
hover is the host's widgets' — so a window with the pointer moving over it
costs what any native iced window does. The Monitor page keeps the
counters per app: ticks against the redraws it slept through, and how many
of its frames crossed without their tree.

## The sandbox

Every tick runs with `FUEL_PER_TICK` (100M, roughly one per instruction —
about a 60 Hz frame of wall clock, and two hundred times the busiest tick
any app here has) and a 64 MB memory limit, in a store that allows eight core instances (the
app, the stub adapters, the bindings' shims — all the component's own),
one memory and four tables of at most a million elements — a table is
allocated at its declared minimum when the component is instantiated,
before any other limit is consulted. An app that spins burns its budget
and traps; an app that allocates past the limit traps on the grow. A trap
ends that instance — its window shows the reason (the message the guest's
panic hook handed the host through `panicked` on its way out: a trapped
instance can never be entered again, so nothing is read back) and a
Restart button
that asks the store to reload the component on its executor, where the
compile does not stall the window, and swap it into the same handle,
keeping the window and everything the app wrote to storage — and nothing
else notices: the other guests keep ticking, the store keeps answering.
The app's card and its Monitor row show what the last tick cost, plus the
bus deliveries the guest was not there to take.

Fuel and memory bound what a module does to itself. What it can make the
*host* do is bounded by the constants in `limits.rs`:

| limit | value | what a guest past it gets |
|---|---|---|
| `MAX_FRAME_BYTES` | 8 MiB | the instance ends, "frame too large" |
| `TICK_BUDGET` / `MAX_REST` | 8 ms per redraw, 250 ms of waiting | its next redraw waits as long as this one overran: an expensive guest runs at a few frames a second instead of at the window's rate, and the windows sharing that thread keep theirs |
| `MAX_REQUESTS_PER_TICK` | 256 | `Err "too many requests this tick"` for the rest |
| `MAX_PAYLOAD_BYTES` | 1 MiB | `Err`, whatever the request was |
| `MAX_TICKERS` | 16 per guest | `Err` from `clock.ticks` |
| `MAX_DUE` | 1024 answers the host still holds | `Err` from `clock.sleep` |
| `MAX_SUBSCRIPTIONS` | 64 per guest | `Err` from `bus.subscribe` |
| `MAX_TOPIC_BYTES` | 256 per subscription | `Err` from `bus.subscribe` |
| `MAX_CANCELS` | one tick's worth of everything the host holds | the rest of that frame's cancels ignored |
| `MAX_REPLY_BYTES_PER_TICK` | 4 MiB | `Err` for the rest of that tick, checked before the work: it counts the payloads the requests carried in and the copies a publish made, not only the answers |
| `MAX_BUS_BYTES` | 64 KiB per message | `Err` from `bus.publish` |
| `BUS_WAKE_INTERVAL` | 50 ms | the other windows are woken for a guest's publishes at most twenty times a second; the messages are in the subscribers' inboxes at once, and the last of a burst wakes when the interval is up |
| `MAX_INBOX` / `MAX_INBOX_BYTES` | 1024 events, 1 MiB | its oldest bus deliveries dropped, and counted in its status line |
| `MAX_VALUE_BYTES` | 1 MiB | `Err` from `storage.set` |
| `MAX_APP_KEYS` | 1024 per app | `Err` from `storage.set` |
| `MAX_APP_STORAGE` | 64 MiB per app | `Err` from `storage.set`, summed over the app's directory, a block per key |
| `MAX_RANDOM_BYTES` | 4096 per answer | `Err` from `host.random` |
| `MAX_LOG_BYTES` | 1024 per line | the rest is cut, and the line is escaped before it reaches a terminal |
| `MAX_FAULT_BYTES` | 1024 | its window shows the first line of that |
| `MAX_NAME_BYTES` · `MAX_DESCRIPTION_BYTES` · `MAX_CAPABILITIES` | 64 B · 256 B · 16 of 32 B | its module is left out of the catalog — the sidebar shapes every manifest field of every entry, before anything is installed |
| `MAX_MODULE_BYTES` | 64 MiB | its `.wasm` file is left out of the catalog, unread — or, loaded by a path the catalog didn't just scan, `Err` naming the limit before cranelift ever sees it |

The numbers *inside* a frame are the other half of the same boundary: the
guest chooses every one, and the host lays out what it is given and
allocates by the counts it is given.

The shape comes first, because reading a tree is itself recursive. A
`Node` holds its children, so a chain of containers is a chain of stack
frames in the decoder — and in `sanitize`, the renderer and `Drop` after
it. A few thousand links is a frame of about 100 KB, well inside any byte
cap, and walking one takes a host thread off its stack: an abort, not a
fault, with every window in the process gone. So `wire::decode` counts
what it descends into and refuses a frame nested past `MAX_DEPTH` or
carrying more than `MAX_DECODED_NODES` (16 × `MAX_NODES`) before there is
a tree to walk. That refusal ends the one instance, like any other.

What survives the door, `wire::sanitize` pulls into range: depth to
`MAX_DEPTH` (64, the host's layout recurses that far and no further),
nodes to `MAX_NODES` (8192, a screen's worth — a list past that is the
guest's to window), every string to `MAX_STRING_BYTES` (64 KiB), text
sizes to `MAX_TEXT_PIXELS` (512, since every glyph at one is rasterized
and cached), other sizes, spacings and paddings to finite pixels no
larger than a wall, colours to `0..=1`. A key used twice is moved off the
one already taken (`key`, then `key#2`): a key is the node's widget
state, its focus target, its accessibility id and, for an input, the text
the host owns on its behalf, so two nodes sharing one share all of that.
Every frame passes through it, tree or no tree — an `unchanged` one still
carries request kinds the host formats into refusals and shows. A
well-behaved tree comes out untouched; a hostile one is cut, not refused,
so a guest that overshoots by one node still shows.

## What is not here yet

An honest inventory, grouped by where the work would land. Items marked
**bug** are wrong today rather than merely absent.

### Wire and rendering

- The wire carries `box`, `col`/`row`, `scroll`, `text`, `input`,
  `button`, `space` and `rule`, with `if`/`for`/`match` around them. Every
  other Ice construct — checkbox, toggle, pick list, slider, images, SVG,
  canvas, grids, stacks, overlays, mounted components, gradients, utility
  styles — fails the app's build at its `.ice` line with E190. Each is a
  node kind to add to the wire, an emitter arm and a renderer arm.
- A changed tree re-encodes and re-sends every node; there is no delta,
  only the whole-frame "unchanged" short-circuit.
- No scale factor or locale reaches the guest. The colour mode does, as a
  `host.theme` stream the app has to subscribe to and act on itself.
- The host's accessibility tree names every node by its key, but nothing
  reads it back into the guest.

### Events and input

- Only a button's press, an input's edit and an input's submit cross.
  Keys, pointer position, hover, scroll position, drag and drop, window
  focus and close requests are the host's widgets' and never reach the
  guest.
- A guest cannot move focus, scroll to a row or select text: widget
  operations are the host's, and a task that asks for one is dropped.

### Tasks and runtime

- Only `Action::Output` of a task is honoured. Widget operations (focus,
  scroll-to), clipboard, window, font and exit actions are dropped.
- Ice `subscribe` blocks never run: the driver never calls
  `__subscription`, and iced's timers (`every`) have no executor in wasm.
  Host streams (`clock.ticks`, `bus.subscribe`) are the only long-lived
  sources.
- Task fairness is fixed: 8 rounds of messages per tick, 64 self-wakes per
  stream. A task that produces more waits for the next tick — and nothing
  schedules one for it: the frame cannot say it was cut short, so the rest
  arrives whenever the next event or answer ticks the guest. A future that
  becomes ready on its own, with no host answer behind it, waits the same
  way.
- No cooperative long computation: work heavier than one fuel budget
  cannot be spread over ticks except by chaining host sleeps. No
  preemption short of the trap that ends the app.
- `println!` and `tracing` inside a module still go nowhere; `host::log` is
  the way out and the host prints it on its own stderr. Nothing shows the
  lines inside the app store itself.
- Inside a module `SystemTime::now()` still aborts and `getrandom` still
  needs JS glue; `clock.now` and `host.random` are how an app gets the time
  and entropy. No locale, timezone or environment.

### Capabilities and security

- The manifest is self-declared and unsigned: any module can claim
  `storage`. No signature or hash check on modules, no consent prompt at
  install, no per-operation prompt, no runtime revocation, no policy file.
- Storage: a write is atomic (a sibling temp file, then a rename) but not
  fsync'd, so a power cut can still lose the last one. Keys are compared
  byte for byte, so on a case-insensitive filesystem (the default on macOS
  and Windows) `Items` and `items` are one file that the quota and
  `storage.list` count as two. The quota is one scan of the app's directory,
  kept on the instance and moved by every write — one `stat` for the key a
  `set` replaces, another scan after a `delete` — so it is only as true as the
  host being the sole writer, and a reinstall or a restart scans again. No
  sharing between apps, no migration on app upgrade, and nothing outside the
  app can read or list what it stored.
- Bus: no topic ownership — any app with `bus` can publish `counter\n999`
  under its own name. No rate limit beyond the per-tick byte budget the
  fan-out is charged to, no replay for late subscribers, no request/reply
  between apps, no wildcard beyond `*`.
- Beyond the sandbox table: no cumulative CPU budget across ticks, and
  nothing that ends an app for being slow — `TICK_BUDGET` lowers an
  expensive guest's rate, it never refuses it. A request answered this tick
  wakes the window at once, so an app that asks in a loop still runs at
  whatever rate the governor leaves it.
- `define_unknown_imports_as_traps` accepts every import a component
  declares and traps the first call; a component built against JS glue
  loads, and fails at the first frame that touches it instead of at
  install.

### Store and lifecycle

- What comes back at start is the library and the list of apps that had a
  window; app state is not persisted or suspended — a reopen, like a
  Restart after a trap, leaves an app with nothing but what it wrote to
  storage.
- Every guest window opens at 560×420 (at least 320×240) wherever the
  platform puts it the first time, and where it was last seen after that;
  the app's own `window size` is ignored. One instance per module.
- The manifest has no icon, version, author or preferred size.
- The catalog is one local directory, rescanned only when the user presses
  Rescan: no watch, no remote catalog, no download, no upgrade path, no data
  migration. Scanning reads every module in full just for its manifest, and
  the in-memory module cache notices a rebuilt module by its timestamp only
  at the next load.
- Uninstall keeps the app's storage — an app can delete its own keys, the
  store cannot. It asks once, on the app's detail page, and nowhere else.
- A guest that publishes on every tick makes the store update on every
  tick — that update is the wake that carries the message to the other
  windows, spaced only by `BUS_WAKE_INTERVAL`.

### SDK and developer experience

- `export_app!` needs the generated message enum's name (`__XMessage`), a
  coupling to codegen internals.
- Capability payloads are ad-hoc bytes (`key\nvalue`, little-endian
  integers) with no schema, no generated bindings, no versioning and no
  `host.capabilities` introspection; every app declares its own
  `HostError`.
- Building is the manual `cargo build --target wasm32-unknown-unknown`
  plus `componentize.sh`; no `cargo ice bundle` / `dev` integration, no
  `wasm-opt`. Every component still links iced's widget set and winit's
  web backend as dead code (about 600 KB after the wasm-bindgen metadata
  is stripped); a guest needs only iced's types.
- Native tests drive the app through the wire — `press`, `type_into`,
  `submit` by key or label, `answer` / `item` / `refuse` for the host —
  and read the tree back with `texts` and `find`; the Ice test harness
  (`agent_inspect`) is not available inside a module (`test = false`). No
  request-log or fuel profiler for debugging.

### What a ducktape host would add

- The real capabilities — `query`, `submit`, pages, identity, `duck://`
  navigation, signing prompts — and the permissions UI around them.
- Modules from the network: content-addressed ids, signatures, version
  pinning, upgrade with migration, precompiled artifacts.
- Typed intents between apps instead of a broadcast bus, notifications,
  badges, detached windows, background (daemon) apps, suspend when hidden.


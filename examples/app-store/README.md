# app-store — an OS-shaped host for Ice apps compiled to wasm

A native Ice application that reads a catalog of wasm modules, instantiates
one on Install inside a fuel and memory budget, gives it a window, and drops
the instance on Uninstall. Every app in the catalog is an ordinary Ice
application; nothing in the language or the code generator changes to make
it installable. What the host adds is everything an app cannot do alone —
time, storage, other apps — and it adds them as capabilities the app's
manifest has to declare.

```
frame/          the wire: events in, a Frame of quads and laid-out text lines out,
                plus Request / Response for everything else
sdk/            what an app needs to run in wasm: a headless Driver (layout, draw,
                task executor), `host::request` / `host::subscribe`, and
                `export_app!`, which adds the four C exports and the manifest
apps/counter/   three buttons; Auto is a chain of host timers, every change goes
                on the bus and into the store's log
apps/todo/      a list kept in the host's storage — it survives uninstall/reinstall
apps/clock/     host uptime from a subscription, UTC from one `clock.now` plus
                arithmetic; the module has no clock
apps/activity/  a live feed of what the other apps publish, and who published it
apps/chaos/     spins, eats memory, panics, floods the host, asks for an
                undeclared capability
host/           the store: catalog, windows, capabilities (clock, storage, bus),
                the fuel/memory sandbox, and the widget that shows a guest
```

## Run it

```
cd examples/app-store
cargo build -p app-store-todo -p app-store-counter -p app-store-clock \
            -p app-store-activity -p app-store-chaos \
            --release --target wasm32-unknown-unknown
cargo run -p app-store-host --release
```

| variable | default | what it names |
|---|---|---|
| `APP_STORE_CATALOG` | `target/wasm32-unknown-unknown/release` | the directory scanned for modules |
| `APP_STORE_DATA` | `target/app-store-data` | app storage (`<app>/<key>`) and the store's own `installed` list |

The windowing backends the host asks iced for (`x11`, `wayland`) are
requested only on Linux, so a macOS or Windows build resolves without them —
only Linux is exercised here.

Install everything: every app gets a window, all of them live at once.
Press `+` on the counter and watch Activity — and the store's own stderr,
where the counter's log lines come out. Toggle a todo, uninstall it,
reinstall it. Then open Chaos: Spin forever and Eat 1 GB end it on a fuel or
memory trap, Panic ends it with the module's own message, Flood shows what a
guest hears past the per-tick request cap. Restart it from its own window.
What you leave installed comes back: the store writes the ids to
`<data dir>/installed` and reinstalls them, one compile at a time, at the
next start.

## Writing an app

```rust
ui_lang::include_app!("src/ui/app.ice");
app_store_sdk::export_app!(Clock, __ClockMessage, "Clock", "Host uptime.", ["clock"]);
```

That is the whole crate. `export_app!` implements the sdk's `WasmApp` trait
over the generated `__boot` / `__view` / `__update` / `__theme`, emits the
exports, and writes name, description and capabilities into an
`ice.manifest` custom section, so the catalog lists the app — and shows what
it will touch — by reading the file: no compilation, no instantiation.

An app talks to the host from ordinary Ice tasks. A one-shot ask is
`host::request`; something that keeps coming is `host::subscribe`:

```ice
extern crate::host
  stream ticks(every_ms:i64) -> i64 ! ClockError

on mount
  stream every ticks(1000) -> ticked _ | clock_failed _
```

```rust
pub fn ticks(every_ms: i64) -> impl Stream<Item = Result<i64, ClockError>> + Send + 'static {
    host::subscribe("clock.ticks", &every_ms.to_le_bytes()).map(|answer| /* bytes → ms */)
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
| `host` (always) | `echo` · `log` · `random` (`u32` LE count) | the text back · nothing, the line goes to the store's stderr as `[<app>] …` · that many bytes |
| `clock` | `sleep` (ms) · `ticks` (every ms, stream) · `now` | nothing at the deadline · host uptime per tick · the unix millisecond and the host uptime it was read at, two `u64` LE |
| `storage` | `get` (key) · `set` (`key\nvalue`) · `delete` (key) · `list` | the value or empty · nothing · nothing · every key, newline-separated; one file per key per app |
| `bus` | `publish` (`topic\ntext`) · `subscribe` (topic or `*`, stream) | how many heard it · every matching message, as `from\ntopic\ntext` |

Publishing does not need the topic: any app with `bus` may publish under
any name, and `from` — the publisher's app id, which the host fills in — is
what says who did. A real host would route `query` / `submit` / `page`
through the same table and answer from its own subscriptions.

## How a frame crosses

1. The sdk's driver polls the app's tasks (re-polling one that wakes
   itself, which every `Task::stream` does once), builds the view, lays it
   out with a `UserInterface`, and draws into `iced_tiny_skia`'s recording
   layers — the same renderer the desktop uses, minus the rasterization.
2. Those layers are flattened into a `Frame`: quads verbatim; every paragraph
   as the lines cosmic-text already broke, each with its position, size, line
   height and font family. Text crosses as lines, not glyphs, so the host can
   be any iced renderer. The frame also carries the requests the tasks made.
3. The host widget translates iced events into the app's coordinates, ticks
   it once per redraw with a fresh fuel budget, answers its requests, and
   replays the frame inside `with_layer` / `with_translation`. A press inside
   a guest's bounds focuses it and a press anywhere else clears that focus, so
   only one guest at a time receives the keyboard — except the modifier state,
   which is state rather than input and goes to every guest, so one that lost
   focus with Shift down learns it came up. Answers are delivered as
   events: an echo on the next redraw, a timer at its deadline via
   `request_redraw_at`, a bus message when another guest publishes.

There is no executor thread and no clock inside a module. `Instant::now()`
is a stub that answers zero; the host's uptime rides on every `Redraw`, and
added to zero it is a monotonic clock — enough for iced's own animations.

Both sides pin the default font by name (`Fira Sans`, embedded via iced's
`fira-sans` feature): natively fontdb resolves `Font::DEFAULT` through the
system font list, in wasm only the embedded family exists, and a mismatch
shows up as every button a few pixels wide of where the app put it.

## The sandbox

Every tick runs with `FUEL_PER_TICK` (200M, roughly one per instruction)
and a 64 MB memory limit, in a store that allows one instance, one memory
and four tables of at most a million elements — a table is allocated at its
declared minimum when the module is instantiated, before any other limit is
consulted. An app that spins burns its budget and traps; an app that
allocates past the limit traps on the grow. A trap ends that instance — its
window shows the reason (the module's own panic message when the sdk's hook
left one) and a Restart button that asks the store to reload the module on
its executor, where the compile does not stall the window, and swap it into
the same handle, keeping the window and everything the app wrote to
storage — and nothing else notices: the other guests keep ticking, the
store keeps answering. The status line in every window is what the last
tick cost, plus the bus deliveries the guest was not there to take.

Fuel and memory bound what a module does to itself. What it can make the
*host* do is bounded by the constants at the top of `store.rs`:

| limit | value | what a guest past it gets |
|---|---|---|
| `MAX_FRAME_BYTES` | 8 MiB | the instance ends, "frame too large" |
| `MAX_REQUESTS_PER_TICK` | 256 | `Err "too many requests this tick"` for the rest |
| `MAX_PAYLOAD_BYTES` | 1 MiB | `Err`, whatever the request was |
| `MAX_TICKERS` | 16 per guest | `Err` from `clock.ticks` |
| `MAX_DUE` | 1024 answers the host still holds | `Err` from `clock.sleep` |
| `MAX_SUBSCRIPTIONS` | 64 per guest | `Err` from `bus.subscribe` |
| `MAX_CANCELS` | one tick's worth of everything the host holds | the rest of that frame's cancels ignored |
| `MAX_REPLY_BYTES_PER_TICK` | 4 MiB | `Err` for every later answer that tick |
| `MAX_BUS_BYTES` | 64 KiB per message | `Err` from `bus.publish` |
| `MAX_INBOX` / `MAX_INBOX_BYTES` | 1024 events, 1 MiB | its oldest bus deliveries dropped, and counted in its status line |
| `MAX_VALUE_BYTES` | 1 MiB | `Err` from `storage.set` |
| `MAX_APP_KEYS` | 1024 per app | `Err` from `storage.set` |
| `MAX_APP_STORAGE` | 64 MiB per app | `Err` from `storage.set`, summed over the app's directory, a block per key |
| `MAX_RANDOM_BYTES` | 4096 per answer | `Err` from `host.random` |
| `MAX_LOG_BYTES` | 1024 per line | the rest is cut, and the line is escaped before it reaches a terminal |
| `MAX_FAULT_BYTES` | 1024 | its window shows the first line of that |

The numbers *inside* a frame are the other half of the same boundary: the
guest chooses every one, and the host's renderer panics on some (a colour
past 1, a font size of 0, a bordered quad narrower than a pixel) and
allocates by others — a shadow is a buffer the size of the quad, so a
100000-pixel one is 40 GB. `wire::sanitize` pulls them into range on the way
in: positions and extents to twice the window in every direction, colours to
`0..=1`, blur to 64 px, text size and line height to `1..=128` px. Clamped
rather than refused, because "not finite" is not the same as "hostile" —
every iced frame's base layer carries an infinite clip. The counts are the
same boundary: a frame draws at most 16384 quads and 4096 lines carrying
64 KiB of text between them, and spends a budget of four million shadow
pixels in order — past it a quad keeps its shape and loses its shadow.

## What is not here yet

An honest inventory, grouped by where the work would land. Items marked
**bug** are wrong today rather than merely absent.

### Wire and rendering

- Images, SVG, canvas geometry (paths, strokes, meshes) and shaders do not
  cross; gradients flatten to their first stop. Images and SVG need a
  host-side handle cache keyed by app so bytes cross once, not per frame.
- Rich text loses per-span colour, underline and strikethrough: a paragraph
  crosses with one colour per line.
- Every redraw re-encodes and re-sends the whole frame; there is no
  "nothing changed" short-circuit, no delta, no dirty rectangles. Text is
  laid out in the guest and shaped again on the host, per line.
- Overlays (pick_list menus, tooltips, combo boxes) are clipped to the app's
  window; they cannot float over the desk.
- A guest's quad shadow is drawn without the layer's clip mask (the shadow
  pixmap goes to tiny-skia with `None`), so a shadow may paint outside the
  guest's window — up to the two-window reach `sanitize` allows — over the
  store column. Fixing it is a change in the vendored `iced_tiny_skia`, not
  here: the mask the engine has is chosen from the quad's bounds, not the
  shadow's. **bug**
- No scale factor, theme (`ThemeChanged`) or locale reaches the guest; an
  app cannot follow the host's dark mode.

### Events and input

- Only a subset of named keys cross and physical key and location are always
  `Unidentified`. Mouse Back/Forward, touch, IME composition, window
  focus/unfocus, file drops and close requests do not cross at all.
- Focus is the host's, not the guest's: clicking a guest gives it every key,
  and `Tab` inside one never leaves it. No focus ring, no keyboard-only way
  to reach a window.
- Clipboard is `clipboard::Null`: copy and paste inside an app silently do
  nothing. A `clipboard` capability (`read` request, `write` in the frame)
  is the shape.
- No accessibility tree leaves the guest, although the runtime still
  builds its snapshot machinery into every module (dead weight).

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
  arrives whenever the next event, timer or foreign redraw ticks the guest.
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
  `storage.list` count as two. Every `set` stats the
  whole app directory to check the quota, so `MAX_APP_KEYS` is what keeps
  that scan short — there is no counter the host maintains. No sharing
  between apps, no migration on app upgrade, and nothing outside the app can
  read or list what it stored.
- Bus: no topic ownership — any app with `bus` can publish `counter\n999`
  under its own name. No rate limit, no replay for late subscribers, no
  request/reply between apps, no wildcard beyond `*`.
- Beyond the sandbox table: no cumulative CPU budget (an app may burn its
  200M every frame) and no wall-clock timeout. A request answered this tick
  wakes the window at once, so an app that asks in a loop pins the whole
  desk's redraw rate.
- `define_unknown_imports_as_default_values` stubs every import a module
  declares; a module built against JS glue loads and misbehaves instead of
  failing at install.

### Store and lifecycle

- Only the installed set comes back. App state is not persisted or
  suspended — a restart, like a restart-in-place after a trap, leaves an app
  with nothing but what it wrote to storage.
- Windows are fixed at 500×380: no resize, move, z-order, minimise or
  maximise; the app's own `window size` is ignored. One instance per
  module.
- The manifest has no icon, version, author or preferred size.
- The catalog is one directory scanned once at boot: no rescan, no remote
  catalog, no download, no upgrade path, no data migration. Each install —
  and each Restart — compiles from scratch (about 1.7 s) — no `Module::serialize` cache, no
  sharing between installs of the same module. Scanning reads every module
  in full just for its manifest.
- Uninstall keeps the app's storage — an app can delete its own keys, the
  store cannot — and asks no confirmation.
- A guest scrolled out of the desk skips its tick only while it has nothing
  to do; every other guest still ticks on every window redraw, and one app's
  timer or publish still wakes the whole window. No per-window redraw.

### SDK and developer experience

- `export_app!` needs the generated message enum's name (`__XMessage`), a
  coupling to codegen internals.
- Capability payloads are ad-hoc bytes (`key\nvalue`, little-endian
  integers) with no schema, no generated bindings, no versioning and no
  `host.capabilities` introspection; every app declares its own
  `HostError`.
- Building needs the manual `cargo build --target wasm32-unknown-unknown`;
  no `cargo ice bundle` / `dev` integration, no `wasm-opt`, and every module
  embeds Fira Sans and the full widget set (about 2.9 MB).
- Native tests drive the app by clicking on text positions; the Ice test
  harness (`agent_inspect`) is not available inside a module
  (`test = false`). No frame snapshot tests, no request-log or fuel
  profiler for debugging.

### What a ducktape host would add

- The real capabilities — `query`, `submit`, pages, identity, `duck://`
  navigation, signing prompts — and the permissions UI around them.
- Modules from the network: content-addressed ids, signatures, version
  pinning, upgrade with migration, precompiled artifacts.
- Typed intents between apps instead of a broadcast bus, notifications,
  badges, detached windows, background (daemon) apps, suspend when hidden.

Numbers behind the design are in `docs/decisions/0010-view-in-wasm-spike.md`.

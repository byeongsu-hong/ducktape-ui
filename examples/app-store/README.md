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
apps/counter/   three buttons; Auto is a chain of host timers, changes go on the bus
apps/todo/      a list kept in the host's storage — it survives uninstall/reinstall
apps/clock/     shows host uptime from a subscription; the module has no clock
apps/activity/  a live feed of everything the other apps publish on the bus
apps/chaos/     spins forever, eats memory, asks for an undeclared capability
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

The catalog is `target/wasm32-unknown-unknown/release` (override with
`APP_STORE_CATALOG`); storage lands in `target/app-store-data/<app>/<key>`
(override with `APP_STORE_DATA`). Install everything: every app gets a
window, all of them live at once. Press `+` on the counter and watch
Activity; toggle a todo, uninstall it, reinstall it. Then open Chaos.

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
"Use the clock" shows the refusal text).

| capability | operations | answer |
|---|---|---|
| `host` (always) | `echo` | the text back |
| `clock` | `sleep` (ms) · `ticks` (every ms, stream) | nothing at the deadline · host uptime per tick |
| `storage` | `get` (key) · `set` (`key\nvalue`) | the value or empty · nothing; one file per key per app |
| `bus` | `publish` (`topic\ntext`) · `subscribe` (topic or `*`, stream) | how many heard it · every matching message |

A real host would route `query` / `submit` / `page` through the same
table and answer from its own subscriptions.

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
   replays the frame inside `with_layer` / `with_translation`. Answers are
   delivered as events: an echo on the next redraw, a timer at its deadline
   via `request_redraw_at`, a bus message when another guest publishes.

There is no executor thread and no clock inside a module. `Instant::now()`
is a stub that answers zero; the host's uptime rides on every `Redraw`, and
added to zero it is a monotonic clock — enough for iced's own animations.

Both sides pin the default font by name (`Fira Sans`, embedded via iced's
`fira-sans` feature): natively fontdb resolves `Font::DEFAULT` through the
system font list, in wasm only the embedded family exists, and a mismatch
shows up as every button a few pixels wide of where the app put it.

## The sandbox

Every tick runs with `FUEL_PER_TICK` (200M, roughly one per instruction)
and a 64 MB memory limit. An app that spins burns its budget and traps; an
app that allocates past the limit traps on the grow. A trap ends that
instance — its window shows the reason — and nothing else notices: the
other guests keep ticking, the store keeps answering. The status line in
every window is what the last tick cost.

## What is not here yet

An honest inventory, grouped by where the work would land. Items marked
**bug** are wrong today rather than merely absent.

### Wire and rendering

- Images, SVG, canvas geometry (paths, strokes, meshes) and shaders do not
  cross; gradients flatten to their first stop. Images and SVG need a
  host-side handle cache keyed by app so bytes cross once, not per frame.
- Layer transformations apply to text only; a scaled or rotated layer's
  quads are replayed untransformed. **bug**
- Rich text loses per-span colour, underline and strikethrough: a paragraph
  crosses with one colour per line.
- Every redraw re-encodes and re-sends the whole frame; there is no
  "nothing changed" short-circuit, no delta, no dirty rectangles. Text is
  laid out in the guest and shaped again on the host, per line.
- Overlays (pick_list menus, tooltips, combo boxes) are clipped to the app's
  window; they cannot float over the desk.
- `Frame.interaction` is filled by the guest and ignored by the host: the
  cursor never changes shape over a guest's text input or link.
- No scale factor, theme (`ThemeChanged`) or locale reaches the guest; an
  app cannot follow the host's dark mode.

### Events and input

- Keyboard events go to **every** guest at once: two apps with focused
  inputs both receive the typing. There is no focus model between windows.
  **bug**
- Only a subset of named keys cross; physical key and location are always
  `Unidentified`; `ModifiersChanged` is dropped, so a guest's modifier
  state can go stale. `CursorEntered`, mouse Back/Forward, touch, IME
  composition, window focus/unfocus, file drops and close requests do not
  cross at all.
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
- A dropped or aborted stream is not told to the host; a `clock.ticks`
  ticker or bus subscription lives until the guest is dropped. A one-shot
  `sleep` cannot be cancelled either. A `Frame.cancels` list fixes both.
- Task fairness is fixed: 8 rounds of messages per tick, 64 self-wakes per
  stream. A task that produces more waits for the next tick.
- No cooperative long computation: work heavier than one fuel budget
  cannot be spread over ticks except by chaining host sleeps. No
  preemption short of the trap that ends the app.
- A guest panic is an `unreachable` trap with no message: no panic hook
  writes the text somewhere the host can read.
- No guest logging: `println!` and `tracing` inside a module go nowhere.
  A `host.log` operation is one line.
- No wall-clock time (`SystemTime::now()` aborts on
  `wasm32-unknown-unknown`), no randomness (`getrandom` does not build
  without `js`), no locale, timezone or environment. `clock.now` and
  `host.random` are missing.
- A trapped app cannot be restarted in place; only uninstall and
  reinstall. Its inbox keeps filling until then. `live wasm instances`
  still counts it.

### Capabilities and security

- The manifest is self-declared and unsigned: any module can claim
  `storage`. No signature or hash check on modules, no consent prompt at
  install, no per-operation prompt, no runtime revocation, no policy file.
- Storage: no quota, no value size limit, no atomic or fsync'd writes (a
  crash mid-write tears the file), no `list` / `delete`, no sharing
  between apps, no migration on app upgrade. A value larger than the
  guest's memory limit traps the guest on delivery.
- Bus: no sender identity in a message and no topic ownership — any app
  with `bus` can publish `counter\n999`. No size or rate limit, no replay
  for late subscribers, no request/reply between apps, no wildcard beyond
  `*`. A subscriber that never drains (faulted) grows its inbox without
  bound.
- Clock: any number of tickers per app; each is a host wake-up.
- Limits stop at fuel per tick and memory: no cumulative CPU budget (an
  app may burn 200M every frame), no wall-clock timeout, no cap on frame
  size, request count or payload size (a guest can hand the host a 100 MB
  frame or a million requests per tick), no table or instance limits.
- `define_unknown_imports_as_default_values` stubs every import a module
  declares; a module built against JS glue loads and misbehaves instead of
  failing at install.

### Store and lifecycle

- The installed set is not persisted: restarting the host forgets it.
  App state is not persisted or suspended either — only what an app
  writes to storage survives.
- Windows are fixed at 500×380: no resize, move, z-order, minimise or
  maximise; the app's own `window size` is ignored. One instance per
  module.
- The manifest has no icon, version, author or preferred size.
- The catalog is one directory scanned once at boot: no rescan, no remote
  catalog, no download, no upgrade path, no data migration. Each install
  compiles from scratch (about 1.7 s) — no `Module::serialize` cache, no
  sharing between installs of the same module. Scanning reads every module
  in full just for its manifest.
- Uninstall keeps the app's storage with no way to remove it, and asks no
  confirmation.
- Every guest ticks on every window redraw, visible or not, so one app's
  timer wakes all of them. No visibility gating, no per-window redraw.

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

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

## What is deliberately not here yet

- Images, SVG, gradients and canvas geometry do not cross (gradients flatten
  to their first stop). Add primitives when an app needs them.
- No accessibility tree, clipboard, input method or cursor shape crosses the
  boundary. Each is a second small channel next to `Frame`.
- Only `Action::Output` of a task is honoured: widget operations (focus,
  scroll-to), clipboard and window actions a task emits are dropped.
- A subscription lives until its guest is dropped; an app that drops its
  stream keeps receiving into the void until then. A `Frame.cancels` list
  fixes that when an app needs to unsubscribe while alive.
- No CPU limit across ticks: an app may burn its full budget every frame.
  A per-second allowance is one counter in the host.

Numbers behind the design are in `docs/decisions/0010-view-in-wasm-spike.md`.

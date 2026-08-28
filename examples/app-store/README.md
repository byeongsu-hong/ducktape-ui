# app-store — install, run and uninstall Ice apps compiled to wasm

A native Ice application that reads a catalog of wasm modules, instantiates
one on Install, shows it as a tab, and drops the instance on Uninstall. Every
app in the catalog is an ordinary Ice application; nothing in the language or
the code generator changes to make it installable.

```
frame/          the wire: events in, a Frame of quads and laid-out text lines out
sdk/            what an app needs to run in wasm: a headless Driver around the
                generated application and `export_app!`, which adds the four C
                exports and an `ice.manifest` custom section
apps/todo/      an Ice todo list  (src/ui/app.ice + one `export_app!` line)
apps/counter/   an Ice counter    (the smallest installable app)
host/           the store: catalog from manifests, install as an async task,
                one `extern wasm_view(surface)` component per running app
```

## Run it

```sh
cd examples/app-store
cargo build -p app-store-todo -p app-store-counter --release --target wasm32-unknown-unknown
cargo run -p app-store-host --release
```

The catalog is every `.wasm` in `target/wasm32-unknown-unknown/release` that
carries an `ice.manifest` section (override the directory with
`APP_STORE_CATALOG=<dir>`). Install compiles and instantiates the module on
iced's executor — the second or so cranelift takes never stalls the window —
and the app appears as a tab with its own live state. Uninstall drops the
last handle to the instance; `live wasm instances` at the bottom of the
sidebar is the number of wasmtime stores alive, so onboarding and offboarding
are visible as 0 → 1 → 2 → 1 → 0. Reinstalling starts the app fresh.

Under a bare Xvfb give the window X focus first (`xdotool windowfocus`);
XTEST keys go to the focused window and there is no window manager to set one.

`cargo test` drives the todo app natively through the same driver the wasm
export uses (typing → Add → row), so a broken event or text path fails without
a window or a runtime.

## Writing an app

```rust
ui_lang::include_app!("src/ui/app.ice");
app_store_sdk::export_app!(Counter, __CounterMessage, "Counter", "Three buttons and a number.");
```

That is the whole crate. `export_app!` implements the sdk's `WasmApp` trait
over the generated `__boot` / `__view` / `__update` / `__theme`, emits the
exports, and writes the name and description into a custom section so the
catalog can list the app by reading the file — no compilation, no
instantiation, a hundred apps cost a hundred file reads.

## How a frame crosses

1. The sdk's driver builds the app's view, lays it out with a
   `UserInterface`, and draws into `iced_tiny_skia`'s recording layers — the
   same renderer the desktop uses, minus the final rasterization.
2. Those layers are flattened into a `Frame`: quads verbatim; every paragraph
   as the lines cosmic-text already broke, each with its position, size, line
   height and font family. Text crosses as lines, not glyphs, so the host can
   be any iced renderer — it shapes one line at a time with the same font and
   iced's text cache dedups unchanged lines across frames.
3. The host widget translates iced events into the app's coordinates, ticks
   it once per redraw, and replays the frame inside `with_layer` /
   `with_translation`.

Both sides pin the default font by name (`Fira Sans`, embedded via iced's
`fira-sans` feature): natively fontdb resolves `Font::DEFAULT` through the
system font list, in wasm only the embedded family exists, and a mismatch
shows up as every button a few pixels wide of where the app put it.

## What is deliberately not here yet

- Images, SVG, gradients and canvas geometry do not cross (gradients flatten
  to their first stop). Add primitives when an app needs them.
- No accessibility tree, clipboard, input method or cursor shape crosses the
  boundary. Each is a second small channel next to `Frame`.
- `Instant::now()` inside an app answers zero (web_time's wasm-bindgen shims
  are stubbed), so time-driven animation is frozen. A `now` field on
  `Event::Redraw` fixes it when something needs it.
- Apps are synchronous: `Task`s returned by handlers are dropped.
- No sandboxing policy beyond wasm's own: no fuel, no memory cap, no
  per-call timeout. A store that takes untrusted apps sets all three on the
  `Store`.

Numbers behind the design are in `docs/decisions/0010-view-in-wasm-spike.md`.

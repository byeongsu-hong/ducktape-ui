# wasm-view — an Ice app running inside wasm, drawn by a native Ice app

The question this answers: can a third party ship a view written in Ice as a
wasm module, and can the host draw it as if it were native? Yes — and nothing
in the language or the code generator changes to make it so.

```
guest/   an ordinary Ice app (src/ui/app.ice) compiled for wasm32-unknown-unknown,
         driven headlessly by src/driver.rs; four C exports are the whole ABI
frame/   the wire: events in, a Frame of quads and laid-out text lines out
host/    a native Ice app whose `extern wasm_view(surface)` component holds a
         wasmtime instance, forwards events into it, and replays its frames
```

## Run it

```sh
cd examples/wasm-view
cargo build -p wasm-view-guest --release --target wasm32-unknown-unknown
cargo run -p wasm-view-host --release
```

The host loads `target/wasm32-unknown-unknown/release/wasm_view_guest.wasm`
(override with `WASM_VIEW_GUEST=<path>`). Click a row's mark to toggle it, type
into the field and press Add: every event crosses into wasm, every pixel the
guest asked for comes back as a primitive the host draws itself. Under a bare
Xvfb give the window X focus first (`xdotool windowfocus`), XTEST keys go to
the focused window and there is no window manager to set one.

`cargo test` runs the guest natively through the same driver the wasm export
uses, so a broken event or text path fails without a window or a runtime.

## How it works

1. The guest's `driver` builds the app's view, lays it out with a
   `UserInterface`, and draws into `iced_tiny_skia`'s recording layers — the
   same renderer the desktop uses, minus the final rasterization.
2. Those layers are flattened into a `Frame`: quads verbatim; every paragraph
   as the lines cosmic-text already broke, each with its position, size, line
   height and font family. Text crosses as lines, not glyphs, so the host can
   be any iced renderer — it shapes one line at a time with the same font and
   iced's text cache dedups unchanged lines across frames.
3. The host widget translates iced events into guest coordinates, ticks the
   guest once per redraw, and replays the frame inside `with_layer` /
   `with_translation`.

Both sides pin the default font by name (`Fira Sans`, embedded via iced's
`fira-sans` feature): natively fontdb resolves `Font::DEFAULT` through the
system font list, in wasm only the embedded family exists, and a mismatch
shows up as every button a few pixels wide of where the guest put it.

## What is deliberately not here yet

- Images, SVG, gradients and canvas geometry do not cross (gradients flatten
  to their first stop). Add primitives when a view needs them.
- No accessibility tree, clipboard, input method or cursor shape crosses the
  boundary. Each is a second small channel next to `Frame`.
- `Instant::now()` inside the guest answers zero (web_time's wasm-bindgen
  shims are stubbed), so time-driven animation is frozen. A `now` field on
  `Event::Redraw` fixes it when something needs it.
- The guest is synchronous: `Task`s returned by handlers are dropped.

Numbers behind the design are in `docs/decisions/0010-view-in-wasm-spike.md`.

# 0010: An Ice view can run inside wasm — measured

- Status: Proposed (spike; shape (b) built as `examples/app-store`)
- Date: 2026-08-28

## Context

ducktape wants third-party modules to ship their own console view, written in
Ice, inside the module's wasm package. Three shapes were on the table:

- **(a) pixels out** — the view rasterizes itself in wasm and hands the host an
  RGBA buffer;
- **(b) layout in wasm, primitives out** — the compiled Ice view, the iced
  widget tree, layout and event handling all run in wasm; a recording
  renderer ships a flat list of draw primitives (quads, positioned glyphs,
  images) to the host, which rasterizes them with its own iced;
- **(c) widget tree out** — the view emits `ui_lang_template::Node` lists each
  frame and the host builds the iced elements (decision 0006's template path,
  with the compiled slot fillers moved into wasm).

(c) was the presumed shape because it needs no renderer work, but it costs a
sealed codegen target (no holes) and a wider `Node` vocabulary before a real
list widget can be expressed. (b) needs neither: the generated Rust compiles
to `wasm32-unknown-unknown` as it is. This record is the measurement that
decides between them.

## What was measured

Everything below is a release build on the 2026-08-28 dev box, 41-frame
medians, one view shape: N rows of `row(text, text, button)` inside a
scrollable in a 1024×768 viewport, built through the same runtime helpers the
code generator emits (`accessible`, `selectable_text`, `bounded_fill_element`).
The spike code is `docs/spikes/view-in-wasm/`: a shared `viewcore` crate, a
`guest` cdylib for wasm32, and a wasmtime `host` that drives the guest and the
same code natively and compares their output byte for byte.

### 1. The template path is not slower than the compiled path

The published-template path (decision 0006) is the only release path today.
Element construction only, template renderer against the inline widget shape:

| rows | compiled | template | ratio | allocations |
| --- | --- | --- | --- | --- |
| 40 | 23.8 µs | 22.8 µs | 0.96× | 1086 → 1168 |
| 200 | 170.6 µs | 153.5 µs | 0.90× | 5406 → 5808 |
| 1000 | 938 µs | 816 µs | 0.87× | 27006 → 29008 |

Decision 0006's revisit trigger — a measurable release frame regression — is
not met.

### 2. The runtime compiles to wasm32 untouched

`cargo check -p ui-lang-runtime --no-default-features --target
wasm32-unknown-unknown` passed with no change. With `--features full-runtime`
(tiny-skia, cosmic-text, swash, the rich text editor) it needed one line: the
editor's caret timer took `std::time::Instant`, which on wasm is
`iced::time::Instant` (`web_time`). That is the fix this record ships with.

### 3. Shape (b): layout in wasm, primitives out

Same session on both sides — a persistent renderer and widget cache, as in a
running app. *Steady* is a redraw with nothing changed; *changed* rewrites
every row's text so every paragraph reshapes.

| rows | native steady | wasm steady | ratio | native changed | wasm changed | ratio |
| --- | --- | --- | --- | --- | --- | --- |
| 40 | 68 µs | 94 µs | 1.38× | 218 µs | 401 µs | 1.84× |
| 200 | 346 µs | 411 µs | 1.19× | 1143 µs | 1952 µs | 1.71× |
| 1000 | 2019 µs | 2235 µs | 1.11× | 7467 µs | 10531 µs | 1.41× |

What crosses the boundary is viewport-bound, not N-bound: **452 primitives,
16 KB, 4.8 µs to decode on the host** for every N, because only the rows
inside the scrollable's viewport draw. Shape (c) has no such bound — the
whole list crosses as nodes, and its decode was measured at 91 µs for 40 rows
and 2.2 ms for 1000, dominated by serde's internally tagged enum buffering.

Per-module costs: `guest.wasm` is **3.6 MB** (fat LTO, stripped; 3.2 MB at
`opt-level = "s"`), of which 0.44 MB is the embedded Fira Sans. Cranelift
compiles it in 1.7 s (cacheable as a precompiled artifact); instantiation is
0.3 ms. The cold first frame is 0.9 ms in wasm against 3.5 ms native — native
pays fontdb's system font scan, wasm has no system fonts to scan.

**Parity.** With the default font pinned by name on both sides, 451 of the
452 primitives are byte-identical between native and wasm; the last differs
in one colour channel by one ulp (a derived scrollbar colour, 0.7432057 vs
0.7432058). Geometry — every glyph position, every quad — is identical.
Before the pin they diverged at the first quad: native fontdb resolved
`Font::DEFAULT` through the system font list, wasm fell through to the
embedded Fira. **The host and the guest must load the same font bytes and
name them; `Font::DEFAULT` is not a shared meaning across the boundary.**

### 4. Shape (a): pixels out

Full-frame raster of the same views at 1×, per frame:

| rows | native | wasm | ratio | bytes/frame |
| --- | --- | --- | --- | --- |
| 40 | 2.0 ms | 4.6 ms | 2.4× | 3.1 MB |
| 200 | 2.1 ms | 4.5 ms | 2.1× | 3.1 MB |

Fifty times the steady cost of (b), a 3 MB copy per frame, and the host sees
pixels: no accessibility tree, no text selection, no IME, no clipboard.
Rejected.

## Decision (proposed)

A third-party view should be the compiled Ice view running in wasm with a
recording renderer — shape (b). It runs at 1.1–1.4× native on a steady
frame, 1.4–1.8× when every paragraph reshapes, crosses the boundary with a
viewport-bound primitive list that decodes in microseconds, and needs no
change to the language, the code generator or the template vocabulary.

`examples/app-store` is shape (b) end to end: Ice apps compiled to wasm32
and driven headlessly, a native Ice host that lists them from a manifest
section, installs one as a wasmtime instance behind one `extern` component,
shows it — clicks, typing and all — and drops it again on uninstall. Its
README lists what does not cross the boundary yet.

What it does need beyond that example:

1. a recording `iced_core::Renderer` — `iced_tiny_skia` minus rasterization,
   its `Layer` serialized with a fixed-tag codec (not serde's internally
   tagged JSON: that is where shape (c)'s decode time went);
2. a guest driver — `UserInterface::build` / `update` / `draw` over marshalled
   events, which is what `iced_test`'s simulator already does headlessly;
3. accessibility: the guest exports its accesskit `TreeUpdate`, the host
   grafts it under the module's node;
4. the font rule above, enforced at package load rather than discovered at
   the first misaligned button;
5. a `wasm32-unknown-unknown` check on `ui-lang-runtime` in CI, so the one
   line this record fixes stays fixed.

Media surfaces and CEF stay host-privileged; a guest names them by a host
handle in a primitive and never touches the bytes.

## What this does not decide

Whether ducktape wants third-party views at all, or when. This record prices
the shape; the product call is ducktape's. Until it is made, first-party
screens stay native Ice and nothing here is on the runtime's roadmap.

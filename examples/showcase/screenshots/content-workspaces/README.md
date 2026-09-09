# Content workspace captures

These captures use native tiny-skia at scale 1, en-US, Linux and reduced motion.
Ice fixtures explicitly select light mode and load Geist; AI chat loads its own
JetBrains Mono faces. Rich chat captures use the joint Showcase/AI test build,
which enables Iced syntax highlighting. The Rust terminal-settlement test uses the app's default
light theme. The [composition guide](../../../../crates/ui-lang-components/docs/content-workspaces.md)
explains the scrolling, image, retained-state and customization boundaries.

| Capture | Root/preset and input | Viewport and observation |
| --- | --- | --- |
| [Rich content, wide](rich-wide.png) | `examples/ai-chat/src/ui/app.ice`, `rich_content` | 1180×800; the answer column caps at 760, the long URL wraps and code retains a horizontal scrollbar. Image references render alt text. |
| [Rich content, compact](rich-compact.png) | Same preset; type a composer draft, then resize | 760×600; the draft and readable content survive the smaller window. |
| [Latest after settlement](latest-after-settlement.png) | Real `streaming` chat; append, wheel into history, click Latest, append again, settle 120 paragraphs with no queued turn | 920×600; the unique final response marker and Copy action remain visible. |
| [Empty data](empty-data.png) | `tests/cases/ui/content_workspace.ice`, default | 760×600; zero rows are explained and Load sample is reachable. |
| [Grid after append](grid-after-append.png) | Load sample, focus grid, Home/F2, type a draft, background append, type `!`, Enter | 760×600; 25 rows and the committed edited value remain visible. |
| [Tree after append](tree-after-append.png) | Load sample, Files, Home/Right, Rename selected, type, background append, type `!`, Enter/Left | 760×600; the renamed child persists and parent selection resumes. |
| [Logs after resume](logs-after-resume.png) | Load sample, Logs, Home, append while paused, Resume tail | 760×600; newest row 24 is visible and unread count returns to zero. The test asserts historical selection before resuming. |
| [Custom data width](custom-data-width.png) | `customized`; begin editing at 560×600, resize, continue typing and commit | 1120×600; the caller's 640-pixel cap and active draft are preserved. |
| [Compact media](media-compact.png) | `tests/cases/ui/media_workspace.ice`; click editor, Ctrl+A, type, resize and type `!` | 560×600; the cover is 520×160, with the focused 120-pixel editor below it. |
| [Wide media](media-wide.png) | Same input, resized back | 1120×600; the cover is 720×160 and the editor retains the complete draft. |
| [Custom media width](custom-media-width.png) | Same root, `customized` | 1120×600; the caller chooses a 480×160 cover surface. |

The fixture paths in this table are relative to `examples/showcase`, except for
the full AI chat path. Geometry, native input, text and accessibility assertions
accompany the captures. Separate renderer pixel tests reject raster/SVG paint
outside the widget and parent clips, and reject a raster corner that should
be rounded away. Layout manifests alone cannot establish these pixel claims.

## Measurement scope

Each native inspection warms eight frames and measures sixty. The debug
populated data fixture at 760×600 reported view/layout/update p95
2592/1255/539 µs. The rich chat at 1180×800 reported 200/151/123 µs, with 420
revision-memo and 120 lazy-memo hits and no misses. These measurements cover
view, layout and update; they do not include raster painting or real streams.

The media fixture at 1120×600 originally decoded its literal image on every
view rebuild: debug layout p50/p95 was 39,007/61,467 µs. With one retained
embedded handle it measures 32/45 µs; view and update p95 are 68/15 µs.
A separate native ID-equality assertion rejects fresh handles deterministically.
The compact, wide and customized media PNGs are byte-identical across this fix.

The [twelve-cover raster capture](twelve-rounded-covers.png) comes from the
explicit `image_clip::twelve_rounded_covers_raster_cost` diagnostic. It paints
twelve 270×130 rounded crops in a 1180×760 software framebuffer, using the
existing album asset and shared decoded handle. Release p50/p95/max is
9,891/11,215/12,214 µs over sixty full paints after eight warmups. This includes
image sampling and each scratch-mask clear, but no application view/layout
work. It is an observed cost, not a portable CI timing assertion. Run it with:

```sh
cargo test --release -j4 -p ui-lang-runtime --test image_clip \
  twelve_rounded_covers_raster_cost -- --exact --ignored --nocapture
```

The existing retained owners separately cover 100,000 fixed-height rows,
reconciliation and allocation budgets. This finite 24-row composition does not
replace those contracts. The native image work does not establish WGPU,
platform accessibility or Tree-host parity, remote Markdown image fetching,
or durable save behavior.

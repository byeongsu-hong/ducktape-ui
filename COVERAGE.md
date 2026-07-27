# iced coverage ledger

This ledger defines what “Ice covers iced” means. The baseline is the versions
resolved by this workspace: `iced 0.14.0` and `iced_widget 0.14.2`.

This is an implementation inventory, not a roadmap. A partial or missing row does
not imply planned Ice syntax; uncommon behavior should use an existing typed
Rust boundary unless it satisfies the Core criteria in [`SPEC.md`](SPEC.md).
Language revision 1.64 and the workspace's pre-1.0 package version `0.1.0`
are intentionally separate version schemes.

- **native**: accepted Ice syntax is parsed, type-checked, lowered, and compiled
  by the reference application or a focused test.
- **partial**: a useful subset exists, but the public iced feature is not fully
  expressible.
- **missing**: there is no accepted Ice representation yet.

An internal use of an iced widget does not count as coverage. For example, the
backend may wrap layouts in `container`, but only explicit accepted Ice syntax
counts toward the row below.

Editor diagnostics use open buffers throughout every open app root's import
graph and fall back to disk when a buffer closes.

Core view control includes checked `if`, `for`, and first-match `match` arms.
Components may own ordinary cloneable state and local handlers, including
Future externs; `run latest` filters stale completions by scope and call site.
Writable component inputs are explicit `bind` props; calls use `<->` with a
direct app state, component-local state, or forwarded bind prop. Ordinary props
never carry write capability.
Generated state is isolated by hierarchical component ID. Structured native
status styles inherit the matching `active` fields before applying the
interaction-specific delta.

Top-level semantic style recipes are native Core declarations in 1.64. They
package checked utility tokens for one declared target (`col`, `row`, `flex`,
`grid`, `stack`, `box`, `text`, `input`, or `button`), optionally specialize
one same-target base, expand base-first across imported source graphs, preserve
child and later-utility precedence, and let direct typed node properties
override recipe defaults. Scaled utilities and exact-pixel spacing,
radius, and decimal text sizes share that checked lowering path. Every recipe
is checked at declaration time, including unused imported recipes. Parser,
checker, and codegen tests
cover expansion, typed overrides, typography, invalid bodies, duplicate
declarations, and target mismatch. Recipe definitions and references also
participate in cross-file LSP definition and safe rename. The workspace-local
`ducktape-ui` interface and showcase compile through the same recipe path, and a
focused test proves its Ice palette matches the retained Rust `LIGHT` palette.

Component contracts in 1.64 support checked prop defaults. Missing named
arguments use pure closed expressions that cannot capture app state, component
state, parameters, or extern calls; bind and mutable component-only values
cannot be defaulted.
Required props must precede defaulted props.
Parser, checker, formatter, and codegen tests cover omission, override, type and
capture errors, and mutable-value rejection.

First-class Ice tests are native in 1.64. Top-level `test` declarations reuse
normal presets, components, checked IDs, expressions, handlers, subscriptions,
and real Rust externs. A persistent headless Iced cache drives click, hover,
press/release, typing, keys, viewport changes, dispatch, update, and recursive
task completion. Assertions cover state, presence, exact visible text and input
content, computed layout bounds, and unambiguous structured tiny-skia paint
output. Absolute and earlier-alias-relative test targets, definition, and rename
stay within one test, and generated runtime failures retain imported `.ice`
paths and lines. Parser, checker,
formatter, codegen, runtime, schema/LSP, migrated examples, and invalid/runtime
failure tests provide direct evidence. There is no legacy external ICE-test
parser or adapter.

## Measured coverage

The scoped implementation score is **100%** on all three executable inventories:

- **59/59 public ledger rows are native.** No row below is `partial` or
  `missing` for the pinned iced baseline.
- **47/47 render-node kinds have a runtime witness.** The dedicated
  `render_surface.ice` contract keeps every branch populated, resolves every
  concrete rendered node by its checked ID, asserts its computed visibility,
  and checks the rendered descendants of `if`, `for`, component, and slot
  nodes. Its visible-text assertions execute the complete fixture through the
  tiny-skia headless draw path. The separate `render_contract_covers_every_render_node`
  gate uses an exhaustive `ViewNode` match with no wildcard and compares that
  same reachable application graph with the exact node inventory. Adding or
  dropping a renderer node breaks the gate.
- **32/32 programmatic render-inspection fields have a runtime witness.** The
  component contracts read every public target field across identity, value,
  visibility, bounds, clipping, scroll content/translation, surface paint, and
  text paint from real post-layout and post-draw targets.

The reference Tasks, extended Showcase, and alternate-theme views also execute
the real headless draw path. First-class component contracts separately assert
all public target fields, computed layout relationships, control events,
rerendered state, and conditional overlay presence without reading pixels.

These percentages are language-surface coverage, not Rust line/branch coverage
and not every possible combination of state, theme, viewport, and style value.
Those combinations remain ordinary application test cases; claiming them as a
single percentage would be misleading.

## Accessibility

Core accessibility is **native for single-window Linux and Windows
applications** through Ice-owned AccessKit adapters for AT-SPI and UI
Automation. It remains **partial at cross-platform system scope** because other
targets do not yet export to native screen readers.

| Core surface | Delivered contract |
| --- | --- |
| `text` | AccessKit `Label` with the visible text as its value |
| `input` | `TextInput` with value, or `PasswordInput` with no exported value; leading text is the default name and checked `label=`/`description=` may override/extend it |
| `button` | `Button` with focus/click actions; compact text is the default name, child content requires `label=`, and `description=` is optional |
| `checkbox` | `CheckBox` with toggled state and focus/click actions; visible text is the default name and checked `label=`/`description=` may override/extend it |
| `image` | a labeled image is an `Image`; an unlabeled image is decorative and omitted, and `description=` requires `label=` |
| focus | source/view-tree read and Tab/Shift+Tab order, disabled-target skip, button Enter/Space, checkbox Space, and a visible wrapper focus outline; no numeric focus order |

AccessKit tree construction and action dispatch are deterministic on every
target. Native export is single-window on Linux and Windows. Daemon and
multi-window adapters, native export on other targets, and exact desktop
screen-coordinate bounds are unsupported on stock Iced 0.14.0. Rich text and
advanced widget semantics are not claimed. `scripts/a11y-smoke.sh` proves that
the Linux AT-SPI tree is discoverable and an invoked action reaches the Iced
bridge; `scripts/a11y-windows-check.sh` cross-compiles the Windows adapter and
the generated reference app's production and test forms. Headless tests cover
dispatch to the app message. On Windows, Iced's automatically created initial
main window starts hidden, windowed, and non-maximized. The bootstrap resolves
its ID with `window::oldest()`, then defers configured-mode restoration, the
selected boot or preset task, and received messages until UI Automation subclass
attachment;
it then restores the mode and releases the initial task alongside queued
messages, preserving queue order. Named windows retain their configured settings
and remain outside native export.

## Typed system reachability

Ice 1.64 has thirty-three checked Rust boundaries:

| Boundary | Rust ABI | Covers |
| --- | --- | --- |
| `name(args)` | `async fn(...) -> Output` or `Result<Output, Error>` | domain I/O and arbitrary futures through native `Task::perform` |
| `component name(args)` | `fn<'a>(..., &'a T, ...) -> Element<'a, Event, Theme, Renderer>` or an owned `'static` form | any owned or app-state-borrowing widget tree using the configured theme and renderer, including custom widgets |
| app `renderer rust_path` | concrete `iced::program::Renderer` type | application-wide custom graphics renderer/compositor selection, propagated through every generated `Element` and checked by rustc |
| `selector name(args)` | `fn(...) -> impl widget::selector::Selector<Output = Event>` | custom native matching over every widget candidate with arbitrary checked outputs |
| `shader name(args)` | `fn(...) -> impl shader::Program<Event>` | native wgpu primitives, pipeline/storage, state, events, redraw, capture, and mouse interaction |
| `task name(args)` | `fn(...) -> Task<Event>` or `Task<Result<Event, Error>>` | widget/window/clipboard/font/system operations and arbitrary task composition |
| `stream name(args)` | `fn(...) -> impl Stream<Item = Event>` or `Stream<Item = Result<Event, Error>>` | native repeated `Task::run` output and `Subscription::run`/`run_with` workers from channels, iterators, async generators, and other streams |
| `sip name(args)` | `fn(...) -> impl Sipper<Output, Progress>` or `Straw<Output, Progress, Error>` | native repeated progress plus one final output through `Task::sip` |
| `recipe name(args)` | `fn(...) -> impl Recipe<Output = Event>` | custom subscription identity, runtime-event input, streams, cancellation, and arbitrary recipe behavior through native `from_recipe` |
| `event-filter name()` | `fn(subscription::Event) -> Option<Event>` | native raw runtime-event filtering with an explicit hashable identity, including interaction window IDs/status and system-theme changes |
| `sync name(args)` | `fn(...) -> Output` | checked synchronous domain conversions usable in Ice expressions |
| `subscription name(args)` | `fn(...) -> Subscription<Event>` | event, keyboard, mouse, window, system, channel, timer, stream, and custom subscription sources |
| `theme name(args)` | `fn(...) -> iced::Theme` | native app and nested default-renderer themes, including `custom`, `custom_with_fn`, and complete palette/extended-palette logic |
| `themer name(args) -> Event` | factory returning `Option<Theme>`, `Element<'static, Event, Theme>`, and optional Theme-dependent text/background callbacks | native alternate `Theme: Base` subtrees inside the default-Theme app, including `Themer::new`, default Theme fallback, event mapping, `text_color`, and `background` |
| `window name(args)` | `fn(&dyn iced::window::Window, ...) -> Output` | exact typed access to native window/display handles and other callback-only window behavior through `window::run` |
| `markdown-viewer name(args)` | `fn(...) -> impl for<'a> markdown::Viewer<'a, Event>` | native custom rendering of every Markdown item through `view_with` while preserving checked link-event routing |
| `editor-binding name(args)` | `fn(text_editor::KeyPress, ...) -> Option<text_editor::Binding<Event>>` | native custom key mapping across every built-in Binding plus typed custom application routes |
| `editor-highlighter name(args)` | generic adapter from plain `TextEditor` to default `Element` | native `highlight_with` access to arbitrary Highlighter settings, highlights, iterators and Theme-aware formats |
| `editor-style name(args)` | `fn(&Theme, text_editor::Status, ...) -> text_editor::Style` | native theme/status-aware runtime editor style callbacks, equivalent to the default Theme's advanced class representation |
| `text-style name(args)` | `fn(&Theme, ...) -> text::Style` | native theme-aware runtime text and rich-text style callbacks, equivalent to the default Theme's advanced class representation |
| `slider-style name(args)` | `fn(&Theme, slider::Status, ...) -> slider::Style` | native theme/status-aware runtime slider style callbacks, equivalent to the default Theme's advanced class representation |
| `progress-style name(args)` | `fn(&Theme, ...) -> progress_bar::Style` | native theme-aware runtime progress style callbacks, equivalent to the default Theme's advanced class representation |
| `button-style name(args)` | `fn(&Theme, button::Status, ...) -> button::Style` | native status-aware runtime button style callbacks, equivalent to the default Theme's advanced class representation |
| `checkbox-style name(args)` | `fn(&Theme, checkbox::Status, ...) -> checkbox::Style` | native checked/status-aware runtime checkbox style callbacks, equivalent to the default Theme's advanced class representation |
| `toggler-style name(args)` | `fn(&Theme, toggler::Status, ...) -> toggler::Style` | native checked/status-aware runtime toggler style callbacks, equivalent to the default Theme's advanced class representation |
| `radio-style name(args)` | `fn(&Theme, radio::Status, ...) -> radio::Style` | native selection/status-aware runtime radio style callbacks, equivalent to the default Theme's advanced class representation |
| `box-style name(args)` | `fn(&Theme, ...) -> container::Style` | native theme-aware runtime container style callbacks, equivalent to the default Theme's advanced class representation |
| `svg-style name(args)` | `fn(&Theme, svg::Status, ...) -> svg::Style` | native theme/status-aware runtime SVG style callbacks, equivalent to the default Theme's advanced class representation |
| `input-style name(args)` | `fn(&Theme, text_input::Status, ...) -> text_input::Style` | native theme/status-aware runtime text-input style callbacks, equivalent to the default Theme's advanced class representation |
| `scroll-style name(args)` | `fn(&Theme, scrollable::Status, ...) -> scrollable::Style` | native theme/status-aware runtime scrollable style callbacks, equivalent to the default Theme's advanced class representation |
| `pick-list-style name(args)` | `fn(&Theme, pick_list::Status, ...) -> pick_list::Style` | native theme/status-aware runtime pick-list style callbacks, equivalent to the default Theme's advanced class representation |
| `menu-style name(args)` | `fn(&Theme, ...) -> menu::Style` | native theme-aware runtime pick-list/combo overlay menu callbacks, equivalent to the default Theme's advanced class representation |
| `panes-style name(args)` | `fn(&Theme, ...) -> pane_grid::Style` | native theme-aware runtime panes callbacks, equivalent to the default Theme's advanced class representation |

Generated probes verify the concrete Rust signatures. Reachability is not the
same as native coverage: a row stays partial or missing until its complete
public behavior has direct documented Ice syntax and tests.

## Widgets and layout

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| `button` | native | native string or arbitrary child content, disabled route, typed size/padding/clip, all eight iced presets, every concrete field across all four statuses including linear backgrounds, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `canvas` | native | declarative rectangle/circle/line/text/path geometry; complete path builder segments, fill rules, solid/linear fill and stroke, caps/joins/dashes, transforms, clips, typed `if`/`for`, complete raster/SVG frame drawing fields, dependency-keyed geometry cache with shared named groups, typed local `Program::State`, all five event families and every variant, state updates, publish/capture/next-frame/timed-redraw actions, pointer routes, and static/state-dependent/out-of-bounds interaction cover the complete public Program behavior |
| `checkbox` | native | native label/value/disabled event, size/width/spacing, text typography/wrapping, complete font descriptors and custom icon; all four presets, every concrete Style field across active/hovered/disabled checked and unchecked statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `column` | native | children, typed spacing/per-side padding, all `Length` bounds, max width, cross-axis alignment, clipping and wrapping column spacing/alignment |
| `flex` | native | dependency-free runtime flexbox with row/column reverse directions, nowrap/wrap/wrap-reverse, justify/content/items alignment, axis gaps, padding and clipping; box items support stable order, grow/shrink/basis/self alignment, and fixed/percentage/auto margins |
| `combo_box` | native | direct checked ID; native typed replaceable and incrementally pushable search state/selection, every builder setter, complete text-input icon, every concrete input Style field across active/hovered/focused/focused-hovered/disabled statuses, complete menu overlay Style fields, typed native input/menu style callbacks, and all events |
| `box` | native | native one-child container with ID, complete concrete layout API, every concrete Style field including linear background, text, per-corner border, shadow and pixel snap, plus typed theme-aware runtime callbacks covering the default Theme's advanced classes |
| `float` | native | one child, positive scale, all original-bounds and viewport geometry exposed as scoped f64 translation inputs, and every concrete Style field through checked shadow color/offset/blur and per-corner shadow radius |
| `grid` | native | dynamic children, pixel spacing/width, fixed columns, CSS-like minimum-cell wrapping, native maximum-cell wrapping, and aspect-ratio or all `Length` height modes |
| `image` | native | path, encoded-memory and RGBA handles; all four iced length variants, fit, filter, floating/solid rotation, opacity, scale, expand, per-corner radius and crop cover the complete concrete widget API |
| `image::Viewer` | native | path or memory/RGBA handle, all length and fit modes, both filters, padding, minimum/maximum scale and scale step cover the complete public builder API |
| `keyed` | native | typed list template with bool/i64/f64 identity keys, automatic keyed child scopes, spacing/per-side padding/all `Length` bounds, max width and alignment |
| `lazy` | native | hash-keyed rebuilds with bool/i64/str, `Hash + Clone` extern values, recursive list/optional dependencies, a dependency-only scope and statically enforced owned `Element<'static>` subtrees |
| `markdown` | native | owned parsed/replaced/incrementally appended content, image URI access, syntax highlighting, every `Settings` and `Style` field, str link events, and a typed custom `Viewer` boundary covering every item renderer through native `view_with` |
| `mouse_area` | native | all button/enter/move/scroll/exit events, scroll unit preservation, and all cursor interactions |
| `overlay` | native | structured content/layer sections, conditional visibility, all three alignments on both axes, padding, checked backdrop color, modal button/scroll blocking and backdrop dismissal lower through native Stack/Float behavior; typed owned Element adapters cover the complete advanced `Overlay` trait including layout, draw, operate, update, mouse interaction, nested overlays, and `index()` ordering |
| `pane_grid` | native | recursive initial split trees with stable named nested-split resize, closed panes, list-keyed runtime pane templates with typed dynamic references, scoped per-pane maximized callback flags, bounds, click, interactive resize/drag, maximize/query, adjacency, swap, close, move-to-edge, root resize and region drop; native Content/TitleBar, full and responsive compact Controls, per-side title padding and visibility; every concrete PaneGrid Style field including linear hovered backgrounds plus typed native runtime callbacks covering advanced classes; every concrete Content/TitleBar container Style field including linear background, per-corner border, shadow and pixel snap |
| `pick_list` | native | direct checked ID; native typed choices/optional selection, every builder setter, all arrow/static/dynamic/none handles, every concrete Style field across active/hovered/opened/opened-hovered statuses, complete menu overlay Style fields, and typed native field/menu callbacks covering the default Theme's advanced classes |
| `pin` | native | one child, all `Length` bounds and pixel x/y positioning; x/y is behaviorally identical to iced's `position(Point)` helper |
| `progress_bar` | native | native range/value, all length/girth variants, horizontal/vertical, five presets, checked solid/linear track and bar backgrounds, border and per-corner radius, plus typed theme-aware runtime style callbacks covering the default Theme's advanced classes |
| `qr_code` | native | named UTF-8 or arbitrary byte data, all correction levels and normal/micro versions, cell/total size, and checked cell/background colors |
| `radio` | native | direct checked ID; native bool/i64/f64/str/extern payload values, explicit bool selection, complete sizing/typography/font setters, every concrete Style field across active/hovered selected/unselected statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `responsive` | native | arbitrary size-dependent child tree with scoped width/height bindings, breakpoint sugar and all `Length` bounds |
| `row` | native | children, typed spacing/per-side padding, all `Length` bounds, cross-axis alignment, clipping and wrapping row spacing/alignment |
| `rule` | native | axis/thickness, every fill mode, default/weak presets, checked color/opacity, per-corner radius and snap cover all concrete style fields; advanced classes are an alternate extension mechanism |
| `scrollable` | native | native content/ID, every concrete builder setter, all Viewport getters, every Status field through ordered selectors, every concrete Style field for container, rails, scrollers, gap and auto-scroll overlay, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `sensor` | native | show/resize dimensions, hide, comparable owned keys, anticipation and delay; owned keys provide the same continuity behavior as `key_ref` without borrowed lifetimes |
| `shader` | native | typed factory for any concrete native `shader::Program<Event>`, complete width/height builder API, checked message routing, and generated Program/Element probes; the Rust program retains complete State, Primitive, Pipeline/Storage, update/action, draw and mouse-interaction behavior |
| `slider` | native | direct checked ID; native f64 or arbitrary typed extern numeric values with Rust-verified iced Slider bounds; complete default/normal+shift step, sizing and change/release behavior; every concrete Style field across active/hovered/dragged including solid/linear rail and handle backgrounds, border/per-corner radius and circle/rectangle handles; typed theme/status-aware runtime callbacks cover advanced classes |
| `space` | native | optional fixed/fill/fill-portion/shrink width and height cover the complete widget API |
| `stack` | native | ordered children, all `Length` widths/heights, clipping and `push_under` base-layer behavior via `under=N` |
| `svg` | native | native path or UTF-8/raw byte memory source, all four iced length variants, fit, rotation, opacity, complete idle/hovered color style, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `table` | native | typed cloned rows, arbitrary header/cell subtrees, automatic row/column identity scopes, all table width/padding/separator setters and all column width/alignment setters |
| `text` | native | direct checked ID on text and rich text; native string/numeric text plus structured rich spans; plain text supports mouse selection across wrapped lines with platform copy/select-all shortcuts; complete Text/Rich bounds, size, relative/absolute line height, font, alignment, wrapping and color, plus Text shaping and Rich str link events; every concrete Span field including solid/linear highlight background, border/per-corner radius/padding/underline/strike; typed theme-aware runtime callbacks cover the default Theme's advanced classes |
| `text_editor` | native | app-owned direct or explicit `bind` component-prop content, generated action application, every concrete builder setter, all five built-in themes, typed arbitrary native Highlighter adapters, complete native key bindings with custom routed payloads, every concrete Style field across all statuses, and typed Theme/Status callbacks covering advanced classes |
| `text_input` | native | app-owned, explicit `bind` component-prop, or component-local str binding, ID, every concrete builder setter, complete custom icon, every concrete Style field across active/hovered/focused/focused-hovered/disabled statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `themer` | native | default/app/all 22 built-in and arbitrary typed `Theme: Base` subtrees; checked default text color and solid/linear background plus typed alternate-Theme text/background callbacks cover the complete public builder behavior |
| `toggler` | native | direct checked ID; native label/value/disabled event, size/width/spacing, text typography/wrapping/alignment and complete font descriptors; every concrete Style field across active/hovered/disabled checked and unchecked statuses, plus typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `tooltip` | native | native two-child content, all positions, gap, padding, viewport snap, delay, nine container presets, every concrete container Style field, and checked `box-style` callbacks covering the default Theme's advanced classes |

## Application and runtime

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| application settings | native | state-dependent title, all built-in/custom theme selection, base background/text style and guarded scale callbacks; application ID, custom typed executor and renderer, ordered checked font byte preloads, default text size/font, antialiasing, vsync, codec-free checked RGBA icons, complete initial/named window settings including structured Linux, Windows, macOS, and Wasm fields, structured state/task boot presets, run, and generated first-class Ice tests covering component interaction, computed layout, real task/sync flow, and structured paint checks |
| `Daemon` | native | `daemon Name` lowers to `iced::daemon`, rejects an unnamed initial window, exposes the current typed window ID to each per-window view/title/theme/scale callback, preserves named window templates and all shared settings, and standalone `exit` lowers to the native lifecycle task |
| `Animation<T>` | native | first-class checked `animation[bool]`, `animation[f64]`, and rustc-verified custom Float state map to native `Animation<T>`; every built-in or typed custom easing, preset/ms/s duration, delay, finite/forever repetition, auto-reverse, implicit/exact-instant transition, value/progress/remaining queries, f32/optional-f32 interpolation projection, and active-only native frame subscription are covered |
| explicit image allocation | native | `task image allocate handle` lowers to native `image::allocate` with required exact success/error routes; `image-allocation` retains GPU memory and exposes handle plus exact `Size<u32>`, `image-error` preserves all five native variants with kind/message projections, and `image-memory` plus downgrade/upgrade covers weak retention; requires iced's `image` feature |
| debug timing | native | `debug-span?` owns exact non-clone `iced::debug::Span` state; checked `debug start name -> state` finishes any prior span before native `time`, `debug finish state` consumes it exactly once, `debug.active(state)` reads its presence, and generic `debug.time_with(name, value)` preserves the value type; iced's `debug` feature activates reporting while its native no-op implementation remains available without the feature |
| `Theme` and styles | native | all 22 built-in default-renderer themes, generated app palettes, typed native factories including `custom`/`custom_with_fn` and complete extended-palette logic, app/nested selection, checked color tokens and target-scoped utilities, imported semantic recipes with deterministic precedence, complete widget-native catalogs, concrete style fields, and typed runtime callbacks |
| `theme::Mode` | native | default and all none/light/dark variants, compact kind projection, equality, exact typed extern passage, equivalent app theme/factory behavior, and deliberate ordering/lazy rejection matching the native enum cover the complete public value behavior |
| `Task` | native | complete public `iced::Task` construction and composition through async/task/stream/sip externs, direct `done`/`none`, system/clipboard/font/widget/window tasks, `batch`, `chain`, abortable handles including abort-on-drop/query, `map`, output-dependent `then`, optional-or-result `and_then`, `map_err`, result-preserving `collect`, `discard`, and `units`; `future`/`stream` identity forms are represented by perform/run extern sources, and default/unit conversion by `none` |
| `Subscription` | native | complete application-facing construction and composition: typed arbitrary adapters, `none`, `batch`, checked conditional activation/status filters, direct every/repeat timers, native `listen`/`listen_with`/`listen_raw` generic events, input-method/keyboard/mouse/touch/window sources (with optional typed IDs on all eleven discrete window events) and system theme changes, typed `run`/`run_with` workers, custom `Recipe` factories through `from_recipe`, raw `EventStream` filters with hashable identity, `with` identity context, typed `map` routing, noncapturing typed `filter_map`, and `units`; advanced `into_recipes` is runtime-consumer plumbing rather than subscription construction or behavior |
| widget operations | native | all 13 core focus/cursor/selection/scroll operations with checked static/dynamic identity paths through component, layout, slot, keyed, table and pane scopes, typed focus query, native `find`/`find-all` over ID, text, point and focused selectors with complete normalized target metadata, plus custom typed selector factories |
| clipboard | native | standard and primary read/write tasks; reads preserve iced's optional string payload and writes are checked fire-and-forget effects |
| fonts | native | ordered app-level relative font files are checked and embedded into iced's startup loader; runtime bytes lower to native `font::load`; every family/weight/stretch/style descriptor, checked named reference, application default and all widget font setters are covered |
| system | native | current theme task, theme-change subscription, and every information field with optionality preserved; information requires iced's `sysinfo` feature |
| time | native | `instant` maps to iced's native monotonic value; `task time now`, payload-producing `every`, and typed async `repeat` cover the complete enabled `iced::time` task/subscription API with checked positive `ms`/`s` durations (`repeat` requires iced's `tokio` feature) |
| window | native | every initial and named-open setting, including codec-free RGBA icons and structured Linux/Windows/macOS/Wasm fields; typed `window-id`, open/oldest/latest, direct targeting for every per-window close/drag/resize/constraints/state/move/mode/focus/level/menu/attention/passthrough/monitor/raw-ID/screenshot/icon task, automatic tabbing, native or flattened lossless screenshot payloads, all 12 event forms with optional IDs on all 11 discrete events, and an exact typed `window::run` callback boundary for raw window/display handles |
| `window::Id` | native | native unique construction, decimal display, equality, ordering, hashable lazy identity, exact typed extern passage, and direct task/daemon/subscription payload reuse cover the complete public value behavior |
| window value enums | native | all variants and defaults of `Direction`, `Level`, `Mode`, and `UserAttention`, compact kind projections, exact typed extern passage, equality only where the native type implements it, deliberate ordering/lazy rejection, and equivalent task keyword sugar cover the complete public behavior of these four enums |
| `window::Position` | native | default, centered, and exact Point construction, kind/optional-point projection, typed extern passage, native `SpecificWith(fn(Size, Size) -> Point)` preservation and invocation through a checked sync adapter, equivalent initial-setting sugar, and deliberate comparison/lazy rejection cover the complete public value behavior |
| `window::RedrawRequest` | native | all next-frame/at-instant/wait variants, kind/optional-instant projection, equality, ordering, exact typed extern passage, equivalent canvas/shader/raw-event behavior sugar, and deliberate lazy rejection matching the native enum cover the complete public behavior |
| event routing | native | all five structured families plus first-class generic `event` values through native `listen`/`listen_with`/`listen_raw`, optional window IDs, status filters, transforms, handler routing, and typed extern passage; system-theme runtime events remain a separate native source because iced does not represent them as `iced::Event` |
| `event::Status` | native | both ignored/captured variants, native captured-first merge semantics, compact kind projection, equality, exact typed extern passage, equivalent subscription filter sugar, and deliberate ordering/lazy rejection matching the native enum cover the complete public behavior |
| keyboard | native | all three native events preserve exact `Key`, `Physical`, `Location`, and `Modifiers` values; every named/code/native/location/modifier constructor, structured matching, safe runtime native-code conversion, exact extern passage, and native latin translation are checked Ice expressions |
| mouse/touch | native | every mouse and touch event has a direct typed subscription with exact native `Button` and `Finger` payloads; every button/finger variant, `Cursor`, advanced `Click`, and all 27 `Interaction` variants, constructors, queries, projections, ordering, transformations, typed extern passage, and direct MouseArea/Canvas passage are covered |
| `mouse::Interaction` | native | default and all 27 variants, compact kind projection, equality/order, exact typed extern passage, direct MouseArea/Canvas builder input, equivalent cursor-name sugar, and deliberate lazy rejection matching the native enum's lack of `Hash` cover the complete public value behavior |
| `mouse::ScrollDelta` | native | both Lines/Pixels variants, exact f32 coordinate construction and f64 projection, compact kind, equality, exact typed extern passage, readable event-route destructuring sugar, and deliberate ordering/lazy rejection matching the native floating-point enum cover the complete public value behavior |
| `Pixels` | native | zero, f32/u32 numeric construction with checked runtime u32 conversion, value projection, equality/order, every native pixels/pixels and pixels/scalar addition, multiplication and division form, and typed extern passage cover the complete public behavior |
| geometry primitives (default `f32`) | native | complete native `Point`, `Vector`, `Size`, and `Rectangle` constructors, constants, fields, array projections, point display, equality, arithmetic, distance, per-component size operations, conversions, containment, offset, intersection/union, exact `u32` snapping, four-side padding expansion/shrinking, radians rotation, zoom, anchoring, transformation application, and typed extern passage |
| `Padding` | native | zero/default, uniform/per-side/axis constructors, f32/u16-equivalent scalar and axis conversions, exact Pixels conversion, every side and x/y projection, all six native builder methods, `fit`, Size conversion, Rectangle expansion/shrinking, equality, and typed extern passage cover the complete public behavior |
| `Degrees` / `Radians` | native | numeric construction and f64 projection, equality/order including native angle-left scalar comparison, full range constants and containment, Degrees scaling, exact Degrees-to-Radians conversion, PI/display, every native Radians mixed arithmetic form including remainder and reverse scalar multiplication, both `to_distance` points, geometry rotation/vertex integration, and typed extern passage cover the complete public behavior |
| `Rotation` | native | exact floating/solid/default/f32 conversion, radians/degrees/kind projection, native `radians_mut` update, equality, size application, typed extern passage, and direct Image/SVG builder input cover the complete public enum behavior |
| `ContentFit` | native | all five variants/default, compact kind/native display, equality/hashable lazy identity, exact size fitting, typed extern passage, and direct Image/SVG/Viewer builder input cover the complete public enum behavior |
| `Color` | native | default/constants, normalized/static and dynamically checked 8-bit/linear/array constructors, all accepted hexadecimal parse forms with optional rejection, exact channel/RGBA8/linear/display projections, inverse/in-place inversion/alpha scaling, luminance/contrast/readability, equality, and typed extern passage cover the complete public behavior; native floating channels correctly remain unavailable as lazy hash identities |
| `Background` / `Gradient` / `Linear` / `ColorStop` | native | both background variants, every background conversion and alpha scaling, the complete current linear-only gradient enum, linear construction from f32/radians, native sorted single/multiple stop insertion including invalid/eight-stop behavior, alpha scaling, exact optional-stop array projection, every field, equality, typed extern passage, and equivalent solid/linear style sugar cover the complete public behavior; floating values correctly remain unavailable as lazy identities |
| `Font` / `Family` / `Weight` / `Stretch` / `Style` | native | native default and named/monospace constants, exact complete font construction, every descriptor default and variant, static named families, every field and compact kind/name projection, equality, hashable lazy identity, typed extern passage, and equivalent human-readable widget font declarations cover the complete public value behavior |
| text `Alignment` / `Shaping` / `Wrapping` / `LineHeight` | native | every variant and feature-aware default, both line-height payloads/conversions and absolute resolution, all alignment conversions, compact projections, equality, hashable lazy identity, exact typed extern passage, and equivalent human-readable widget properties cover the complete public value behavior |
| `Length` | native | all four variants, static/dynamically checked portion and u32 construction, exact f32/Pixels/u32 conversions, fill factor/fluidity/kind/payload projections, fluid/enclose operations, equality, typed extern passage, and direct passage through every builder that accepts native Length cover the complete public behavior; pixel-only Grid width and slider short axes retain numeric checks, and floating fixed values correctly remain unavailable as lazy hash identities |
| `Alignment` / `Horizontal` / `Vertical` | native | every variant, every bidirectional native conversion, compact kind projection, equality, hashable lazy identity, typed extern passage, and equivalent compact view-property sugar cover the complete public behavior |
| `Border` / `Radius` | native | default/exact border construction, all three border free constructors and builders, every radius free constructor and builder, all four radius numeric conversions with safe dynamic integer forms, native corner-array conversion and scaling, every field, equality, typed extern passage, and equivalent compact style sugar cover the complete public behavior; floating values correctly remain unavailable as lazy identities |
| `Shadow` | native | default and exact color/offset/blur construction, all three field projections, equality, typed extern passage, and deliberate rejection as a floating-point lazy identity cover the complete public behavior |
| `Transformation` | native | identity/default, orthographic, translate, scale, inverse, scale/translation inspection, composition, lossless matrix conversion, equality, typed extern passage, and native application to every supported geometry and pointer value cover the complete public behavior |
| `window::Screenshot` | native | exact construction and capture task delivery as one native value, public RGBA/physical-size/scale fields, borrowed and owned byte access, native crop success, both crop error kinds and messages, debug formatting, typed extern passage, and deliberate comparison/lazy rejection cover the complete public value behavior |
| custom widget | native | typed owned or app-state-borrowing `Element` adapters with checked event routing, selected Theme/Renderer propagation, alternate-Theme subtrees, and the complete advanced Widget/Overlay escape hatch |
| custom renderer | native | checked application-wide concrete `iced::program::Renderer` type path propagated through every generated `Element`, including extern components, shaders, alternate themes, and editor adapters |

The free `iced_runtime::task` constructors such as `oneshot`, `channel`,
`blocking`, and `effect` are not re-exported by `iced::task`; they are outside
this public iced baseline. A typed `task` extern can still adapt runtime-specific
work when an application intentionally depends on `iced_runtime`.

## Evidence rule

A row moves to **native** only when every public application-facing behavior in
the pinned iced surface has:

1. documented Ice syntax and static types;
2. parser and semantic-checker coverage, including invalid input;
3. generated Rust that compiles against the pinned iced release;
4. a reference or focused runtime example when behavior is interactive.

A first-class test claim counts only when it parses and checks as Ice, compiles
to an auto-discovered Rust test, and its runtime assertion observes the real
generated program or mounted component. Schema-only descriptions and manually
duplicated Rust assertions do not count.

The repository does not claim complete iced coverage while any row is partial
or missing.

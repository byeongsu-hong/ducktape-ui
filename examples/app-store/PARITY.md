# Native and wasm parity worklist

There are three distinct execution paths. The native language target emits an
Iced application with Rust externs and owns its window. Tree-native packages run
the Tree generator and guest Driver in a trusted child process. Wasm components
run that same Tree generator and Driver in Wasmtime. Both Tree paths mount the
host's native widgets. A native executable does not gain Wasm isolation.

This is a functional worklist, not a claim that full parity is complete. The
Tree refusal inventory is in README's **What is not here yet** and Core's Tree
lowering diagnostics; those refusals apply to both Tree execution paths.

| User capability | Native language target | Hosted Tree native / wasm | Next acceptance evidence |
|---|---|---|---|
| Build, catalog, explicit install, independent app windows | Direct executable | Native package backend in this PR; wasm component backend exists | Build all five apps; never execute during scan; consent binds manifest and executable |
| Input routes, host requests, palettes and native painting | Native toolkit | Shared host and Tree protocol | Same mounted Counter click, bus reply, owned state and pixels on both backends |
| Snapshot and approved reload | Native app-specific lifecycle | Shared owned-state snapshot and host replacement; shipped persistent streams use subscriptions | Preserve state and window; stale completion cannot replace live app |
| Fault containment | Application process | Native process deadline; wasm fuel, memory and epoch limits | Chaos runaway ends while another app remains usable |
| Accessibility | Native runtime semantics | Mapped host widgets preserve roles, names and semantic action targets | Identical mounted semantic actions and results on both backends; OS bridge smoke remains a separate platform gate |
| Embedded and memory raster images | Native image widget | Copied encoded/RGBA sources, typed bounded host cache and native options | Native/Wasm pixels, dimension changes, lazy remount, Resync and reload |
| Copied-image viewer | Native Iced viewer | Same raster sources/cache and native zoom/pan widget | Native/Wasm pixel comparison for options, wheel zoom, drag, limits, frames and keyed identity |
| Raster destination positioning | Floating-point destination translation before scaling | Same corrected native renderer | Native/Wasm white-margin and interior-color assertions, Red/Green against source-pixel origin truncation |
| Combo boxes | Native searchable ComboBox | App-owned typed options and native search/overlay; Rust styles and unowned value parameters refused; Component combo state retains common E103 | Actual native/Wasm keyboard and pointer selection, set/push, shared state, hide/readd and reload; stale overlay refusal |
| Dynamic image paths and non-container gradients | Existing native widgets/options | Container linear backgrounds preserve native angle/eight stops; control faces, layout surfaces, rich spans, canvas and typed background values remain gaps | Actual native/Wasm container pixels cover angle, alpha, palette-stop updates and resize; other gradients need their own evidence |
| Resize dividers | Native grabbed-pointer ResizeHandle | Same host widget, copied drag/press/release routes and cursor | Actual native/Wasm outside movement, release, size policy, removal and replacement |
| Floating content | Native layout | Host-evaluated placement, scale, shadow and radius; bounded arithmetic | Actual bundled component tests placement, resize, pixels and translated clicks |
| Mounted components in lazy containers | Native state and lifetime machinery | Cached scope sightings replay; local component changes invalidate containing caches; unmount discards mounted caches | Actual native/Wasm cache-hit, local edit, same-key remount, cancellation and obsolete-route/reply checks |
| Mounted components in host-conditional containers | Native state and lifetime machinery | Tree refusal remains until host-selected activation is reported | Visibility-fenced mount/unmount and cancellation |
| Editor bindings, ordered input and guest history | Native Iced key-binding callback | Tree factory declares copied claims and retains guest decision/post-commit callbacks; atomic patches and native fallback share an ordered logical-document lane | Actual native/Wasm typing, paste, IME, Undo/Redo, dynamic claims and shared identity pass; one-MiB document transfer, edit, snapshot/restore and Undo pass on both backends, including reload without refocusing; Pages rich presentation remains separate |
| Interaction styles and missing editor/slider/toggler/rule/scroll options | Native recipes/callbacks | Declarative slider circle/rounded-rectangle handles are copied per interaction face; remaining Rust callbacks and other options refused | Declarative recipes or host-owned semantics with equivalent states and pixels |
| Sensor reset key | Native reset semantics | Copied scalar keys re-arm the shared host sensor without replacing widget identity | Retained runtime and actual wasm fixture verify one fresh measurement per changed key |
| Recursive records, enums and opaque surface parameters | Typed Rust values | Copied scalar/list/option/record surface contract; other shapes refused | Owned declarative data or named host resources with validated lifetime and routes |
| Mouse subscriptions | Native runtime | Opt-in guest-local events with captured status; latest move per redraw, ordered discrete events | Real native/Wasm events, overlay capture, coordinate translation, units, coalescing and removal |
| Window focus, close, file-drop and IME subscriptions | Native runtime | Opt-in copied observations after native widget handling; captured status, bounded UTF-8 preedit ranges, one terminal close tick with effects discarded | Actual native/Wasm mounted events; window geometry/frame clocks and touch remain outside this contract |
| Window/system/image/font/reload/exit task effects | Native Iced actions | Own-window focus/resize/close/maximize/minimize/resizable and guest exit dispatch through the host; OS theme query/subscription uses a separate host fact; system info/image/font tasks and other window operations are refused | Actual native/Wasm host dispatch and independent OS-mode replies; raw Rust unsupported actions log explicitly, and OS application is not implied by submission acknowledgement |
| Widget commands and selectors | Full native operation boundary | Only documented Tree subset | Same selector scope and completion replies on mounted widgets |
| Clock, randomness and platform environment | OS APIs | Host clock/random; wasm direct platform calls can trap | Equivalent authored functionality via host contracts; native-only OS access is not portable |
| First-class authored Ice tests | Native harness | Counter's same authored preset/state/dispatch/click/text and keyed-row scenarios run on native and Wasm test artifacts | Explicit test artifacts support presets, typed state/dispatch, live state-keyed targets, click, exists/missing and literal rendered text; input focus/typing/selection and keyboard actions reuse the mounted Driver on both backends (action arguments are literals); mounts and other actions remain follow-up |
| Restart persistence and multi-instance store | App-specific | Reload snapshot only; one instance per module | Explicit persistence and identity semantics, not merely a backend switch |

Raw Rust closures, native GPU programs and opaque handles are
not wire values. Functional parity requires declarative data or explicitly
registered host providers that retain native behavior. Their current rejection
is a tracked implementation gap where it blocks an authored user capability.

The five shipped apps declare theme changes as persistent subscriptions. Activity's
bus feed and Clock's ticks also use subscriptions; Todo's storage reads and
Clock's initial wall-clock query remain finite mount tasks. Counter and Todo
have actual native/Wasm repeated-reload coverage for complete state, including
count, unsaved draft and theme, with exactly one restarted theme subscription.
Pending user writes still prevent reload until they settle.

Mounted native and Wasm Counter tests dispatch AccessKit Click through the
host message boundary and observe the actual counter increment. Native
platform adapter smoke remains a separate operating-system gate.

Populated-store headless inspection now uses capability-name-scoped `Chip`
instances. Display capabilities preserve first declaration order and omit repeated
names without changing manifest bytes or artifact hashes. Actual `cargo ice
inspect` at 1240×800 passed with all five native packages, including a copied
Counter manifest with repeated clock/bus declarations; its JSON contains 11
uniquely scoped chip instances and the PNG was inspected. The catalog regression
first failed its ordered-list assertion without the display filter, then passed
with original manifest declarations, bytes and hash preserved.

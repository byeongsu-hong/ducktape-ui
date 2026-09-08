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
| Snapshot and approved reload | Native app-specific lifecycle | Shared owned-state snapshot and host replacement; shipped on-mount theme streams currently prevent quiescence | Preserve state and window; stale completion cannot replace live app |
| Fault containment | Application process | Native process deadline; wasm fuel, memory and epoch limits | Chaos runaway ends while another app remains usable |
| Accessibility | Native runtime semantics | Mapped host widgets preserve roles, names and semantic action targets | Identical mounted semantic actions and results on both backends; OS bridge smoke remains a separate platform gate |
| Combo boxes, images, gradients | Existing native widgets/options | Tree refusals remain | Real selection/image decode/gradient pixels, not codegen strings |
| Floating content | Native layout | Host-evaluated placement, scale, shadow and radius; bounded arithmetic | Actual bundled component tests placement, resize, pixels and translated clicks |
| Mounted components in lazy or host-conditional containers | Native state and lifetime machinery | Tree refusals remain | Mount/unmount lifetime, cancellation, fresh routes and retained state |
| Interaction styles and missing editor/slider/toggler/rule/scroll options | Native recipes/callbacks | Some declarative recipes exist; remaining Rust callbacks refused | Declarative recipes or host-owned semantics with equivalent states and pixels |
| Sensor reset key | Native reset semantics | Copied scalar keys re-arm the shared host sensor without replacing widget identity | Retained runtime and actual wasm fixture verify one fresh measurement per changed key |
| Recursive records, enums and opaque surface parameters | Typed Rust values | Copied scalar/list/option/record surface contract; other shapes refused | Owned declarative data or named host resources with validated lifetime and routes |
| General mouse, window, focus, close, drag/drop and IME subscriptions | Native runtime | No complete guest subscription contract | Real native events reach both backends without duplicate widget delivery |
| Window/system/image/font/reload/exit task effects | Native Iced actions | Guest poller currently logs and drops these action families | Host-owned action contracts; observable result or explicit diagnostic, never silent success |
| Widget commands and selectors | Full native operation boundary | Only documented Tree subset | Same selector scope and completion replies on mounted widgets |
| Clock, randomness and platform environment | OS APIs | Host clock/random; wasm direct platform calls can trap | Equivalent authored functionality via host contracts; native-only OS access is not portable |
| First-class authored Ice tests | Native harness | Guest crates disable their library test harness | Same authored scenario against both guest backends with meaningful route/render assertions |
| Restart persistence and multi-instance store | App-specific | Reload snapshot only; one instance per module | Explicit persistence and identity semantics, not merely a backend switch |

Raw Rust closures, native GPU programs, editor bindings and opaque handles are
not wire values. Functional parity requires declarative data or explicitly
registered host providers that retain native behavior. Their current rejection
is a tracked implementation gap where it blocks an authored user capability.

The shipped Counter's `on mount / stream every theme_changes()` is currently a
non-subscription Driver task and prevents snapshots indefinitely on both
backends. Existing `subscribe / run theme_changes()` can represent the stream
as a persistent subscription; its Result routing and both-backend reload need
a focused follow-up. The native backend's reload evidence uses the existing
versioned reload fixtures, and does not claim this shipped-app gap is fixed.

Mounted native and Wasm Counter tests dispatch AccessKit Click through the
host message boundary and observe the actual counter increment. Native
platform adapter smoke remains a separate operating-system gate.

Plain headless inspection of the populated store currently encounters duplicate
logical IDs for repeated `Chip` instances. Actual native catalog screenshots
can be captured from the running host, but authored inspection needs uniquely
keyed capability chips as a separate example/harness follow-up.

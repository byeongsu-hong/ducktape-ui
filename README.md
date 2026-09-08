# Ice

Ice is a statically checked declarative frontend language for
[iced](https://iced.rs/). It covers the pinned public, application-facing iced
surface through canonical Ice syntax and typed Rust boundaries. Humans write
the screen and interaction flow in compact `.ice` files; Rust keeps domain
rules, I/O, and custom platform code.

```text
.ice source -> parser -> semantic checker -> normalized HIR -> generated Rust -> iced
```

Normal builds have no source parser or general runtime interpreter.
`ui-lang-build` compiles app roots from `build.rs` into Cargo's `OUT_DIR`, and
`ui_lang::include_app!` includes that ordinary generated Rust. Development
restarts use the same ahead-of-time build path; applications never parse or
interpret Ice source at runtime.

The standard Cargo setup generates every app or daemon root below `src/ui`:

```toml
[dependencies]
iced = "=0.14.0"
ui-lang = "=0.1.0"
ui-lang-runtime = "=0.1.0"

[build-dependencies]
ui-lang-build = "=0.1.0"

[dev-dependencies]
# The headless Ice test driver; generated `#[cfg(test)]` code requires it.
ui-lang-runtime = { version = "=0.1.0", features = ["test-runtime"] }
```

```rust
// build.rs
fn main() {
    ui_lang_build::compile_dir("src/ui").expect("compile Ice sources");
}
```

```rust
ui_lang::include_app!("src/ui/tasks.ice");
```

Generated files live below `OUT_DIR/ui-lang-generated`, one file per source
fragment plus a root, published atomically under a manifest and removed by
`cargo clean`. Generated items suppress backend-only Rust and Clippy warnings;
generated errors stay visible and map back to their `.ice` lines.

## Taste of the language

```ice
app Tasks
  title "Ice Tasks"
  window
    size 960 720
    min-size 480 360
    position centered

use "extern/backend.ice"
use "theme.ice"
use "components/panel.ice"

state
  draft = ""
  loading = false

derived
  normalized_draft = trim(draft)
  can_submit = !loading && !empty(normalized_draft)

on submit
  let title = normalized_draft
  return if !can_submit
  loading = true
  run every create_task(title) -> created _ | failed _

view
  col w=fill h=fill p=24.0 gap=16.0 @bg-bg
    Panel title="Create task" #create-task
      row w=fill gap=12.0
        input "New task" #new-task <-> draft w=fill p=12.0 @bg-surface
        button "Add" disabled=!can_submit p=12.0 @bg-primary text-white -> submit
```

Handlers can end in an exhaustive match over a fieldless UI enum. Every variant
must appear exactly once, there is no wildcard, and only the selected arm is
evaluated:

```ice
on live_updated(next)
  match next.kind
    LiveKind.chat
      messages = fold_live_chat(messages, next)
    LiveKind.tip
      tip = next.tip
    LiveKind.ready
      ready = true
```

`derived` names a read-only computation over state. It may use deterministic
Ice built-ins or a declared `pure` extern. It is cached across frames and
recomputed only after a write to a state field it reads, a dependency the
compiler derives from the expression; it is not a signal, runtime dependency
graph, or state mirror that handlers must synchronize. `pure` is
a trusted Rust contract: the same arguments must produce the same value without
observable effects. Immediate `sync` externs may observe the environment,
perform an effect, or create retained identity, so Ice confines them to
top-level app state initializers and immediately evaluated handler expressions.
Explicit expressions producing ordinary cloneable Ice data in `run every`,
`run latest`, and `run replace` Future completion routes, and in `task`
statement completion routes, become owned snapshots when the statement
launches; both success and failure snapshots are materialized then, while `_`
is supplied only by the delivered completion.
They remain pure-only so an unused branch cannot perform an effect. Run a
`sync` extern in a preceding handler `let` and route that local when an
immediate value is needed. Stream, sip, flow, and native query route timing is
unchanged.
For example, `run every fetch(query) -> loaded(query, _)` captures `query` at
launch, not completion; that explicit value may come from state, a derived
value, a handler parameter, or a `let` local.

Every directly routed handler Future and stream names its delivery mode.
`run every` delivers every Future completion; `stream every` delivers every
item until that stream ends. Superseding Futures use `run latest lane=<name>`
to filter older completions without canceling their work, or
`run replace lane=<name>` to abort the prior Iced task. Superseding streams use
`stream replace lane=<name>`; `stream latest` is rejected because an obsolete
stream may never finish. Bare handler `run` and `stream` are not syntax.
Subscription `run` remains the distinct long-lived stream-source form, while task-flow
`from run call()`/`from stream call()` and corresponding `then` sources remain
Task adapters without a directly routed delivery mode.
`stream every` owns no compiler lane or handle: repeatedly starting a stream
that never terminates intentionally keeps every producer and its captures
alive. Extern-aware tooling therefore defaults stream completion to
`stream replace lane=<qualified-function-name>`; choose `every` explicitly only
when those independent lifetimes are intended.
A direct app, daemon, preset, or component handler statement whose immediate
intent supersedes an in-flight request can use `invalidate lane=<name>` before
its state changes. Invalidation advances the existing owner-scoped delivery
lane without starting work or declaring another lane, so any earlier Future
completion or stream item is stale; it leaves a `latest` Future running and
aborts the current `replace` task.
A lane is a static, finite qualified name owned by the app, the daemon shared
across its windows, or one component instance, so the same fully qualified name
deliberately joins calls from different handlers of that owner. Unaliased app
and preset fragments remain in the root namespace and can share root lanes.
Aliased imports cannot contribute app or preset handlers; lanes inside an
aliased component remain owned by each component instance. `latest` can retain
stale Futures and their captures until they finish. `replace` releases work
owned by the aborted task but cannot roll back effects already performed or
stop work the Rust backend detached. A stream `replace` lane keeps one handle
across all current-generation items and releases it only when the stream ends,
a replacement starts, the lane is invalidated, or its owner is dropped. Items
already queued before replacement are still discarded by the generation gate.
If an outer `abortable` suppresses a matching Future `run replace` completion
before update, that lane's one current handle remains until the next
replacement, explicit invalidation, or owner drop; it does not accumulate.
Per-owner lane bookkeeping is fixed by source-declared names; one lane cannot
mix Future and stream effects or `latest` and `replace` modes. Component-owner
count follows the existing retained/mounted lifetime contract. Component
handlers accept `stream replace` but reject `stream every`; handler streams are
also rejected under `abortable`, where two cancellation owners would conflict.

The punctuation has one job each:

- indentation is the tree;
- `@` starts checked semantic color, font-emphasis, and design-token utilities;
- `#name` is a scoped component/widget identity;
- `<->` is a two-way state or explicit `bind` component-prop binding;
- `->` routes a widget or async result to a handler;
- `_` is the payload supplied by that route.

`use` resolves relative to the importing file; imported declarations share one
checked app graph, and errors point into the fragment that caused them. Beyond
this taste — components with local state, events and slots, themes and
palettes, typed enums with exhaustive `match`, recipes, flexbox — the full
authoring surface is listed by `cargo ice schema`, its contract is
[`SPEC.md`](SPEC.md), and the [agent skill](#agent-skill) teaches it
interactively.

## Accessibility

Ice lowers its checked control surface into a deterministic AccessKit tree:

| Ice node | AccessKit role | Exported state |
| --- | --- | --- |
| `text` | `Label`, or `Heading` with `heading=1..6` | visible text value; a heading carries its level; `live=polite\|assertive` announces changes to it |
| `input` | `TextInput`, or `PasswordInput` when `secure=true` | current value, caret/selection position, and a caret a screen reader can move, for non-secure input; passwords never export their value |
| `button` | `Button` | name, description, optional checked/expanded state, disabled state, focus/click actions |
| `checkbox` | `CheckBox` | name, description, checked/disabled state, focus/click actions |
| `toggler` | `Switch` | name, description, checked/disabled state, focus/click actions |
| `slider` | `Slider` | default name, current value, numeric value/range/step, focus action, increment/decrement actions that run the change route with the next value |
| `progress` | `ProgressIndicator` | default name, current value, numeric value/range |
| `pick`, `combo` | `ComboBox` | placeholder name, selected value, focus action |
| `editor` | `MultilineTextInput` | placeholder/default name, current value, caret/selection position per line, a caret a screen reader can move, disabled state, focus action |
| labeled `image` | `Image` | name and description |
| any node inside a `scroll` | its own role | `ScrollIntoView` scrolls it into view through every scroll around it |
| the node under a `tooltip` | its own role | the tip's text as its description |

Visible labels are the default accessible names; explicit `label=` (and
`description=`) override them with checked `str` expressions. A button with
child content must declare `label=`, and an image without one is decorative.
Enabled controls use source-order Tab focus with a visible outline; Enter/Space
activate. Native screen-reader export covers single-window Linux, Windows, and
macOS applications through AccessKit's AT-SPI, UI Automation, and
NSAccessibility adapters (the Windows bootstrap holds the initial window hidden
until the UI Automation subclass is ready, preserving queue order; macOS
subclasses the AppKit view beside boot, on the main thread). On macOS a
`daemon` exports too, one adapter per window: each window attaches as it
opens, publishes a tree scoped to itself, keeps its own focus state, and takes
its adapter with it when it closes. Daemon export on Linux and Windows, exact
desktop bounds, rich text, and unlisted widgets are outside this Core
contract. `ui_lang_runtime::accessibility_settings()` reads the system
preferences no screen reader relays — Reduce Motion, Increase Contrast, and
whether a screen reader is running — from `NSWorkspace` on macOS and the
desktop portal on Linux; Windows reports a screen reader only once one
activates the tree.

## Installing a build

Tagged releases publish the showcase catalog as a macOS `.dmg`, a Debian
`.deb`, and a Windows `.msi`, each beside its SHA-256.

These builds are signed ad hoc, so the first launch takes one extra gesture:
on macOS, Control-click the app and choose **Open** instead of double-clicking
it, and on Windows choose **More info** then **Run anyway** at the SmartScreen
prompt. `sudo apt install ./showcase_*.deb` needs nothing extra. Building from
source needs nothing extra either — the prompt exists because the file was
downloaded, not because the application differs.

`cargo ice bundle -p PACKAGE` produces the same artifact for your own Ice
application, and signs and notarizes it when you supply a certificate; see
[`docs/tooling.md`](docs/tooling.md).
Bundle metadata `resources = ["../target/views"]` includes prebuilt module
views beside the installed executable on all three desktop platforms; paths
are relative to the package manifest (see [resources](docs/tooling.md#resources)).

## Examples

```bash
cargo run -p music-example     # macOS-Music-style flows, liquid-glass player
cargo run -p browser-example   # native CEF child inside an Ice shell (see examples/cef-browser)
cargo ice dev -p hotreload-example # side-by-side hot reload preview and Ice editor
cargo run -p markdown-example  # native Markdown notes app (see examples/markdown-editor/DESIGN.md)
cargo run -p terminal-example  # native PTY terminal component (see examples/terminal)
cargo run -p showcase          # the default component catalog (crates/ui-lang-components)
cargo run -p ice-starter       # the minimal copyable build/include/test path
cargo run -p candles-example   # native lightweight financial chart (see examples/candles)
cargo run -p trading-example   # live Hyperliquid markets, positions, and fills (see examples/trading)
cargo run -p tray-example      # the smallest macOS menu bar app: status item, live label, native menu
cargo run -p ai-chat-example   # streaming Codex chat: reasoning, tool calls, Markdown (see examples/ai-chat)
cargo run -p hotreload-example # edit `.ice` while it runs: `cargo ice dev` hot reload (see examples/hotreload)
cargo run -p two-windows-example # a daemon with two windows sharing one state
```

Importing a wallet needs a signed build: the Secure Enclave and the
data-protection keychain serve signed code only, so `cargo run` reads markets
and watches addresses but refuses to store a key it cannot seal. On macOS,
`scripts/sign-dev.sh -p trading-example` builds, signs and runs in one step —
see [what needs a Mac](examples/trading/README.md#what-needs-a-mac) for what
the signature takes.

Trade with real money on `cargo run --release -p trading-example`. The dev
profile optimizes that app itself (see `[profile.dev.package.trading-example]`),
which takes a third off a debug frame on the terminal's densest screen, but only
a release build carries that screen with room to spare at 60Hz.

The `tray` app-setting block puts an app in the macOS menu bar: codec-free
RGBA status icons selected by `when` guards, a live `label` expression beside
them, and a native `menu` whose rows are expressions and whose routed rows
call handlers. A row that carries an indented block is a submenu, to any
depth; it names no route, because the platform opens it rather than delivering
it. The platform owns the menu's opening, placement and dismissal, so a
program declares no window for it and carries no tray state.
A row's trailing `when` takes it out of the menu while false — removed, not
disabled, a submenu with the rows it owns — and puts it back in its place.
`expect tray label|icon|item|command` asserts what the program decided the
item should show, and `tray choose` runs a menu row the way the platform does
— a nested row by its text like any other, since the row table is flat at
every depth; both run on every platform. Other targets compile the same source
with the tray as a no-op.

`showcase` also exercises the 100k-row collection widgets behind typed extern
boundaries — no Core syntax involved: [`VirtualList`](crates/ui-lang-components/docs/virtual-list.md),
[`TreeView`](crates/ui-lang-components/docs/tree-view.md), [`DataGrid`](crates/ui-lang-components/docs/data-grid.md).

## First-class tests

Apps and components ship headless behavior tests written in Ice, discovered as
ordinary generated `#[test]` functions — no Rust wrapper or registration:

```ice
test counter_contract
  preset test
  viewport 320 240
  timeout 2s
  mount
    Counter #counter

  target root = #counter/root
  target increment = #counter/increment
  target result = #counter/result

  expect root.width ~= 240.0
  click increment
  expect text "1" within result
```

Tests drive the real generated program — layout, focus, IME, accessibility,
paint — through a semantic driver shared with Rust harnesses, and `capture`
writes PNG + JSON evidence. The full driver, determinism, and evidence
contract: [`docs/testing.md`](docs/testing.md).

## Agent skill

Install the Ice authoring skill with the open
[`skills`](https://github.com/vercel-labs/skills) CLI, then ask your agent to
`Use $design-ice-ui` when designing, writing, reviewing, or debugging `.ice`
files:

```bash
npx skills add byeongsu-hong/ducktape-ui --skill design-ice-ui
```

## Tooling

Wasm bundles normally use `wasm-opt` when it is on PATH. Pass `--no-wasm-opt`
to skip both optimizer discovery and execution. This option requires the sole
target `wasm32-unknown-unknown`; native and mixed-target requests are rejected.
It controls optimization only, not compiler versions or source-path remapping.

The repository ships a Cargo alias, so from the repo root:

```bash
cargo ice fmt [--check]   # normalize .ice indentation and blank lines
cargo ice check           # analyze every Ice graph, then cargo check
cargo ice test [NAME]     # source-mapped preflight, then cargo tests
cargo ice clippy          # clippy with generated errors mapped to .ice lines
cargo ice compat          # lockfile/manifest baseline + app tests
cargo ice expand FILE     # print the generated Rust for a root
cargo ice dev -p PACKAGE  # discover its Ice root, watch, reload, restart as needed; F12 debug metrics
cargo ice bundle -p PKG   # installable app for this host: .dmg, .deb, or .msi
cargo ice bundle -p PKG --target wasm32-unknown-unknown  # ice:view component for an app store
cargo ice bundle -p PKG --target wasm32-unknown-unknown --no-wasm-opt  # skip optimizer discovery and execution
cargo ice inspect FILE    # headless render -> PNG + JSON manifest
cargo ice inspect FILE --frames 60 [--release]  # per-phase frame cost + memo hits in the manifest
cargo ice inspect FILE --test FLOW --trace  # release interaction timings -> trace.json
cargo ice inspect FILE --fuzz interactions --seed 42 --steps 500  # deterministic semantic campaign
cargo ice diff A B        # compare two manifests + PNGs
cargo ice api FILE        # public-surface fingerprint; `api diff` classifies changes
cargo ice review FILE --trace  # tests, captures, and linked interaction traces
cargo ice schema          # machine-readable construct table (drives the LSP)
cargo ice lsp             # stdio LSP: diagnostics, completion, rename, code actions
```

Normal Cargo commands work too — the build script and proc macro are ordinary
build-graph members. Per-command manuals, the LSP client config and feature
inventory, analysis warnings, and the incremental `AnalysisDb` embedding API:
[`docs/tooling.md`](docs/tooling.md).

Core end-to-end cases are paired fixtures under
`crates/ui-lang-core/tests/cases/<format|diagnostic|warning|compile>/<case>/`
(`as-is.ice` input, `to-be.*` expectation) and are auto-discovered — a new case
needs no Rust test function.

## Fast dev loop for applications

Ice ships the compile-speed machinery by default: generated code is split per
source fragment (rustc hashes spans into incremental fingerprints — split, an
edit re-checks only its own fragment), render frames stay small through
outlining and ride a `stacker` red zone, and every app gets a generated
`__ice_view_fits_default_stack` contract (boot + presets render in a 4 MiB
thread) that keeps the default `opt-level = 0` dev profile safe. Daemon apps
whose view dispatches on window state should keep one app-side test seeding
real windows.

An application workspace adds one stanza — the Ice compiler runs as its build
script on every `.ice` edit, and build scripts default to opt-0:

```toml
[profile.dev.build-override]
opt-level = 2
```

Do not reach for `-Zincremental-ignore-spans`: on rustc 1.96 it deterministically
corrupts the incremental dep graph after a few edits. Measured on the ducktape
app (a 9.9 MB generated program), a real one-character `.ice` edit went from
12.6 s `cargo check` / ~50 s `cargo build` to **3.0 s / ~6 s** with this setup;
a fast linker (mold, per-target in `.cargo/config.toml`) trims the run loop
further.

## Status

Ice 2.0 Preview is an executable language candidate for the complete public,
application-facing surface of its pinned iced baseline. Common UI concepts use
direct declarative syntax; higher-order and custom native behavior uses checked
`Element`, `Task`, `Subscription`, style, component, and value boundaries.
Completeness belongs to that combined authoring surface and does not require a
dedicated keyword for every Rust method. Language revisions and Cargo package
versions are intentionally separate: the specification is the 2.0 Preview
candidate; the workspace packages use pre-1.0 SemVer `0.1.0`.

[`SPEC.md`](SPEC.md) defines the Core and backend boundary.
[`COVERAGE.md`](COVERAGE.md) is the versioned completeness contract for the
pinned iced 0.14 surface. A future iced baseline is not complete while an
application-facing row remains partial or missing; a gap may close through
canonical Ice syntax or a typed boundary.
[Feature evidence contracts](docs/feature-evidence-contracts.md) define what a
support claim has to show.
[`RELEASING.md`](RELEASING.md) defines lockstep versions and the generated-code
compatibility boundary.

Ice's wasm `tree` target emits host-rendered widget data. Extern widgets can
name host surfaces with copied scalar, list, optional and record arguments
and typed return events; see
[app-store](examples/app-store/README.md) for the runnable host and
[module-owned views](examples/app-store/module-views.md) for the packaging and
runtime gaps identified from Ducktape's existing screens.

Markdown also crosses as source and resolved settings through a named host
surface, with typed link events and source-preserving append/replacement.
The app-store registers the default `ice.markdown` renderer; custom viewers
use their declared provider name.

The wasm tree target preserves omitted border style fields: setting a border
colour does not square a checkbox or reset its native border width. See the
[app-store wire rendering notes](examples/app-store/README.md#wire-and-rendering).

Wasm views can execute clipboard read/write Tasks through a declared
`clipboard` capability, including the primary clipboard; the embedding host
performs the platform operation and returns reads to the guest's handler.

Tree-target `shader` calls use named host surfaces with typed data arguments
and event routes. The guest's dimensions bound the region; rendering remains
in the host. See [the wire rendering contract](examples/app-store/README.md#wire-and-rendering).

Checked Ice widget statements also cross to a wasm view's mounted host:
focus/query, input cursor/selection and scroll/snap operations stay confined
to that view. Arbitrary native widget Tasks and host window actions still
need separate boundaries. See the [app-store fixture](examples/app-store/README.md#mounted-widget-operation-fixture).


The example host also binds retained surfaces per guest instance. Its native
log timeline keeps view selection/scroll separate from the host session's
lifetime; see the [retained session fixture](examples/app-store/README.md#retained-host-session-fixture).


The example's [rich composer fixture](examples/app-store/README.md#rich-composer-fixture)
keeps native editing, IME and per-view history in the host while a wasm guest
receives semantic text/selection/submission notices and retains its draft.

The [native terminal fixture](examples/app-store/README.md#native-terminal-fixture)
binds a host-selected PTY to a wasm guest, retaining output while its view is
hidden and delivering bounded title/running/attention notices.

Tree-target responsive conditions run as bounded host container rules: resize
selects native children without calling wasm. The [responsive fixture](examples/app-store/README.md#responsive-rules-fixture)
checks nested sizes, guest thresholds, native input and instance isolation.

Tree-target canvas sends bounded declarative geometry to the host, including
solid paths, transforms, clips and guest-selected drawing branches. The
[canvas fixture](examples/app-store/README.md#declarative-canvas-fixture) checks
actual wasm pixels and local pointer routing.

Tree `stack`, `hover`, and modal `overlay` use native host layout and input;
the [layered fixture](examples/app-store/README.md#layered-layouts-fixture)
checks wasm routes, sizing, hover, focus blocking, and modal resource cleanup.

## Wasm text presentation

The tree target copies plain-text wrapping/shaping, named font descriptors,
relative/absolute line height, height, vertical alignment and grapheme tracking
to native host widgets. Hosts register trusted family names through
`ui_lang_runtime::view_tree::register_font_family` and load the matching font
bytes. Unregistered names use native sans-serif. Tracking uses a non-selectable
grapheme row and shares the host node budget. Boxes support maximum dimensions
and clipping; columns support `max-w=` and non-virtual rows/columns support `clip=`.
Linear clipping uses the native paint viewport and does not change native
pointer routing. The surrounding fill surface retains its native width while
the column bounds its child content. Buttons support padding utilities.
The Linear wire layout changed; these wire fields require
host and guest rebuilds together.

### Tree box shadows

Boxes copy `shadow=`, `shadow-x=`, `shadow-y=` and `shadow-blur=` into the
host's native container shadow. Signed offsets, blur and color alpha preserve
native paint outside the box without changing layout or pointer bounds.
Omitted fields retain native defaults. Boxes and tooltips share the shadow
value and sanitization: offsets are finite and bounded in either direction,
blur is nonnegative and bounded, and color channels are clamped. Other widget
shadow options retain their existing support boundaries.

The Container wire layout changed; rebuild hosts and guest bundles together.

### Tree button accessibility

Tree buttons preserve optional `checked=`, `expanded=` and `description=`.
The host forwards them to the native accessible wrapper: `false` is distinct
from omission, and descriptions share the frame text budget. This changes the
Button wire layout; rebuild hosts and guests together.
Native AccessKit snapshot tests cover true/false/absent states;
the widget wasm fixture verifies copied state after a native focus-button click.

Tree buttons also copy resolved recipes and all eight native presets, including
label typography, disabled colors/opacity, padding and keyboard focus rings.
Named label fonts use the same trusted host registry as plain text. Rebuild
hosts and guests together for the ButtonStyle wire change.

### Tree wrapping rows and columns

Tree `row wrap` and `col wrap` use the native host wrapping widgets. They carry
`wrap-gap=` and `wrap-align=` alongside ordinary spacing, dimensions, padding
and child alignment. The host reflows when its available size changes; guests
provide children and copied values without measuring pixels. Optional wrapping
settings distinguish an ordinary layout from a wrapping layout with defaults.
Inter-line spacing uses the existing wire and child-count spacing limits.
Rebuild hosts and guests together for the Linear wire field.

### Tree tooltips

Tree tooltips use the host's native overlay, with copied position, gap, padding,
delay and viewport snapping. Native container presets and concrete solid
background/text/border/shadow/snap values cross; Rust style callbacks and
gradients remain refused. The tip's visible text supplies the first eligible
content node's accessible description, preserving an explicit description.
Tooltip children share tree limits, numeric style values are sanitized and
delay is capped at 60 seconds. Rebuild hosts and guests together for the wire
variant.

### Tree SVG button colors

Memory SVG children support `color=inherit` through the nearest host button's
resolved text color, including hover on button padding, pressed and disabled
states. Explicit SVG palette colors stay independent. The guest copies only
the inheritance flag; the host owns the existing native button ink cell.
Rust SVG style callbacks remain refused. Rebuild host and guests together
for the SVG wire field.

### Tree input presentation

Tree inputs preserve native label semantics: the positional string (or
`label=` override) is the accessible name, while `hint=` supplies the visible
placeholder. Description and disabled state reach the native accessibility
wrapper; disabled inputs do not produce edits or submit events.

Padding, text size, relative line height, horizontal alignment and named or
default/mono fonts are copied. Absent typography options use guest app defaults.
Input recipes preserve utility padding and fill width, with explicit options
winning, and utility colors/borders precede active and status overrides. Focus
ring color applies after active and before explicit focused/focused-hovered
styles. Text metadata shares frame budgets and layout numbers are bounded.
Rebuild hosts and guests together for InputOptions and the extended InputStyle.
Secret handles, input icons, paste routes and Rust style callbacks remain
refused.

Tree editors copy size, padding, relative/absolute line height, wrapping, fonts
and declarative status faces to the native host editor. Caret, selection and
undo remain host-owned; disabled editors produce no edits. Rebuild hosts and
guests together for `EditorOptions`. Native editor callbacks remain refused.
The [bundled fixture](examples/app-store/tests/editor-guest/src/ui/app.ice)
exercises layout, selection colors and guest-bound edits.

Tree module views support keyed row identity and `virtual-row=` columns. The
host preserves row input state and focus across reordering and owns viewport
mounting; see [the bundled evidence](examples/app-store/README.md#keyed-and-virtual-rows).
Tree `lazy` caches guest subtrees and callable routes, while the host memoizes
native views/layout and owns their unmount lifetime. See [lazy module views](examples/app-store/README.md#lazy-module-views).
Tree `flex` copies layout and item rules to the existing native engine, including
wrapping, sizing and order. See [flex module views](examples/app-store/README.md#flex-module-views).


Tree `pin` copies widget-local offsets and optional dimensions to the native host.
See [pinned module children](examples/app-store/README.md#pinned-module-children).

Tree `rich-text` carries styled spans and link routes to a single native paragraph.
See [rich text module views](examples/app-store/README.md#rich-text-module-views).

Tree mounted components initialize once per appearance and cancel their work on
removal. See [component module lifetimes](examples/app-store/README.md#mounted-component-module-views).

Tree `qr` copies UTF-8 or byte payloads and encoding/style options to the native
host. See [QR module views](examples/app-store/README.md#qr-module-views).

Native `log_timeline` and `virtual_list`, including their themed components
wrappers, accept temporary row slices when the row
closure returns owned elements, allowing static host surfaces without retaining
the source rows. See [the log surface](examples/app-store/README.md#retained-host-session-fixture).

Tree keyboard subscriptions receive host-delivered press/release/modifier events
with native captured/ignored status. Hosts initialize guests with their macOS
modifier convention; see [keyboard module views](examples/app-store/README.md#keyboard-module-views).

Tree `task widget scroll-to-key` reveals a keyed virtual row inside the requesting
guest, using the host’s native measured-row positioning.

The app-store host polls local artifacts asynchronously and replaces an explicitly
approved rebuild in its existing window, preserving compatible guest state and
keyed native focus/scroll. Failed or stale candidates retain the old instance and
consent pin. See [approved host replacement](examples/app-store/README.md#approved-host-replacement).

Tree guests export bounded `snapshot` / `restore` state transfer, including
retained/mounted components, without replaying boot. See
[guest snapshots](examples/app-store/README.md#guest-state-snapshots) for supported
state, quiescence and host integration requirements.

### Tree scroll offsets

A Tree `scroll` supports `scroll=` with the native four-argument route:
absolute X/Y offsets in logical pixels and anchor-relative X/Y fractions.
The host emits the route only when the native viewport changes, including
end-anchored scrollback. Offsets are local to the scrollable; window coordinates
are not exposed. Missing routes produce no guest events. The full `viewport=`
route and scroll status styles remain refused. Hosts and guests must rebuild
for the added Scroll field and ScrollOffset event.

Tree pick lists carry native text metrics, menu styling, dynamic handles and
open/close routes. See [pick module options](examples/app-store/README.md#pick-module-options).

### Guest preferred window size

Tree compilation carries the primary `window size` into the versioned
`ice.manifest.v1` custom section without another `export_app!` argument. The
strict five fields are version, name, description, comma-terminated capabilities
(or empty), and `none` or `width,height`. There is no legacy fallback.
The app-store catalog accepts finite positive f32 dimensions up to 8192 logical
pixels; Tree rejects declarations outside that range at the `size` source line,
without changing native-target limits. Saved placement takes precedence over the declaration, then the host
560×420 default. The initial native open uses the final size; restart preserves
the existing window. Hosts and guest modules must rebuild together.

Evidence: Core `tree_manifest_preferred_window_size_is_static_and_preserves_f32`,
host `manifest_format_and_preferred_size_are_strict`,
`preferred_window_settings_choose_saved_declared_then_default`, and the actual
wasm `bundled_preferred_size_reaches_initial_native_open` test documented in
[the app-store guide](examples/app-store/README.md#preferred-window-size-evidence).


Hosts can consume the Ice view contract without a graphics dependency through
`ui-lang-wire`. `WIT` exposes the canonical text; `with_view_wit!(callback)`
passes that same literal to a local macro for Wasmtime or wit-bindgen's `inline`
option. `export_app!` retains its four arguments and the same `ice:view` ABI.
`manifest::Manifest::parse` reads the strict five-line metadata;
`manifest::PreferredSize::dimensions` returns `[f32; 2]`. The optional `manifest`
feature adds `manifest::read_manifest` for extracting exactly one manifest from
component bytes, including nested core modules. Neither the default dependency
set nor this feature enables iced or a renderer.

Metadata extraction does not validate executable code. Hosts can compile the
component, resolve imports with `Linker::instantiate_pre`, and check exports
with the generated `ViewPre::new` without creating a store or running the guest.
This checks required ABI types; it is not proof that instantiation, init, or boot
will succeed. Import policy remains the host's responsibility.

### Tree float placement

Tree `float` supports host-evaluated original/viewport translation arithmetic,
scale, shadow and per-corner radius. Resizing recomputes placement without
sending geometry to the guest. Geometry-dependent Rust calls remain E190; each
axis permits at most 64 arithmetic operations. Rebuild hosts and guests together
for the new wire node. The app-store float fixture checks real wasm painting at
two viewport sizes and clicks at the translated position.

Tree keyed columns and lazy boundaries preserve authored `#id` directly on
the wire node, including nested lazy and virtual keyed rows. Their descendants
retain that scope for guest test selectors and widget operations. Identified
Tree structures emit no native Iced container wrapper.

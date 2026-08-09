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
  run create_task(title) -> created _ | failed _

view
  col w=fill h=fill p=24.0 gap=16.0 @bg-bg
    Panel title="Create task" #create-task
      row w=fill gap=12.0
        input "New task" #new-task <-> draft w=fill p=12.0 @bg-surface
        button "Add" disabled=!can_submit p=12.0 @bg-primary text-white -> submit
```

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
authoring surface is defined in [`SPEC.md`](SPEC.md), and the
[agent skill](#agent-skill) teaches it interactively.

## Accessibility

Ice lowers its checked control surface into a deterministic AccessKit tree:

| Ice node | AccessKit role | Exported state |
| --- | --- | --- |
| `text` | `Label` | visible text value |
| `input` | `TextInput`, or `PasswordInput` when `secure=true` | current value for non-secure input; passwords never export their value |
| `button` | `Button` | name, description, disabled state, focus/click actions |
| `checkbox` | `CheckBox` | name, description, checked/disabled state, focus/click actions |
| `toggler` | `Switch` | name, description, checked/disabled state, focus/click actions |
| `slider` | `Slider` | default name, current value, focus action |
| `progress` | `ProgressIndicator` | default name and current value |
| `pick`, `combo` | `ComboBox` | placeholder name, selected value, focus action |
| `editor` | `MultilineTextInput` | placeholder/default name, current value, disabled state, focus action |
| labeled `image` | `Image` | name and description |

Visible labels are the default accessible names; explicit `label=` (and
`description=`) override them with checked `str` expressions. A button with
child content must declare `label=`, and an image without one is decorative.
Enabled controls use source-order Tab focus with a visible outline; Enter/Space
activate. Native screen-reader export covers single-window Linux and Windows
applications through AccessKit's AT-SPI and UI Automation adapters (the Windows
bootstrap holds the initial window hidden until the UI Automation subclass is
ready, preserving queue order). Other targets, daemon and multi-window
adapters, exact desktop bounds, rich text, and unlisted widgets are outside
this Core contract.

## Examples

```bash
cargo run -p iced-app          # the reference task app + native-surface fixtures
cargo run -p music-example     # macOS-Music-style flows, liquid-glass player
cargo run -p browser-example   # native CEF child inside an Ice shell (see examples/cef-browser)
cargo ice dev examples/hotreload/src/ui/app.ice -- -p hotreload-example # side-by-side hot reload preview and Ice editor
cargo run -p markdown-example  # native Markdown editor (see examples/markdown-editor/DESIGN.md)
cargo run -p terminal-example  # native PTY terminal component (see examples/terminal)
cargo run -p showcase          # the ducktape-ui component catalog (crates/ui)
cargo run -p ice-starter       # the minimal copyable build/include/test path
cargo run -p candles-example   # native lightweight financial chart (see examples/candles)
cargo run -p trading-example   # live Hyperliquid markets, positions, and fills (see examples/trading)
cargo run -p tray-example      # the smallest macOS menu bar app: status item, live label, popover
```

The `tray` app-setting block puts an app in the macOS menu bar: a
codec-free RGBA status icon, a live `label` expression beside it, and — for
daemons — a `popover` window toggled under the icon, which dismisses itself
when it loses focus. The read-only `popover` binding tells a view which of its
windows it is drawing, and `tray click` presses the status item from a test. Other targets compile
the same source with the tray as a no-op; see `SPEC.md` for the mapping.

`showcase` also exercises the 100k-row collection widgets behind typed extern
boundaries — no Core syntax involved: [`VirtualList`](crates/ui/docs/virtual-list.md),
[`TreeView`](crates/ui/docs/tree-view.md), [`DataGrid`](crates/ui/docs/data-grid.md).

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

The repository ships a Cargo alias, so from the repo root:

```bash
cargo ice fmt [--check]   # normalize .ice indentation and blank lines
cargo ice check           # analyze every Ice graph, then cargo check
cargo ice test [NAME]     # source-mapped preflight, then cargo tests
cargo ice clippy          # clippy with generated errors mapped to .ice lines
cargo ice compat          # lockfile/manifest baseline + app tests
cargo ice expand FILE     # print the generated Rust for a root
cargo ice dev FILE        # watch + rebuild + zero-downtime process swap
cargo ice inspect FILE    # headless render -> PNG + JSON manifest
cargo ice diff A B        # compare two manifests + PNGs
cargo ice api FILE        # public-surface fingerprint; `api diff` classifies changes
cargo ice review FILE     # run Ice tests into one JSON/HTML evidence bundle
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
[`docs/decisions`](docs/decisions) records accepted boundaries (`Accepted` is a
normative direction, not a support claim), with the matching
[feature evidence contracts](docs/feature-evidence-contracts.md).
[`RELEASING.md`](RELEASING.md) defines lockstep versions and the generated-code
compatibility boundary.

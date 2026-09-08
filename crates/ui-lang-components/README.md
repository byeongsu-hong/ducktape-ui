# ui-lang-components

Default, composable UI components for [Ice](https://github.com/byeongsu-hong/ui-lang-components) and [iced](https://github.com/iced-rs/iced).

Ice is the canonical application authoring surface: `.ice` owns layout, state, routes, styles, and accessibility. The feature-gated Rust modules remain the typed native boundary for retained widgets whose behavior is intentionally lower-level than Ice.

Core controls remain native Ice nodes so its accessibility tree stays intact;
`ui-lang-components` supplies their checked semantic recipes. Widgets with opaque
retained state cross typed `extern` boundaries instead.

The workspace follows that split:

- [`../../examples/showcase/src/ui/app.ice`](../../examples/showcase/src/ui/app.ice) is the complete showcase application.
- [`src/ice/components.ice`](src/ice/components.ice) contains the reusable Ice-native composition.
- [`../../examples/showcase/src/adapters.rs`](../../examples/showcase/src/adapters.rs) and [`../../examples/showcase/src/ui/extern/adapters.ice`](../../examples/showcase/src/ui/extern/adapters.ice) contain the catalog-only retained-widget adapters.
- [`../../examples/showcase/src/main.rs`](../../examples/showcase/src/main.rs) only compiles and runs the Ice app.

## Ice interface in this workspace

Ice `use` paths are relative to the importing `.ice` file; Cargo packages do
not currently provide package-aware Ice imports. The workspace showcase uses
the checked source interface directly:

```ice
app App

use "../../../../crates/ui-lang-components/src/ice/default.ice"

state
  email = ""

on save

view
  col @page
    PageHeader title="Account" description="Manage the address used for product updates."
    Panel title="Profile" description="Fields and actions use the shared component contract."
      Field label="Email" description="We only use this address for product updates."
        input "Email" <-> email hint="you@example.com" @control
      button "Save" @primary_action -> save
```

Outside this source workspace, vendor the `src/ice` directory at a stable
application-relative path or use the Rust library below. The workspace entry
file [`src/ice/default.ice`](src/ice/default.ice) imports only the default
theme, reusable recipes, the shared Ice components in
[`src/ice/components.ice`](src/ice/components.ice), and the retained-widget
frames in [`src/ice/virtual-list.ice`](src/ice/virtual-list.ice) and
[`src/ice/tree-view.ice`](src/ice/tree-view.ice). Visual
variants use checked compound names such as `Alert.Success`, `Badge.Warning`,
and `Typography.Caption`; there are no free-form variant strings that can silently
render an empty component. Its Ice tokens are checked against the retained Rust
`LIGHT` palette, so the default path needs no repeated accent argument or
parallel control-style callbacks. Custom retained themes use the Rust component
API, where callers pass a complete `Theme`; the Ice interface intentionally
does not expose partial accent-only theming. Applications that need retained
widgets define a small typed `extern` boundary for their own data and events;
the showcase adapter interface is not part of the default application surface.

Large fixed-row collections use the feature-gated
[`VirtualList`](docs/virtual-list.md). Its state/event API lives in
`ui-lang-runtime`, `ui-lang-components` applies semantic theme tokens, and
`VirtualList.Frame` provides the reusable Ice composition around an
application-owned typed extern. [`TreeView`](docs/tree-view.md) adds retained
hierarchy, expansion, lazy loading, rename state, and tree accessibility on the
same fixed-row engine; `TreeView.Frame` is its Ice composition. Give either
extern slot a bounded height outside any vertical scrollable ancestor; the
retained collection owns vertical scrolling. These are intentionally runtime
widgets, not new Core syntax.

Append-only build output, agent traces, and service logs use the feature-gated
[`LogTimeline`](docs/log-timeline.md). It composes `VirtualListState` for the
same bounded fixed-row rendering, stable keys, selection, keyboard, headless,
and AccessKit behavior, then adds default tail-follow, pause-on-history
navigation, unread append counts, and explicit resume. It does not replace
`MessageScroller`: transcripts retain variable-height measurement, message
anchors, prepend restoration, and jump-control behavior.

## Form defaults and customization

Import `src/ice/default.ice` and declare a form with application-owned state:

```ice
Form
  FormSection title="Profile"
    TextField label="Display name" value<->name
```

`Form(max_width=640.0, padding=24.0)` owns vertical scrolling and centers its
bounded content. `FormSection(title, description="", padding=20.0, radius=11.0)`
provides a section surface and heading. Each accepts one content root; use a
`col w=fill gap=20.0` for multiple fields. Text wraps rather than using fixed
row heights. Do not wrap Form in another vertical scroll container.

`TextField(label, bind value, description="", error="", placeholder="",
disabled=false, secure=false, padding=11.0, radius=10.0)` supplies the native
input and default control recipe. Change just geometry when needed:

```ice
TextField label="Workspace" value<->workspace radius=4.0 padding=14.0
```

Colors continue to follow the existing semantic palette. Geometry changes do
not replace binding, focus or accessibility. Errors are visible wrapping text
with polite live announcements; callers supply validation policy. The input's
accessible name is its label and its description is the supplied help text.

For a different control or a complete input-style override, use
`Field(label, description="", error="")` and its content slot. The caller
sets that custom control's label and styles. Empty help/error strings add no
text node or spacer row. Field's `root`, `label`, `description`, and `error`
IDs and TextField's `field/root/input` path support semantic inspection.

The [settings example](../../examples/settings/README.md) demonstrates default
and custom geometry, a checkbox slot, narrow layouts and validation feedback.
These are reusable Ice components, not new language keywords or automatic
platform-native controls.

## Optional header descriptions

`PageHeader(title, description="")` and `Panel(title, description="")` omit the
description node and its spacing when the description is empty. A title-only
header therefore occupies only its title's natural height. Titles and supplied
descriptions fill the available content width and wrap at word boundaries;
they grow vertically instead of requiring a fixed header height.

```ice
PageHeader title="Settings"
Panel title="Profile"
  text "Your profile information" @body
```

Both expose `root/title` and, when present, `root/description` for semantic
inspection. Panel retains its existing content slot and padding/section spacing.

## Rust library quick start

Each component remains individually feature-gated, and enabling one also enables its internal component dependencies.

```toml
[dependencies]
ui-lang-components = { git = "https://github.com/byeongsu-hong/ui-lang-components", features = ["button", "input", "card"] }
iced = "=0.14.0"
```

`ui-lang-components` does not silently choose an iced renderer or platform. Consumers
that use it without a separate default-featured `iced` dependency opt into
`wgpu` or `tiny-skia` and then `x11` or `wayland`; either standalone native
platform feature includes iced's minimal thread-pool executor. The executor is
also available as a direct `thread-pool` passthrough, while wasm consumers
leave the native platform features disabled and select the renderer appropriate
to their target.

```rust
use ui_lang_components::ui::{
    button::{Button, ButtonVariant},
    theme::{LIGHT, SHADCN_LIGHT},
};
use iced::widget::{row, text};

#[derive(Debug, Clone)]
enum Message {
    Save,
}

fn view() -> iced::Element<'static, Message> {
    Button::new(row![text("★"), text("Save")].spacing(8), &LIGHT)
        .variant(ButtonVariant::Default)
        .on_press(Message::Save)
        .into()
}
```

`Button::new` accepts any iced element; `button("Save", &theme)` is the text-label convenience. Its builder also exposes `height`, `padding`, and a native iced `style` callback. The same pattern is used across the library: application state and messages stay with the caller, composable components accept caller-owned content slots, and every visual component receives a `Theme`.

All theme fields are public, so an application can swap a complete visual
profile or derive its own tokens without copying library source. `LIGHT` and
`DARK` retain the approved Ducktape contract; `SHADCN_LIGHT` and
`SHADCN_DARK` apply a neutral shadcn-style palette, radius, spacing,
typography, and control geometry to the same component APIs.

```rust
let mut theme = SHADCN_LIGHT;
theme.radius.button = 4.0;
theme.spacing.lg = 20.0;
```

Radius roles are named `chip`, `row`, `button`, `card`, and `modal`; control
metrics cover button sizes and padding plus input padding; typography
uses the design roles from `display` through `badge` instead of generic size
aliases. `Theme::glass` exposes the exact thin, regular, and sheet alpha colors
without claiming a blur implementation, while `Theme::elevation` provides the
popover, toast, modal, and two-layer application-window shadows.

The source success color on its tint measures 2.86:1, so status labels use the
neutral foreground and keep success as a redundant dot/icon. Avatar initials
remain text: the default `#4f4d47` foreground clears 4.5:1 against the avatar
fill.

Native typography roles name the canonical Geist and Geist Mono families and
encode their exact sizes and weights. This crate does not bundle font assets:
the consuming application must preload both families through iced's application
`font` settings before rendering these roles. Ice applications likewise load
the font bytes at the app boundary; their default font supplies Geist while the
shared `font-mono` recipes select the loaded monospace face.

## Custom content

Convenience APIs keep the stock shadcn-style presentation. Every component that otherwise owns fixed visible UI also exposes a caller-rendered path:

- segmented controls, pagination, and carousel controls/indicators use their `*_with_content` functions
- Select and Date Picker replace their full trigger with `.trigger(...)`
- Calendar localizes labels with `CalendarLabels` and replaces navigation content with `.controls(...)`
- Input OTP replaces group separators with `.separator(...)`
- Alert Dialog accepts full cancel/action elements through `alert_dialog_with_controls`
- Command replaces empty and result rows through `.empty_content(...)` and `.item_content(...)`
- Message Scroller uses `controlled_message_scroller_with_end_content` for its jump control
- Sonner uses `sonner_with_content`; each `SonnerControl` supplies a stable ID and message for fully custom controls, plus `.content(...)` for the stock control treatment

Text-only convenience arguments remain customizable strings, while structural content is passed as `iced::Element`. Existing default functions delegate to these composable paths, so adopting the library does not require source copies.

Use the `full` feature for the complete catalog. The individual feature names and their transitive relationships are listed in [`Cargo.toml`](Cargo.toml). Full shadcn/ui behavior coverage is tracked in [the parity matrix](docs/parity.md).

## Showcase

```bash
cargo run -p showcase
```

The showcase compiles the same shared components imported by applications and
crosses typed Rust boundaries only for retained native behavior such as menus,
charts, modal focus, transcript measurement, and resizable panels. Its local
Ice file contains demos, not a second component library. The full Rust feature
set is compiled and exercised by the test suite. The same
[`app.ice`](../../examples/showcase/src/ui/app.ice) graph declares a
first-class `test app_behavior`: it boots the `test` preset, selects rendered
controls by scoped ID, drives click, typing, keys, and checked handler dispatch,
then asserts generated state and visible content. `cargo test -p showcase`
discovers that generated test normally; `cargo ice test` checks every Ice graph
before running the workspace tests. No separate case format, Rust registration,
or image snapshot is required.

## Development

```bash
cargo ice fmt --check
cargo ice test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo check -p ui-lang-components --no-default-features --features button,x11
cargo check -p ui-lang-components --target wasm32-unknown-unknown --no-default-features --features button
```

## Action rows at narrow widths

When actions should retain their natural label widths and move to the next line
as space runs out, put wrapping on the row that directly owns the buttons:

```ice
Card.Footer
  row wrap w=fill gap=8.0 wrap-gap=8.0
    button "Discard all changes" @secondary_action -> cancel
    button "Save workspace settings" @primary_action -> save
```

At 280px the demonstrated card places these actions on separate lines; at 640px
they share one line. Wrapping preserves source order for keyboard navigation.
The [compiling example and tests](../../examples/showcase/tests/cases/ui/action_layout.ice)
check placement, label containment, Tab/Enter and pointer activation.

The slot accepts one root: the caller owns this inner row and its reflow policy.
Wrapping only an outer Card.Footer, Dialog.Actions or ButtonGroup container does
not rearrange descendants inside the caller's row. This pattern demonstrates
explicit layout; those components do not yet choose that policy automatically.

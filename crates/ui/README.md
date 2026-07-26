# ducktape-ui

Reusable Ducktape Design System components for
[Ice](https://github.com/byeongsu-hong/ducktape-ui-lang) and
[iced](https://github.com/iced-rs/iced).

Ice is the application authoring surface. `.ice` owns layout, state, routes,
styles, and accessibility. Feature-gated Rust modules provide retained widgets
whose behavior needs typed native state.

## Source boundary

- [`src/ice/default.ice`](src/ice/default.ice) is the public Ice interface. It
  imports the design tokens, recipes, and shared components.
- [`src/ice/components.ice`](src/ice/components.ice) contains parameterized
  components only. It must not contain product copy, demo state, endpoints,
  hashes, people, or other sample records.
- [`../../examples/showcase/src/ui/components.ice`](../../examples/showcase/src/ui/components.ice)
  contains the concrete reference blocks and sample content.
- [`../../examples/showcase/src/ui/showcase.ice`](../../examples/showcase/src/ui/showcase.ice)
  assembles the twelve-section catalog.
- [`src/ui`](src/ui) contains feature-gated retained iced controls.

The component and showcase split is tracked in the
[equivalence ledger](docs/equivalence.md).

## Ice quick start

```toml
[dependencies]
ducktape-ui = { git = "https://github.com/byeongsu-hong/ducktape-ui-lang" }
iced = "=0.14.0"
ui-lang = { git = "https://github.com/byeongsu-hong/ducktape-ui-lang", version = "=0.1.0" }
ui-lang-runtime = { git = "https://github.com/byeongsu-hong/ducktape-ui-lang", version = "=0.1.0" }
```

```rust
ui_lang::include_app!("src/app.ice");

fn main() -> iced::Result {
    App::run()
}
```

```ice
app App

use "path/to/ducktape-ui/src/ice/default.ice"

state
  email = ""

on save

view
  col @page
    PageHeader mark="D" title="Account" edition="/ Settings" description_before="Manage the " description_emphasis="address" description_after=" used for product updates."
      HeaderTag label="workspace settings"
    Panel title="Profile" description="Fields and actions use the shared component contract."
      Field label="Email" description="We only use this address for product updates."
        input "Email" <-> email hint="you@example.com" @control
      button "Save" @primary_action -> save
```

Compound visual variants use checked names such as `Alert.Success`,
`Badge.Pending`, and `Typography.Muted`; there are no free-form variant strings.
Applications own all concrete values and events.

## Rust library quick start

Each retained component is individually feature-gated. Enabling one also
enables its internal dependencies.

```toml
[dependencies]
ducktape-ui = { git = "https://github.com/byeongsu-hong/ducktape-ui-lang", features = ["button"] }
iced = "=0.14.0"
```

```rust
use ducktape_ui::ui::{
    button::{Button, ButtonVariant},
    theme::LIGHT,
};
use iced::widget::{row, text};

#[derive(Debug, Clone)]
enum Message {
    Save,
}

fn view() -> iced::Element<'static, Message> {
    Button::new(row![text("Save")], &LIGHT)
        .variant(ButtonVariant::Default)
        .on_press(Message::Save)
        .into()
}
```

All theme fields are public, so an application can derive its tokens directly:

```rust
let mut theme = LIGHT;
theme.radius.md = 4.0;
theme.spacing.lg = 20.0;
```

Use the `full` feature for every retained control. Feature names and their
dependencies are listed in [`Cargo.toml`](Cargo.toml).

## Showcase

```bash
cargo run -p showcase
```

The showcase uses the same shared tokens and parameterized components as an
application. Its local Ice file owns every concrete demo value. Behavior cases
live under `examples/showcase/tests/cases/`; scrolled headless snapshots cover
the complete catalog.

## Development

Run all work from a dedicated Git worktree.

```bash
cargo ice fmt --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo check -p ducktape-ui --no-default-features --features button
```

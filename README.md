# ducktape-ui

Default, composable UI components for [Ice](https://github.com/byeongsu-hong/ducktape-ui-lang) and [iced](https://github.com/iced-rs/iced).

Ice is the canonical application authoring surface: `.ice` owns layout, state, routes, styles, and accessibility. The feature-gated Rust modules remain the typed native boundary for retained widgets whose behavior is intentionally lower-level than Ice.

Core controls remain native Ice nodes so its accessibility tree stays intact; `ducktape-ui` supplies their checked style callbacks. Widgets with opaque retained state cross typed `extern` boundaries instead.

The repository itself follows that split:

- [`src/showcase.ice`](src/showcase.ice) is the complete showcase application.
- [`src/ice/components.ice`](src/ice/components.ice) contains reusable Ice-native composition.
- [`src/ice.rs`](src/ice.rs) exposes the native widgets Ice actually needs through checked `extern` adapters.
- [`src/main.rs`](src/main.rs) only compiles and runs the Ice app.

## Ice quick start

```toml
[dependencies]
ducktape-ui = { git = "https://github.com/byeongsu-hong/ducktape-ui", features = ["ice"] }
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

extern ducktape_ui::ice
  button-style button_style(variant:str, accent:color)

theme
  bg #f8fafc
  fg #0f172a
  primary #2563eb
  danger #dc2626

on save

state
  accent:color = color.rgb8(37, 99, 235)

view
  button "Save" height=36.0 padding=8.0 style=button_style("default", accent) -> save
```

## Rust library quick start

Each component remains individually feature-gated, and enabling one also enables its internal component dependencies.

```toml
[dependencies]
ducktape-ui = { git = "https://github.com/byeongsu-hong/ducktape-ui", features = ["button", "input", "card"] }
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
    Button::new(row![text("★"), text("Save")].spacing(8), &LIGHT)
        .variant(ButtonVariant::Default)
        .on_press(Message::Save)
        .into()
}
```

`Button::new` accepts any iced element; `button("Save", &theme)` is the text-label convenience. Its builder also exposes `height`, `padding`, and a native iced `style` callback. The same pattern is used across the library: application state and messages stay with the caller, composable components accept caller-owned content slots, and every visual component receives a `Theme`.

All theme fields are public, so an application can derive its own tokens without copying library source:

```rust
let mut theme = LIGHT;
theme.radius.md = 4.0;
theme.spacing.lg = 20.0;
```

## Custom content

Convenience APIs keep the stock shadcn-style presentation. Every component that otherwise owns fixed visible UI also exposes a caller-rendered path:

- segmented controls, pagination, and carousel controls/indicators use their `*_with_content` functions
- Select and Date Picker replace their full trigger with `.trigger(...)`
- Calendar localizes labels with `CalendarLabels` and replaces navigation content with `.controls(...)`; the compatibility API is `calendar_with_content`
- Input OTP replaces group separators with `.separator(...)`
- Alert Dialog accepts full cancel/action elements through `alert_dialog_with_controls`
- Command replaces empty and result rows through `.empty_content(...)` and `.item_content(...)`
- Message Scroller uses `controlled_message_scroller_with_end_content` for its jump control
- Sonner uses `sonner_with_content`; each `SonnerControl` supplies a stable ID and message for fully custom controls, plus `.content(...)` for the stock control treatment

Text-only convenience arguments remain customizable strings, while structural content is passed as `iced::Element`. Existing default functions delegate to these composable paths, so adopting the library does not require source copies.

Use the `full` feature for the complete catalog. The individual feature names and their transitive relationships are listed in [`Cargo.toml`](Cargo.toml). Full shadcn/ui behavior coverage is tracked in [the parity matrix](docs/parity.md).

## Showcase

```bash
cargo run --features showcase --bin showcase
```

The showcase compiles the full Ice graph and crosses typed Rust boundaries only for retained native behavior such as menus, charts, modal focus, transcript measurement, and resizable panels. The full Rust feature set is compiled and exercised by the test suite.

## Development

```bash
cargo ice fmt --check
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --no-default-features --features button
```

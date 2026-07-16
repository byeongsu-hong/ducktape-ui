# ducktape-ui

The source-owned UI toolkit for [iced](https://github.com/iced-rs/iced), built first for Ducktape and reusable by any iced application.

Like shadcn/ui, ducktape-ui copies readable component source into your project. There is no runtime `ducktape-ui` widget dependency: you own the files, edit the tokens, and keep iced's native state and message model.

## Quick start

```bash
cargo install --path . --bin ducktape-ui
cargo new my-iced-app
cd my-iced-app

ducktape-ui init
ducktape-ui add button input card badge separator
```

Then expose the generated module from your app:

```rust
mod ui;

use ui::button::{ButtonVariant, button};
use ui::theme::LIGHT;

#[derive(Clone)]
enum Message {
    Save,
}

fn view() -> iced::Element<'static, Message> {
    button("Save", &LIGHT)
        .variant(ButtonVariant::Default)
        .on_press(Message::Save)
        .into()
}
```

`add` installs transitive component dependencies and adds missing Cargo dependencies. Existing component files are preserved; use `diff` before opting into `--overwrite`.

## Commands

| Command | Purpose |
| --- | --- |
| `ducktape-ui init [--ui src/ui]` | Configure the output directory. |
| `ducktape-ui list` | List registry components. |
| `ducktape-ui view button` | Inspect metadata and source. |
| `ducktape-ui add button card` | Copy components and their dependencies. |
| `ducktape-ui add button --dry-run` | Preview writes. |
| `ducktape-ui diff button` | Compare owned source with the registry. |
| `ducktape-ui add button --overwrite` | Explicitly replace an installed component. |

## Components

The first vertical slice includes:

- semantic light/dark theme tokens
- button variants, sizes, pressed/hovered/disabled states
- default and invalid text inputs
- composable cards
- badge variants
- horizontal and vertical separators

Every component is an ordinary `.rs` file under `src/ui`. Edit `theme.rs` once to change semantic colors, radii, spacing, and typography across the installed set. The defaults mirror Ducktape's warm light/dark palettes and support its three runtime accents with `Theme::with_accent`.

## Showcase

```bash
cargo run --features showcase --bin showcase
```

The showcase switches light/dark themes and exercises every current component. It is also the canonical registry source: the CLI embeds the same files compiled by the showcase, so examples and installed templates cannot silently drift apart.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The end-to-end test creates a blank iced project, runs `init` and `add button`, and compiles the installed source.

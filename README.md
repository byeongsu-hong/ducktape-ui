# ducktape-ui

The source-owned UI toolkit for [iced](https://github.com/iced-rs/iced), built first for Ducktape and reusable by any iced application.

Like shadcn/ui, ducktape-ui copies readable component source into your project. There is no runtime `ducktape-ui` widget dependency: you own the files, edit the tokens, and keep iced's native state and message model.

## Quick start

```bash
cargo install --git https://github.com/byeongsu-hong/ducktape-ui --bin ducktape-ui
cargo new my-iced-app
cd my-iced-app

ducktape-ui init
ducktape-ui add button input textarea checkbox field card badge alert progress empty separator segmented-control
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

`add` installs transitive component dependencies, missing Cargo dependencies, and required Cargo features. The registry enables iced's `advanced` feature automatically, so `focus-control` needs no manual `Cargo.toml` edit. Existing component files are preserved; use `diff` before opting into `--overwrite`.

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

The current toolkit includes:

- semantic light/dark theme tokens
- keyboard-focusable button variants, sizes, pressed/hovered/disabled states
- default and invalid text inputs
- visible field labels with help and error text
- semantic surfaces and composable cards
- badge variants, sizes, and labeled status markers
- alerts, bounded progress bars, and reusable empty states
- keyboard-focusable styled checkboxes and multiline text editors
- responsive aspect ratios, avatars, breadcrumbs, items, pagination, and skeletons
- labels, key caps, and theme-backed typography roles
- grouped controls, tables, scroll areas, and reduced-motion-aware spinners
- attachments, message bubbles, markers, transcript composition, and message scrolling
- controlled accordion and collapsible state, a searchable combobox, and a keyboard-complete styled native pick list
- keyboard-complete single/range/multiple calendars, swipeable focus-scoped carousels, and a headless data-table recipe
- an iced-specific focus-control shell with stable IDs, visible focus, and pointer, touch, Enter, and Space activation
- keyboard-complete tabs, radio groups, toggles, toggle groups, and switches
- grouped native OTP input plus draggable single-, range-, and multi-thumb sliders
- horizontal/vertical resizable panel groups with keyboard, pointer, and touch handles
- focus-contained Dialog and Alert Dialog primitives with explicit LTR/RTL alignment
- composable legacy Toast surfaces and a controlled timed Sonner queue
- a searchable grouped Command palette with native editing and complete result navigation
- Canvas line, area, bar, pie, and donut charts with controlled tooltips and visible companion data
- collision-aware Popover and Tooltip overlays plus pointer- and focus-safe interactive Hover Cards
- a responsive controlled Sidebar system with collapse modes, shortcut, rail, full menu composition, and RTL
- Dropdown, Context, and Menubar overlays plus grouped Select on one shared keyboard-complete menu model
- modal/non-modal Sheet and draggable Drawer panels on all four edges with focus restoration
- single/range Date Picker composition and responsive Navigation Menu disclosures with stable focus
- horizontal and vertical separators
- a controlled segmented selector built from native buttons

`segmented-control` remains a lightweight Tab-focusable selector; the separate `tabs` component owns roving focus and arrow-key behavior.

Full shadcn/ui coverage is tracked component-by-component in [the parity matrix](docs/parity.md). Components stay at **Foundation** until their focus, keyboard, overlay, and state contracts are complete; visual similarity alone is not parity.

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

The end-to-end test creates a blank iced project, installs the complete component set, and compiles its copied source and tests.

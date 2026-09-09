# Start from a screen

For a new default-styled Ice screen, start with the runnable
[Settings application](../examples/settings/README.md):

```sh
cargo run -p settings-example
cargo test -p settings-example
```

Its [app source](../examples/settings/src/ui/app.ice) imports the default
components, binds native inputs, customizes one field, and handles local
validation and success. The [starter](../examples/starter/README.md) is the
smaller build/include integration example; Settings supplies the default UI
composition. The catalog is useful when browsing individual controls.

## Choose the closest working composition

| Screen | Read and run | Ownership and customization |
| --- | --- | --- |
| Settings or form | [Settings](../examples/settings/src/ui/app.ice); `cargo test -p settings-example` | `Form` owns scrolling, 24px outer padding and a 640px width cap. State owns values and validation. `TextField` accepts radius and padding without replacing its input. |
| List and detail | [Project browser](../examples/showcase/tests/cases/ui/list_detail_navigation.ice), [Rust data helpers](../examples/showcase/tests/list_detail_navigation.rs); `cargo test -p showcase --test list_detail_navigation` | Stable project IDs own selection and per-record drafts. Back explicitly restores focus. The customized preset changes inset and reading width. |
| Dialog | [Public native dialog composition](../examples/showcase/tests/overlay_focus_customization.rs); `cargo test -p showcase --test overlay_focus_customization` | `dialog_view` composes caller-owned body/actions; `dialog_update` owns open state and applies `FocusScope` transition tasks. The component bounds the scrolling body and keeps actions reachable. Product-specific retained controls use a typed Rust boundary. |
| Card collection | [Minimum-cell collection](../examples/showcase/tests/cases/ui/grid_collection.ice); `cargo test -p showcase --test grid_collection` | The app owns items and selection. The component exposes minimum cell width, spacing and inset; native grid owns equal tracks and aspect-ratio card sizing. |

The focused Showcase files are executable native contracts, including app or
widget composition and assertions. Copy the relevant composition into your app,
not its test-only helpers or the catalog adapter module. Settings keeps Save
visible with the scrolling body and action row as siblings
in a bounded column. The [responsive workspace](../crates/ui-lang-components/docs/responsive-workspace.md)
uses the same ownership in a larger composition.

## Resolve the source import once

Ice `use` paths are relative to the importing file. Settings uses:

```ice
use "../../../../crates/ui-lang-components/src/ice/default.ice"
```

That path is correct for `examples/settings/src/ui/app.ice`, not every app
directory. Outside this workspace, copy the complete
[`src/ice` directory](../crates/ui-lang-components/src/ice) to a stable vendored
location and import its `default.ice` by a relative path. Its sibling imports
must remain present. A Cargo dependency does not create a package-aware Ice
import. Keep the app's build script, Rust `include_app!` path and bundled font
paths consistent with its new location. See the
[source interface contract](../crates/ui-lang-components/README.md#ice-interface-in-this-workspace).

## Decide layout ownership before styling

Use one outer inset owner: `Page` for an ordinary bounded pane, or `Form` for
a scrolling form. If a Form lives inside an already padded Page, explicitly
choose its padding rather than accidentally doubling the edge distance. A
surface's internal padding is a separate boundary.

Keep required actions outside the scrolling body when they must stay visible.
Give the body a finite remaining height, and align the action row to the same
reading-width cap as its fields. A `h=fill` child needs a bounded ancestor;
putting the whole page in another vertical scroller changes that ownership.

Use the default component and semantic recipe first, then override the intended
property. Optional explanations should disappear with their row. Give long
labels a bounded width and wrapping policy. Text alignment positions glyphs
inside a widget; parent alignment positions the widget. Check the painted text
when an alignment decision matters.

## Verify the changed screen

Pick cases from the screen's actual risks: wide and narrow/short windows,
empty and long content, and the interaction states affected by the change.
For each, assert useful geometry and perform the user's actual click, typing
or keyboard route. A direct state dispatch does not prove the button works.

Capture and inspect those states after assertions. Confirm edge spacing,
readable text, action reachability and focus visibility in the rendered image;
a correct outer rectangle does not prove its text fits. For a regression,
temporarily restore the faulty behavior or make one minimal behavior mutation
and observe the intended assertion fail, then restore and rerun it. Compiler
errors and setup failures are not regression evidence.

Use the existing [native test tooling](testing.md) and
[test skill](../skills/test-ice-ui/SKILL.md). Report commands, observed outcomes,
capture paths and any remaining limits. Add frame measurements for changes to
live streams or large collections; an idle frame does not establish append or
paint cost.

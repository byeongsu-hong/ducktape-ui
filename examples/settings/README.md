# Workspace preferences

A compact settings screen built from Ice's default PageHeader, Form,
FormSection, TextField and Field components. Display name, email, workspace
name and notifications remain application-owned values. Saving validates only
that email is nonempty and shows local success; no service or persistence is
involved.

```sh
cargo run -p settings-example
cargo test -p settings-example
```

The bounded screen keeps its heading and Save changes row outside Form, which
owns the only vertical scroller. All three regions share a 640px width cap and
16px horizontal inset. At 360×320 the fields scroll and the last notification
control remains reachable; Save and its success/error hint stay visible.

The workspace input overrides only radius (4px) and padding (14px). It keeps
native editing, focus, Tab navigation and its accessible label. Empty email
shows wrapping feedback in the field and a persistent “Check email above” hint
next to Save. Correcting and saving preserves other edits. The `no_explanation`
preset checks an absent optional heading explanation alongside the long
workspace label.

Six first-class native tests cover both 960×820 and 360×320, keyboard editing,
checkbox/Save activation, full visibility after scrolling, recovery and optional
content. Their tuple is light application palette, bundled Geist font, scale 1,
en-US, Linux and reduced motion. Native bounds and visible-height assertions
are paired with inspected captures; scrolled text paint lookup has a coordinate
limitation, so it is not claimed as a passing paint-metric check.

```sh
ICE_TEST_ARTIFACT_DIR="$PWD/examples/settings/screenshots" \
  cargo test -p settings-example -- --nocapture
```

![Wide workspace preferences](screenshots/settings_wide_fixed_actions/wide.png)

![Short window at the end of the form](screenshots/settings_short_window_scroll/short_end.png)

![Wrapping email feedback](screenshots/settings_error_recovery/error.png)

The [screen authoring guide](../../docs/ui-authoring.md) links related list/detail,
overlay and collection compositions.

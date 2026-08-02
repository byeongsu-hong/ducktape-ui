# Ice component showcase

Run the complete default component catalog with:

```sh
cargo run -p showcase
```

The catalog uses a paired grid at the default window size and stacks every
section into one column at the 720-pixel minimum width.

The 100,000-row `VirtualList`, `TreeView`, and 100,000-by-16 `DataGrid` are
fixed, bounded regions above the independently scrolling catalog. This is the
supported integration shape: each retained collection owns its scroll axes and
must not be nested in another scrollable.

![Buttons, fields, and selection controls](screenshots/catalog-buttons.png)

![Paired modal and data-table sections](screenshots/catalog-layout.png)

![OTP caret follows the active digit slot](screenshots/catalog-otp-focus.png)

![Embedded component scrollbar reserves shortcut space](screenshots/catalog-command-scroll.png)

![Navigation shell and toast](screenshots/catalog-navigation.png)

![Single-column navigation shell](screenshots/catalog-narrow.png)

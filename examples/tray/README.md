# Tray

A focus timer that lives in the menu bar: the minutes count down beside the
icon whether or not anything is open, and the item's menu is the whole
program.

```bash
cargo run -p tray-example
cargo test -p tray-example
```

There is no window at all. `daemon` starts without one and stays alive after
every window closes, which is exactly what a menu bar app is — and a `menu`
needs no window of its own, because the platform owns it.

```ice
daemon Tray
  tray
    icon-rgba "icon.rgba" 22 22
    icon-template true
    label clock(remaining)
    menu
      phase(running, remaining)
      clock(remaining)
      separator
      start_label(running) -> toggle
      "Reset" -> reset
      separator
      "Quit" -> quit
```

`label` is an expression over state, so the menu bar reads `17:42` and keeps
counting because a `subscribe every 1s` moves the state — nothing pushes text
at the icon. That is the whole point of the tray being a language feature
rather than a handle you poke: the status item is another thing the view layer
keeps in step. `icon-template` hands macOS a black and alpha image to
recolour, so the icon reads on a light and a dark menu bar from the one file.
The icon itself is raw RGBA — `width × height × 4` bytes, checked when the app
is compiled — because an image codec is not something a UI language needs to
own.

Every row is an expression, so the same rule applies inside the menu: `phase`
and `clock` are figures you read, and the platform draws them disabled because
they name no route. `start_label(running) -> toggle` is both — the row says
what pressing it will do, and pressing it does that.

## Tests

`tray choose` picks the row carrying that text and runs it, the way the
platform reports one: by row, through the same generated row-to-handler table
the live subscription maps a chosen row through. It is the only step that
covers a menu row end to end, so a row index that drifts in code generation
fails a test instead of going quietly dead in the menu bar.

```ice
test choosing_a_command_row_runs_its_handler
  expect running == false
  tray choose "Start"
  expect running
```

`expect tray label|icon|item|command` reads what the program last decided the
item should show. Every platform keeps that record whether or not it has a
status item, so these assertions run and mean the same thing on Linux CI as on
a Mac.

One macOS-only thing is pinned in CI rather than left to eyes:
`a_guard_driven_swap_still_asks_for_template_rendering`, in
`ui-lang-runtime`'s tray module, runs on the macOS runner and asserts that
swapping the icon under a `when` guard still carries the template flag. That is
the failure worth automating, because a menu bar that quietly stops recolouring
looks fine until someone switches to light mode.

What no test here covers, and only a Mac can show: that the menu actually
raises on a left click, that a disabled row draws as a legible grey stat rather
than something that looks broken, and that the icon really does recolour. A CI
runner has no window server, so no status item is ever created and the native
calls never run — the assertion above pins the argument, not the pixels. On
other targets the same source compiles and runs against no-op stubs.
`ICE_TRAY_DEBUG=1` traces the native boundary when a status item looks inert.

# Tray (System Status Item) Support — Design

Date: 2026-08-08
Status: Approved (scope, approach, and syntax confirmed in brainstorming session)

## Goal

Ice apps can declare a system tray presence — on macOS, an `NSStatusItem` in the
menu bar — with an icon, live reactive label text, a tooltip, and a popover
window that toggles on click, anchored under the icon. Iced does not provide
this; Ice supplies it through the runtime.

Driving use case: the trading example shows a mini status in the macOS menu bar
(live coin price next to a small icon) and opens a compact status panel when
clicked.

## Scope

Phase 1 (this design):

- Tray icon (raw RGBA, compile-time embedded and size-checked).
- Reactive `label` and `tooltip` expressions re-evaluated after every update.
- `popover <window>` — left click toggles a named window anchored under the
  icon.
- Platform-neutral syntax; macOS implementation only. Other targets compile and
  run with the tray as a no-op.
- Works for both `app` and `daemon` roots.

Explicitly out of phase 1: native drop-down menus (NSMenu), custom `on-click`
handlers, dock-icon hiding (activation policy), Windows/Linux tray backends,
auto-dismiss on outside click (shown as a `subscribe window` pattern in the
example instead).

## Approach

First-class `tray` app-setting block (approach A from brainstorming), backed by
a new runtime module wrapping the `tray-icon` crate (tauri). Rationale:

- A tray is part of the app shell, like `window` and `title` — not an ad-hoc
  iced API wrapper, so it does not conflict with SPEC §13.
- Reactive label and popover anchoring are only valuable when the language
  wires them; an extern-only boundary would push glue into every app.
- `tray-icon` has a safe public API, so the workspace-wide
  `unsafe_code = "forbid"` lint holds. Its `Icon::from_rgba(rgba, w, h)`
  matches Ice's existing raw-RGBA icon convention exactly (no image decoding).

## Language surface

```ice
app Trading
  title "Ducktape Trading"
  tray
    icon-rgba "assets/tray.rgba" 22 22   # required; path relative to the .ice file
    icon-template true                    # optional; macOS template image
    label price_line(self)                # optional; reactive str expression
    tooltip "Ducktape Trading"            # optional; reactive str expression
    popover status                        # optional; named window reference
  window status
    size 320 240
    decorations false
```

Grammar (SPEC addition):

```
app_setting   = ... | tray_decl
tray_decl     = "tray" INDENT tray_setting*
tray_setting  = "icon-rgba" string u32 u32
              | "icon-template" bool
              | ("label" | "tooltip") expr
              | "popover" name
```

Semantics:

- `icon-rgba` follows the window icon convention: raw RGBA file, embedded with
  `include_bytes!`, validated at check time and again in generated Rust as
  `width × height × 4` bytes. Encoded formats (PNG etc.) stay out, matching the
  existing language decision.
- `label` / `tooltip` have app-`title` semantics: expressions over state,
  re-evaluated after every update; the native call happens only when the value
  changed (diffing lives in the runtime module).
- `popover <name>` references a named window declared on the same root. Left
  click closes the window if open, otherwise opens it centered under the icon's
  screen rect (scale-factor conversion is the runtime's job). The window's
  close event clears the tracked id.
- Only the left click is wired in phase 1; right/middle clicks and hover are
  ignored (custom handlers are future work).
- `tray` is allowed once per root, in both `app` and `daemon`.
- Platform mapping is documented in SPEC: macOS is fully mapped; on other
  targets the runtime stubs are no-ops in phase 1 (and Windows never supports
  `label` natively).
- Naming: the text is `label`, not `title`, to avoid clashing with the app
  `title` (window title) setting.

## Runtime architecture (`ui-lang-runtime/src/tray.rs`)

- Dependency: `[target.'cfg(target_os = "macos")'.dependencies]`
  `tray-icon = "0.21"`. Safe public API; no unsafe in workspace code.
- `TrayIcon` (NSStatusItem handle) is `!Send` and main-thread-only, so it never
  enters iced state. The module owns it in a thread-local and exposes:
  - `tray::init(TrayConfig)` — called from generated boot. Iced's boot runs on
    the main thread after the winit event loop starts, satisfying macOS's
    "event loop must be running" requirement. Validated by an implementation
    spike before the rest lands.
  - `tray::set_label(&str)` / `tray::set_tooltip(&str)` — remember the last
    value and no-op when unchanged.
  - `tray::events() -> Subscription<TrayEvent>` — bridges
    `TrayIconEvent::set_event_handler` into a channel wrapped by
    `Subscription::run_with`, the same pattern as the accessibility action
    channel (`lib.rs` `ActionSubscription`).
  - `tray::anchor_position(icon_rect, window_size) -> Point` — pure function
    converting the icon's physical screen rect to a logical top-left for the
    popover (bottom-centered under the icon). Retina and external displays are
    explicit validation cases.
- Non-macOS targets compile the same signatures as no-op stubs, so Linux CI
  builds apps that declare `tray`.

## Codegen

Emitted only when a `tray` block is declared:

1. Reserved message variant `__TrayEvent(tray::TrayEvent)` (same mechanism as
   the `__Accessibility*` variants).
2. `tray::events().map(M::__TrayEvent)` prepended to the subscription batch.
3. Boot chains `tray::init(...)` with the embedded RGBA bytes and initial
   label/tooltip evaluations.
4. Update gains a `__TrayEvent` arm. With `popover w`: a hidden field
   `__ice_tray_popover: Option<window::Id>` toggles — close when `Some`,
   otherwise compute the anchor, override the named window settings' position
   with `Specific(anchor)`, open, and store the id. The window's close event
   resets the field.
5. After user handler arms, label/tooltip are re-evaluated and passed to
   `tray::set_label` / `set_tooltip` (runtime diffs).

Layers touched: `parser/settings.rs` → `ast/app.rs` (`TraySettings`) →
`lower.rs` (`ResolvedAppSettings.tray`) → `check/application.rs` →
`codegen/settings.rs` + `codegen.rs`. The same path the `window` block takes.

## Diagnostics

- Duplicate `tray` block → error.
- `popover x` referencing an undeclared named window → error.
- Missing `icon-rgba` in a `tray` block → error.
- RGBA byte length ≠ `w × h × 4` → reuse the existing window-icon error.
- `label` / `tooltip` must type-check as `str` (existing expression checking).

## Spike findings (2026-08-08, validated on this machine)

- Creating the `TrayIcon` inside iced's `boot` works: iced calls boot after
  `EventLoop::build()` (NSApplication initialized) and the status item lands in
  the menubar — verified via `TrayIcon::rect()` returning real screen
  coordinates, and `set_title` updates live from `update`.
- tray-icon `Rect` is **physical** pixels (`LogicalPosition * backingScaleFactor`
  in its macOS source; empirically menubar height 60 phys = 30 logical × scale
  2.0). iced `Position::Specific`/`move_to` take **logical** points.
- Anchoring therefore uses: open popover `visible: false` at default position →
  `iced::window::scale_factor(id)` → logical anchor = physical / scale →
  `move_to` → `set_mode(Mode::Windowed)` (source-verified to call
  `set_visible(true)`) → `gain_focus`. `&dyn iced::window::Window` exposes only
  raw handles, so it cannot help here.
- `iced::window::close_events() -> Subscription<Id>` and `Task::discard` exist
  in iced 0.14 — used for popover close tracking and the open task.
- tray-icon pinned at 0.24 (latest; safe public API).
- Known limitation (documented): with mixed-DPI multi-monitor setups the scale
  is read from the popover's initial monitor, which matches the menubar's
  monitor in the common case.

## Testing & evidence

- Fixtures (auto-discovered): `cases/compile/tray-basic/` golden codegen;
  `cases/diagnostic/` for each error above; `cases/format/tray-block/` for
  `cargo ice fmt` stability.
- Runtime unit tests: `anchor_position` math and label diffing.
- Trading example: `tray` block with a live price `label`, a `status` named
  window (320×240, undecorated) rendered via the `window` binding branch in the
  view, and unfocus→close wiring via `subscribe window`. PR includes menubar +
  popover screenshots per the visual-example rule.
- Docs: SPEC.md grammar + semantics + platform mapping table, COVERAGE.md tray
  row, README feature list.

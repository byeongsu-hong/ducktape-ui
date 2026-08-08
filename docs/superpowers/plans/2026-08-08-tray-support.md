# Tray Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** First-class `tray` app-setting block: menubar icon + reactive label/tooltip + click-toggled anchored popover window, macOS implementation via the `tray-icon` crate.

**Architecture:** New safe runtime module `ui_lang_runtime::tray` (thread-local `TrayIcon`, subscription-bridged events, no-op stubs off-macOS). Codegen wires boot init, a reserved `__TrayEvent` message, label re-sync after update, and popover toggle against a hidden `__ice_tray_popover` field. Parser→AST→check→lower→codegen follows the exact path the `window` block takes.

**Tech Stack:** Rust 2024, iced 0.14 (pinned `=0.14.0`), tray-icon 0.21 (macOS-gated), existing Ice fixture harness.

## Global Constraints

- `unsafe_code = "forbid"` workspace-wide; no crate may opt out. tray-icon is used only through its safe API.
- Icon assets are raw RGBA (`icon-rgba "path" w h`), compile-time embedded, byte length `w × h × 4`; encoded formats stay out (SPEC 2.0 decision).
- Syntax platform-neutral; phase-1 behavior on non-macOS targets is a compiled no-op.
- The reactive text setting is named `label` (not `title`).
- Only left click is wired; right/middle clicks are ignored.
- All edits in `.worktree/tray-menubar` on branch `feat/tray-menubar`; conventional-commit subjects; SPEC.md/COVERAGE.md/README.md updated with syntax changes.

---

### Task 0: Spike — validate native assumptions (throwaway, not committed)

**Files:** temporary edits to `examples/starter/` (revert with `git checkout -- examples/starter` afterwards; keep notes in the PR body).

Answers required before Tasks 1/5 lock in:

- [ ] **S1**: Add `tray-icon = "0.21"` to starter, create a `TrayIconBuilder` inside the generated boot path (call from `main.rs` wrapper following the cef-browser pre-run pattern is NOT enough — must run after event loop start, so patch the generated boot via a hand-rolled `iced::application(...)` in `main.rs` that calls tray init in `boot`). Run `cargo run -p starter` on this Mac; confirm the icon appears in the menubar.
- [ ] **S2**: Log `TrayIconEvent::Click { position, rect, .. }` values while clicking on the Retina display; determine whether values are physical or logical points (menubar y≈0–24 logical; Retina physical would read ≈0–48/50).
- [ ] **S3**: Check `iced-0.14.0` source (`~/.cargo/registry/src/*/iced-0.14.0` and `iced_runtime`/`iced_winit`) for: `iced::window::close_events()` existence, `Task::discard`, `iced::window::Settings.position` interpretation (logical?), and what `&dyn iced::window::Window` (used by `iced::window::run`) exposes (`scale_factor()`? `set_outer_position()`?).
- [ ] **S4**: Confirm `.with_title` renders text beside the icon and `set_title` updates live from `update`.
- [ ] **S5**: Record findings as a short "Spike findings" section appended to the spec doc; pick the anchor mechanism:
  - Preferred: rect is logical → `anchor_position` is pure arithmetic, open with `Position::Specific`.
  - Fallback A: rect physical + `dyn Window` exposes scale/set_outer_position → open hidden, position via `iced::window::run`, then show.
  - Fallback B: rect physical, divide by main-display scale obtained in the spike-proven way.
- [ ] **S6**: `git checkout -- examples/starter && git status` clean.

### Task 1: Runtime `tray` module

**Files:**
- Create: `crates/ui-lang-runtime/src/tray.rs`
- Modify: `crates/ui-lang-runtime/src/lib.rs` (add `pub mod tray;`)
- Modify: `crates/ui-lang-runtime/Cargo.toml` (add `[target.'cfg(target_os = "macos")'.dependencies] tray-icon = "0.21"`)

**Interfaces (Produces):**
```rust
pub struct TrayConfig {
    pub icon_rgba: &'static [u8],
    pub icon_width: u32,
    pub icon_height: u32,
    pub icon_template: bool,
}
#[derive(Clone, Debug)]
pub struct TrayRect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 } // logical points (per spike)
#[derive(Clone, Debug)]
pub enum TrayEvent { LeftClick { icon: TrayRect } }
pub fn init(config: TrayConfig);
pub fn set_label(value: &str);
pub fn set_tooltip(value: &str);
pub fn events() -> iced::Subscription<TrayEvent>;
pub fn anchor_position(icon: &TrayRect, window_size: iced::Size) -> iced::Point;
```

- [ ] **Step 1: failing unit tests** in `tray.rs` `#[cfg(test)]`: `anchor_position` centers under the icon (`x = icon.x + icon.width/2 - size.width/2`, `y = icon.y + icon.height + 4.0` margin) and clamps `x` to ≥ 0.
- [ ] **Step 2:** `cargo test -p ui-lang-runtime tray` → FAIL (module missing).
- [ ] **Step 3: implement.** macOS body: `thread_local! { static TRAY: RefCell<Option<TrayState>> }` with `TrayState { icon: tray_icon::TrayIcon, label: String, tooltip: String }`; `init` builds `TrayIconBuilder::new().with_icon(Icon::from_rgba(...)).with_icon_as_template(config.icon_template)` and installs `TrayIconEvent::set_event_handler` forwarding left-clicks into a `std::sync::mpsc`-backed shared channel consumed by `events()` via the `Subscription::run_with` + stream pattern copied from `Bridge::subscription` (`lib.rs:1531`). `set_label`/`set_tooltip` diff against stored values. Non-macOS: same signatures, empty bodies, `events()` returns `Subscription::none()`-equivalent (`iced::Subscription::run_with` over an empty stream or simply `Subscription::none()` — must still typecheck as `Subscription<TrayEvent>`). Coordinate conversion per spike outcome.
- [ ] **Step 4:** `cargo test -p ui-lang-runtime tray` → PASS; `cargo check -p ui-lang-runtime` on macOS.
- [ ] **Step 5:** Commit `feat(runtime): tray status-item module with event subscription`.

### Task 2: Parser + AST

**Files:**
- Modify: `crates/ui-lang-core/src/ast/app.rs` (after `MacosWindowSettings` ~line 317)
- Modify: `crates/ui-lang-core/src/parser/settings.rs` (`parse_app_settings` ~line 66; new `parse_tray_settings` beside `parse_window_settings`)
- Tests: parser unit tests beside existing ones (find `mod tests` in parser or sibling test file; follow that placement)

**Interfaces (Produces):**
```rust
pub struct TraySettings {
    pub icon: Option<WindowIcon>,
    pub icon_template: Option<bool>,
    pub label: Option<AppExpression>,
    pub tooltip: Option<AppExpression>,
    pub popover: Option<String>,
    pub setting_spans: BTreeMap<String, Span>,
    pub span: Span,
}
// AppSettings gains: pub tray: Option<TraySettings>
```

- [ ] **Step 1: failing tests**: parse `app A\n  tray\n    icon-rgba "assets/t.rgba" 22 22\n    label price(self)\n    popover status\n  window status\n    size 320 240` → fields populated; duplicate `tray` → E014 `duplicate app setting `tray``; unknown key inside tray → E015 `unknown tray setting`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: implement** in `parse_app_settings`, before the leaf split (mirror the `window` branch at settings.rs:67): `if item.text == "tray" { ... settings.tray = Some(parse_tray_settings(item)?); }`. `parse_tray_settings` mirrors `parse_window_settings`: keys `icon-rgba` (reuse `config_window_icon`), `icon-template` (`config_bool`), `label`/`tooltip` (`app_expression`), `popover` (`identifier`). Missing `icon-rgba` → E015 "tray requires `icon-rgba`" at block close.
- [ ] **Step 4:** tests PASS.
- [ ] **Step 5:** Commit `feat(parser): tray app-setting block`.

### Task 3: Check + HIR

**Files:**
- Modify: `crates/ui-lang-core/src/hir.rs` (`AppSettingExprId` — add `TrayLabel`, `TrayTooltip`)
- Modify: `crates/ui-lang-core/src/hir/from_ast.rs` (carry tray into `CheckedAppSettings.source` — verify it flows automatically since `source` is the AST `AppSettings`; extend any per-field mirroring found there)
- Modify: `crates/ui-lang-core/src/check/application.rs` (`check_app_settings`)
- Tests: checker unit tests beside existing check tests; plus diagnostic fixtures in Task 6

- [ ] **Step 1: failing tests**: `label` non-str expr → type error; `popover missing_window` → E173 `unknown app window `missing_window``; tray label/tooltip analyses inserted under `CheckedExprOwner::AppSetting(TrayLabel/TrayTooltip)`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: implement** in `check_app_settings`: for `label`/`tooltip` mirror the `title` block (application.rs:37-51) but analyze against plain `states` (no `window` binding — tray text is app-scoped); popover name validated against `document.settings.windows` exactly like check/handler.rs:446-458. Wire any expression-shape validators that enumerate `AppSettingExprId` (`validate_app_setting_expression_shape`, `validate_app_setting_expression_graphs` in lower.rs — grep for exhaustive matches over the enum and extend).
- [ ] **Step 4:** tests PASS; `cargo test -p ui-lang-core` still green.
- [ ] **Step 5:** Commit `feat(check): validate tray label, tooltip, and popover references`.

### Task 4: Lower

**Files:**
- Modify: `crates/ui-lang-core/src/lower.rs` (`ResolvedAppSettings` ~1276, `lower_app_settings` ~4686, `validate_checked_app_settings` ~4809)

**Interfaces (Produces):**
```rust
pub(crate) struct ResolvedTraySettings {
    pub(crate) icon: ResolvedWindowIcon,          // reuse the window icon resolved type
    pub(crate) icon_template: bool,
    pub(crate) label: Option<ResolvedAppExpression>,
    pub(crate) tooltip: Option<ResolvedAppExpression>,
    pub(crate) popover: Option<NamedWindowId>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}
// ResolvedAppSettings gains: pub(crate) tray: Option<ResolvedTraySettings>
```

- [ ] **Step 1: failing test** (lower tests live under `#[cfg(test)]` in lower or codegen/tests): lowering the Task-2 source yields `tray.popover == Some(NamedWindowId(0))` and label expression resolved.
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: implement**: in `lower_app_settings` map `source.tray` → `ResolvedTraySettings` (label/tooltip via `checked_app_setting_expression(AppSettingExprId::TrayLabel, ..)`; popover via `windows.iter().position(..)`; icon via the same resolution `lower_window_settings` uses for `icon`).
- [ ] **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(lower): resolve tray settings`.

### Task 5: Codegen

**Files:**
- Modify: `crates/ui-lang-core/src/codegen.rs` (message enum ~648-753; state struct ~600-640; builder — no builder change needed)
- Modify: `crates/ui-lang-core/src/codegen/application.rs` (`generate_boot` ~247, `generate_update` ~438)
- Modify: `crates/ui-lang-core/src/codegen/subscription.rs` (`generate_subscription` ~21)
- Modify: `crates/ui-lang-core/src/codegen/settings.rs` (new `tray_init_code` helper)
- Tests: `crates/ui-lang-core/src/codegen/tests/application.rs` assertions

Generated pieces (tray declared):

```rust
// message enum, after accessibility variants:
__TrayEvent(::ui_lang_runtime::tray::TrayEvent),
// + when popover: __TrayPopoverClosed(::iced::window::Id),

// state struct, when popover:
__ice_tray_popover: ::std::option::Option<::iced::window::Id>,   // init None in __state

// __boot (both kinds), before returning, guarded #[cfg(not(test))]:
::ui_lang_runtime::tray::init(::ui_lang_runtime::tray::TrayConfig {
    icon_rgba: { const __ICE_TRAY_RGBA: &[u8] = include_bytes!("<joined path>");
        const _: () = ::std::assert!(__ICE_TRAY_RGBA.len() == <byte_len>,
            "tray icon RGBA byte length does not match width × height × 4");
        __ICE_TRAY_RGBA },
    icon_width: <w>, icon_height: <h>, icon_template: <bool>,
});
state.__tray_sync();

// generated method when label/tooltip present:
fn __tray_sync(&self) {
    #[cfg(not(test))] {
        ::ui_lang_runtime::tray::set_label(&(<label expr code>));      // if label
        ::ui_lang_runtime::tray::set_tooltip(&(<tooltip expr code>));  // if tooltip
    }
}

// subscription batch entries:
::ui_lang_runtime::tray::events().map({message}::__TrayEvent),
// + when popover (mechanism per spike S3):
::iced::window::close_events().map({message}::__TrayPopoverClosed),

// update arms (before user handlers):
{message}::__TrayEvent(__event) => { match __event {
    ::ui_lang_runtime::tray::TrayEvent::LeftClick { icon } => {
        // no popover declared → return ::iced::Task::none();
        if let ::std::option::Option::Some(__id) = self.__ice_tray_popover.take() {
            return ::iced::window::close(__id);
        }
        let mut __settings = Self::__window_<N>();
        __settings.position = ::iced::window::Position::Specific(
            ::ui_lang_runtime::tray::anchor_position(&icon, __settings.size));
        let (__id, __task) = ::iced::window::open(__settings);
        self.__ice_tray_popover = ::std::option::Option::Some(__id);
        return __task.discard();   // or map-to-noop per spike S3
    } } },
{message}::__TrayPopoverClosed(__id) => {
    if self.__ice_tray_popover == ::std::option::Option::Some(__id) {
        self.__ice_tray_popover = ::std::option::Option::None; }
    return ::iced::Task::none(); },
```

Plus: after the `match` in `__update` (where the post-update accessibility snapshot happens — locate the tail of `generate_update`), insert `self.__tray_sync();` so every state change refreshes the label. And expose the view binding `tray_popover: window-id?` resolved to `self.__ice_tray_popover` — inject it wherever the daemon `window` binding is injected into view/title envs (grep `"window"` insertions in check + codegen env setup), gated on `tray.popover.is_some()`.

- [ ] **Step 1: failing codegen tests** asserting the snippets above for a tray fixture source.
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: implement.**
- [ ] **Step 4:** PASS; full `cargo test -p ui-lang-core`.
- [ ] **Step 5:** Commit `feat(codegen): emit tray wiring`.

### Task 6: Fixtures

**Files:**
- Create: `crates/ui-lang-core/tests/cases/compile/tray-basic/{as-is.ice,to-be.txt}` (assert: `__TrayEvent`, `tray::init`, `close_events`/toggle arm, `__tray_sync`, `anchor_position`)
- Create: `crates/ui-lang-core/tests/cases/diagnostic/tray-duplicate/{as-is.ice,to-be.txt}`
- Create: `crates/ui-lang-core/tests/cases/diagnostic/tray-unknown-popover/{as-is.ice,to-be.txt}`
- Create: `crates/ui-lang-core/tests/cases/diagnostic/tray-missing-icon/{as-is.ice,to-be.txt}`
- Create: `crates/ui-lang-core/tests/cases/format/tray-block/{as-is.ice,to-be.ice}` (4-space → 2-space reprint)

- [ ] **Step 1:** write fixtures (they ARE the failing tests) — match existing `to-be.txt` shape (substring-per-line; check a diagnostic case first for exact format, including error codes like `E014`/`E015`/`E173`).
- [ ] **Step 2:** `cargo test -p ui-lang-core` → fixtures pass (auto-discovered).
- [ ] **Step 3:** Commit `test(cases): tray fixtures`.

### Task 7: Trading example

**Files:**
- Create: `examples/trading/assets/tray-icon.rgba` (generate: python3 script writing a 22×22 template glyph — black pixels + alpha channel forming a candlestick; bytes = 22*22*4)
- Modify: `examples/trading/src/ui/app.ice` (tray block + `window status` + view branch on `tray_popover == some(window)` + `subscribe window unfocused` → close popover)
- Modify: `examples/trading/src/ui/extern/hyperliquid.ice` + `examples/trading/src/hyperliquid.rs` (add `sync tray_status(tape:Tape, coin:str) -> str` → `"BTC 65,432.1"` from the current mid/last price; reuse existing price formatting helpers)

- [ ] **Step 1:** generate asset; wire `.ice`; implement extern.
- [ ] **Step 2:** `cargo run -p trading-example` on this Mac; verify menubar icon + live label; click → popover opens anchored; click again / unfocus → closes. Capture evidence: `screencapture -R` of the menubar region and of the open popover into the PR.
- [ ] **Step 3:** `cargo ice fmt` and `cargo ice check` green.
- [ ] **Step 4:** Commit `feat(trading): menubar mini status via tray`.

### Task 8: Docs

**Files:** `SPEC.md` (grammar `tray_decl` beside `window_decl` ~line 326; semantics subsection with platform mapping table; §13 note that tray is app-shell, not an iced wrapper), `COVERAGE.md` (tray row + evidence pointers to fixtures/example), `README.md` (feature mention).

- [ ] **Step 1:** write docs; keep the grammar EBNF in the SPEC style shown at lines 318-341.
- [ ] **Step 2:** Commit `docs: specify tray support`.

### Task 9: Verification + PR

- [ ] `cargo check --workspace && cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets --no-deps`
- [ ] `cargo fmt --all -- --check` and `cargo ice fmt --check`
- [ ] `cargo ice check`
- [ ] Push `feat/tray-menubar`, open PR with behavior description, commands run, screenshots; review full diff + CI; merge only under CLAUDE.md confidence gates; remove worktree after merge.

## Self-review notes

- Spec coverage: syntax/runtime/codegen/diagnostics/fixtures/example/docs all have tasks; spike covers the flagged validation items (boot timing, coordinates, `close_events`).
- Types consistent: `TrayEvent::LeftClick { icon: TrayRect }` used in runtime and codegen; `TrayConfig` fields match between Task 1 and Task 5.
- Known open point deliberately deferred to spike: exact anchor mechanism (three fallbacks enumerated) and `.discard()` availability.

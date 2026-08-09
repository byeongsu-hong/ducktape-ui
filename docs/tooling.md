# `cargo ice` tooling reference

The README lists the commands; this is the per-command manual. `cargo ice
schema` prints the machine-readable construct table the LSP and this tooling
share.

## test

`cargo ice test` analyzes every discovered app graph before invoking workspace
Cargo tests. Ordinary `cargo test` discovers the same generated `#[test]`
functions; generated Ice tests need no Rust wrapper, registration, or direct
`iced_test` dependency in the application. Arguments after `test` pass through
to Cargo, so `cargo ice test render_contract -- --nocapture` runs one generated
contract.

## inspect and diff

`cargo ice inspect ROOT.ice` selects the Cargo package containing that root,
runs its generated headless inspection entry, and prints absolute PNG and JSON
paths. Pin inputs with `--viewport WIDTHxHEIGHT`, `--preset`, `--theme`,
`--system-theme`, `--scale`, `--locale`, `--platform`, and
`--reduced-motion`; `--output`, `--name`, and `--package` control artifact and
package selection. `cargo ice diff BASE.json CURRENT.json` writes
`report.json` and `diff.png`, then fails when structured values differ or the
changed-pixel ratio exceeds explicit `--pixel-threshold`,
`--max-changed-ratio`, or `--value-tolerance` settings.

## review

`cargo ice review ROOT.ice` runs every declared Ice test in the root graph, or
only repeated `--test NAME` selections. Captures are collected below a unique
run directory without deleting older evidence. `--baseline DIR` accepts a
previous review directory (or a capture directory), compares captures by their
stable `test-name/capture-name.json` key, and treats changed, new, removed, or
unreadable evidence as a failed review. A report baseline must be a successful
`ice_review_bundle`; capture-diff reports and failed or structurally incomplete
review reports are rejected. With explicit `--test` selections, baseline keys
are filtered before manifest paths are resolved or read, so evidence belonging
to unselected tests is outside that run while a full review validates every
entry. Capture manifests use schema 2 and review/diff reports use schema 1 with
distinct artifact kinds. Direct diff and review share one structural manifest
validator covering the published required fields, source provenance, nested
geometry/accessibility/paint shapes, and sibling PNG identity. Every failure
after opening an output directory publishes a new run-ID failure bundle; an
already-written detailed failure report for that run is retained.
`--package`, `--output`, and the same pixel/ratio/value tolerance flags control
execution and policy. The output contains `report.json`, `report.html`,
`diagnostics.json`, test logs, current PNGs/manifests, and per-capture
`diff.png`/`report.json` files.

## dev

`cargo ice dev -p PACKAGE [<cargo-build-args>] [-- <app-args>]` discovers the
package's unique Ice root, builds and launches its native app or daemon, watches
the complete Ice and Cargo input graph, and uses native filesystem notifications
to trigger content verification. Packages with multiple Ice roots can use
`cargo ice dev FILE -- <cargo-build-args> [-- <app-args>]`. If native
notifications cannot be installed, it emits the WARN tracing event `native
notifications unavailable; using polling safety mode` and checks the relevant
metadata inventory every 750 milliseconds instead. Idle
native waits and fallback metadata polls do not reread file contents; a
complete content rescan runs every 30 seconds as a safety net for lost or
metadata-invisible events. Ordinary edits to known files reuse the accepted
input inventory and content stamps, then reread only the paths named by the
notification. New untracked files, deletions, renames, and directory events
refresh the metadata inventory before hashing new or affected files. A changed
snapshot must remain identical across two reads before the background rebuild
starts.

An accepted edit is first offered to the running process as a view reload. The
runner re-runs parse, check, and lowering — so the edit is diagnosed exactly as
before — and republishes the view as data. When the running binary still fills
the slot table the new view asks for, the runner rewrites the published
template file, emits the INFO tracing event `view reloaded in place`, and the
app renders the change on its next frame with application, window, and widget
state untouched.
[Decision 0006](decisions/0006-view-as-data.md) describes that contract:
structure, literals, colours, spacing, and accessibility
segments reload; reading new state, adding a handler, or using a node the
template vocabulary does not model does not.

Every other accepted edit starts a shadow candidate through the ordinary
generated Rust path. The current process remains open until the candidate
reports that its first root widget draw completed. Parse, check, build,
startup, or readiness failure leaves that last-known-good process running. A
successful candidate replaces the old process, so application, window, and
widget state restart. A daemon reports readiness through its first drawn
window; a windowless daemon candidate cannot satisfy this draw boundary and is
rejected after the 30-second readiness timeout without replacing the current
process.

`ICE_TEMPLATE_PATH` names the published template a launched app reads. The
runner sets it; an app started without it renders the template compiled into
it. While it is set, the app also subscribes to a 150 ms tick that notices a
republished file — iced rebuilds a view only when something asks it to, and an
idle window never asks — and subscribes to nothing when it is unset, so a
release build carries no polling. An unreadable or unparseable file leaves the
last good view rendering, so a half-written save cannot blank the window.

## api

`cargo ice api ROOT.ice` checks an ordinary app root or a declaration-only
interface root and prints a versioned deterministic JSON fingerprint. The
fingerprint records the containing Cargo package and independent Ice language
revision plus checked components, flattened recipes, theme tokens, UI/extern
types, and every typed extern boundary. Imported declarations keep their
qualified alias identity; absolute source paths and codegen/HIR internals are
not part of the hash. `cargo ice api diff BASE.json CURRENT.json` prints a
human report by default, or versioned JSON with `--format json`, classifying
changes as `breaking`, `behavioral_review`, or `additive`. Breaking changes
exit nonzero. The reviewed public baseline for `ducktape-ui` lives at
`api/baselines/ducktape-ui.json`.

## lsp

`cargo ice lsp` is a stdio server with full-document synchronization, UTF-16
diagnostics, whole-document formatting, and cursor-aware completion for
declarations, handlers, views, typed match arms, widget statuses, component
contracts, theme contracts, and tests. Its structural cursor context comes from
the error-tolerant core editor model shared with the language frontends rather
than a second indentation parser in the server.

Component hover/signature help exposes read/bind/default props, output, named
events, and required/optional slots; recipe hover shows base-first expansion.
Workspace-edit code actions repair component bindings and event routes, create
handler/error-route skeletons, label child-content buttons, extract repeated
inline utilities into recipes, close direct app-handler captures through named
events, and expand long node metadata into a `with` block. They also add every
missing explicit Option/Result/UI-enum match arm and qualify an unresolved
component, recipe, extern, or type reference when exactly one import alias
makes the complete source graph check. For an existing app file it overlays
every open buffer in the import graph, reanalyzes only reverse-dependent or
previously failed app roots after buffer changes, and publishes imported errors
at the imported URI. Checked component, app-handler, recipe, and test-target
symbols support definition and collision-checked rename against those current
buffers and every closed app root under the initialized workspace. Test-target
aliases are scoped to one test, so the same alias may be reused elsewhere.
Closing a buffer falls back to disk. Component-local handlers are lexical
implementation details and are not offered as workspace navigation symbols.

Plain components and compound-family roots rename; renaming a family root
updates its dotted descendants, while direct dotted descendants and the
implicit `mount` handler are definition-only. Rename is offered only when every
reference has an exact retained source span and every workspace app root
checks.

Configure any custom LSP client with:

```json
{
  "languageId": "ice",
  "extensions": [".ice"],
  "command": "cargo",
  "args": ["ice", "lsp"],
  "cwd": "<Cargo-workspace-root>",
  "transport": "stdio"
}
```

Clients that support source actions can invoke `Run Ice lint` from an open
`.ice` file (workspace command `ice.lint`, no arguments). It runs workspace
Clippy and publishes generated Rust diagnostics at their responsible `.ice`
URI, line, and column; ordinary Rust diagnostics remain owned by the Rust
language server. The action publishes error-level generated diagnostics,
including type and extern-contract failures. Warning-level Rust and Clippy
findings from backend output are suppressed because they are not actionable
Ice diagnostics; Ice's non-CLI-only semantic warnings (`W001-W009` and
`W011-W015`) continue to appear directly from the language checker. Save every
open Ice buffer first so Cargo and the published source ranges describe the
same source revision.

Keep the importing `app` or `daemon` root open while editing a fragment; Ice
checks fragments as part of their source graph instead of treating them as
standalone programs. Initialize the Cargo workspace folder to enable safe
cross-file rename. Running `cargo ice lsp` directly waits quietly for
Content-Length-framed JSON-RPC, so launch it through the editor rather than
typing into its terminal.

## Analysis warnings

Analysis reports unreachable component and handler declarations, state with no
reachable reader or writer, immediate and effect-driven handler cycles that can
refresh forever, repeated-stream feedback that can multiply work, unfiltered
raw-event redraw feedback, position-based stateful component identity, and
retained state under unbounded dynamic identities. `cargo ice` also reports
`.ice` sources outside every root import graph. Unused derived and handler
bindings, constant no-ops and dead gates, unreachable statements, and duplicate
subscriptions are diagnosed at their Ice source lines. Component and handler
reachability is combined across every discovered app root, subscription,
preset, implicit mount, and first-class test mount or dispatch, so shared
libraries are warned only when no root uses the definition. All
language-checker warnings appear in the LSP; the workspace-orphan `W010`
remains `cargo ice`-only. Generated Rust errors from `cargo ice check` and
`clippy` are mapped back to the responsible root or imported `.ice` syntax;
`test` and `compat` run the same source-mapped check preflight before invoking
Cargo's normal test runner. The generated Rust coordinate remains available as
a note for backend debugging.

## The analysis database

All file-backed frontends share `ui_lang_core::AnalysisDb`, a process-local
incremental analysis API. Its parsed-file key contains the canonical path,
SHA-256 content hash, Ice language revision, and compiler feature set. The DB
retains parsed files, direct and reverse import edges, and checked roots. A
changed overlay or disk file invalidates only roots reachable through reverse
imports; byte-identical updates keep checked roots reusable. Missing imports
and failed roots remain tracked so creating or repairing a dependency retries
the owning root. It also exposes per-session counters for files and bytes
loaded and hashed, files scanned for imports, roots checked/reused, symbols
indexed, codegen roots, and load/check/codegen elapsed time.
`cargo ice dev` also passes the final watcher-stabilized bytes for notified Ice
files into the DB. Reanalysis scans those files once and reuses every unchanged
parsed file in the retained import closure instead of reading and hashing the
whole affected graph again.

The cache lifetime is explicit: the LSP owns one DB for its server lifetime,
`cargo ice dev` owns one for its rebuild loop, `cargo ice check` owns one for a
command, and `ui-lang-build` owns one for a build-script compilation batch.
There is no global singleton, background daemon, Salsa dependency, or
process-persistent cache. Library callers that need the same behavior create
and retain their own DB:

```rust
let mut db = ui_lang_core::AnalysisDb::default();
db.set_overlay("src/ui/part.ice", unsaved_source)?;
let checked = db.analyze_root("src/ui/app.ice")?;
let metrics = db.take_metrics();
```

## compat and accessibility scripts

`cargo ice compat` analyzes every app graph, checks the exact `iced 0.14.0`,
`iced_widget 0.14.2`, `ui-lang-build`, `ui-lang-runtime`, and AccessKit
lockfile baseline, verifies the direct reference-app and runtime manifest
pins—including the target-scoped Unix and Windows adapters—and runs the app
tests.

On Linux, `scripts/a11y-smoke.sh` creates an isolated D-Bus/AT-SPI session and
checks that the native tree is discoverable and an AT-SPI action reaches the
Iced bridge. `scripts/a11y-windows-check.sh` cross-compiles the Windows runtime
and both production and test forms of the generated reference app. Headless
tests cover dispatch from the bridge to the app message.

`cargo ice fmt` normalizes indentation and blank lines. It does not translate
removed vocabulary; old syntax fails analysis.

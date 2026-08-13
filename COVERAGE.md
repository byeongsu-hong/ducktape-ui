# iced coverage ledger

This ledger is the versioned completeness contract for what “Ice covers iced”
means. The baseline is the versions resolved by this workspace: `iced 0.14.0`
and `iced_widget 0.14.2`.

This is both an implementation inventory and the compatibility gate for a
pinned iced baseline. A partial or missing application-facing row is a coverage
gap and blocks a complete-coverage claim. Closing a gap does not necessarily
mean adding dedicated Ice syntax: common declarative concepts belong in Core,
while higher-order or custom native behavior may use an existing or extended
typed Rust boundary under the criteria in [`SPEC.md`](SPEC.md). The implemented
2.0 Preview candidate and the workspace's pre-1.0 package version `0.1.0` are
intentionally separate version schemes.

- **native**: accepted Ice syntax is parsed, type-checked, lowered, and compiled
  by the reference application or a focused test.
- **partial**: a useful subset exists, but the public iced feature is not fully
  expressible.
- **missing**: there is no accepted Ice representation yet.

An internal use of an iced widget does not count as coverage. For example, the
backend may wrap layouts in `container`, but only explicit accepted Ice syntax
counts toward the row below.

Editor diagnostics use open buffers throughout every open app root's import
graph and fall back to disk when a buffer closes.
The shared process-local `AnalysisDb` keys parsed files by canonical path,
SHA-256 content hash, language revision, and compiler feature set. It records
direct and reverse imports, invalidates only reverse-dependent checked roots,
retains failed roots and unresolved import edges for recovery, and reports
loaded/hashed byte, source/asset metadata-probe, import-scan,
checked/reused-root, indexed-symbol, codegen-root, and phase-timing counters.
Every retained root also owns lexical-link identity, resolved-target identity,
metadata, and content hashes for its complete non-overlay source and host-asset
closure. Semantic queries validate those inputs on a bounded epoch before
returning a shared result, so correctness does not depend on a language client
supporting, accepting, or reliably delivering file-watch notifications. The
unwatched/rejected safety epoch is 750 milliseconds; an active watcher uses a
five-second content-verification backstop for dropped events. Requests within
an epoch perform no disk probes. Metadata-only source changes are
content-verified without rechecking semantics. Focused fixtures prove an unrelated
large root is not loaded after a leaf edit, a shared fragment invalidates every
dependent root, missing/malformed/deleted/cyclic imports recover, add/rename/
remove replaces reverse edges, symlinked missing overlays resolve to one key,
an overlay close returns to disk, byte-identical content is reused, and
transitive reverse edges are retained. They also prove notification-free import
edits, same-length timestamp-preserving and atomic replacements, source and
asset symlink retargeting, stable overlay close across a root-symlink retarget,
font deletion/recreation, media-file edits, and invalid icon-byte changes are
observed.
Watcher-validated source batches reuse the unchanged parsed closure. A focused
fixture asserts one leaf edit loads, hashes, and import-scans exactly one file,
checks only its affected root, and leaves an independent root as a cache hit.
LSP diagnostics, the dev
preflight loop, `cargo ice` analysis, and each `ui-lang-build` compilation batch
own and reuse this same DB API without global or process-persistent state.
Completion, hover, signature help, code actions, definition, and rename now
query the exact retained analysis used by LSP diagnostics. An unchanged root
returns the same shared analysis allocation with zero source loads, hashes,
import scans, semantic checks, or symbol indexing; qualification candidates run
against discarded snapshots limited to the selected root closure without
copying unrelated workspace state or invalidating the retained root. The LSP
synchronizes overlay strings only on open/change/close, retains a
workspace app-root index instead of rediscovering and rereading every `.ice`
file during navigation, and carries `Arc<FileAnalysis>` through diagnostics and
all semantic request families. Pointer-identity and `dhat` allocation contracts
guard against checked-document and open-overlay copies. A 500-node mixed-request
performance contract uses a nonempty workspace plus an open imported fragment,
exercises all five request families including navigation, and proves zero
source loads, hashes, import scans, semantic checks, workspace rescans, or
workspace source reads under explicit wall-time and heap-allocation budgets. A
1,000-file real-disk closure contract proves repeated requests perform no
metadata calls inside the validation epoch, and a many-root/many-alias `dhat`
contract exercises the actual qualification branch.
The server dynamically registers a `**/*` workspace watch so both Ice sources
and arbitrary font/icon asset paths are covered, records pending, accepted, and
rejected registration state including the client error, and treats events only
as eager hints. Workspace-index completeness follows that state: rename forces
a complete rescan even when an active watcher may have dropped an event, while
ordinary navigation uses watcher-specific bounded validation epochs. A relevant disk change refreshes the affected input and
reverse-root set before semantic requests reuse the cache, including read
failures and deletions, while an open overlay continues to win over disk
notifications.
Successful analysis reports unreachable components and handlers,
readerless/writerless state using only reachable handler accesses, immediate
and future/task/query/stream/progress routing cycles, unfiltered raw-event redraw
feedback, positional stateful component identity, retained dynamic state,
id-less component calls that hide widget targets, and unused bindings. Constant
no-ops/dead gates and unreachable statements include
preset boot statements; statically disabled subscriptions are excluded from
duplicate-delivery warnings. Component and handler reachability is combined
across all workspace or open-editor roots. `cargo ice` additionally reports
workspace `.ice` files outside every root graph as CLI-only `W010`. Cargo JSON
diagnostics from marked generated Rust regions map back to root and imported Ice
syntax for `cargo ice` commands. The LSP `ice.lint` workspace command publishes
the same mapping for error-level Clippy/rustc diagnostics at their `.ice`
document URI and source range. Warning-level backend findings are suppressed at
the generated item boundary so they do not pollute consumer Clippy output;
core semantic warnings continue to be published directly by the language
checker, while `W010` remains `cargo ice`-only. Consumer build scripts generate
every Ice root below Cargo's package/profile/target-scoped `OUT_DIR`; the proc
macro only includes those outputs. Generated filenames use the full SHA-256 of
the normalized manifest-relative root, and a versioned manifest is the
executable hash-to-source inventory used for collision detection and stale
output pruning.
The schema-v2 inventory also stores each generated content digest. A
directory-scoped cross-process lock covers manifest load, compile, staging,
and publication; changed outputs and the manifest are flushed and synced in a
private transaction directory, outputs are atomically replaced before the
manifest, and the manifest is the final atomic commit. Unit contracts prove
that a later compile error publishes none of an earlier root, corrupt manifests
and interrupted output replacement cause full cache regeneration, stale
transaction artifacts are removed, concurrent publishers retain both roots,
and an unchanged pass preserves output and manifest mtimes.
The Linux native job additionally starts real, separate `cargo ice dev` and
`cargo check` processes against the same `showcase` target while the dev build
fingerprint forces generation. It requires both commands to complete their
overlap, requires the distinct dev-fingerprint and normal-check output caches,
validates every manifest content digest and absence of transaction debris, then
proves the dev process shuts down cleanly. This complements the deterministic
same-directory lock tests with the actual Cargo command boundary.

App, implicit mount, component, and preset handler bodies now cross a complete
normalized HIR boundary before Rust emission. Stable typed arenas own handlers,
preorder statements, immediate and flow tasks, body routes, checked locals,
latest/replace lane members, and explicit lane invalidations. Route payloads
retain ordered indices and concrete types; tasks retain output/error types and
finality; every node retains a root or imported origin chain. Handler code
generation has no statement-AST
expression fallback, checker type query, extern name rediscovery, or source-line
async identity. Snapshot, post-check mutation, invalid-state, imported-marker,
and compiled fixtures guard those invariants. An ignored full-pipeline
500/4,000-statement contract records exact zero handler type rechecks, checked
scope full clones, and codegen full environment clones while enforcing linear
output and wall-time growth.

Handler bodies also support one final exhaustive `match` over a fieldless UI
enum. The parser admits only named `Enum.variant` arms; checking rejects missing,
duplicate, foreign-enum, payload, and wildcard patterns; normalized statement,
expression, task, route, reachability, lifecycle, and source-fact arenas retain
each branch. Rust emission uses an exhaustive native `match` with no fallback.
The `handler-enum-match` compile fixture pins that shape, and the showcase
runtime fixture proves an expression in an unselected arm is never evaluated.

The runtime `RichTextEditor` uses caller-owned `ContentVersion` identity to
skip full native-buffer materialization for caret and selection layouts.
`EditorChange` optionally supplies an exact `from`/`to` content-version pair and
logical-line replacement span. The fast path is accepted only when both
versions match the cached and current layout, retain the same document identity,
and the span passes overflow, bounds, and line-count checks. Skipped or batched
revisions, stale hints, document replacement, and active composition use exact
diff discovery. Accepted spans perform zero mapping-discovery comparisons;
styled-signature comparisons and stateful highlighter work are counted
separately. The markdown editor derives these transitions from its real edit,
undo, redo, selection-replacement, and IME-commit history.

The explicit 100,000-line contracts separately drive 1,000 caret layouts,
1,000 pointer drag events, `ㅇ → 으 → 응` preedit, one-character insertion,
viewport resize, and a format-key-only formatting change under wall-time
budgets. The insertion timer starts immediately before the native content
mutation, so its budget includes mutation, materialization, parsing, and rich
layout. The contracts record materialized source bytes, owned parsed-line
strings and bytes, owned styled text, line-vector slots, mapping and
styled-signature comparisons, highlighting, rebuilding, and shaping.

A separate release-mode integration-test process installs the safe `dhat`
allocator wrapper and starts its profiler only after the initial 100,001
logical lines have been shaped. Monotonic `HeapStats` snapshots give the total
allocation count and requested bytes routed through Rust's global allocator
for each exact operation scope, including allocations in runtime dependencies.
The wrapper keeps unsafe allocator implementation out of this workspace's
forbid-unsafe source. These totals do not include the fixture setup, resident
memory, GPU/driver allocations, or native allocations that bypass Rust's
global allocator. They are therefore allocator-request evidence for the hot
operation, not a whole-process or physical-memory measurement. Each operation
and heap record is flushed and synced before its budget is gated. The runner
continues through the remaining scenarios after a budget failure, then reports
the aggregate failure, so the uploaded evidence retains every measurable
actual value. CI rejects duplicate JSON object keys at any nesting depth,
non-finite values, and the existing schema, identity, numeric, and budget
violations before accepting the strict 12-line JSONL artifact. The same gate is
reproducible with `scripts/editor-performance-contracts.sh [artifact-path]`.

Text revisions still materialize and parse the native buffer, line layout
still prepares O(N) slots and top offsets, and a stateful highlighter or format
change may rescan the suffix or whole document; the counters and allocator
totals make those remaining costs explicit.

The checked public package contract is separately executable through
`cargo ice api`: a declaration-only or application root produces a sorted,
versioned SHA-256 JSON fingerprint over component, recipe, theme, type, and
extern surfaces, retaining imported namespace identity without source paths or
backend/HIR details. `cargo ice api diff` validates artifact schema, hash,
canonical ordering, unique names, and required/default consistency, emits human
or machine-readable breaking/behavioral/additive classifications, and fails on
breaking changes. Focused contracts prove that adding a named event to an
existing component is breaking because its routes are closed, while adding a
new component that already owns events remains one additive component change.
The extracted-crate downstream fixture runs the packaged `cargo-ice` binary to
prove deterministic emission, a zero JSON diff, rejection of that named-event
change, and rejection of a corrupt fingerprint outside the workspace.
Pull-request CI requires an exactly regenerated `ui-lang-components` artifact,
compares it with the target commit's reviewed baseline, and accepts a breaking
result only through a maintainer-controlled label event for the latest head.
The release packages job independently regenerates the artifact and requires
byte equality plus a zero JSON diff before a tag can publish. This is tooling
evidence over the existing Core contract, not a new syntax or LSP capability.

`cargo ice bundle` turns a checked app into an installable artifact in the
format its host platform knows: a signed, notarized `.app` and `.dmg` on macOS,
a `.deb` carrying a desktop entry and hicolor icon theme on Linux, and a
per-user `.msi` with a Start menu shortcut on Windows. Identity is not restated
to get one: the `app` name becomes the product name, its `id` becomes the
bundle identifier, desktop-entry name, and registry key, and the Cargo manifest
supplies the version, description, authors, and homepage each packager needs.
Only icon, category, copyright, and minimum system version are declared in
`[package.metadata.ice.bundle]`, where an unknown key fails instead of doing
nothing. One SVG becomes every raster the three ask for — the `.icns` entry
table, the `.ico` directory, and the hicolor sizes — and because the renderer
carries no fonts, an icon still holding a `<text>` element is refused rather
than drawn with a hole in it. Host-independent contracts cover those three icon
containers, the `Info.plist` keys Gatekeeper reads, the desktop entry and
Debian control stanza, the Windows Installer authoring including its stable
upgrade code and the versions it cannot compare, the layout a rebuild leaves
behind, request parsing, per-target binary paths, and the refusal to notarize
an ad-hoc signature before the upload rather than after it. Two demands are
checked before a build starts because both are invisible until installation: a
Windows crate root must set `windows_subsystem`, or the installed application
opens a console window behind itself, and a Debian package name must be one
dpkg accepts. One contract resolves the real showcase manifest and Ice root
into a complete identity for all three platforms, so a renamed icon, a dropped
`id`, or a missing manifest field fails in ordinary CI rather than on a release
tag. Linux and macOS CI additionally drive the real packaging tools —
`dpkg-shlibdeps` and `dpkg-deb`, `codesign`, `ditto`, and `hdiutil` — and
verify the resulting package and disk image. A tag builds both Apple
architectures and joins them with `lipo`, installs and removes the `.deb`
through `apt` and validates its desktop entry, installs and removes the `.msi`
through `msiexec`, and publishes attested, checksummed artifacts. Those
artifacts are signed ad hoc by decision, so the notarization and Authenticode
paths the command supports are exercised by their own contracts rather than by
this repository's releases. This is distribution evidence over the
existing Core contract, not a new syntax or LSP capability.

Every `cargo ice` command runs its analysis on a thread that asks for eight
megabytes, because a Windows main thread carries one and the showcase graph
needs more than that. A Linux contract reproduces that budget with `ulimit -s`
and proves the analysis survives it, so the platform-specific abort cannot
return unnoticed.

`cargo ice dev` exercises that same ahead-of-time path. `-p PACKAGE` discovers
the package's unique Ice app or daemon root. Content stamps cover the selected
Ice import graph, embedded fonts, icons, and media files,
participating project
Rust packages, Cargo manifests and lock/config/toolchain files, rustc dep-info,
and build-script `rerun-if-changed` inputs. The dep-info resolver accepts both
hard-linked executables and byte-identical copies produced on macOS while still
rejecting Cargo's aggregate dep-info. Native notification tests prove that
a source edit wakes the runner while an idle wait performs no snapshot poll.
Injected NFS, unsupported-mount, and inotify-limit creation failures prove the
runner selects its 750-millisecond polling safety mode; fallback tests cover
changed, created, and deleted imports and Rust build inputs while proving that
the metadata trigger performs no content reads before the existing two-pass
stamp verification. Configuration-change plus periodic-rescan tests cover the
other complete-snapshot safety paths. Selective-snapshot tests prove that a known
Rust file or source-only Ice edit performs two content reads regardless of graph
size, while new and removed files refresh the inventory. The second settled Ice
read is retained as the analysis input; an explicit 10,000-source performance
contract requires exactly one loaded, hashed, and scanned file, one checked
root, and an independent root cache hit within five seconds. A changed snapshot
is settled and built while the accepted process remains alive. The shadow
executable is adopted only after its generated root completes a draw, atomically
publishes the runner's exact readiness token, and is confirmed alive; failure
and timeout tests keep the previous process and clean the candidate. This is
process replacement, so no application, window, or widget state-preservation
coverage is claimed.

Deterministic semantic and render-inspection tests continue to use the
headless tiny-skia backend. A separate native CI matrix forces iced's `wgpu`
compositor with no tiny-skia fallback and requires the generated root to
publish its exact readiness token after the first child draw. Linux and Windows
boot the component showcase through Vulkan and DX12 respectively, while macOS
boots the native Markdown editor through Metal. The harness fails on early process exit,
malformed readiness output, renderer initialization failure, or a 60-second
first-draw timeout, and requires the process to remain alive for one second
after readiness so fatal submission/device errors cannot pass on the draw
callback alone. This is a native startup and first-frame contract, not a
cross-platform pixel-golden claim.

Source graphs support both bare fragment imports and aliased module imports.
Aliases preserve checked `::` identity for components, recipes, extern
functions/types, and fonts, including nested imports and repeated imports of
one canonical file under distinct aliases. Theme tokens remain the single
app-global contract. Definition and rename operate on the source spelling while
retaining the namespace prefix at each call site.

Top-level derived values are checked, cycle-free pure expressions over app
state and other derived values, including declared typed `pure` extern calls;
generated computations remain read-only without a runtime reactive graph,
persistent cache, handler-maintained mirror, or cross-frame retention. Their
observable evaluation count is not guaranteed; codegen may coalesce equivalent
safe reads within one eager view build. Focused regression contracts cover eager
structural coalescing across template slots, outlined components, and multiple
slot chunks, plus exclusion of opaque owned, mixed structural/owned, and
deferred reads.
Derived expressions reject immediate `sync` externs and recomputation-unsafe
built-ins (`window_id.unique`, `aborted`, `debug.time_with`, `image.upgrade`, the
unqualified `encoded`/`rgba` image constructors, and animation queries with an
omitted instant). The category covers both runtime reads and calls that create a
fresh retained identity. The same built-ins remain accepted in top-level app
state initializers, handlers, and views.
Handler-local `let` values use the same closed typed
expression language, are immutable and non-shadowing, and remain available to
later assignments, guards, and the final task. Parser, checker, codegen, schema,
README, and reference-app tests are direct evidence for both constructs.
Every handler Future and stream names its delivery mode. `run every` delivers
every Future completion and `stream every` delivers every stream item without
a lane. `run latest`, `run replace`, and `stream replace` require a static
owner-scoped lane; bare handler `run`/`stream` and `stream latest` are rejected.
Subscription `run` and task-flow `run`/`stream` sources retain their distinct
source syntax without a directly routed delivery mode.
Named delivery lanes join starts with the same fully qualified lane name,
effect kind, and mode across handlers and source locations for one state owner.
Apps own one top-level scope, daemons share one scope across their windows, and
each component instance is independent. Component lanes support direct or
nested `run latest`, `run replace`, and `stream replace`; component
`stream every` and every handler stream nested under `abortable` are rejected.
`run latest lane=<name>` filters stale success and failure delivery without
canceling old work; `run replace` and `stream replace` also abort the prior Iced
task without rolling back performed effects or stopping detached backend work.
A replacement stream retains one handle across items and releases it only at
natural termination, replacement, invalidation, or owner drop. Static qualified names
keep bookkeeping finite per owner; component-owner count follows the existing
retained/mounted lifetime contract, while runtime message queues and detached
work remain outside that bound. If an outer abort suppresses a Future
replacement completion, one fixed current handle remains until replacement,
invalidation, or owner drop; handles do not accumulate per owner. A direct
`invalidate lane=<name>` advances an
existing owner-scoped lane before aborting replace work, so already queued old
completions/items are stale. Parser, checker, formatter, normalized HIR,
terminal-envelope codegen, schema/LSP completions and route actions, controlled
cross-handler/component lifecycle tests, production stream migrations, and an
Apple Music queued-completion integration test provide direct evidence.
Core view control includes checked `if`, `for`, first-match literal `match`
arms, and exhaustive Option/Result/UI-enum payload patterns. UI enums are
non-generic, non-recursive cloneable data; fieldless enums support equality,
while payload enums remain match-only and match payloads are block-scoped.
Components may own ordinary cloneable state and local handlers, including
Future externs and instance-owned Future/replacement-stream delivery lanes.
Writable component inputs are explicit `bind` props; calls use `<->` with a
direct app state, component-local state, or forwarded bind prop. Ordinary props
never carry write capability.
Components expose closed checked contracts: named events carry zero or more
ordered typed payloads, every call site routes each event in caller scope, and
direct app-handler references from component bodies are rejected. The single
typed `->` output remains the default-event shorthand.
The normalized program stores component definitions, calls, and view topology
by typed IDs. It orders props and applies defaults, resolves each bind to an app
state, component state, or forwarded bind prop, converts every event entry to a
direct or forwarded route, orders required/optional slots, fixes component
scope and storage, and records imported physical origins before Rust generation
starts. Structural tests cover direct and forwarded events, defaults, writable
references, required/optional/provided slots, retained and mounted storage,
explicit/implicit identity, nested namespaced imports, and root/import origin
paths. A 10,000-call fixture and a 2,000-call wide-contract fixture enforce the
indexed `O(P + E + S)` work per call and the compile-time non-cloneable
component contract under a two-second debug-build lowering budget;
representative generated-Rust and source-map tests provide backend evidence.
Ordered widget payload routes, including sensor show/resize dimensions, may
emit those named events directly from a component view.
An explicit `forward` block accepts only outer events with the exact same name
and payload signature; wildcard and verbose identity forwarding are rejected.
Component contracts also support required and optional slots; missing optional
slots lower to no child, and `provided(Name)` is folded at each call site.
Canonical `with` metadata blocks preserve long checked property and utility
lists without changing the view tree; the formatter alone decides inline versus
wrapped form and orders metadata before events, forwarding, slots/statuses, and
content.
`lifetime mounted` prunes disappeared scopes, dropping local state, delivery
lane generations, and abort-on-drop handles;
the default `retained` lifetime preserves state for the app lifetime.
Generated state is isolated by hierarchical component ID. Structured native
status styles inherit the matching `active` fields before applying the
interaction-specific delta.

Top-level semantic style recipes are native Core declarations in 2.0. They
package checked utility tokens for one declared target (`col`, `row`, `flex`,
`grid`, `stack`, `box`, `text`, `input`, or `button`), optionally specialize
one same-target base, expand base-first across imported source graphs, preserve
child and later-utility precedence, and let direct typed node properties
override recipe defaults. Scaled utilities and exact-pixel spacing,
radius, and decimal text sizes share that checked lowering path. The
button-only `focus-visible:border-*` variant styles the accessible wrapper's
origin-aware keyboard focus ring — pointer-acquired focus never paints it,
keyboard/accessibility/programmatic focus always does, and a key press on a
pointer-focused control restores it, matching the web's `:focus-visible`
semantics; text-entry controls keep their native focused rendering, which is
focus-visible whenever focused. Runtime unit tests cover both origins and the
key-press restore, a compile fixture covers the ring lowering, a diagnostic
fixture rejects the variant off buttons, and the showcase `focus_visible`
first-class test proves click-no-ring/Tab-ring at the painted-quad level.
Button-target
text size, line-height, family, and weight utilities lower onto the generated
text for compact string labels; arbitrary child content retains explicit
ownership of its own typography. Every recipe
is checked at declaration time, including unused imported recipes. Parser,
checker, and codegen tests
cover expansion, typed overrides, compact button label typography, explicit
child typography ownership, invalid bodies, duplicate
declarations, target mismatch, and semantic disabled button background/text
overrides. Recipe definitions and references also
participate in cross-file LSP definition and safe rename. The workspace-local
`ui-lang-components` interface and showcase compile through the same recipe path, and a
focused test proves its Ice palette matches the retained Rust `LIGHT` palette.
The private HIR assigns recipe, style-use, target, and variant IDs and stores
each recipe as a cycle-free, base-first semantic patch. A style use merges that
fixed-size patch and its direct utilities once during lowering; Rust generation
does not call `expand_styles`, search recipe names, walk inheritance, or parse
utility strings. Structured lowering tests cover three-level inheritance,
later recipe and direct-utility precedence, every supported interaction
variant, exact pixels, typography, token opacity, invalid checked-state
invariants, and namespaced imported origins. The explicit performance fixture
normalizes 128 theme tokens, 256 deeply inherited recipes, and 10,000 uses in
under the two-second debug-build lowering budget while retaining zero inherited
utility copies per use.

The LSP contract uses the core error-tolerant cursor-context model and is covered by focused protocol tests for cursor-scoped
completion, checked component and extern signatures, base-first recipe hover,
and direct workspace-edit code actions for bindings, named events, handlers,
fallible routes, accessibility labels, repeated-utility recipe extraction,
closed-component event routing, exhaustive typed-match arms, unambiguous import
qualification, and multiline metadata. Completion and component hover retain
optional-slot and theme-contract context.

Fixed-height `VirtualList` is an explicit typed-runtime boundary, not a Core
coverage claim. Runtime tests cover unique-key reconciliation, reorder/delete,
empty and out-of-range behavior, owned non-`Copy` keys, mouse focus/selection,
focus transfer to sibling lists and inputs, actual child-capture and native
scrollbar touch/mouse precedence from a fresh native offset, scrolled row taps
with an unavailable or unrelated cursor, all six keyboard movements,
first-layout/remount/zero-offset programmatic scroll, measured fresh-mount and
resize viewport events, touch taps, interactive-child cursor semantics, and
AccessKit collection name/focus/count/active-descendant plus mounted-item
position/size/selected state. Retained typed-key semantic identity stays stable
across reorder and remains distinct under adversarial key-hash collisions and
duplicate logical list names; mounted widget state follows the same keys across
reorder and one-row mounted-window slides. Explicitly forked retained state
requires a distinct logical name and receives a new native and semantic
namespace; a concurrent headless-driver test proves each list and row selector
has exactly one match using canonical helper selectors and a list name shaped
like an old row path. Separate constructors with duplicate logical names retain
native and accessibility namespace safety under the documented caller-unique
selector contract. Release 100,000-item CI contracts separately measure
unchanged build/diff/layout/draw frames, constant-time `update_snapshot` plus
`Scrolled` reducer replacement, and explicit full reconciliation with p50/p95
wall-time and instrumented allocation budgets. The reducer path requires zero
allocations and bytes for scalar keys; rendering retains the visible+overscan row
callback and exact mounted child-slot budgets.
These interaction contracts use a bounded-height mount with no vertical
scrolling ancestor: the list owns the tested native scroll offset and viewport.
Arbitrary standard Iced scrolling ancestors are explicitly outside v1 because
Iced 0.14 does not pass descendants enough information to map raw touch events
through an unavailable or unrelated cursor. The runnable showcase keeps the
list in a fixed non-scrolling region and gives only the catalog below it an
independent vertical scrollable.
The extracted-crate downstream consumer compiles and executes the public
runtime and `ui-lang-components` boundary.
The showcase consumes it through a typed Ice extern and first-class tiny-skia
capture; direct `ui-lang-runtime` and `ui-lang-components` minimal-feature checks cover
native X11 and wasm, and the extracted runtime package repeats the direct native
`virtual-list,x11` contract. Bare `virtual-list` intentionally leaves native
platform selection to the caller. The
Windows native WGPU job requires a renderer primitive from a measured mounted
row subtree before accepting the first frame. V1 explicitly excludes
variable-height measurement, scrolling-ancestor touch transforms, and new Ice
syntax.

Fixed-height `LogTimeline` composes that exact `VirtualListState` boundary
under the existing runtime `virtual-list` feature. Focused tests cover default
tail following, exact live-edge synchronization, pause after upward native
scroll and historical keyboard navigation, explicit-only resume, saturating
unread append accounting, stable selectors across append, typed-key scrolling,
atomic duplicate/history rejection, explicit stream replacement, and bounded
headless windows for 100,000 caller-owned rows. A separate ignored release
contract measures a 100,000-row prefix validation, keyed reconciliation,
single-row append, and inspection with p50/p95 time and allocation budgets.
The Ducktape wrapper has a minimal-feature import/build test and inherits the
runtime list's mounted-only AccessKit collection/item contract. This is not a
second transcript scroller: unlike variable-height `MessageScroller`, it has
no measurement, message anchors, prepend restoration, or built-in jump control.

Fixed-height `TreeView` reuses the same mounted-window engine and has separate
runtime evidence for atomic preorder validation including referenced-leaf and
closed-subtree rejection, retained expansion, collapse selection rehoming,
lazy-load requests, hierarchical Left/Right navigation,
rename commit/cancel, drag-target geometry, canonical selectors, and 100,000
logical nodes with visible-plus-overscan mounting. Headless AccessKit evidence
checks Tree/TreeItem roles, level, sibling position and size, expanded state,
selection, and mounted-only node count. Release contracts measure unchanged
100,000-node rendering, flat and maximum-depth preorder reconciliation,
late-key hierarchical toggle/navigation, and the constant-time `update_snapshot`
plus scroll reducer with zero allocation for scalar keys. The showcase consumes
`TreeView.Frame` through a typed extern; a first-class Ice test exercises
hierarchical navigation plus rename focus, commit/cancel, and tree-focus
restoration. Native and wasm minimal-feature checks
compile the public boundary, and WGPU readiness requires both VirtualList and
TreeView mounted-row draw probes.

Fixed-height `DataGrid` reuses the mounted-row engine without extending Ice
Core, `Table`, or `DataTableState`. Runtime evidence covers atomic duplicate and
invalid-width rejection, stable typed row and column identity across reorder,
single active-cell and row selection, all directional/row/grid/page keyboard
movements, two-axis reveal, constant-time key lookup and `scroll_to_cell`, typed
sort requests, and caller-owned edit begin/commit/cancel. Interactive children
receive pointer and key events first: a captured editor click owns focus
exclusively, and Escape/Tab cannot leak following arrows back to the grid. This
preserves native text input, IME, submission, and control chords. Headless
inspection proves visible and mounted row ranges, complete
fixed-column geometry, mounted row/cell counts, active/selected/editing state,
viewport geometry, and both offsets. Mounted-only AccessKit tests cover the
Grid, header Row, ColumnHeader, data Row, and Cell hierarchy, total row/column
counts, one-based indexes, selected state, caller-supplied sort direction,
stable semantic identity, and mounted active descendant.

Release contracts separately measure unchanged 100,000-row by 16-column
build/diff/layout/draw frames, full reconciliation, and the constant-time
`update_snapshot` plus scroll and `scroll_to_cell` reducer path with zero scalar
allocations. Native and wasm minimal-feature checks compile the runtime and
themed boundary, and the extracted runtime package repeats the direct
`data-grid,x11` contract. The showcase
owns rows, sort direction, draft and committed cell values behind a typed Ice
extern; its first-class test covers keyboard focus, native editing, commit, and
navigation. Windows WGPU readiness additionally requires the DataGrid mounted
cell draw probe. V1 excludes variable-height rows, variable/resizable or
virtualized columns, range selection, frozen data columns, and new Core syntax.

Component contracts in 2.0 support checked prop defaults. Missing named
arguments use pure closed expressions that cannot capture app state, component
state, or parameters; declared `pure` extern calls are allowed, while `sync`
calls, recomputation-unsafe built-ins, bind props, and mutable component-only
values cannot be defaulted. Component state initializers apply the same
`pure`/recomputation-unsafe boundary because rendering may evaluate them again;
the forbidden set includes the unqualified `encoded`/`rgba` image constructors.
Required props must precede defaulted props.
Parser, checker, formatter, and codegen tests cover omission, override, type and
capture errors, and mutable-value rejection.

Theme contracts and dynamic palettes are native Core declarations in 2.0.
The checker requires the four Iced base tokens, rejects incomplete or
contract-mismatched palettes and unknown/duplicate/non-color entries, and type
checks the app's active palette expression as the nominal `palette[Contract]`
type. Generated exhaustive code selects one complete color table per view with
no string fallback and uses it for both the custom Iced theme and all
semantic-token style callbacks. Parser, checker, codegen, schema/LSP, formatter,
example, and workspace compilation tests provide the executable evidence.

## Normalized compiler boundary

The production compiler follows one path from checked source to generated
Rust: the checker constructs `CheckedDocument`, lowering consumes it into an
owned `LoweredProgram`, and code generation accepts only that normalized
program. Release `LoweredProgram` values contain neither the source `Document`
nor `CheckedFacts`; test builds retain poisonable sidecars solely for boundary
tests.

Stable typed arenas own declarations, expressions, values, locals, handlers,
statements, tasks, views, routes, subscriptions, tests, components, styles,
themes, and physical origins. Lowering resolves declaration identity, Rust
targets, defaults, coercions, lexical ownership, expression DAGs, static view
topology, route payload order and types, style/theme tokens, and imported source
locations before emission. The expression backend consumes
`ResolvedExpressionProgram` directly and has no checker-fact, type-query,
extern-name, or raw-expression fallback. Canonical semantic values such as
`Type` and `Span` remain shared compiler types; they carry no AST topology or
checker state.

The `hir_boundary` integration ratchet requires an empty production
code-generation inventory for:

- source-AST imports and semantic references;
- checked-document, raw-document-wrapper, checker-fact, and declaration-index
  escapes;
- checker semantic references, type re-analysis, extern re-resolution, and raw
  expression fallback;
- direct `Document`, `Expr`, `Route`, and `Statement` references.

Its dependency-free scanner ignores comments and literals, discovers top-level
AST exports, follows qualified imports and aliases, and fingerprints each
occurrence by containing item and call site. Focused scanner tests cover
same-named local items, grouped and local imports, alias chains, non-ASCII
identifiers, and delete/add relocation.

Executable boundary evidence is grouped by ownership rather than backend
implementation stage:

| Area | Evidence |
| --- | --- |
| Declarations, settings, components, styles, themes, subscriptions | normalized-ID and structural snapshots; defaults, routes, storage, palettes, extern targets, windows, and imported origins; corruption and source-marker tests |
| Expressions, handlers, tasks, tests, and control flow | complete arena consumption; owner/scope/type/DAG validation; statement and route payload ordering; post-check and post-lowering poisoning; generated-program fixtures |
| Views and widgets | normalized static topology and expression partitions for Canvas, media, content, controls, selection, layout, collections, wrappers, interaction, extern adapters, and nested themes; malformed-ID and same-arena identity attacks; native Rust output fixtures |
| Diagnostics and source mapping | shared `OriginId` parent chains, imported physical paths, source-mapped `E196`, and exact generated source markers without AST location recovery |
| Scale | ignored 500-to-4,000-node and 10,000-call contracts bound lowering plus emission, arena lookups, environment cloning, allocations, output growth, and wall time |

The detailed parser, checker, formatter, generated-Rust, runtime, and fixture
coverage for each public construct remains recorded in its feature section and
in the tests themselves. This ledger treats a feature as backend-complete only
when its release emission is represented by the normalized boundary above.

First-class Ice tests are native in 2.0. Top-level `test` declarations reuse
normal presets, components, checked IDs, expressions, handlers, subscriptions,
and real Rust externs. All semantic operations lower through the public,
raw-event-independent `testing::Action` enum and one `Driver::perform_action`
entry point rather than exposing the private generated application message.
The private test HIR assigns stable test, target, and step IDs; retains target
aliases as typed locals; and gives every dynamic path key and action/assertion
operand a deterministic checked expression owner. Configuration and path/action
topology are frozen as semantic keys, direct paths carry checked key-expression
IDs, dispatch carries an exact App handler ID/name/signature, and equality
expectations retain the checked comparison children. Lowering revalidates
complete arena consumption, origins, owner scope, expression graphs and types,
numeric/index/positive constraints, aliases, and handler identity. Production
test emission consumes only `ResolvedTest` records and checked expression IDs;
it has no `TestDecl`, `TestStep`, raw `Expr`, or raw route path. Structural and
corruption tests cover stable IDs, missing owners, config/target/step mutations,
post-check raw-expression poisoning, exact retained source text, imported
locations, and all semantic action families. An ignored 4,000-step contract
bounds combined lowering and Rust emission.
A persistent headless Iced cache drives click, hover,
press/release, pointer buttons and coordinates, wheel/scroll/drag/drop, exact
focus, held keys/modifiers/chords, typing/selection/IME, touch, window/system/file
events, dispatch, update, bounded time, deterministic redraw advancement,
accessibility actions, and recursive task completion. Assertions cover state,
presence, exact visible text and input content, AccessKit semantics, computed
layout bounds, primitive counts, text/image bounds and baseline, scale-aware
pixel alignment, focus, and unambiguous structured tiny-skia paint output;
named captures persist PNG plus a versioned JSON frame manifest with separate
configured, resolved-render, and system theme fields, and retain RGBA output
for runtime callers. The capture draw updates the same native widget tree with
a redraw request first, so status-aware controls do not fall back to disabled
paint; redraw-emitted messages remain unapplied and capture stays
observation-only. Generated identified targets retain their originating
imported `.ice` path, line, and column. `cargo ice inspect` activates an
otherwise inert generated entry for one real app `Program`, fixed environment,
and preset. Its release-only trace modes measure authored tests,
deterministically generate semantic actions from a refreshed live inventory,
or replay an exact artifact environment/sequence on a fresh boot. The strict
trace-schema-1 artifact retains raw phase samples and tail summaries,
source/target provenance, stable confirmed-finding fingerprints, strictly
smaller reductions where one exists, and untimed worst-state PNG/manifests. A
seeded stateful runtime fixture proves discovery, confirmation, fresh-boot
replay, and dependency-preserving reduction; a mutation disabling only its
injected cliff is Red at the finding assertion. `cargo ice diff` externally
compares structured values and RGBA
pixels and writes JSON/PNG reports. `cargo ice review` selects declared Ice
tests, records their exact process results and captures, reuses the same diff
engine, summarizes live AccessKit metadata, and maps structured changes back to
the target or capture statement source. Unit contracts cover option/test
selection, HTML escaping, accessibility aggregation, source mapping, and the
shared pixel/manifest comparison. Direct diff and review share a typed capture
schema-2 validator for required fields and core nested provenance, geometry,
accessibility, and paint shapes. Typed review-schema-2 baseline tests reject
wrong artifact kinds, failed reports, malformed capture entries, duplicates,
and unsafe paths. Run-ID failure tests prove stale success is replaced while a
current detailed failure is preserved. Pull-request CI exercises a full
showcase bundle, a selected comparison whose unselected baseline path is
invalid, and a full-scope removed capture failure; it verifies accessibility
and artifact paths and uploads the bundle. macOS and Windows CI also execute a
baseline-free selected review. Screenshot output is checked as RGBA8 and
capped at
16,777,216 physical pixels before renderer allocation.
The artifact root defaults to `target/ice-test-artifacts`, is replaceable with
`ICE_TEST_ARTIFACT_DIR`, and still isolates each test; the runtime configuration
can also select an exact per-test directory. Test configuration can replace the
headless program theme result with `Theme::default(mode)`, override its scale
factor, and pin locale/platform/reduced-motion metadata; application-owned
palette state still changes through a preset or dispatch. Rust harnesses may
independently pin the startup system-theme query with `Config::system_theme`;
later theme notifications remain semantic actions. The single headless current
window keeps widget state across rerenders, while a task-issued window open
starts a fresh widget cache and window-local input lifecycle.
Targeted focus, scroll, selection, and cursor operations validate the native
widget capability they invoke, reject ambiguous candidates, and use the actual
matched widget ID. Convenience taps allocate around retained multi-touch
contacts instead of reusing an active finger ID.
Absolute and earlier-alias-relative test targets, definition, and rename stay
within one test, and generated runtime failures retain imported `.ice` paths and
lines. Parser, checker, formatter, codegen, runtime, schema/LSP, reference
examples, and invalid/runtime failure tests provide direct evidence. The test
runtime has no general virtual clock or built-in pixel-golden comparator;
comparison policy belongs to `cargo ice diff`.

## Measured coverage

The scoped implementation score is **100%** on all three executable inventories:

- **59/59 public ledger rows are native.** No row below is `partial` or
  `missing` for the pinned iced baseline.
- **48/48 render-node kinds have a runtime witness.** The dedicated
  `render_surface.ice` contract keeps every branch populated, resolves every
  concrete rendered node by its checked ID, asserts its computed visibility,
  and checks the rendered descendants of `if`, typed `match`, `for`, component,
  and slot nodes. Its visible-text assertions execute the complete fixture through the
  tiny-skia headless draw path. The separate `render_contract_covers_every_render_node`
  gate uses an exhaustive `ViewNode` match with no wildcard and compares that
  same reachable application graph with the exact node inventory. Adding or
  dropping a renderer node breaks the gate.
- **32/32 programmatic render-inspection fields have a runtime witness.** The
  component contracts read every public target field across identity, value,
  visibility, bounds, clipping, scroll content/translation, surface paint, and
  text paint from real post-layout and post-draw targets.

The reference Tasks, extended Showcase, and alternate-theme views also execute
the real headless draw path. First-class component contracts separately assert
all public target fields, computed layout relationships, control events,
rerendered state, and conditional overlay presence without reading pixels.

These percentages are language-surface coverage, not Rust line/branch coverage
and not every possible combination of state, theme, viewport, and style value.
Those combinations remain ordinary application test cases; claiming them as a
single percentage would be misleading.

## Accessibility

Core accessibility is **native for single-window Linux and Windows
applications** through Ice-owned AccessKit adapters for AT-SPI and UI
Automation. It remains **partial at cross-platform system scope** because other
targets do not yet export to native screen readers.

| Core surface | Delivered contract |
| --- | --- |
| `text` | AccessKit `Label` with the visible text as its value |
| `input` | `TextInput` with value, or `PasswordInput` with no exported value — permanently so for a `secret` binding; leading text is the default name and checked `label=`/`description=` may override/extend it |
| `button` | `Button` with focus/click actions and optional checked/toggled or expanded state; compact text is the default name, child content requires `label=`, and `description=` is optional |
| `checkbox` | `CheckBox` with toggled state and focus/click actions; visible text is the default name and checked `label=`/`description=` may override/extend it |
| `toggler` | `Switch` with toggled/disabled state and focus/click actions; visible text is the default name and checked `label=`/`description=` may override/extend it |
| `slider` | `Slider` with a stable default name, current value, and descendant focus action |
| `progress` | `ProgressIndicator` with a stable default name and current value |
| `pick`, `combo` | `ComboBox` with a placeholder name, selected value, and descendant focus action |
| `editor` | `MultilineTextInput` with a placeholder/default name, current value, disabled state, and descendant focus action |
| `image` | a labeled image is an `Image`; an unlabeled image is decorative and omitted, and `description=` requires `label=` |
| focus | source/view-tree read and Tab/Shift+Tab order, disabled-target skip, button Enter/Space, checkbox/toggler Space, and a visible wrapper focus outline; no numeric focus order |

### Gap: an extern's published accessibility node cannot be targeted

An `extern` component may publish its own AccessKit node from inside the Rust
adapter. Ice test targets resolve against the view tree the generator emits, and
that tree stops at the extern's wrapping node: in the trading example
`.../chart-frame/chart` is the deepest id any target can name, while the node the
adapter actually publishes — `StableId::new("trading-chart")`, an `Image` with a
summary label — is created below that boundary and is not a view-tree id at all.
No path syntax reaches it.

The consequence is one fact seen from two sides. From Ice, nothing an extern
draws is assertable: the trading chart's candles, axes, price lines and
crosshair are reachable only by `capture`, which is not an assertion, and by
Rust tests on the adapter beneath. This repository therefore has **no rendered
assertion that touches chart-drawn content**. From assistive technology, the
same subtree is one labelled `Image`: the summary sentence is exported, and
nothing the chart draws inside it is. A chart that silently stopped drawing its
content would break neither.

Closing this needs a way to address an extern's published accessibility node
from Ice — a target form that resolves into the adapter's own subtree, or a
declared contract by which an extern re-publishes named nodes into the tree the
target resolver walks. Nothing in the trading example closes it; that example is
where the gap was found, not where it lives.

AccessKit tree construction and action dispatch are deterministic on every
target. Native export is single-window on Linux and Windows. Daemon and
multi-window adapters, native export on other targets, and exact desktop
screen-coordinate bounds are unsupported on stock Iced 0.14.0. Rich text and
widgets outside the table above have no Core semantics claim. First-class
showcase tests exercise every newly mapped control role, exported state, and
action.
`scripts/a11y-smoke.sh` proves that
the Linux AT-SPI tree is discoverable and an invoked action reaches the Iced
bridge; `scripts/a11y-windows-check.sh` cross-compiles the Windows adapter and
the generated reference app's production and test forms. Headless tests cover
dispatch to the app message. On Windows, Iced's automatically created initial
main window starts hidden, windowed, and non-maximized. The bootstrap resolves
its ID with `window::oldest()`, then defers configured-mode restoration, the
selected boot or preset task, and received messages until UI Automation subclass
attachment;
it then restores the mode and releases the initial task alongside queued
messages, preserving queue order. Named windows retain their configured settings
and remain outside native export.

## Typed system reachability

Ice 2.0 Preview has thirty-four checked Rust boundaries:

| Boundary | Rust ABI | Covers |
| --- | --- | --- |
| `name(args)` | `async fn(...) -> Output` or `Result<Output, Error>` | domain I/O and arbitrary futures through native `Task::perform` |
| `component name(args)` | `fn<'a>(..., &'a T, ...) -> Element<'a, Event, Theme, Renderer>` or an owned `'static` form | any owned or app-state-borrowing widget tree using the configured theme and renderer, including custom widgets |
| app `renderer rust_path` | concrete `iced::program::Renderer` type | application-wide custom graphics renderer/compositor selection, propagated through every generated `Element` and checked by rustc |
| `selector name(args)` | `fn(...) -> impl widget::selector::Selector<Output = Event>` | custom native matching over every widget candidate with arbitrary checked outputs |
| `shader name(args)` | `fn(...) -> impl shader::Program<Event>` | native wgpu primitives, pipeline/storage, state, events, redraw, capture, and mouse interaction |
| `task name(args)` | `fn(...) -> Task<Event>` or `Task<Result<Event, Error>>` | widget/window/clipboard/font/system operations and arbitrary task composition |
| `stream name(args)` | `fn(...) -> impl Stream<Item = Event>` or `Stream<Item = Result<Event, Error>>` | native repeated `Task::run` output through explicit handler `stream every`/`stream replace` delivery, plus `Subscription::run`/`run_with` workers from channels, iterators, async generators, and other streams |
| `sip name(args)` | `fn(...) -> impl Sipper<Output, Progress>` or `Straw<Output, Progress, Error>` | native repeated progress plus one final output through `Task::sip` |
| `recipe name(args)` | `fn(...) -> impl Recipe<Output = Event>` | custom subscription identity, runtime-event input, streams, cancellation, and arbitrary recipe behavior through native `from_recipe` |
| `event-filter name()` | `fn(subscription::Event) -> Option<Event>` | native raw runtime-event filtering with an explicit hashable identity, including interaction window IDs/status and system-theme changes |
| `pure name(args)` | `fn(...) -> Output` | trusted same-arguments/same-result, side-effect-free Rust computations usable in every checked expression context, including derived values, views/settings, component defaults and state initializers, subscription filters, easing, handlers, and tests |
| `sync name(args)` | `fn(...) -> Output` | immediate effect/environment/retained-identity calls in top-level app state initializers and immediately evaluated app/component/preset handler expressions, including nested task arguments; component state initializers are excluded because rendering may recreate them. Explicit `run every`/`run latest`/`run replace` Future and `task` statement success and failure route expressions are owned snapshots of ordinary cloneable Ice data materialized at statement launch, but direct `sync` and recomputation-unsafe builtin calls remain forbidden because both branches materialize even though only one completion is delivered; evaluate either in a preceding handler `let` and route the local. Stream/sip/flow/native query route timing is unchanged |
| `subscription name(args)` | `fn(...) -> Subscription<Event>` | event, keyboard, mouse, window, system, channel, timer, stream, and custom subscription sources |
| `theme name(args)` | `fn(...) -> iced::Theme` | native app and nested default-renderer themes, including `custom`, `custom_with_fn`, and complete palette/extended-palette logic |
| `themer name(args) -> Event` | factory returning `Option<Theme>`, `Element<'static, Event, Theme>`, and optional Theme-dependent text/background callbacks | native alternate `Theme: Base` subtrees inside the default-Theme app, including `Themer::new`, default Theme fallback, event mapping, `text_color`, and `background` |
| `window name(args)` | `fn(&dyn iced::window::Window, ...) -> Output` | exact typed access to native window/display handles and other callback-only window behavior through `window::run` |
| `markdown-viewer name(args)` | `fn(...) -> impl for<'a> markdown::Viewer<'a, Event>` | native custom rendering of every Markdown item through `view_with` while preserving checked link-event routing |
| `editor-binding name(args)` | `fn(text_editor::KeyPress, ...) -> Option<text_editor::Binding<Event>>` | native custom key mapping across every built-in Binding plus typed custom application routes |
| `editor-action name()` | `fn(&mut text_editor::Content, text_editor::Action)` | in-place native edit observation for bounded history and dirty tracking without per-key document copies |
| `editor-highlighter name(args)` | generic adapter from plain `TextEditor` to default `Element` | stock native `highlight_with` access to arbitrary Highlighter settings, highlights, iterators, Theme-aware colors, and fonts; layouts that need mixed metrics or decorations use a custom widget such as the runtime `RichTextEditor` |
| `editor-style name(args)` | `fn(&Theme, text_editor::Status, ...) -> text_editor::Style` | native theme/status-aware runtime editor style callbacks, equivalent to the default Theme's advanced class representation |
| `text-style name(args)` | `fn(&Theme, ...) -> text::Style` | native theme-aware runtime text and rich-text style callbacks, equivalent to the default Theme's advanced class representation |
| `slider-style name(args)` | `fn(&Theme, slider::Status, ...) -> slider::Style` | native theme/status-aware runtime slider style callbacks, equivalent to the default Theme's advanced class representation |
| `progress-style name(args)` | `fn(&Theme, ...) -> progress_bar::Style` | native theme-aware runtime progress style callbacks, equivalent to the default Theme's advanced class representation |
| `button-style name(args)` | `fn(&Theme, button::Status, ...) -> button::Style` | native status-aware runtime button style callbacks, equivalent to the default Theme's advanced class representation |
| `checkbox-style name(args)` | `fn(&Theme, checkbox::Status, ...) -> checkbox::Style` | native checked/status-aware runtime checkbox style callbacks, equivalent to the default Theme's advanced class representation |
| `toggler-style name(args)` | `fn(&Theme, toggler::Status, ...) -> toggler::Style` | native checked/status-aware runtime toggler style callbacks, equivalent to the default Theme's advanced class representation |
| `radio-style name(args)` | `fn(&Theme, radio::Status, ...) -> radio::Style` | native selection/status-aware runtime radio style callbacks, equivalent to the default Theme's advanced class representation |
| `box-style name(args)` | `fn(&Theme, ...) -> container::Style` | native theme-aware runtime container style callbacks, equivalent to the default Theme's advanced class representation |
| `svg-style name(args)` | `fn(&Theme, svg::Status, ...) -> svg::Style` | native theme/status-aware runtime SVG style callbacks, equivalent to the default Theme's advanced class representation |
| `input-style name(args)` | `fn(&Theme, text_input::Status, ...) -> text_input::Style` | native theme/status-aware runtime text-input style callbacks, equivalent to the default Theme's advanced class representation |
| `scroll-style name(args)` | `fn(&Theme, scrollable::Status, ...) -> scrollable::Style` | native theme/status-aware runtime scrollable style callbacks, equivalent to the default Theme's advanced class representation |
| `pick-list-style name(args)` | `fn(&Theme, pick_list::Status, ...) -> pick_list::Style` | native theme/status-aware runtime pick-list style callbacks, equivalent to the default Theme's advanced class representation |
| `menu-style name(args)` | `fn(&Theme, ...) -> menu::Style` | native theme-aware runtime pick-list/combo overlay menu callbacks, equivalent to the default Theme's advanced class representation |
| `panes-style name(args)` | `fn(&Theme, ...) -> pane_grid::Style` | native theme-aware runtime panes callbacks, equivalent to the default Theme's advanced class representation |

Generated probes verify the concrete Rust signatures. Reachability is not the
same as native coverage: a row stays partial or missing until its complete
public behavior has direct documented Ice syntax and tests.

## Widgets and layout

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| `button` | native | native string or arbitrary child content, compact-label typography utilities, disabled route, optional checked/toggled or expanded accessibility state, typed size/padding/clip, all eight iced presets, every concrete field across all four statuses including linear backgrounds, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes; hands its status-resolved text ink to child content — color-less text inherits it, an svg child joins it with `color=inherit`, and an explicitly-colored text child may declare a `disabled:text-*` arm keyed on the button's status |
| `canvas` | native | declarative rectangle/circle/line/text/path geometry; complete path builder segments, fill rules, solid/linear fill and stroke, caps/joins/dashes, transforms, clips, typed `if`/`for`, complete raster/SVG frame drawing fields, dependency-keyed geometry cache with shared named groups, typed local `Program::State`, all five event families and every variant, state updates, publish/capture/next-frame/timed-redraw actions, pointer routes, and static/state-dependent/out-of-bounds interaction cover the complete public Program behavior |
| `checkbox` | native | native label/value/disabled event, size/width/spacing, text typography/wrapping, complete font descriptors and custom icon; all four presets, every concrete Style field across active/hovered/disabled checked and unchecked statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `column` | native | children, typed spacing/per-side padding, all `Length` bounds, max width, cross-axis alignment, clipping and wrapping column spacing/alignment, and `virtual-row=` viewport-bounded layout whose generated scroll synchronization cannot be outrun by a rapid wheel transaction |
| `flex` | native | dependency-free runtime flexbox with row/column reverse directions, nowrap/wrap/wrap-reverse, justify/content/items alignment, axis gaps, padding and clipping; box items support stable order, grow/shrink/basis/self alignment, and fixed/percentage/auto margins |
| `combo_box` | native | direct checked ID; native typed replaceable and incrementally pushable search state/selection, every builder setter, complete text-input icon, every concrete input Style field across active/hovered/focused/focused-hovered/disabled statuses, complete menu overlay Style fields, typed native input/menu style callbacks, and all events |
| `box` | native | native one-child container with ID, complete concrete layout API, every concrete Style field including linear background, text, per-corner border, shadow and pixel snap, plus typed theme-aware runtime callbacks covering the default Theme's advanced classes; `border-dash=` is composed rather than native — `iced::Border` has no dash style, so it lowers to a radius-tracing canvas stroke stacked over the surface in place of the solid border |
| `float` | native | one child, positive scale, all original-bounds and viewport geometry exposed as scoped f64 translation inputs, and every concrete Style field through checked shadow color/offset/blur and per-corner shadow radius |
| `grid` | native | dynamic children, pixel spacing/width, fixed columns, CSS-like minimum-cell wrapping, native maximum-cell wrapping, and aspect-ratio or all `Length` height modes |
| `image` | native | path, encoded-memory and RGBA handles; a literal relative path is a checked, tracked, compile-time asset embedded into the binary, while absolute literals and computed paths load from the process filesystem; all four iced length variants, fit, filter, floating/solid rotation, opacity, scale, expand, per-corner radius and crop cover the complete concrete widget API |
| `image::Viewer` | native | path or memory/RGBA handle, all length and fit modes, both filters, padding, minimum/maximum scale and scale step cover the complete public builder API |
| `keyed` | native | typed list template with bool/i64/f64 identity keys, automatic keyed child scopes, spacing/per-side padding/all `Length` bounds, max width and alignment, and `virtual-row=` viewport-bounded layout that carries per-row state, measured heights and focus by key while generated scroll synchronization follows every captured wheel event |
| `lazy` | native | hash-keyed rebuilds with bool/i64/str, `Hash + Clone` extern values, recursive list/optional dependencies, cheap-key `by` projections (bool/i64/str) over a state-rooted value — an app or component's own state field, a component prop every call feeds from one, or a `for`/keyed-column row over such a place — that reaches the builder by reference and is cloned only on rebuild, immutable bare-key snapshots available inside the keyed subtree, a dependency-only value scope with the enclosing component's routing context preserved (local handlers, `forward`, `emit`), `_`-only call-site payloads for lazy-delivered events, and statically enforced owned `Element<'static>` subtrees |
| `markdown` | native | owned parsed/replaced/incrementally appended content, image URI access, syntax highlighting, every `Settings` and `Style` field, str link events, and a typed custom `Viewer` boundary covering every item renderer through native `view_with` |
| `mouse_area` | native | all button/enter/move/scroll/exit events, scroll unit preservation, and all cursor interactions; `press-at=` composes the runtime press observer, which reports the local press position once per left press even when a child captured it |
| `overlay` | native | structured content/layer sections, conditional visibility, all three alignments on both axes, padding, checked backdrop color, modal button/scroll blocking and backdrop dismissal lower through native Stack/Float behavior; an open overlay is modal to the keyboard too, confining Tab/Shift+Tab to `layer`, dropping focus `content` still held, and withholding keystrokes from `content`, proved by a Tab traversal and a typed key over the published overlay node; typed owned Element adapters cover the complete advanced `Overlay` trait including layout, draw, operate, update, mouse interaction, nested overlays, and `index()` ordering |
| `pane_grid` | native | recursive initial split trees with stable named nested-split resize, closed panes, list-keyed runtime pane templates with typed dynamic references, scoped per-pane maximized callback flags, bounds, click, interactive resize/drag, maximize/query, adjacency, swap, close, move-to-edge, root resize and region drop; native Content/TitleBar, full and responsive compact Controls, per-side title padding and visibility; every concrete PaneGrid Style field including linear hovered backgrounds plus typed native runtime callbacks covering advanced classes; every concrete Content/TitleBar container Style field including linear background, per-corner border, shadow and pixel snap |
| `pick_list` | native | direct checked ID; native typed choices/optional selection, every builder setter, all arrow/static/dynamic/none handles, every concrete Style field across active/hovered/opened/opened-hovered statuses, complete menu overlay Style fields, and typed native field/menu callbacks covering the default Theme's advanced classes |
| `pin` | native | one child, all `Length` bounds and pixel x/y positioning; x/y is behaviorally identical to iced's `position(Point)` helper |
| `progress_bar` | native | native range/value, all length/girth variants, horizontal/vertical, five presets, checked solid/linear track and bar backgrounds, border and per-corner radius, plus typed theme-aware runtime style callbacks covering the default Theme's advanced classes |
| `qr_code` | native | literal or runtime UTF-8/byte payload expressions, all correction levels and normal/micro versions, cell/total size, and checked cell/background colors; the matrix is owned by the widget, so a payload minted during a view renders, and a literal one is still encoded at check time |
| `radio` | native | direct checked ID; native bool/i64/f64/str/extern payload values, explicit bool selection, checked/selected `RadioButton` accessibility state, complete sizing/typography/font setters, every concrete Style field across active/hovered selected/unselected statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `responsive` | native | arbitrary size-dependent child tree with scoped width/height bindings, breakpoint sugar and all `Length` bounds |
| `row` | native | children, typed spacing/per-side padding, all `Length` bounds, cross-axis alignment, clipping and wrapping row spacing/alignment |
| `rule` | native | axis/thickness, every fill mode, default/weak presets, checked color/opacity, per-corner radius and snap cover all concrete style fields; advanced classes are an alternate extension mechanism |
| `scrollable` | native | native content/ID, every concrete builder setter, all Viewport getters, every Status field through ordered selectors, every concrete Style field for container, rails, scrollers, gap and auto-scroll overlay, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes; `anchor-y=keep` adds what iced has no setter for — a start-anchored offset that follows content inserted above the viewport, so a list whose newest row is on top stops moving under a reader who has scrolled into it |
| `sensor` | native | show/resize dimensions route to handlers or named component events; hide, comparable owned keys, anticipation and delay; owned keys provide the same continuity behavior as `key_ref` without borrowed lifetimes |
| `shader` | native | typed factory for any concrete native `shader::Program<Event>`, complete width/height builder API, checked message routing, and generated Program/Element probes; the Rust program retains complete State, Primitive, Pipeline/Storage, update/action, draw and mouse-interaction behavior |
| `slider` | native | direct checked ID; native f64 or arbitrary typed extern numeric values with Rust-verified iced Slider bounds; complete default/normal+shift step, sizing and change/release behavior; every concrete Style field across active/hovered/dragged including solid/linear rail and handle backgrounds, border/per-corner radius and circle/rectangle handles; typed theme/status-aware runtime callbacks cover advanced classes |
| `space` | native | optional fixed/fill/fill-portion/shrink width and height cover the complete widget API |
| `stack` | native | ordered children, all `Length` widths/heights, clipping and `push_under` base-layer behavior via `under=N` |
| `svg` | native | native path or UTF-8/raw byte memory source, with literal relative paths checked, tracked, and embedded exactly like `image`; all four iced length variants, fit, rotation, opacity, complete idle/hovered color style, `color=inherit` button-content ink that keys hover on the button's bounds and disabled on its status, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `table` | native | typed cloned rows, arbitrary header/cell subtrees, automatic row/column identity scopes, all table width/padding/separator setters and all column width/alignment setters |
| `text` | native | direct checked ID on text and rich text; untracked plain text supports mouse selection across wrapped lines with platform copy/select-all shortcuts; native string/numeric text plus structured rich spans; complete Text/Rich bounds, size, relative/absolute line height, font, alignment, wrapping and color, plus Text shaping and Rich str link events; every concrete Span field including solid/linear highlight background, border/per-corner radius/padding/underline/strike; typed theme-aware runtime callbacks cover the default Theme's advanced classes; `tracking=` is composed rather than native — iced carries no letter spacing, so a non-selectable tracked run lowers to one text widget per grapheme in a spaced row while retaining the complete accessibility value |
| `text_editor` | native | app-owned direct or explicit `bind` component-prop content, generated or typed adapter action application, pure cursor/line/selection inspection, every concrete builder setter, all five built-in themes, typed arbitrary native Highlighter adapters, complete native key bindings with custom routed payloads, every concrete Style field across all statuses, and typed Theme/Status callbacks covering advanced classes |
| `text_input` | native | app-owned, explicit `bind` component-prop, component-local str binding, or a declared `secret` buffer held outside application state, ID, every concrete builder setter, complete custom icon, every concrete Style field across active/hovered/focused/focused-hovered/disabled statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `themer` | native | default/app/all 22 built-in and arbitrary typed `Theme: Base` subtrees; checked default text color and solid/linear background plus typed alternate-Theme text/background callbacks cover the complete public builder behavior |
| `toggler` | native | direct checked ID; native label/value/disabled event, size/width/spacing, text typography/wrapping/alignment and complete font descriptors; every concrete Style field across active/hovered/disabled checked and unchecked statuses, plus typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `tooltip` | native | native two-child content, all positions, gap, padding, viewport snap, delay, nine container presets, every concrete container Style field, and checked `box-style` callbacks covering the default Theme's advanced classes |

## Application and runtime

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| application settings | native | state-dependent title, all built-in/custom theme selection, base background/text style and guarded scale callbacks; application ID, custom typed executor and renderer, ordered checked font byte preloads, default text size/font, antialiasing, vsync, codec-free checked RGBA icons, complete initial/named window settings including structured Linux, Windows, macOS, and Wasm fields, structured state/task boot presets, run, and generated first-class Ice tests covering pinned theme/scale/locale/platform/motion environments, semantic input/window/accessibility interaction, computed layout, real pure/sync/task flow, structured paint, and named in-memory RGBA capture |
| `Daemon` | native | `daemon Name` lowers to `iced::daemon`, rejects an unnamed initial window, exposes the current typed window ID to each per-window view/title/theme/scale callback, preserves named window templates and all shared settings, and standalone `exit` lowers to the native lifecycle task |
| `Animation<T>` | native | first-class checked `animation[bool]`, `animation[f64]`, and rustc-verified custom Float state map to native `Animation<T>`; every built-in or typed custom easing, preset/ms/s duration, delay, finite/forever repetition, auto-reverse, implicit/exact-instant transition, value/progress/remaining queries, f32/optional-f32 interpolation projection, and active-only native frame subscription are covered |
| explicit image allocation | native | `task image allocate handle` lowers to native `image::allocate` with required exact success/error routes; `image-allocation` retains GPU memory and exposes handle plus exact `Size<u32>`, `image-error` preserves all five native variants with kind/message projections, and `image-memory` plus downgrade/upgrade covers weak retention; requires iced's `image` feature |
| debug timing | native | `debug-span?` owns exact non-clone `iced::debug::Span` state; checked `debug start name -> state` finishes any prior span before native `time`, `debug finish state` consumes it exactly once, `debug.active(state)` reads its presence, and generic `debug.time_with(name, value)` preserves the value type; iced's `debug` feature activates reporting while its native no-op implementation remains available without the feature |
| `Theme` and styles | native | a checked semantic-token contract with complete named runtime-selectable palettes, all 22 built-in default-renderer themes, typed native factories including `custom`/`custom_with_fn` and complete extended-palette logic, app/nested selection, dynamically selected token styles, target-scoped utilities, imported semantic recipes with deterministic precedence, complete widget-native catalogs, concrete style fields, and typed runtime callbacks |
| `theme::Mode` | native | default and all none/light/dark variants, compact kind projection, equality, exact typed extern passage, equivalent app theme/factory behavior, and deliberate ordering/lazy rejection matching the native enum cover the complete public value behavior |
| `Task` | native | complete public `iced::Task` construction and composition through async/task/stream/sip externs, direct `done`/`none`, system/clipboard/font/widget/window tasks, `batch`, `chain`, abortable handles including abort-on-drop/query and owner-scoped named Future/stream replace lanes with explicit invalidation and terminal-only stream-handle release, `map`, output-dependent `then`, optional-or-result `and_then`, `map_err`, result-preserving `collect`, `discard`, and `units`; every immediate task producer has one exhaustively checked final-statement classification, while multiple tasks require `parallel` or `sequential`; `future`/`stream` identity forms are represented by perform/run extern sources, and default/unit conversion by `none` |
| `Subscription` | native | complete application-facing construction and composition: typed arbitrary adapters, `none`, `batch`, checked conditional activation/status filters, direct every/repeat timers, native `listen`/`listen_with`/`listen_raw` generic events, input-method/keyboard/mouse/touch/window sources (with optional typed IDs on all eleven discrete window events) and system theme changes, typed `run`/`run_with` workers, custom `Recipe` factories through `from_recipe`, raw `EventStream` filters with hashable identity, `with` identity context, typed `map` routing, noncapturing typed `filter_map`, and `units`; advanced `into_recipes` is runtime-consumer plumbing rather than subscription construction or behavior |
| widget operations | native | all 13 core focus/cursor/selection/scroll operations with checked static/dynamic identity paths through component, layout, slot, keyed, table and pane scopes, typed focus query, native `find`/`find-all` over ID, text, point and focused selectors with complete normalized target metadata, plus custom typed selector factories |
| clipboard | native | standard and primary read/write tasks; reads preserve iced's optional string payload and writes are checked fire-and-forget effects |
| fonts | native | ordered app-level relative font files are checked and embedded into iced's startup loader; runtime bytes lower to native `font::load`; every family/weight/stretch/style descriptor, checked named reference, application default, generated Rust `App::default_font()` bridge, and all widget font setters are covered |
| system | native | current theme task, theme-change subscription, and every information field with optionality preserved; information requires iced's `sysinfo` feature |
| time | native | `instant` maps to iced's native monotonic value; `task time now`, payload-producing `every`, and typed async `repeat` cover the complete enabled `iced::time` task/subscription API with checked positive `ms`/`s` durations (`repeat` requires iced's `tokio` feature) |
| window | native | every initial and named-open setting, including codec-free RGBA icons and structured Linux/Windows/macOS/Wasm fields; typed `window-id`, open/oldest/latest, direct targeting for every per-window close/drag/resize/constraints/state/move/mode/focus/level/menu/attention/passthrough/monitor/raw-ID/screenshot/icon task, automatic tabbing, native or flattened lossless screenshot payloads, all 12 event forms with optional IDs on all 11 discrete events, and an exact typed `window::run` callback boundary for raw window/display handles |
| system tray | native | an Ice-owned surface beyond iced (backed by the `tray-icon` crate, and its already-present `muda` menu, on macOS): the `tray` block's required codec-free RGBA icons with the same checked byte-length contract as window icons and the same `cargo ice check` asset walk, repeatable with `when` guards resolved first-match-wins against a mandatory unguarded last line, macOS template flag carried through every guard-driven swap, reactive `label`/`tooltip`/row/guard expressions re-evaluated per update and diffed above the platform seam with string literals hoisted to a single startup application and a first sync placed after the state each entry point starts from, and a native `menu` of `str` rows and `separator`s whose routed rows call zero-parameter handlers through the payload-free `subscribe` route path and whose unrouted rows are created disabled, nested to any depth by indenting a block under a row — a submenu is a third thing beside a command and a stat, drawn enabled, choosable by nobody, and rejected if it names a route. The menu stays one flat declaration-ordered table at every depth, a submenu recording how many of the rows after it are its own, so one row index remains the checked-expression id, the snapshot slot, the native item and the row-to-handler entry alike. The row count is fixed at compile time and there is deliberately no way to generate rows from a list: a varying count would rebuild the native menu on every update, which is the whole of what the diff above the platform seam buys. The platform owns the menu's opening, placement and dismissal, so a program declares no window for it, carries no tray state and no tray message variants, installs no subscription without a routed row, and works identically under `app` and `daemon`. `expect tray label|icon|item|command` asserts what the program last decided the item should show, and `tray choose` runs a row through the same generated row-to-handler table the live subscription maps a chosen row through — a nested row by its text like any other, with no path to spell, and a text carried by more than one row fails `tray choose` and `expect tray command` naming every match rather than silently taking the first, because grouping is what makes two rows share a word. Every non-macOS target compiles the same source against no-op stubs, and `ICE_TRAY_DEBUG` traces the native boundary. Evidence: `tray-basic`/`tray-menu`/`tray-under-app` compile fixtures, thirteen tray diagnostics, `tray-block`/`tray-menu` format fixtures, `check_assets` tray-icon unit tests, runtime snapshot, diffing, first-match-guard and command/stat unit tests, a tray-menu handler-root reachability test, codegen structure tests including boot-sync ordering and subscription presence, and the `tray` example's seven Ice tests, which press a row and assert the handler ran, the command/stat split, the rows a chosen row leaves behind, the declared icon, and for submenus: that a nested routed row reaches its OWN handler, that a submenu title is not a command while the rows under it are, and that a nested row's text re-reads at its own index. Red for all three came from one mutation numbering the row-to-handler table and `__tray_sync` over delivered rows only, which ran the wrong handler for one nested row and left another dead, and one marking submenu rows as commands. Also a `tray-submenu` compile fixture pinning the flat table's nested counts at depth two, a `tray-submenu-route` diagnostic, a `tray-submenu` format fixture, and runtime unit tests for the submenu/command/stat split, a nested row's own text slot, and the ambiguous-text failure. HELD BY macOS CI: `a_guard_driven_swap_still_asks_for_template_rendering` runs on the `Workspace (macos-latest)` runner and pins the argument — a guard-driven icon swap carries the configured template flag rather than dropping it, which is the regression that silently un-templated a menu bar after the first swap. NEEDS A HUMAN ON A MAC, asserted nowhere: that the menu raises on a left click; that a disabled row draws as a legible grey stat rather than something that looks broken; that a template icon actually recolours on a light bar; and, new with submenus, that a submenu row draws its disclosure arrow and opens its block on hover, and that the `muda::Submenu` tree the flat table is rebuilt into nests the rows the author wrote. The tree build is the one part of the submenu path that is macOS-only — the portable half a test reaches is the flat table, not the native nesting. A CI runner has no window server, so no status item is ever created — which also means WHICH native setter consumes the flag (`set_icon_with_as_template`, not `set_icon`) is held by a doc comment and by review, not by a test |
| `window::Id` | native | native unique construction, decimal display, equality, ordering, hashable lazy identity, exact typed extern passage, and direct task/daemon/subscription payload reuse cover the complete public value behavior |
| window value enums | native | all variants and defaults of `Direction`, `Level`, `Mode`, and `UserAttention`, compact kind projections, exact typed extern passage, equality only where the native type implements it, deliberate ordering/lazy rejection, and equivalent task keyword sugar cover the complete public behavior of these four enums |
| `window::Position` | native | default, centered, and exact Point construction, kind/optional-point projection, typed extern passage, native `SpecificWith(fn(Size, Size) -> Point)` preservation and invocation through a checked pure adapter, equivalent initial-setting sugar, and deliberate comparison/lazy rejection cover the complete public value behavior |
| `window::RedrawRequest` | native | all next-frame/at-instant/wait variants, kind/optional-instant projection, equality, ordering, exact typed extern passage, equivalent canvas/shader/raw-event behavior sugar, and deliberate lazy rejection matching the native enum cover the complete public behavior |
| event routing | native | all five structured families plus first-class generic `event` values through native `listen`/`listen_with`/`listen_raw`, optional window IDs, status filters, transforms, handler routing, and typed extern passage; system-theme runtime events remain a separate native source because iced does not represent them as `iced::Event` |
| `event::Status` | native | both ignored/captured variants, native captured-first merge semantics, compact kind projection, equality, exact typed extern passage, equivalent subscription filter sugar, and deliberate ordering/lazy rejection matching the native enum cover the complete public behavior |
| keyboard | native | all three native events preserve exact `Key`, `Physical`, `Location`, and `Modifiers` values; every named/code/native/location/modifier constructor, structured matching, safe runtime native-code conversion, exact extern passage, and native latin translation are checked Ice expressions |
| mouse/touch | native | every mouse and touch event has a direct typed subscription with exact native `Button` and `Finger` payloads; every button/finger variant, `Cursor`, advanced `Click`, and all 27 `Interaction` variants, constructors, queries, projections, ordering, transformations, typed extern passage, and direct MouseArea/Canvas passage are covered |
| `mouse::Interaction` | native | default and all 27 variants, compact kind projection, equality/order, exact typed extern passage, direct MouseArea/Canvas builder input, equivalent cursor-name sugar, and deliberate lazy rejection matching the native enum's lack of `Hash` cover the complete public value behavior |
| `mouse::ScrollDelta` | native | both Lines/Pixels variants, exact f32 coordinate construction and f64 projection, compact kind, equality, exact typed extern passage, readable event-route destructuring sugar, and deliberate ordering/lazy rejection matching the native floating-point enum cover the complete public value behavior |
| `Pixels` | native | zero, f32/u32 numeric construction with checked runtime u32 conversion, value projection, equality/order, every native pixels/pixels and pixels/scalar addition, multiplication and division form, and typed extern passage cover the complete public behavior |
| geometry primitives (default `f32`) | native | complete native `Point`, `Vector`, `Size`, and `Rectangle` constructors, constants, fields, array projections, point display, equality, arithmetic, distance, per-component size operations, conversions, containment, offset, intersection/union, exact `u32` snapping, four-side padding expansion/shrinking, radians rotation, zoom, anchoring, transformation application, and typed extern passage |
| `Padding` | native | zero/default, uniform/per-side/axis constructors, f32/u16-equivalent scalar and axis conversions, exact Pixels conversion, every side and x/y projection, all six native builder methods, `fit`, Size conversion, Rectangle expansion/shrinking, equality, and typed extern passage cover the complete public behavior |
| `Degrees` / `Radians` | native | numeric construction and f64 projection, equality/order including native angle-left scalar comparison, full range constants and containment, Degrees scaling, exact Degrees-to-Radians conversion, PI/display, every native Radians mixed arithmetic form including remainder and reverse scalar multiplication, both `to_distance` points, geometry rotation/vertex integration, and typed extern passage cover the complete public behavior |
| `Rotation` | native | exact floating/solid/default/f32 conversion, radians/degrees/kind projection, native `radians_mut` update, equality, size application, typed extern passage, and direct Image/SVG builder input cover the complete public enum behavior |
| `ContentFit` | native | all five variants/default, compact kind/native display, equality/hashable lazy identity, exact size fitting, typed extern passage, and direct Image/SVG/Viewer builder input cover the complete public enum behavior |
| `Color` | native | default/constants, normalized/static and dynamically checked 8-bit/linear/array constructors, all accepted hexadecimal parse forms with optional rejection, exact channel/RGBA8/linear/display projections, inverse/in-place inversion/alpha scaling, luminance/contrast/readability, equality, and typed extern passage cover the complete public behavior; native floating channels correctly remain unavailable as lazy hash identities |
| `Background` / `Gradient` / `Linear` / `ColorStop` | native | both background variants, every background conversion and alpha scaling, the complete current linear-only gradient enum, linear construction from f32/radians, native sorted single/multiple stop insertion including invalid/eight-stop behavior, alpha scaling, exact optional-stop array projection, every field, equality, typed extern passage, and equivalent solid/linear style sugar cover the complete public behavior; floating values correctly remain unavailable as lazy identities |
| `Font` / `Family` / `Weight` / `Stretch` / `Style` | native | native default and named/monospace constants, exact complete font construction, every descriptor default and variant, static named families, every field and compact kind/name projection, equality, hashable lazy identity, typed extern passage, and equivalent human-readable widget font declarations cover the complete public value behavior |
| text `Alignment` / `Shaping` / `Wrapping` / `LineHeight` | native | every variant and feature-aware default, both line-height payloads/conversions and absolute resolution, all alignment conversions, compact projections, equality, hashable lazy identity, exact typed extern passage, and equivalent human-readable widget properties cover the complete public value behavior |
| `Length` | native | all four variants, static/dynamically checked portion and u32 construction, exact f32/Pixels/u32 conversions, fill factor/fluidity/kind/payload projections, fluid/enclose operations, equality, typed extern passage, and direct passage through every builder that accepts native Length cover the complete public behavior; pixel-only Grid width and slider short axes retain numeric checks, and floating fixed values correctly remain unavailable as lazy hash identities |
| `Alignment` / `Horizontal` / `Vertical` | native | every variant, every bidirectional native conversion, compact kind projection, equality, hashable lazy identity, typed extern passage, and equivalent compact view-property sugar cover the complete public behavior |
| `Border` / `Radius` | native | default/exact border construction, all three border free constructors and builders, every radius free constructor and builder, all four radius numeric conversions with safe dynamic integer forms, native corner-array conversion and scaling, every field, equality, typed extern passage, and equivalent compact style sugar cover the complete public behavior; floating values correctly remain unavailable as lazy identities |
| `Shadow` | native | default and exact color/offset/blur construction, all three field projections, equality, typed extern passage, and deliberate rejection as a floating-point lazy identity cover the complete public behavior |
| `Transformation` | native | identity/default, orthographic, translate, scale, inverse, scale/translation inspection, composition, lossless matrix conversion, equality, typed extern passage, and native application to every supported geometry and pointer value cover the complete public behavior |
| `window::Screenshot` | native | exact construction and capture task delivery as one native value, public RGBA/physical-size/scale fields, borrowed and owned byte access, native crop success, both crop error kinds and messages, debug formatting, typed extern passage, and deliberate comparison/lazy rejection cover the complete public value behavior |
| custom widget | native | typed owned or app-state-borrowing `Element` adapters with checked event routing, selected Theme/Renderer propagation, alternate-Theme subtrees, and the complete advanced Widget/Overlay escape hatch |
| custom renderer | native | checked application-wide concrete `iced::program::Renderer` type path propagated through every generated `Element`, including extern components, shaders, alternate themes, and editor adapters |

The free `iced_runtime::task` constructors such as `oneshot`, `channel`,
`blocking`, and `effect` are not re-exported by `iced::task`; they are outside
this public iced baseline. A typed `task` extern can still adapt runtime-specific
work when an application intentionally depends on `iced_runtime`.

### Per-row animation and computed surface opacity

The `Animation<T>` row above is a claim about iced's `Animation` API. Reaching a
row on screen with one takes three further things, and all three are now
expressible.

**An animation may belong to a row.** A component may declare
`state fade:animation[f64]`, keyed by the component's hierarchical instance
scope like every other local value, so two rows animate on independent clocks.
It requires `lifetime mounted`: an animation's identity is the instant it
started, so its storage is created the first time the instance renders rather
than re-derived every pass, and dropped when the instance leaves the tree.
`lifetime retained` is `E103`
(`crates/ui-lang-core/tests/cases/diagnostic/row-animation-lifetime`).

**The declaration starts it.** An `animation_setting` `from` gives the start
value; the animation is built holding it and sent to the declared value at the
moment it comes into being. A row materialized by a `for` therefore fades in
without any event to assign on. A `from` whose literal does not match the
animated type is `E103`
(`crates/ui-lang-core/tests/cases/diagnostic/animation-from-type`).

**A computed number reaches a surface's colour.** A container's `bg=` accepts a
parenthesised opacity expression — `bg=flash/(animation.project(fade, value, value))` — on the
same `0..=100` scale as the literal `bg=flash/40`, replacing the colour's alpha
every view pass. It is a container property with a single background colour;
anywhere else it is rejected
(`crates/ui-lang-core/tests/cases/diagnostic/computed-opacity-surface`).

Component animations keep native frames alive on the same active-only
`window::frames()` subscription app animations use. Interpolation happens where
it always did — in the view pass — so an animated surface must sit outside any
`lazy` boundary that memoizes the row around it; inside one it is frozen at the
value the subtree was built with, and `SPEC.md` says so.

Evidence: `crates/ui-lang-core/tests/cases/compile/row-animation` (generated
`from` transition, per-instance materialization, frame subscription over
component storage), `crates/ui-lang-core/tests/cases/format/row-animation`,
`examples/showcase/tests/cases/ui/animation.ice` (`ArrivalRow` demo, and the
`computed_surface_opacity` first-class test asserting the painted alpha), and
`a_row_owns_its_fade_and_that_fade_ends` in
`examples/showcase/src/native_fixtures/tasks.rs`.

## Evidence rule

A row moves to **native** only when every public application-facing behavior in
the pinned iced surface has:

1. documented Ice syntax and static types;
2. parser and semantic-checker coverage, including invalid input;
3. generated Rust that compiles against the pinned iced release;
4. a reference or focused runtime example when behavior is interactive.

A first-class test claim counts only when it parses and checks as Ice, compiles
to an auto-discovered Rust test, and its runtime assertion observes the real
generated program or mounted component. Schema-only descriptions and manually
duplicated Rust assertions do not count.

The repository does not claim complete iced coverage while any row is partial
or missing.

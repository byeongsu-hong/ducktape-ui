# Module-owned wasm views

The two goals share one execution boundary: grow Ice's `tree` target into a
complete application runtime, and make a module's UI independently packageable
with its consensus code. A view component executes on the desktop host, never
in consensus. It emits declarative UI and sends capability requests; native
rendering, devices, credentials and network access remain host responsibilities.

This inventory reads ducktape at `ffc1629b54337734910e60a4ad282943f5586515`
and Ice at `007ab0b9`. It is a source audit, not evidence that the existing
Ducktape daemon compiles for wasm. Ducktape is not modified by this work.

## Actual consumers

Paths in this table are relative to ducktape's `app/src/ui/`. A screen is
not necessarily a one-to-one module: ownership below is a proposed extraction
boundary, and shared shell UI must not be duplicated into every package.

| Consumer and source | Blocking UI requirements | Host / module boundary |
| --- | --- | --- |
| Chat and DMs: `screens/chat.ice`, `components/chat.ice`, `components/dm.ice` | stack, hover, flex, overlay, keyed/lazy lists; `rich_composer(&editor, str, bool, f64, f64, f64) -> ComposerEvent`; focus and scroll operations | chat queries/submissions and live deltas; navigation, clipboard and notifications through host capabilities |
| Huddle: `components/huddle.ice`, `extern/call.ice` | existing one-string video surfaces express the tiles/stage; their native providers are still required | host-owned call session and media; stream cancellation ends the session; mute/camera/screen commands; detached window stays host-owned |
| Pages: `screens/pages.ice`, `extern/editor.ice` | stack/overlay; `page_document(&editor, bool, bool, &[PageBlock], &[str]) -> PageEvent`; editing shortcuts | page load/save/comments, dirty-buffer conflict handling; editor content/event contract cannot be replaced by a plain input without losing behavior |
| Forge: `screens/forge.ice`, `components/forge.ice` | stack/pin/keyed/lazy/flex; rich composer; `forge_markdown(str, str, bool) -> str`, `forge_code(str, str, bool)`, `picture(str, str)` | repository data, links and file assets; native git work and HTTP fetching stay host-side |
| Files: `screens/storage.ice` (`FilesScreen`) | overlay/keyed/stack/lazy/flex, editor, picture/markdown/code surfaces | file listing/read/write/history and dropped-file access; a guest path is not permission to read a host file |
| Agent UI: `screens/shell.ice` | keyed/lazy, markdown links, rich composer, `agent_terminal_surface(&AgentTerminalSession)` | agent streams and session capabilities; terminal process, retained terminal state and focus stay in host |
| Governance: `screens/governance.ice` | basic controls, plus shared shell/component dependencies | proposal query/vote/submit and signing authorization |
| Members and agents roster: `screens/roster.ice` | basic controls, plus shared shell/component dependencies | identity, membership and agent data; roles/authorization remain backend rules |
| Node/logs: `screens/node.ice`, `view.ice`, `ducktape-ui/log-timeline.ice` | `node_log_timeline(&NodeLogTimelineState, &str) -> NodeLogTimelineEvent` | node/log/peer streams; retained log state needs a host resource or a declarative replacement |
| Explorer: `screens/storage.ice` (`ExplorerScreen`) | flex/stack; shared components | cross-module search and block/operation data; not a single module's state |
| Shell, settings and onboarding: `view.ice`, `components/shell.ice`, `components/onboarding.ice`, `screens/settings.ice` | stack/pin/tooltip/qr, window lifecycle, keyboard modifiers/shortcuts, theme/font assets | host shell, network selection, keystore, signing prompts, tray and native windows; not automatically owned by an app module |

The actual root is `daemon Ducktape` in `app.ice`, importing all state,
handlers, screens and tests. `view.ice` dispatches by native window identity;
`handlers/lifecycle.ice` subscribes to keyboard, dropped-file and window events.
There are no independent module guest entry points in this source graph.
The `ducktape-app` Cargo dependency graph includes native camera/audio, git,
keystore, node and networking code. Changing only `Target::Tree` cannot turn
that binary into an isolated view package.

## Gaps beyond the widget table

- **Package ownership:** `crates/kernel/module-artifact/src/lib.rs` in ducktape
  encodes only `component` and optional `index`, and hashes that entire frame.
  A future module-owned view must enter that same commitment and activation
  path (including view removal), with its assets/manifest covered by the
  commitment. An independently scanned UI catalog is not that guarantee.
  Keep consensus and desktop view as separate wasm components within the
  deployable unit; importing desktop capabilities into consensus is wrong.
- **Mount contract:** the example host currently gives one app one window.
  A module view also needs mounting inside a host tab, initial route/context,
  cross-module navigation, instance-scoped state, unmount cancellation and
  host-resource cleanup. Do not equate a module id with a unique UI instance.
- **Data boundary:** ordinary record/list data can cross as values; native
  `AgentTerminalSession`, editor content and log state cannot cross as Rust
  pointers. Surfaces need explicit resource identities and semantic events.
  Those identities must be scoped to the guest instance and released at
  unmount. Do not stringify arbitrary native state to claim support.
- **Effects:** `ui-lang-guest` executes task outputs and clipboard actions;
  clipboard uses a manifest capability and the mounted host's platform interface.
  Checked Ice focus/selection/scroll statements now use the mounted host's
  widget request channel. Arbitrary native Widget actions and window/font/
  image/reload/exit actions remain dropped and logged. Keyboard handling and
  these effects are functional requirements of the
  existing screens, not optional visual polish. Existing host request/stream
  transport can carry domain capabilities; ducktape must supply authorization,
  signing, query/submit/page and cancellation semantics later.
- **Assets and presentation:** embedded SVG works, but picture providers refer
  to host caches; guest file paths cannot access those caches. Named text fonts/layout and button recipes/accessibility now cross as copied values. SVG icon Rust style callbacks remain deliberately refused; declarative colors and host button-ink inheritance are needed. Images,
  theme inheritance and keyboard behavior need
  observable parity, including unknown-surface placeholders.
- **Reload:** `ice:view` exports only `init` and `tick`. Snapshot/restore must
  preserve serializable UI state, rebuild handler tables and subscriptions,
  invalidate stale events and respect the installed artifact commitment.
  Restoring a view must not replay submissions or retain native resource
  pointers. The host keeps its window and replaces a guest only after the
  replacement can restore successfully; an incompatible snapshot needs an
  explicit failure, not a compatibility decoder.

## Connected phases and completion evidence

| Phase | Work in Ice / app-store | Evidence required before calling it complete |
| --- | --- | --- |
| 3a — surface values and routes | Typed scalar arguments/events and records/lists/options for data-backed surfaces (implemented, including nested validation). Recursive records and sum types remain pending. Preserve borrowed-call syntax by copying wire values. Reject opaque native values. | Actual bundled wasm uses mixed arguments and link/event routes; host-rendered interaction returns the right payload; wrong payload type and unknown surface are exercised; wire limits and patches remain bounded. |
| 3b — host surfaces and retained state | Named shader and markdown lowering implemented; Guest-owned registries and a retained native log session/view boundary implemented. A native rich-composer input/semantic-notice boundary is implemented; a host-selected native terminal provider with background polling is implemented; application highlighting, page semantic adapters and module process/session adapters remain. Use representative module examples. | Markdown link route; editor edit/submit/selection/IME behavior; terminal/log resource lifecycle, two concurrent instances and cleanup. A no-op provider or placeholder does not count as parity. |
| 3c — declarative graphics and responsive layout | Canvas geometry data and host-evaluated container conditions implemented, with widget-local opt-in measurements only. Arbitrary measured widget properties and native callbacks remain refused. | Geometry rendering and interaction; multiple container widths with correct branches and no guest layout callback; bounded sensor feedback. |
| 4a — actual app layouts first | stack/hover/overlay and wrapping rows/columns and tooltip implemented with native host routing and modal lifecycle; keyed/virtual rows, lazy caching and native flex layouts implemented; pin implemented with native local offsets. Preserve union sizing, hit routing, identity, virtualization and scroll behavior. | Representative chat list and menus, page overlay, file/forge list; reorder/edit/scroll assertions and frame measurements, not compile-only coverage. |
| 4b — remaining content/layout/style | rich text/markdown/qr/image/combo, table/pane grid/theme/themer/float/resize handle and remaining supported surface shapes. | Per-feature native/wire behavior checks and real wasm bundle builds; preserve intentional rejection of native callbacks. |
| Runtime alongside 3–4 | Clipboard Tasks and checked widget focus/selection/scroll requests implemented through the mounted host; window requests, input subscriptions, assets, mount/unmount and host context remain. | Focus/scroll/copy, keyboard and cancellation driven through a real host boundary; separate guest instances cannot affect one another. |
| Hot reload after state/lifecycle boundary | Generated snapshot/restore exports and catalog watch in the example host. | Same window and UI draft survive replacement; failure retains usable old instance; no duplicated side effects, stale routes or leaked subscriptions. |
| Ducktape integration — deferred | Extract per-module guest roots, bind real capabilities/surfaces, add view to module artifact and build/hydration/activation paths, mount from module packages. | Every existing module-owned screen builds and runs from its package; no wasm embedded in desktop binary; module+index+view hash/activation/removal agree; real workflows and permissions pass. |

The two goals are compatible, but `51 emitted / 42 refused` is not a measure
of module portability. Completion requires view construction, interaction,
effects, lifecycle and packaging together. Purely local visual state belongs
in the guest; the host must not become a second implementation of module
business logic. Ducktape integration remains deferred until authorized.

### Retained editor/terminal/log detail

The actual `ComposerEvent()` and `PageEvent()` declarations in
`app/src/ui/extern/editor.ice` are opaque Rust types, not Ice enums.
`app/src/editor.rs` carries `Submit`, rich editor actions and formatting
marks; `app/src/pages/mod.rs` additionally carries todo, link, menu, gutter,
drag/drop and comment events. A generic Ice enum codec alone does not make
these native editor actions portable. Phase 3b needs semantic edit/selection
commands and a retained host editor identity, preserving IME, undo and focus.
`PageBlock` itself has scalar fields and can use the record/list boundary.
`AgentTerminalSession` wraps a retained terminal session; log state holds a
native virtual timeline and shared row buffers. Neither is a record to copy
across the wire. Guest resource handles must be scoped to the mounted instance;
the backing session can have a longer host-owned lifetime. In particular,
`handlers/lifecycle.ice` keeps terminal event subscriptions active while the
operator visits another pane. Releasing a view handle must not implicitly
terminate that session. Phase 3b must distinguish view leases from session
ownership and preserve background status updates.

### Markdown boundary implementation notes

The existing Forge/Files views use the already-supported scalar extern
component `forge_markdown(source, doc, dark) -> str`; they do not use an Ice
`markdown-viewer` declaration. Their actual host provider and asset policy
still belong to the later Ducktape integration.

General Ice markdown now retains source alongside the native parser, whose
`Content` has no source getter. Incremental `append`, replacement and
`markdown_images()` preserve native behavior. Source, resolved settings and
guest palette cross to `ice.markdown`, or to the declared custom viewer name
with its typed arguments. Parser internals and native viewer state stay local.
The default host owns parsed content and its borrowing native widgets in a
cache scoped to the mounted widget tree. Unchanged frames preserve link and
scroll state; changed documents rebuild the content and unmount drops it.
Runtime mouse/raster tests and the bundled surface fixture cover link routes,
append/replacement, guest colors and custom viewer arguments/events. The
remaining phase 3b work is the editor/terminal/log resource boundary; actual
Ducktape markdown providers and image assets still require host integration.

### Retained host session boundary

The example now constructs a surface registry for every Guest. A provider
captures its host-authorized session, and its mounted native widget retains
view state independently. `session_log` demonstrates this with the actual
LogTimelineState/virtual-list renderer, semantic notice records, a bounded
host log ring and preserved paused history during front eviction. The native
Element is retained as well as its Iced Tree, preserving button/scrollbar
feedback across update and draw.

Two registries may share one host session without sharing selection or scroll.
Registry identity distinguishes replacement guests even at identical node
keys. Unmount releases the view lease; host ownership can keep the session
receiving data after the view or guest exits. The bundled fixture tests these
boundaries; this supplies the ownership pattern for terminal/log adapters,
not the actual Ducktape terminal, log filter or rich page/editor adapter.
Opaque native values remain refused. Those adapters still need deliberate
semantic argument/event definitions at their eventual integration boundary.


### Native composer boundary

The `rich_composer` example binds a native `RichTextEditor` document to a
mounted guest key and returns copied `ComposerNotice` records. It preserves
live Enter/Shift+Enter classification, focused formatting chords, selection,
IME and per-view bounded history. Guest echoes keep native state; explicit
reset or changed document text replaces it. A Tree-owned lease survives frame
rebuilds and releases on unmount, while the serializable draft stays guest-owned.

This is the reusable input boundary required by the chat/DM/Forge/agent
composers, demonstrated by an actual bundled guest. It does not port the
existing opaque `ComposerEvent` API, Ducktape's Markdown highlighter, submission
policies, page block/menu/comment semantics, or terminal process/session adapter.
Those remain explicit work; the existing Ducktape sources are unchanged.


### Native terminal boundary

The app-store terminal fixture now binds a real host-selected PTY to one guest.
Native keyboard/ANSI output, background output while unmounted, exit and final
frame remount, stream cancellation and separate instance ownership are covered.
Native Tree lifetime clears focus and pending clipboard work on unmount. The
engine drains buffered reads and unfinished synchronized updates before exit.
This closes the example-host terminal provider prerequisite in phase 3b; actual
Ducktape agent/SSH process selection, session routing and policy are still host
integration work. No Ducktape files are changed.

### Host installer resources

Desktop `cargo ice bundle` metadata can carry prebuilt module views using
`resources = ["../target/views"]` (relative to the host package manifest).
The installed `views/` directory is beside the executable on macOS, Windows,
and Linux; macOS copies it before signing. This closes the installer payload
gap reported by the Ducktape port. Module manifests, discovery, and coordinated
host/guest builds remain host integration work. The reported Tree component
`rev_memo` compilation failure is fixed and covered by an actual wasm fixture.
The layered fixture also exercises component composition. Text options, utility
styles, diagnostic origins, and the remaining layouts stay in the phase backlog.

The actual typed NodeScreen root now reaches only its opaque activity-log
extern value after input presentation lowering. Use a data-only extern
component declaration and the existing host Surface registry for that slot.
Its retained native state stays host-owned; opaque event types also need a
wire data contract. This is the concrete next host task in phase 3b.


### Flex layout boundary

Ducktape 06e0378b4 uses five flex sites for reaction chips/pickers, the Forge
composer/send row and Explorer filters. Tree now carries their wrapping, axis
gaps, alignment and fill/fixed sizing through the existing host engine. The
actual bundled fixture exercises these patterns with native clicks and keyboard
input, including a lazy boundary and host resize. This removes the flex kind
refusal; pin, rich text, retained component lifetimes and opaque extern/data
contracts still need their own implementation or host integration.

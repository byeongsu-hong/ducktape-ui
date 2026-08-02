# 0001: Fully normalized HIR boundary

- Status: Accepted
- Date: 2026-07-31

## Context

Successful Ice analysis currently produces a nominal `CheckedDocument`, but it
still contains and dereferences to the source `Document`. The Rust code
generator consumes that checked AST directly and imports checker helpers for
some semantic decisions. As component defaults, bindings, named events, slots,
recipes, typed matches, scoped widget identity, and asynchronous lifetimes have
grown, this makes it increasingly easy to repeat or diverge on the same
decision in the checker and code generator.

New Core syntax is frozen while this boundary is established. The objective is
not to introduce another public language surface or prepare speculative
backends. It is to make every accepted Ice construct have one checked,
canonical meaning before backend emission begins.

## Decision

Ice will introduce a complete, private, typed high-level intermediate
representation named `LoweredProgram`. The target pipeline is:

```text
Source AST
  -> CheckedDocument
  -> LoweredProgram
  -> Rust/Iced code generation
```

The source AST preserves how a program was written for parsing, formatting,
editor features, and diagnostics. `LoweredProgram` preserves what the checked
program means. It contains Ice semantics and runtime-required identities, but
no Rust token fragments or Iced-specific types. Concrete Rust and Iced API
choices remain the responsibility of the backend emitter.

The lowering boundary has these invariants:

1. Only the lowering module can construct a `LoweredProgram`.
2. Rust code generation accepts a `LoweredProgram`, never a source `Document`
   or `CheckedDocument`.
3. Code generation does not import checker helpers or repeat semantic
   validation.
4. Every HIR node carries an `OriginId`. A shared origin table retains the root
   or imported source location and its origin stack without copying paths into
   every node.
5. Invalid or unresolved states are not representable in HIR. Lowering either
   produces a complete program or returns a source-mapped diagnostic.

Before code generation, lowering resolves or removes all language sugar,
including:

- declaration and reference names into stable typed IDs;
- component prop ordering, defaults, read/bind capability, and concrete state
  references;
- named event routes, default-event shorthand, and `forward` blocks;
- required and optional slots and folded `provided(Slot)` expressions;
- recipe inheritance, utilities, variants, palettes, and theme token IDs;
- exhaustive match arms and payload bindings;
- component scopes, widget identity, and reconciliation identity;
- task, stream, `run latest`, and `run replace` call-site identity;
- retained or mounted lifetime behavior and its cleanup obligations;
- all remaining type, capability, and ownership choices required by emission.

The HIR specification covers the whole accepted language. Delivery remains
incremental: component calls and contracts first, then styles and themes,
expressions and matches, asynchronous behavior and lifetime, application
settings and tests, and finally the remaining backend surface. Each migrated
slice removes its old code-generation path in the same change. There is no
permanent mixed AST/HIR fallback or compatibility shim.

### Implementation status

The component-contract and component-call slice is implemented. The private
owned `LoweredProgram` is now the only input accepted by Rust generation.
Component and call IDs, ordered props with selected defaults, writable state
references, direct/forwarded named events, ordered required/optional slots,
output routes, scope identity, and storage lifetime are fixed before emission.
The component call render path selects classified calls and contracts by source
site and typed ID instead of repeating those decisions. Lowering still resolves
component source names, while other unmigrated generation paths still consume
AST names and nodes. Component state storage, boot, update, mounted cleanup, and
call rendering consume the resolved contract.

Supplied component arguments now carry an unconditional checked expression-use
ID; defaults retain the same representation. Bind writability is decided from
the checked path root and projections, so lowering no longer re-resolves a raw
argument expression or keeps a second argument-expression variant. Component
calls and checked view facts share one `ComponentCallId`/`ViewId` arena.

Normalized component records carry root/import locations through `OriginId`.
The table's parent links are scaffolding for future expansion stacks: current
lowering errors and generated source markers do not traverse them and continue
to use source spans and the existing physical line-origin map.

The recipe/utility and theme-declaration slice is also implemented. Typed
recipe, style-use, target, variant, theme-contract, theme-token, palette, and
extern-function IDs preserve the checked relationships. Recipe inheritance is
flattened once into a fixed-size backend-neutral semantic style, and direct
utilities are applied once at each source site. The ten render paths that use
Core utilities consume only the resolved style and no longer inspect recipe
names, inheritance, or utility strings. Theme contracts and palettes carry
token declaration order and complete RGBA tables; app and nested theme
selection, static or dynamic active palettes, native theme factory calls,
nested gradients, token references, and opacity are resolved before their
migrated emitters run.

This is deliberately not a claim that every style-shaped AST node has been
migrated. Expression-bearing native widget status blocks and direct
non-Canvas/non-Media/non-Tooltip view color fields remain AST-backed with the
expression/view family.
Palette enum paths inside the general expression emitter also remain in that
family. Their backend helpers are removed only when those expressions and view
options gain normalized nodes; no compatibility fallback is added.

The program still owns AST-backed nodes for semantic families not yet migrated.
The remaining expression-backed native styles/colors and widget options outside
Media, Tooltip, MouseArea, ResizeHandle, Sensor, Overlay, Float, Pin, Responsive, Lazy, KeyedColumn, Table, PaneGrid, If, For, and Match therefore remain open
implementation slices; this status does not satisfy the migration-complete
criteria below.
Migrated handler and application-setting generation uses the shared origin arena
directly for imported and root source markers.

The checker now also preserves the first expression-family facts in a private,
owned arena. Stable expression, expression-use, value-owner, and view-owner IDs
connect initializer expression trees to concrete types and to resolved value,
enum, palette, extern, builtin, field-projection, and operator facts. The arena
uses direct typed-ID indexing and does not recover facts from AST locations
during lowering. Structs, struct fields, enums, enum variants, and extern
functions are origin-aware semantic declaration records rather than positional
lookups back into `Document` vectors. Declaration references use the same
`AppStateId`, `DerivedId`, `ComponentParamId`, `ComponentStateId`, `PaletteId`,
`ExternFnId`, struct, and enum identities as the rest of HIR. Checked facts,
component lowering, and style lowering also allocate from one shared origin
arena. Imported physical locations and declaration-parent links therefore
survive across expression and declaration slices without a parallel origin
table.

This slice covers app-state, derived-value, component-default, component-state,
and supplied component-argument expressions through production Rust emission.
Lowering retains their checked expression-use IDs, explicit initializer
coercions, and resolved animation options, including a typed extern ID for
custom easing. The backend emits those facts directly. The former initializer
and component-argument AST helpers, unchecked `Default` recovery, and optional
raw-expression argument representation are removed.

Lexical view-flow expressions are normalized as well. Checked view records own
`if`, `for`, exhaustive typed `match`, keyed, lazy, table, pane-template, and
responsive expression uses plus their resolved local bindings. Match arms carry
resolved option/result/enum/palette patterns, variant or palette IDs, payload
locals, and arm origins. `provided(Slot)` carries a resolved
`ComponentSlotId`. Daemon windows, loop items, match payloads, keyed items, lazy
dependencies, table rows, pane maximized/template locals, and responsive size
locals all have explicit owner roles. Ordinary and flex flow generation,
component arguments, keyed/table/lazy/pane/responsive generation consume this
checked arena. Component call facts also fix whether each prop was supplied or
defaulted; raw argument topology cannot silently change that choice after
checking. Emission matches checked paths to semantic value/local owner IDs and
validates raw view kind/children against the stable checked topology. Invalid
owner, topology, match binding, and enum IDs therefore fail with source-mapped
`E196`. Imported expressions retain physical locations and parent chains;
missing, duplicate, or leftover authoritative analyses also fail with `E196`.
Expression-bearing widget options remain later slices, so the full-HIR
completion criteria remain open.

Handler bodies are now a completed production HIR slice. One deterministic
preorder arena owns app, implicit `mount`, component, and preset handlers;
nested statements; immediate and flow tasks; body routes; and latest/replace
run sites. Checked locals have explicit handler-parameter, statement-let, or
task-transform owners. Every operand has a statement, task, or route owner and
retains its checked expression-use ID, concrete type, resolved writable state,
extern ID, task output/error/finality, and route target/payload contract.
`RunSiteId` is independent of source lines and is the sole generation and
replacement identity used by component state and messages.

Lowering validates handler, statement, task, route, and run-site ownership,
preorder parentage, complete arena consumption, task finality, and origin
chains. Generated handler code consumes only `ResolvedHandler` and
`ResolvedStatement`; it does not inspect statement AST expressions, rerun
expression typing, rediscover extern functions, or derive async identity from a
line number. Nested statement scopes use borrowed binding layers instead of
cloning the full environment. Structured HIR snapshots cover app, mount,
component, preset, group, sip, flow, abortable, route, payload, transform, and
stable run-site records. Post-check AST mutation, imported-origin, invalid-state,
compiled-fixture, and 500/4,000-statement contracts prove checked-fact
authority, source mapping, linear growth, zero handler type rechecks, and zero
full environment clones.

An integration ratchet inventories selected lexical markers for backend escapes
through `LoweredProgram::document`, the `RenderDocument` raw-document wrapper,
checker imports and uses, type or extern re-resolution, raw expression fallback,
and exported source-AST identifiers. A dependency-free Rust lexer strips
comments and literals before it records normalized containing-item and
call-site fingerprints, so same-file delete/add swaps change the reviewed
ledger while identifier prefixes do not become false AST matches. It also
tracks the symbols behind the existing AST glob and grouped checker imports;
exact aliases from grouped and nested AST imports are tracked per file, while
checker glob imports are rejected. This is a conservative lexical migration
ratchet, not a Rust semantic resolver or completion claim. The selected ledger
must reach zero, and ordinary semantic review must confirm that no unrepresented
boundary remains before HIR is complete.

Application and daemon settings are now normalized as one complete vertical
slice. `AppSettingExprId` and `NamedWindowId` identify the retained title,
theme, palette, background, foreground, scale, theme-factory arguments, and
ordered named windows. The checker hands one authoritative analysis per
expression to fact construction; dynamic theme and palette classification is
recovered from those facts, not from source spelling. A typed current-window
local supplies daemon callback scope. The HIR also owns renderer and executor
choices, ordered font assets, the exact default-font
family/weight/stretch/style and declaration origin, common application settings, a folded
runtime-default primary window, every common and platform-specific window
field, icon metadata, and physical origins. Checked facts retain the complete
static setting topology and program kind; lowering rejects any post-check
static mutation with a source-mapped `E196`, including parser-invariant bypasses
such as absolute asset paths or duplicate windows. Renderer, executor, app
fields, fonts, window fields, icons, and platform subfields retain their exact
declaration origins for generated Rust diagnostics. Rust emission for this family no
longer accepts `AppSettings`/`WindowSettings`, rereads a setting expression, or
calls checker/type-inference helpers. Missing, extra, or reclassified retained
facts fail with `E196`; post-check expression mutation cannot change output. A
shared visited-set graph validator checks retained expression IDs, ownership,
topology, and types exactly once per reachable node. Settings contribute only
their owner policy (app state, derived state, expression bindings, and the
daemon-window local), so later HIR slices can reuse the same graph contract
without copying a settings-specific semantic checker.
Structured snapshots, imported extern/origin and direct source-marker tests,
the existing complete application/window compile surface, and a 5,000 named
window lowering contract are the executable evidence.

Application subscriptions are normalized into the same checked arena. Every
subscription has a stable `SubscriptionId`; extern-backed sources and filters
carry `ExternFnId`s; condition, context, worker arguments, and event identity
carry retained typed expression-use IDs; native and extern source payloads are
fixed before filtering; post-filter/context payloads are recorded separately;
and the checked route carries a stable App `HandlerId` plus ordered payload
indices. Lowering shares the settings expression-graph validator and the
handler payload-route core, then publishes private `ResolvedSubscription`,
`ResolvedSubscriptionSource`, and `ResolvedSubscriptionRoute` records with
typed route arguments. The expression policy preserves the checker's
app-state-only scope. The source `Document` subscription list is not consulted
and may be cleared after checking without changing generated Rust. The Rust
subscription emitter iterates only resolved records and expression IDs; it
performs no declaration, extern-name, or raw-subscription lookup. Imported
lowering diagnostics and source markers keep the subscription origin. A
500-to-4,000 contract measures the complete analyze, lower, and codegen path
while verifying exact generated subscription counts, linear expression growth,
one shared app scope, zero full-scope clones, and a bounded debug-build wall
time.

Canvas is now a completed vertical slice. Stable Canvas-local, command, event,
route, and expression IDs partition every Canvas view. Lowering freezes its
options and static topology with checked semantic keys and exhaustively converts
all draw commands, paths, transforms, paints, fonts, interactions, state
updates, redraw actions, and routes into `ResolvedCanvas`. Every dynamic operand
uses the shared checked expression arena; loop and event bindings use typed local
IDs; named values, fonts, theme colors, and payload routes are resolved before
emission. The backend no longer accepts raw Canvas options, commands, events,
paths, or expressions, repeats type inference, or recovers named types, fonts,
and theme-token positions from `Document`. Corrupt graph IDs and post-check
static mutations fail with source-mapped `E196`; post-lowering raw Canvas and
theme-token mutations cannot affect generated Rust. Structural snapshots and an
ignored 4,000-command lower+emit budget complete the executable evidence.

Image, SVG, and viewer Media views are now a completed vertical slice. Stable
Media expression owners partition every dynamic source and option operand.
Checked semantic keys freeze kind, option presence, memory mode, filters,
colors, explicit hover-none behavior, and SVG style call topology. Lowering
retains source and fixed-length types, folds viewer scale defaults, resolves
theme colors and exact SVG style extern IDs, and publishes private
`ResolvedMedia` records. The backend no longer reads raw Media expressions or
options, repeats type inference, rediscovers SVG style externs, or recovers
theme-token positions from `Document`. Static post-check drift and corrupt
expression graphs fail with source-mapped `E196`; post-lowering Media AST and
theme-token mutations cannot affect output. Structural, mutation, production
generation, and ignored 4,000-node lower+emit tests provide the evidence.

Tooltip is now a completed vertical slice. Deterministic Tooltip expression
owners retain geometry, timing, custom-style arguments, gradients, borders,
radii, shadows, and pixel snapping. Checked facts freeze position, option and
style topology, exact container-style extern identity, and theme-color
spellings; lowering resolves those colors to token IDs and publishes a private
`ResolvedTooltip`. Rust emission reads only that record and checked expression
IDs. Post-check expression mutation is ignored, static drift fails with `E196`,
post-lowering raw options and theme-token order cannot change output, and the
4,000-node lower+emit contract remains below two seconds.

MouseArea and ResizeHandle are completed interaction-wrapper slices. Stable
interaction expression and route IDs retain every handler, component output,
named component event, route argument, payload index/type, cursor choice, and
source origin. Lowering validates exact view scope, target identity, ordered
payload contracts, component callback context, expression DAGs, and complete
arena consumption before publishing `ResolvedMouseArea` and
`ResolvedResizeHandle`. Their emitters no longer inspect raw options, routes,
route expressions, or handler names. App and component route generation,
post-check/static mutation, malformed IDs, post-lowering poisoning, and a
4,000-node lower+emit budget provide the executable evidence.

Sensor is a completed interaction-wrapper slice. Its checked contract freezes
the distinct show, resize, and hide route positions plus key, anticipation, and
delay presence. The shared interaction route arena retains app/component
targets, ordered size payloads, argument expressions, and origins, while
canonical option-expression IDs retain the key and timing values. Lowering
revalidates scope, types, expression DAGs, route topology, and complete arena
consumption before publishing `ResolvedSensor`. Its emitter consumes only that
record and checked expression IDs. Post-check expression mutation, static
drift, post-lowering option/route poisoning, component routes, and an ignored
4,000-node lower+emit budget provide the executable evidence.

Float is a completed structural-wrapper slice. Its checked expression arena
owns scale, translation, shadow, and radius operands, while eight typed view
locals model the original and viewport geometry visible only to the translation
callback. The static contract freezes shadow-color identity and style-field
presence. Lowering revalidates expression DAGs, exact local roles and scope,
theme-token IDs, and complete arena consumption before publishing
`ResolvedFloat`. Rust emission consumes only that record and checked IDs; the
old raw Float style emitter has been removed. Expression/static mutation,
post-lowering AST/theme poisoning, malformed IDs, and an ignored 4,000-node
lower+emit budget provide the executable evidence.

Pin is a completed structural-wrapper slice. Its checked expression arena owns
position and fixed-dimension operands, and its static contract freezes absent,
fill, fill-portion, shrink, and fixed length topology. Lowering distinguishes
numeric-fixed from native-length-fixed dimensions, revalidates expression DAGs,
scope, types, and complete arena consumption, then publishes `ResolvedPin`.
Rust emission consumes only that record and checked IDs. Expression/static
mutation, post-lowering AST poisoning, malformed IDs, and an ignored 4,000-node
lower+emit budget provide the executable evidence.

Responsive is a completed structural-wrapper slice. Stable expression IDs own
the breakpoint and fixed outer dimensions, while typed locals model size-mode
width and height bindings. The checked flow freezes mode, binding names, and
dimension topology. Lowering revalidates expression DAGs, scope, exact local
roles, types, and dimension contracts before publishing `ResolvedResponsive`.
Rust emission consumes that record and checked IDs; source nodes provide only
the branch children. Expression/static mutation, post-lowering AST poisoning,
malformed expression/local IDs, and an ignored 4,000-node lower+emit budget
provide the executable evidence.

Lazy is a completed structural-wrapper slice. Its checked flow owns the stable
dependency expression and typed callback local. Lowering revalidates expression
owner mapping, DAG, scope and type plus the local name/type/owner role before
publishing `ResolvedLazy`. Rust emission consumes that record; source nodes
provide only the child subtree. Dependency/binding mutation, post-lowering AST
poisoning, malformed expression/local IDs, owned-static codegen, and an ignored
4,000-node lower+emit budget provide the executable evidence.

KeyedColumn is a completed structural collection slice. Its checked flow owns
the stable list and key expressions, typed item local, normalized dimension
variants, spacing, padding, maximum width, and alignment. Lowering revalidates
expression owner mappings and DAGs, scope, types, local ownership, and static
option topology before publishing `ResolvedKeyedColumn`. Rust emission consumes
that record and uses the source node only for the child template. Malformed IDs,
post-check expression/static mutation, post-lowering AST poisoning, configured
codegen, and an ignored 4,000-node lower+emit budget provide the executable
evidence.

If is a completed control-flow slice. Its checked flow owns a stable boolean
condition expression. Lowering revalidates owner mapping, DAG, scope, type, and
coercion before publishing `ResolvedConditional`. Normal-layout and flex-layout
emission consume that record, while source nodes provide only child subtrees.
Malformed expression IDs, post-check and post-lowering condition poisoning,
existing layout codegen, and an ignored 4,000-node lower+emit budget provide the
executable evidence.

For is a completed control-flow slice. Its checked flow owns a stable list
expression and typed item local, and lowering resolves the reconciliation site
identity. Lowering revalidates owner mapping, DAG, scope, list/item types, local
ID, and local owner role before publishing `ResolvedIteration`. Normal-layout
and flex-layout emission consume that record; source nodes provide only child
subtrees. Malformed IDs, post-check list/binding mutation, post-lowering flow
poisoning, existing reconciliation codegen, and an ignored 4,000-node lower+emit
budget provide the executable evidence.

Table is a completed structural collection slice. Its checked flow owns the row
list, typed row local, table width and bounded metrics, and each column's width,
alignments, and origin. Lowering separates numeric and native-length fixed
widths, revalidates expression ownership and DAGs, scope, list/row types, local
role, origin parentage, and static table/column topology, then publishes
`ResolvedTable`. Rust emission consumes that record and uses source columns only
for header and cell subtrees. Malformed IDs, post-check expression/static
mutation, post-lowering AST poisoning, complete configured codegen, and an
ignored 4,000-table lower+emit budget provide the executable evidence.

PaneGrid is a completed stateful structural collection slice. One checked
record owns its persistent identity, recursive split configuration, dimensions,
metrics, resize/drag/click behavior, custom and typed grid styles, static panes,
dynamic templates, typed item/maximized locals, pane/title surfaces, control
topology, and parented origins. Lowering validates exact lexical-local contracts
for every expression along with DAGs, scope, types, routes, externs, theme
tokens, local roles, and origin links before publishing `ResolvedPaneGrid`.
Application storage, enum/configuration generation, messages, updates, helper
discovery, and rendering consume that record; source nodes provide only
content/title/control subtrees. Structural snapshots, malformed IDs,
pre-/post-lowering AST poisoning, source-merged style ownership before physical
origin remapping, complete production codegen, and an ignored 4,000-grid
lower+emit budget provide the executable evidence.

Overlay is a completed structural interaction slice. The checker retains its
visibility and padding expressions and optional dismiss route in the shared
interaction fact arena. Lowering revalidates expression ownership, DAGs, scope,
types, route IDs, target/argument shape, and origin parentage, resolves backdrop
color and alignment, and publishes `ResolvedOverlay`. Production emission reads
that record while source nodes provide only content/layer subtrees and the
shared widget ID surface. Structural assertions, malformed IDs, pre-lowering
semantic mutation, post-lowering AST poisoning, existing native overlay
codegen, and an ignored 4,000-overlay lower+emit budget provide the executable
evidence.

Match is a completed control-flow slice. Its checked flow owns the stable value
expression, exhaustive patterns, typed payload locals, arm origins, and ordered
child view IDs for each arm.
Lowering revalidates expression ownership and DAG, scope and value type,
Option/Result/enum/palette contracts, payload local types and owner roles,
declaration IDs, checked duplicate/missing/wildcard coverage, origin
parent/source identity, and per-arm child topology before publishing
`ResolvedMatch` with resolved Rust owner and variant names. Normal-layout and
flex-layout emission consume its payload binding type without reopening checked
Match flow/local types or pattern declaration-index entries. Raw child spans are
mapped to stable view IDs only to verify that their subtrees remain attached to
their resolved arms. Malformed IDs fail at the source-mapped arm during
lowering; post-check and post-lowering AST poisoning, coverage/topology
corruption, imported diagnostics, typed-pattern coverage, and an ignored
4,000-node lower+emit budget provide the executable evidence.

First-class tests are now a completed HIR slice. `TestId`, `TestTargetId`, and
`TestStepId` form parented declaration arenas; target aliases are typed locals,
and every dynamic target key or step operand has a deterministic checked
expression owner. Test configuration, target path topology, static action
metadata, dispatch handler identity, and exact source spelling are frozen as
checked semantic contracts before lowering. Lowering converts every action,
key/IME/window/touch event, accessibility assertion, and expectation into a
`ResolvedTestStepKind`; direct paths retain resolved checked key expressions,
aliases retain target IDs, dispatch retains an App `HandlerId`, and equality
assertions retain the checked comparison children used for `check_eq` or
`check_ne`. Numeric range, index, positive-count, expression-owner, handler
signature, origin, and complete-arena contracts are revalidated while the HIR
is built.

The test backend iterates only `ResolvedTest` records and checked expression
IDs. It does not consume `TestDecl`, `TestStep`, raw `Expr`, or raw route
semantics, and post-check expression mutations cannot affect generated Rust or
the retained diagnostic statement. Test mount rendering still uses the general
view renderer and therefore remains subject to the open view-structure HIR
boundary rather than a test-specific fallback. Structural ID/owner snapshots,
config/target/step corruption tests, post-check AST poisoning, imported source
locations, complete semantic-action generation, and an ignored 4,000-step
lower+emit budget are the executable evidence.

Initializer typing and fact lowering are linear in the expression tree. The
checker performs one authoritative post-order analysis for each initializer and
hands the owned result to fact construction; fact construction cannot invoke a
second analysis. It consumes that table instead of restarting subtree type
checks.
Composite list, optional, and result evidence is unified recursively before an
otherwise unconstrained type variable is canonically erased to `unit`; parent
and child facts consequently retain one exact concrete type. Initializer uses
record their source type, destination type, and explicit state-construction
coercion separately, including list-to-combo and value-to-animation conversion.
Context-sensitive builtins share one signature/context model between checking
and fact construction. That model distinguishes ordinary values, binding-name
arguments, and expressions evaluated under a binding. Consequently,
`animation.project` retains its binder as a typed local ID with an owning
expression use and body-argument scope; reads in the projection body resolve to
that local rather than to an unresolved source path. Exact analysis-pass,
query, analyzed-node, cache-hit, local, lowered-expression, layered-scope, and
full-clone counters make the linearity and scope contracts testable. Each
lexical scope constructs its base path and type views once. Binding bodies such
as `animation.project` and view-flow locals borrow that base through small
overlays in checking, fact lowering, and code generation instead of cloning the
full environment. The 500-to-4,000 repeated-projection and sibling-scope
contracts verify exact overlay growth, linear binding allocations, and zero
full-scope clones under wall-clock ceilings. The thread-local
collection context is guarded across both ordinary errors and panic unwinding.

The migration is complete when:

- the code-generation module has no source-AST or checker dependency;
- `CheckedDocument` no longer dereferences to `Document` as an escape hatch;
- structured HIR fixtures cover every accepted semantic family;
- backend tests assert only backend-specific output where possible;
- imported-source diagnostics and generated Rust source maps retain their
  existing behavior; and
- lowering and generation remain within explicit performance budgets.

## Rejected alternatives

### Keep generating from the checked AST

This leaves syntactic alternatives visible to the backend and permits semantic
decisions to spread between checking and emission. The cost grows with every
feature even while Core syntax is frozen.

### Add isolated, permanent mini-IRs

Feature-specific lowering can help migration, but leaving unrelated AST and HIR
paths as the final architecture preserves the same ambiguous boundary. Mini-IRs
are accepted only as steps toward the complete `LoweredProgram`.

### Put Rust or Iced constructs in HIR

Backend-shaped nodes would make semantic tooling depend on emission details and
would prevent API inspection and other consumers from sharing the canonical
program meaning. Backend-specific planning stays private to code generation.

### Replace the pipeline in one large change

A big-bang conversion would make source-map and generated-program regressions
difficult to isolate. The destination remains complete HIR, but migration is
split into independently verified vertical slices.

## Consequences

The compiler gains another owned representation and an explicit lowering pass.
That adds implementation work and some build-time memory, which must be
measured. In return, the checker and lowering boundary become the single source
of semantic truth, code generation becomes a translation step, HIR fixtures can
test meaning without comparing large Rust strings, and future tooling can
consume the same canonical program model.

During migration, every pull request must identify the semantic family it
moves, the old path it deletes, its invalid cases, its source-origin evidence,
and any performance change. New Core syntax remains out of scope until the full
boundary is complete and the Core admission rule independently justifies it.

## Revisit trigger

Revisit the representation when measurements show that HIR construction or
retention materially violates compiler performance budgets, or when an accepted
Ice semantic cannot be represented without backend knowledge. A revisit may
change storage, IDs, or internal node boundaries; it must not restore an AST or
checker backdoor into code generation.

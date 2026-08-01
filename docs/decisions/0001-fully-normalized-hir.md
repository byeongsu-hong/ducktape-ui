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
view/canvas color fields remain AST-backed with the expression/view family.
Palette enum paths inside the general expression emitter also remain in that
family. Their backend helpers are removed only when those expressions and view
options gain normalized nodes; no compatibility fallback is added.

The program still owns AST-backed nodes for semantic families not yet migrated.
The remaining expression-backed native styles/colors, handlers, tasks and
asynchronous call sites, canvas locals, other application settings, tests, and
remaining widget options therefore remain open implementation slices; this
status does not satisfy the migration-complete criteria below. Current
diagnostics and generated source markers still use the established source-map
path rather than traversing `OriginId` parent stacks.

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
Handler/task/canvas/settings/test expressions and expression-bearing
widget options remain later slices, so the full-HIR completion criteria remain
open.

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

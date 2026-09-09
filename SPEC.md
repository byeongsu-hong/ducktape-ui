# Ice Language Specification 2.0

Status: implemented candidate.

Ice is a statically checked declarative frontend language for iced. It covers
the pinned public, application-facing iced surface without embedding Rust
syntax, JSX, or a token shortcut around a procedural macro. A frontend parses
`.ice` source, resolves names and types, checks UI semantics, and lowers a typed
tree to backend code.

This document is the contract: the rules the language is designed to, the
invariants that cannot be read off a construct listing, and the typed boundary
to Rust. It does not restate what the compiler already publishes.

## 1. Sources of truth

| Question | Authority |
| --- | --- |
| Which constructs exist, and their syntax, properties, children, and routes | `cargo ice schema` → `core.constructs` |
| Which types, utilities, recipes, and status cascades are accepted | `cargo ice schema` → `core.types`, `core.style` |
| Which iced surface is covered, and the evidence for it | [`COVERAGE.md`](COVERAGE.md) |
| What a diagnostic means and how to fix it | the diagnostic itself; `cargo ice check` |
| How to author a screen | `skills/design-ice-ui/` |
| How to test one | [`docs/testing.md`](docs/testing.md) |
| Tooling, the LSP, and the development runner | [`docs/tooling.md`](docs/tooling.md) |

`cargo ice schema` is generated from the compiler and cannot drift. Prefer it to
any prose listing, including this one.

Language revisions and Cargo package versions use separate schemes. This
document specifies language revision 2.0. The workspace packages are pre-1.0
SemVer `0.1.0`; their package version does not claim language 0.1. The resolved
iced/iced_widget versions are a third, independent backend baseline.

## 2. Design contract

Ice optimizes for two readers:

- a person should understand the screen, state, and effects by scanning it;
- an agent should see one canonical construct for each operation and receive a
  local error instead of guessing framework conventions.

The language therefore follows these rules:

1. Structure is indentation, with no closing delimiters.
2. UI state and transitions are explicit; generated messages and borrows are
   not.
3. Expressions are a small closed language, not embedded Rust.
4. Style utilities are a checked vocabulary. Unknown or ineffective utilities
   are errors — silent CSS-like no-ops are not allowed.
5. Domain work crosses a typed `extern` boundary.
6. The compiler has one parser and checker shared by every frontend.
7. Every public, application-facing capability in the pinned iced baseline has a
   checked representation through canonical Ice syntax or a typed boundary.

Ice owns transient/display state, layout, style, event routing, and calls to
actions. Rust owns validation, invariants, persistence, networking, security,
observability, and platform-specific behavior.

```text
interaction -> handler -> extern async Rust fn -> result handler -> state -> view
```

UI validation such as disabling an empty submit button is only a convenience.
The Rust action must still validate its input.

### Vocabulary evolution

A new Core construct must be common UI authoring, have one canonical source
form, and be meaningfully more declarative than an existing typed Rust boundary.
Completeness does not require a dedicated keyword for every iced method, but
every coverage gap must close through one of those two checked representations.

The implemented 2.0 vocabulary is frozen during preview stabilization. Removed
spellings are syntax errors; the formatter never translates old vocabulary, and
removed forms and their callers are deleted in the same change rather than
retained behind compatibility paths.

When the pinned iced baseline changes, every new or changed public,
application-facing behavior is a coverage obligation. The upgrade is not
complete until [`COVERAGE.md`](COVERAGE.md) records a native checked
representation and its evidence rule passes. That obligation may extend direct
syntax or an existing typed boundary; it does not by itself justify another
keyword.

## 3. Compiler model

```text
UTF-8 .ice source graph
  -> relative `use` resolution + source map
  -> indentation-aware parser
  -> AST
  -> name resolution + type inference + semantic checks
  -> CheckedDocument
  -> private normalized LoweredProgram
  -> iced Rust backend + published view template
  -> rustc
```

`ui-lang-core` owns the parser, AST, checker, formatter, and backend.
`ui-lang-build` is the Cargo build-script adapter, `ui-lang` is the include-only
proc macro, `ui-lang-template` is the single definition of the published view
format the backend writes and the runtime reads, and `cargo-ice` owns workspace
tooling. There is no runtime parser: the runtime deserializes that template,
never Ice source.

### Build contract

A consuming package declares `ui-lang-build` as a build dependency and compiles
its Ice source directory through Cargo's standard build-script phase:

```rust
// build.rs
fn main() {
    ui_lang_build::compile_dir("src/ui").expect("compile Ice sources");
}
```

```rust
ui_lang::include_app!("src/ui/tasks.ice");

fn main() -> iced::Result {
    Tasks::run()
}
```

The build helper discovers every top-level `app` or `daemon` root below that
directory, checks each complete import graph, emits dependency tracking for all
root and imported `.ice` files, and writes generated Rust below
`OUT_DIR/ui-lang-generated`. Cargo therefore isolates output by consuming
package, profile, and target, and removes it with `cargo clean`.
`OUT_DIR/ui-lang-generated/manifest.json` is the canonical versioned mapping
from generated filenames back to source roots and content digests. Generated
lint macro names depend on the app name; outlined fragment identifiers hash
paths relative to the app declaration, including sibling `../` imports. These
identities do not depend on relocation within one filesystem or the ambient
Cargo environment.
Physical include paths and diagnostic source locations remain unchanged. This
removes one source of binary nondeterminism; reproducible Wasm builds still need
a pinned toolchain, consistent optimization and path remapping.

Publication is a directory-locked transaction: each changed output and the next
manifest are staged, flushed, and synced before outputs are atomically replaced
and the manifest is replaced last. Missing, malformed, unsupported, incomplete,
or digest-mismatched cache state is disposable and triggers full regeneration.
A hash collision is a hard build error, and byte-identical output is not
replaced, so its mtime stays stable.

The macro performs no parsing, code generation, or filesystem writes: it maps
the manifest-relative literal to the corresponding `OUT_DIR` file and expands one
`include!`. Generated Rust emits probes for every declared extern struct field,
function, and component, including declarations with no view call site, so rustc
rejects missing, private, or shape-incompatible Rust items even when a
declaration is never reached at runtime.

Generated Rust refers to the public `::iced` and `::ui_lang_runtime` paths, so a
consuming application must declare `iced`, `ui-lang-runtime`, and the
`ui-lang-build` build dependency directly at their exact pinned versions. The
headless test driver is not a default runtime feature; test builds require
`ui-lang-runtime` with `test-runtime` as a dev dependency. `cargo ice compat`
verifies the lockfile and direct-manifest contract.

### Representation boundary

Successful semantic analysis returns the nominal `CheckedDocument`; only the
checker can construct it. Lowering consumes that value and publishes an owned
`LoweredProgram`, the only input Rust generation accepts. Neither representation
contains iced values or generated Rust fragments.

The release `LoweredProgram` contains no source `Document` and no checker-fact
arena. It owns typed arenas and stable IDs for declarations, expressions,
values, locals, handlers, statements, tasks, views, routes, subscriptions,
tests, components, styles, themes, and physical origins. Lowering fixes
defaults, ownership, lexical scope, coercions, static topology, route payloads,
extern and named-type identities, Rust targets, and source locations before the
backend runs. `OriginId` values index one physical origin arena, so imported
diagnostics and generated source markers never recover locations from AST nodes.

Expression emission reads the owned `ResolvedExpressionProgram`. Release code
generation cannot reach `CheckedFacts`, repeat checker analysis, resolve an
extern by source name, or fall back to a raw expression. The `hir_boundary`
integration ratchet keeps that inventory empty and is the executable statement
of this section.

## 4. Source rules

- Files are UTF-8 and use the `.ice` extension.
- Tabs are errors. `cargo ice fmt` prints two spaces per indentation level.
- A deeper indentation level makes the following lines children of the prior
  line. Indentation may only return to an existing level.
- Empty lines are ignored by the parser and normalized by the formatter.
- A line whose first non-space characters are `//` is a comment. Inline and
  block comments are not part of 2.0.
- Identifiers use ASCII letters, digits, and `_`; they cannot begin with a digit
  or `__`, and `_`, `none`, and Rust keywords are reserved. Rust path segments
  follow Rust identifier rules instead.
- App, extern-struct, and component names use `PascalCase`; state, field,
  function, handler, and parameter names use `snake_case`; static IDs use kebab
  case after `#`, for example `#task-list`.
- Strings use double quotes and support `\n`, `\r`, `\t`, `\"`, and `\\`.
- `use "relative/file.ice"` includes declarations relative to the importing
  file. Paths must end in `.ice`, use `/`, and cannot be absolute.
- `use "relative/file.ice" as ui` imports components, recipes, extern items,
  fonts, and named types under `ui::`. Theme tokens remain app-global.
- Imports may be nested. Re-importing the same canonical file is idempotent;
  aliased instances are unique by canonical file and namespace. Import cycles
  and missing files are errors.

An Ice source graph has exactly one `app` or `daemon` root and exactly one
`view`; the view and each component have exactly one root node. Top-level
declarations are order-independent, but canonical source orders them
`app|daemon`, `use`, `extern`, `theme contract`, `palette`, `recipe`, `state`,
`preset`, `component`, `on`, `subscribe`, `view`, `test`.

A graph may have multiple `extern` namespaces, so imported plugin fragments can
bind their own Rust modules beside the application's backend. Bare extern type
and function names are graph-global and duplicates are errors; aliased imports
retain their namespace identity instead.

### Component slot cardinality

A component declaration uses `slot` (the default `children` name) or `slot name`
for a required single-root slot, `slot name?` for an optional single-root slot,
and `slot name*` for zero or more caller roots. A multi-child slot can be omitted.
Multiple direct component children fill its default `children` slot; named
`name:` blocks may similarly contain multiple roots. Compound component children
continue to map to their family's named slots.

A multi-child slot expands into the receiving layout's sibling list, including
conditional and iterative branches. It may forward to another multi-child slot.
It is invalid as scalar content in a box, button, scroll viewport or component
root, or when forwarded into a single-root slot. No implicit column or other
widget is introduced, and an explicit caller layout remains one grouped child.
Empty content contributes no child or inter-child spacing. Expression/handler
bindings stay in the caller; rendered scopes stay at the receiving placement.
API extraction records whether each slot is required and accepts multiple roots.
`stack under=N` counts the resulting rendered children, including children
expanded by slots, conditions and loops; it does not count source declarations.

## 5. The Rust boundary

This is the one part of the contract the schema does not carry: what a
declaration requires of the Rust item behind it. Generated probes type-check
every declaration against the actual item.

### Types

| Ice | Rust extern type |
| --- | --- |
| `bool` | `bool` |
| `i64` | `i64` |
| `f64` | `f64` |
| `str` | `String` |
| `bytes` | `Vec<u8>` |
| `[T]` | `Vec<T>` |
| `T?` | `Option<T>` |
| `result[T,E]` | `Result<T, E>` |
| `unit` | `()` |
| `Name` | the named struct in the extern namespace |
| declared UI enum `Name` | generated Rust enum `Name`; fieldless enums are `Copy + Eq + Hash`, payload enums are `Clone` |
| `combo[T]` | `iced::widget::combo_box::State<T>` |
| `animation[bool]` | `iced::Animation<bool>` |
| `animation[f64]` | `iced::Animation<f32>`; expressions convert at the Ice numeric boundary |
| `animation[Name]` | `iced::Animation<crate::...::Name>`; rustc verifies `Copy + PartialEq + iced::animation::Float` |
| `image` | `iced::widget::image::Handle` |
| `image-allocation` | `iced::widget::image::Allocation` |
| `image-memory` | `Weak<iced::advanced::image::Memory>` |
| `image-error` | `iced::widget::image::Error` |
| `size-u32` | `iced::Size<u32>` |
| `debug-span` | `iced::debug::Span`; only valid as optional owned state |
| `markdown` | `iced::widget::markdown::Content` |
| `editor` | `iced::widget::text_editor::Content` |
| `event` / `event-status` | `iced::Event` / `iced::event::Status` |
| `key-press` | generated native keyboard press payload |
| `instant` | `iced::time::Instant` |
| `task-handle` | `iced::task::Handle` |
| `window-id` / `window-screenshot` | `iced::window::Id` / `iced::window::Screenshot` |
| `window-position` / `window-direction` / `window-level` / `window-mode` / `window-attention` | the matching `iced::window` value type |
| `redraw-request` | `iced::window::RedrawRequest` |
| `color` / `background` / `gradient` / `linear-gradient` / `color-stop` | the matching `iced` or `iced::gradient` type |
| `font` / `font-family` / `font-weight` / `font-stretch` / `font-style` | the matching `iced::Font` or `iced::font` type |
| `theme-mode` | `iced::theme::Mode` |
| `text-alignment` / `text-shaping` / `text-wrapping` / `text-line-height` | the matching `iced::widget::text` type |
| `length` / `alignment` / `horizontal-alignment` / `vertical-alignment` | the matching `iced` or `iced::alignment` type |
| `border` / `radius` / `shadow` | `iced::Border` / `iced::border::Radius` / `iced::Shadow` |
| `pixels` / `padding` / `degrees` / `radians` / `rotation` / `content-fit` | the matching `iced` value type |
| `mouse-interaction` / `scroll-delta` | `iced::mouse::Interaction` / `iced::mouse::ScrollDelta` |

Values crossing into iced messages must satisfy the traits generated code
requires, notably `Clone`. Generated app and message debug output is opaque, so
extern state and payload types do not additionally need `Debug`.

Struct declarations are read-only views of Rust data: Ice may read a declared
field (`task.title`) but cannot construct or mutate the struct. Declaring a
field or function does not create it.

### Effect kinds

One namespace keeps declarations short, and the namespace is the Rust module
path:

```ice
extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  list_tasks() -> [Task] ! AppError
  pure normalize_error(error:NetworkError) -> AppError
  sync now() -> instant
```

- A bare function is `async fn(...) -> B`; `! E` makes it
  `async fn(...) -> Result<B, E>`, and a fallible extern requires both success
  and error routes.
- `pure` is a trusted promise about the Rust body: same arguments, same value,
  no observable side effect. Ice may therefore reevaluate it in every checked
  expression context — state initializers, application settings, derived values,
  views, component defaults, subscription filters, custom easing, handlers, and
  tests.
- `sync` makes no determinism or effect promise. It is accepted only where
  evaluation happens exactly once and immediately: a top-level app state
  initializer, or an immediately evaluated app, component, or preset handler
  expression. Component state initializers reject it because rendering may
  initialize them again; task route expressions reject it because both branches
  materialize as owned snapshots at launch.
- Neither synchronous kind may declare `! Error`.

The compiler probes types but never inspects Rust bodies, so a dishonest `pure`
declaration is a backend contract violation, not a compile error.

### Borrowed parameters

A `pure`, `sync`, or extern-component parameter may borrow with `&type`. The
call site is unchanged; the generated call passes a reference to the state
field, local, `for` row, or lazy alias instead of cloning it. `&str` lowers to
`&str`, `&bytes` and `&[T]` to slices, `&editor` to
`&iced::widget::text_editor::Content`, and any other `&T` to a shared reference —
the signature probe requires that reference, so an owned Rust parameter fails
`cargo check`.

Because the output is owned Ice data, the borrow ends with the call, so borrowed
parameters work in handlers, views, derived values, subscription conditions,
component bodies, and `lazy` subtrees alike. Asynchronous externs and the
fixed-shape adapters reject `&`: a spawned future cannot borrow app state, and
easings and subscription filters receive values by value. A `secret` parameter
cannot borrow either — a reading is handed over once and wiped on return.

An immediate self-assignment through one `pure` or `sync` call moves an `editor`
or list field into that call when the right-hand side references the field
exactly once through an owned parameter: `rows = append(rows, next)` transfers
the owned list. Reading the target more than once keeps clone semantics, and a
read that feeds a `&` parameter borrows and then assigns.

### Typed adapters

Typed adapter declarations expose framework capabilities without embedding Rust
expressions in Ice. Their required Rust signatures are:

```rust
fn native_help(active: bool) -> iced::Element<'static, bool>;
fn borrowed_help<'a>(label: &'a str, active: &'a bool)
    -> iced::Element<'a, bool, iced::Theme, AppRenderer>;
fn by_kind(kind: String) -> impl iced::widget::selector::Selector<Output = String>;
fn status_shader(speed: f64) -> impl iced::widget::shader::Program<bool>;
fn copy_text(text: String) -> iced::Task<()>;
fn task_steps(count: i64) -> impl iced::futures::Stream<Item = i64> + Send + 'static;
fn download(url: String) -> impl iced::task::Straw<Vec<u8>, f64, AppError> + Send + 'static;
fn events(channel: i64) -> impl iced::advanced::subscription::Recipe<Output = String>;
fn runtime_event(event: iced::advanced::subscription::Event) -> Option<String>;
fn app_events() -> iced::Subscription<bool>;
fn app_theme(dark: bool) -> iced::Theme;
fn alternate_panel(active: bool) -> (
    Option<AlternateTheme>,
    iced::Element<'static, bool, AlternateTheme>,
    Option<fn(&AlternateTheme) -> iced::Color>,
    Option<fn(&AlternateTheme) -> iced::Background>,
);
fn docs_viewer(prefix: String) -> impl for<'a> iced::widget::markdown::Viewer<'a, String>;
fn editor_keys(event: iced::widget::text_editor::KeyPress, readonly: bool)
    -> Option<iced::widget::text_editor::Binding<EditorCommand>>;
fn editor_highlight<'a, Message: 'a>(
    editor: iced::widget::text_editor::TextEditor<'a, iced::advanced::text::highlighter::PlainText, Message>,
    token: String,
) -> impl Into<iced::Element<'a, Message>>;
fn editor_surface(theme: &iced::Theme, status: iced::widget::text_editor::Status, readonly: bool)
    -> iced::widget::text_editor::Style;
```

Every `*-style` adapter follows the same shape: the current `&iced::Theme`, then
the widget's native `Status` where it has one, then the declaration's owned
arguments, returning that widget's native `Style`. `box-style`, `menu-style`,
`panes-style`, `text-style`, and `progress-style` receive no `Status`.
Structured Ice properties and utilities override the returned base.

Declared adapters are infallible; errors are ordinary event payloads when an
adapter needs them. `theme` is the one adapter whose return type is implicit, so
no `->` appears in Ice. Native consumers must enable the matching iced Cargo
feature (`wgpu` for shaders, `canvas`, `sipper`, `selector`, `image`).

On the tree target, `markdown` state holds source-preserving
`ui_lang_guest::Markdown`. A default markdown view emits the `ice.markdown`
surface; `viewer=name(args)` emits the declared viewer name. Argument zero
is a `ui_lang_wire::MarkdownDocument` record containing source, resolved
settings and the guest palette; subsequent arguments and return events use
the ordinary typed surface encoding. The native viewer factory is not linked
into the guest. The host owns parsing, rendering and any image assets.
Default and monospace fonts are supported; named fonts and inline gradients
are rejected with E190 on this target. The default runtime provider requires
the `markdown` feature (included in `full-runtime`).

Native `task widget focus` and `task widget focused` accept identified buttons,
including buttons inside component and keyed-row scopes. They use the generated
accessible button's native focus ID; assign navigation state before requesting
focus on a newly rendered destination.

On the tree target, checked `task widget` focus, focused-query, input cursor/
selection and scroll/snap statements emit `host.widget` requests containing
`ui_lang_wire::WidgetCommand`. Qualified widget paths are copied exactly;
no native widget Id or operation object crosses. The host waits for the
requesting frame to be mounted and traverses only that guest's widgets and
overlays. Focused queries return bool (false for a missing target); mutations
acknowledge unit, preserving sequential Task ordering. `scroll-to-key` copies the row identity as the same bool/integer/float bits
used by virtual columns and invokes native measured-row reveal. Missing keys
do nothing. Widget selectors and explicit host-window targets are rejected
with E190 on this target. Native extern
Tasks returning opaque widget operations are outside this lowering contract.

Host surface registries may bind host-owned sessions per guest instance.
Copied node keys are local identities, not authority to look up process-wide
native resources. The example host distinguishes registry/mount identity from
session ownership: unmount releases view state without ending an independently
owned host session. Only semantic copied arguments/events cross this boundary.

## 6. Semantic invariants

These are the rules a construct listing cannot express.

### Expressions

The expression language is closed. There is no arbitrary Rust expression, method
call, closure, general allocation API, or implicit truthiness; a new operation
either belongs in the universal built-in set or behind a typed extern. It
contains:

- literals: strings, booleans, `i64`, `f64`, `none`, list literals, and
  hexadecimal `bytes(00 ff ...)`;
- paths: `state_name`, `parameter`, `item.field`;
- `!` and unary `-`; `* / % + -`; `== != < <= > >=`; `&& ||`; parentheses;
- value built-ins: `len`, `empty`, `trim`, `some`, `ok`, `err`, `encoded`,
  `rgba`, `aborted`, `markdown`, `markdown_images`;
- namespaced native built-ins: `key.*`, `mouse.*`, `touch.*`, `point`,
  `vector`, `size`, `rectangle.*`, `transform.*`, `pixels`, `padding.*`,
  `degrees`, `radians`, `rotation.*`, `fit.*`, `color.*`, `length.*`,
  `alignment.*`, `horizontal.*`, `vertical.*`, `image.downgrade`,
  `image.upgrade`, `window_id.*`, `debug.active`, `debug.time_with`, and the
  animation queries `animation.value`, `animation.animating`,
  `animation.interpolate`, `animation.remaining`, `animation.project`;
- calls to declared `pure` or `sync` externs, subject to the context's effect
  boundary.

Declared `pure` and `sync` names take precedence over ordinary built-ins;
`bytes` stays reserved for the byte-literal syntax. `cargo ice check` reports
the exact signature of any built-in used incorrectly.

The **recomputation-unsafe** built-ins are `window_id.unique`, `aborted`,
`debug.time_with`, `image.upgrade`, the unqualified constructors `encoded` and
`rgba`, and the animation queries when their explicit instant is omitted. They
read runtime state or mint a fresh retained identity per call, so they are
accepted in top-level app state initializers, handlers, and views, and rejected
anywhere the compiler may re-evaluate an expression — notably `derived`.

### Revisions and the derived cache

Every app and component state field carries a compiler-owned revision: a counter
generated code ticks on every write, and that nothing in Ice can read — there is
no `rev()` built-in. An assignment compares the new value with the stored one
first when the Rust type implements `PartialEq` (Ice's own scalars, strings,
lists, optionals, and the enums it generates over those), so storing an equal
value leaves the revision alone. An assignment of a type that cannot compare, an
in-place mutation (an editor action, a combo push, a markdown append, an
animation start), and a self-assignment that already took the old value out of
the field all count as a change.

`derived` declarations are pure read-only expressions over app state and other
derived values. They lower to computations cached on the application: computed
on first read, kept across frames, and cleared when a handler, controlled
widget, or test step writes a state field the expression reads — directly or
through another derived value. The compiler derives that dependency set from the
expression itself. No runtime dependency graph, signal, or handler-maintained
mirror exists, and a handler never keeps a list-shaped derived value in sync by
hand. The write clears the cache before the handler continues, so a read after a
write in the same handler observes the fresh value.

The cache is semantics-preserving by construction, which is why derived
expressions reject `sync` externs and recomputation-unsafe built-ins — those
that read runtime state or mint a fresh retained identity per call. Capture such
a value in state from an initializer or handler, then derive from that state.

### Effects and delivery lanes

Handlers are the only place state changes. Every statement that immediately
returns an iced `Task` must be final; `return if`, pane mutations, and
`invalidate lane=` are synchronous and may precede later statements.

Every Future and stream explicitly selects a delivery mode. `run every` and
`stream every` deliver every completion or item and own no lane. Work that can
be superseded names a static lane:

- `latest` advances the lane generation and routes only the current
  generation's success or failure. It does not cancel stale Futures: they and
  their captured values live until they finish.
- `replace` also aborts the prior task when a replacement is installed.
- `stream latest` is rejected, because an obsolete stream is not guaranteed to
  terminate.

A lane belongs to the state owner, not to a handler or source location. The
top-level app is one owner, a daemon is one owner shared by all of its windows,
and each component instance is an independent owner. Equal fully qualified lane
names therefore join starts across handlers for one owner, while two component
instances never interfere. One owner must use one effect kind and one delivery
mode per name.

Per-owner lane bookkeeping is fixed by the source-declared names; it never
allocates entries from runtime request keys. Aborting is not transaction
rollback: effects already performed remain, detached backend work may continue,
and already queued messages stay queued but fail the generation check. Use
cancellation-safe or idempotent Rust boundaries when that matters.

A subscription can split fallible items with
`run events() -> received _ | failed _` inside `subscribe`. The success handler
receives `T` and the failure handler receives `E` from `Result<T, E>`; an error
item does not terminate the stream. A single route keeps receiving the complete
item, including `Result<T, E>`. Filtering still runs on the original source
payload before routing. A failure route requires the filtered item to remain a
`Result`; `with=` context precedes the payload in both routes, and `when` keeps
its existing activity semantics. Routes accept only `_` payloads, or no arguments
to ignore them. Native and Tree targets use the same branch mapping. A Tree
`every` cannot use a filter: its native source payload would require an `instant`
that a view module cannot create.

### Identity

IDs are identities, not CSS selectors. Static IDs are unique in their local
view/component scope; repeated instances use a stable typed key. The logical
identity is hierarchical:

```text
App / component-instance / local-node
Tasks/task(42)/root
```

A component call must carry an explicit ID to create a public instance segment;
without one it receives an internal source-scoped identity used only for state
isolation. An ordinary `for` adds no public segment. The backend may keep a
private index scope for automatic accessibility identities and no-ID component
state, but that scope is never part of a test or widget-operation target.

Every concrete rendered node accepts a direct `#id`, and its test target uses
that node's actual layout and hit-test bounds. `if`, `for`, and `slot` render no
box and accept no ID. A component-call `#id` is an instance scope and must be
followed by a rendered descendant ID.

### Layout boundaries

A component use is a layout boundary the compiler inserts on its own. When every
expression the use evaluates — its arguments, slot content, body, and the bodies
of the components that body uses — and every `input` binding below it read only
app state, derived values, the instance's own state, palette entries, and locals
the subtree declares, and every widget below it lays out from its own element and
limits alone, the generated code keys the subtree's layout node on the revisions
of those reads and skips both iced's diff of the nodes below and their layout
walk while they hold.

**The element is still built on every pass.** Only the walks are saved. A `&`
parameter, a controlled `input`, or an `editor` binding therefore lives under the
boundary unchanged. The same applies to `lazy` and to `virtual-row=`: a
virtualized row is not an unmounted row — rows stay in the tree, keep their
widget state and their clock, and are simply not measured, drawn, or offered
events while offscreen. Virtualizing a list changes how many rows are laid out,
never how many are built.

Because publishing a child's semantics requires laying it out, a virtualized
column exposes only its visible slice to assistive tech and to `.ice` tests. It
does tell that slice which slice it is: each mounted row is published with its
`position_in_set` among the whole collection and the collection's `size_of_set`,
so a reader moving through the list is placed in it rather than in the
screenful. A row that sets its own position keeps it. What a virtualized column
still cannot do is expose a row without laying it out, so it publishes no active
descendant and nothing offscreen is reachable without scrolling to it; a
collection that must be enumerated without scrolling needs a real list widget.

### Text alignment

Text's `align-x` positions each line inside the paragraph; `align-y` positions
that paragraph inside its assigned height. Parent alignment positions the whole
widget independently. Plain and rich text share these rules.

For `align-x=justified`, soft-wrapped non-final lines distribute their spaces
across the finite available line width. A paragraph's final line retains its
natural width; explicit newline-only text and single-line text do not expand.
Unbounded paragraphs use natural measured widths. A shrink text with justified
soft wraps measures the expanded lines, so it can occupy the available width.

### Native minimum-cell grids

`grid min-cell=M gap=G` chooses as many equal columns as fit its available
content width `W`: `max(1, floor((W + G) / (M + G)))`, capped by the nonempty
item count. Each cell receives `(W - (columns - 1) * G) / columns`; an odd
final row uses those same column tracks. If `W < M`, the one cell receives `W`
so the requested minimum never forces horizontal overflow. Empty grids have no
rows or gaps. Parent and grid padding remain outside the cell calculation.

Rows retain natural height. `min-cell` cannot combine with grid `h=`; an inner
`grid cols=1 h=aspect(width,height)` supplies aspect-ratio cards through existing
composition. Fixed-column and `max-cell` grids retain native Iced sizing.
The Tree target still represents minimum-cell grids as ordinary flex items;
its equal-column and narrower-than-minimum behavior remains separate work.

### Accessibility

First-class native test targets expose `accessibility_level:i64` for retained
heading or hierarchy levels. The accessor fails when no level is present;
capture manifests include the nullable `accessibility.level` property.

Ice owns a checked accessibility layer above stock iced. Generated Core nodes
produce a deterministic AccessKit tree:

| Ice node | AccessKit role | Semantic state |
| --- | --- | --- |
| `text` | `Label`, or `Heading` with `heading=1..6` | the visible text is its value; a heading carries its level; `live=polite\|assertive` makes it a live region, announced when the value changes |
| `input` | `TextInput` | label, optional description, value, disabled/focus state; one `TextRun` child carrying the value grapheme by grapheme, and the caret or selection as a text selection into it; `SetTextSelection` moves the caret |
| secure or `secret` `input` | `PasswordInput` | label, optional description, disabled/focus state; no value is exported |
| `button` | `Button` | label, optional description, toggled/expanded state, disabled/focus state, click action |
| `checkbox` | `CheckBox` | label, optional description, toggled/disabled/focus state, click action |
| `toggler` | `Switch` | label, optional description, toggled/disabled/focus state, click action |
| `radio` | `RadioButton` | label, optional description, selected/checked state, disabled/focus state, click action |
| `slider` | `Slider` | default `Slider` label, current value, numeric value/min/max/step, focus state, increment/decrement actions — each runs the change route with the value one step away, clamped to the range, and is absent at that end of the range |
| `progress` | `ProgressIndicator` | default `Progress` label, current value, numeric value/min/max |
| `pick` / `combo` | `ComboBox` | placeholder or search label, selected value, focus state |
| `editor` | `MultilineTextInput` | placeholder or default label, current value, disabled/focus state; one `TextRun` child per line carrying it grapheme by grapheme, and the caret or selection as a text selection into them; `SetTextSelection` moves the caret through the program's own editor state |
| labeled `image` | `Image` | label and optional description |
| any node inside a `scroll` | its own role | `ScrollIntoView`: every scroll around the node, identified or not, moves just far enough to show it |
| the node under a `tooltip` | its own role | the tip's text is its description, unless it declares one |

A positional input label, compact button string, and visible checkbox or toggler
label are default accessible names; `label=` overrides them. A button whose
content is a child node requires `label=` (`E105`); an image without one is
decorative and omitted from the semantic tree.

Semantic read order and keyboard focus order follow view-tree order. Tab and
Shift+Tab traverse enabled interactive controls; disabled controls expose
disabled state but no focus or click action. Wrapper-focused controls draw their
outline only while focus is visible, matching the web's `:focus-visible`
semantics — keyboard, accessibility, and programmatic focus show it, a pointer
press does not, and a key press on a pointer-focused control restores it. Text
entry controls keep iced's native focused rendering, which follows the web's
text-entry heuristic instead. There is no numeric focus-order syntax. In a
daemon, Tab traverses the window it was pressed in, and every window keeps its
own focus.

Tree construction, focus updates, duplicate-ID disambiguation, and action
routing are deterministic across platforms. The operation-facing semantic state
is independent of the widget message type, so mapped elements retain action
support. Click, increment/decrement, and editor-caret requests select the live
enabled widget, then request a redraw which emits its current typed message
through the normal widget mapping. Removed or disabled targets emit nothing;
window-scoped snapshots restrict these requests and focus to that window. Native
screen-reader export is a
separate, narrower contract: `accesskit_unix` exports a single-window Linux
application over AT-SPI, `accesskit_windows` a single-window Windows
application through UI Automation, and `accesskit_macos` a macOS `app` or
`daemon` through NSAccessibility. A daemon exports one adapter per window: a
window attaches when it opens, publishes the tree scoped to itself, keeps its
own focus state, and drops its adapter when it closes, so two windows holding
the same Ice id never share a tree. Other targets keep the deterministic tree
and action behavior without a native adapter, and a daemon on them exports
nothing. Rich text and advanced widgets are outside this Core semantic
contract.

The preferences a user sets in the operating system that no assistive
technology relays — Reduce Motion, Increase Contrast, and whether a screen
reader is running — are read through `ui_lang_runtime::accessibility_settings()`.
macOS answers from `NSWorkspace`. Linux asks the desktop portal's `Settings`
interface (`enable-animations`, the `contrast` preference, and GNOME's
screen-reader flag) and keeps the answer for a second, so a view may ask every
frame; with no session bus or portal it reports no preference. Windows reports
no motion or contrast preference. Every platform counts an assistive technology
that activated the tree as a screen reader. Ice has no startup hook that seeds
state from it: a program reads it through an `extern`.

### Theme and style

A `theme contract` declares semantic tokens; each `palette` provides exactly one
`#RRGGBB` or `#RRGGBBAA` value for every declared token. `bg`, `fg`, `primary`,
and `danger` are required; other names are app-defined. Palette declarations are
ordered and the first is the initial default. They generate the nominal
`palette[Name]` type, and `palette active` selection is an exhaustive generated
match, not a string lookup or a reactive theme graph. `white`, `black`, and
`transparent` are built in and cannot be redeclared.

The default component source supplies complete `AppTheme.app` (light) and
`AppTheme.dark` palettes; applications can select either through the existing
`palette[AppTheme]` state or supply another complete palette for that contract.

Utilities and recipes are resolved at compile time. There is no CSS engine,
selector matching, runtime cascade, or runtime string parser. Recipes expand in
place with the base first, then the child; later utilities win, and direct typed
properties override recipe defaults. A direct typed property combined with a
direct utility that owns the same field is an error.

### Test mode

A top-level `test` is part of the same checked source graph as production
declarations — there is no second test-file grammar and no Rust registration
step. Each declaration lowers to an ordinary `#[cfg(test)] #[test]` function, so
`cargo test` and `cargo ice test` both discover it.

Every interaction, environment event, time step, capture, and accessibility
action lowers to the semantic, raw-event-independent `Action` enum and crosses
the single `Driver::perform_action(Action, Location)` boundary. That enum is
distinct from the application's private generated message enum, so a non-DSL
conformance harness can replay the same semantic operations without knowing
generated internals.

Environment declarations (`theme`, `scale`, `locale`, `platform`,
`reduced-motion`) are explicit test inputs, not a second application
configuration: they pin driver context and the render theme without synthesizing
an operating-system event or creating app state. A `mount` block replaces only
the view, retaining the generated state, update, theme, task, and subscription
contract.

Revision 2.0 has no DOM, CSS selector engine, computed-style object, synthetic
component bounds, component-local-state writes from test source, external test
format, test mock DSL, general virtual clock, built-in golden-image comparator,
or multi-window orchestration. Named captures expose renderer output without
making exact pixel equality the test contract.

## 7. Reference application

The reference component catalog starts at
[`examples/showcase/src/ui/app.ice`](examples/showcase/src/ui/app.ice). Focused
fixtures under
[`examples/showcase/tests/cases/ui/`](examples/showcase/tests/cases/ui/)
compile-test the extended surface recorded in [`COVERAGE.md`](COVERAGE.md).

## Appendix: nested forms

`cargo ice schema` enumerates every construct's own syntax and properties, but
names its children only by role (`app-setting`, `canvas-command`,
`pane-configuration`, `span`, …). Those child vocabularies are listed here and
nowhere else. Everything not in this appendix is in the schema. `INDENT`
delimits a block; `expr`, `route`, `length`, `color_ref`, `background_value`,
`duration`, and `call` are the ordinary checked forms.

```text
root_decl      = ("app" | "daemon") PascalName (INDENT app_setting*)?
app_setting    = "title" expr | "theme" expr | "palette" expr
               | ("bg" | "fg") expr
               | "id" string | "font" string
               | ("executor" | "renderer") rust_path
               | "text-size" number | "scale" expr
               | ("antialiasing" | "vsync") bool
               | window_decl | tray_decl

window_decl    = "window" name? INDENT window_setting*
window_setting = ("size" | "min-size" | "max-size") number number
               | "icon-rgba" string u32 u32
               | "position" ("default" | "centered" | number number)
               | "level" ("normal" | "always-on-bottom" | "always-on-top")
               | ("maximized" | "fullscreen" | "visible" | "resizable"
                 | "closeable" | "minimizable" | "decorations" | "transparent"
                 | "blur" | "exit-on-close") bool
               | window_platform
window_platform = "platform" "linux" INDENT
                    (("app-id" string) | ("override-redirect" bool))*
                | "platform" "windows" INDENT
                    (("drag-and-drop" | "skip-taskbar" | "undecorated-shadow") bool
                    | "corner" ("default" | "do-not-round" | "round" | "round-small"))*
                | "platform" "macos" INDENT
                    (("title-hidden" | "titlebar-transparent"
                    | "fullsize-content-view") bool)*
                | "platform" "wasm" INDENT ("target" (string | "none"))?

tray_decl      = "tray" INDENT tray_setting*
tray_setting   = "icon-rgba" string u32 u32 ("when" expr)?
               | "icon-template" bool
               | ("label" | "tooltip") expr
               | "menu" INDENT tray_row+
tray_row       = "separator" | expr ("->" name)? ("when" expr)? (INDENT tray_row+)?

state_entry    = name (":" type)? "=" expr (INDENT animation_setting*)?
animation_setting = "easing" name
                  | "duration" (duration | "very-quick" | "quick" | "slow" | "very-slow")
                  | "delay" duration
                  | "repeat" (u32 | "forever")
                  | "auto-reverse" bool
                  | "from" (bool | number)

subscription_use = subscription_source ("with=" expr)? ("filter=" name)?
                   ("status=" event_status)? ("when" expr)? "->" route ("|" route)?
subscription_source
               = call | "every" duration | "repeat" call "every" duration
               | "run" call | "recipe" call
               | "events" expr "using=" name
               | "event" ("raw")? ("with-id")?
               | "input-method" input_method_event
               | "keyboard" ("press" | "release" | "modifiers")
               | "mouse" mouse_event | "touch" touch_event
               | "window" window_event ("with-id")?
               | "system theme"
input_method_event = "opened" | "preedit" | "commit" | "closed"
mouse_event    = "entered" | "left" | "moved" | "pressed" | "released" | "wheel"
touch_event    = "pressed" | "moved" | "lifted" | "lost"
window_event   = "frame" | "opened" | "closed" | "moved" | "resized"
               | "rescaled" | "close-request" | "focused" | "unfocused"
               | "file-hovered" | "file-dropped" | "files-hovered-left"

rich_text_child = rich_span | "for" name "in" expr INDENT rich_span*
rich_span      = "span" expr rich_span_property* styles?
rich_span_property = ("size=" | "line-h=" | "line-h-px=") expr
                   | "font=" font_ref | "color=" color_ref | "link=" expr
                   | "bg=" background_value | "border=" color_ref
                   | "border-w=" expr
                   | ("r=" | "r-tl=" | "r-tr=" | "r-br=" | "r-bl=") expr
                   | ("p=" | "px=" | "py=" | "pt=" | "pr=" | "pb=" | "pl=") expr
                   | "underline" | "underline=" expr
                   | "strike" | "strike=" expr

pane_grid_style = "style" INDENT pane_grid_style_status+
pane_grid_style_status
               = "hovered-region" pane_region_style_property+
               | ("hovered-split" | "picked-split") pane_line_style_property+
pane_region_style_property
               = "bg=" background_value | "border=" color_ref
               | ("border-w=" | "r=" | "r-tl=" | "r-tr=" | "r-br=" | "r-bl=") expr
pane_line_style_property = "color=" name ("/" u8)? | "w=" expr
pane_configuration = pane_view
                   | "split" name? pane_axis ("ratio=" number)?
                     INDENT pane_configuration pane_configuration
pane_view      = "pane" name pane_property* styles? INDENT pane_section* node
pane_template  = "pane" name "in" name "by=" expr
                 pane_property* styles? INDENT pane_section* node
pane_property  = surface_style_property | "maximized=" name
pane_section   = "title" pane_title_property* styles? INDENT node
               | "controls" INDENT node
               | "compact" INDENT node
pane_title_property
               = ("p=" | "px=" | "py=" | "pt=" | "pr=" | "pb=" | "pl=") expr
               | "always-controls" | surface_style_property
pane_axis      = "horizontal" | "vertical"
surface_style_property
               = "bg=" background_value
               | ("text=" | "border=" | "shadow=") color_ref
               | ("border-w=" | "r=" | "r-tl=" | "r-tr=" | "r-br=" | "r-bl="
                 | "shadow-x=" | "shadow-y=" | "shadow-blur="
                 | "px-snap=") expr

pane_operation = "maximize" name | "restore" | "maximized"
               | "adjacent" name pane_edge
               | "swap" name name | "close" name
               | "move" name pane_edge | "resize" (name expr | expr)
               | "drop" name name ("center" | pane_edge)
               | "split" name name pane_axis ("ratio=" expr)?
pane_edge      = "top" | "left" | "right" | "bottom"

window_task    = "task window" window_operation ("target=" expr)? ("->" route)?
window_operation = "open" name? | "oldest" | "latest"
                 | "close" | "drag" | "toggle-maximize" | "toggle-decorations"
                 | "focus" | "system-menu" | "raw-id" | "screenshot"
                 | "drag-resize" direction
                 | ("resize" | "move") expr expr
                 | ("resizable" | "maximize" | "minimize" | "mouse-passthrough"
                   | "auto-tabs") expr
                 | ("min-size" | "max-size" | "resize-step") ("none" | expr expr)
                 | "set-mode" ("windowed" | "fullscreen" | "hidden")
                 | "attention" ("none" | "critical" | "informational")
                 | "level" ("normal" | "always-on-bottom" | "always-on-top")
                 | "size" | "maximized" | "minimized" | "position"
                 | "scale" | "mode" | "monitor-size"
                 | "icon" expr expr expr
                 | call

canvas_item    = canvas_state | canvas_event | canvas_command
canvas_state   = "state" INDENT state_entry+
canvas_event   = "event" canvas_event_source "->" route
               | "event" canvas_event_source ("as" name_list)?
                 INDENT canvas_event_action+
               | "capture" canvas_event_source
               | "redraw" canvas_event_source ("after=" duration)?
canvas_event_source
               = "input-method" input_method_event
               | "keyboard" ("press" | "release" | "modifiers")
               | "mouse" mouse_event | "touch" touch_event
               | "window" window_event
canvas_event_action
               = "set" name "=" expr | "emit" route | "capture"
               | "redraw" ("after=" duration)?
canvas_command = canvas_rect | canvas_circle | canvas_line | canvas_text
               | canvas_path | canvas_group | canvas_if | canvas_for
canvas_rect    = "rect" point size canvas_radius* canvas_paint+
canvas_circle  = "circle" point "r=" expr canvas_paint+
canvas_line    = "line" "x1=" expr "y1=" expr "x2=" expr "y2=" expr canvas_stroke
canvas_text    = "text" expr "x=" expr "y=" expr canvas_text_property*
canvas_text_property = ("max-w=" | "size=" | "line-h=" | "line-h-px=") expr
                     | "color=" color_ref | "font=" name
                     | "align-x=" ("default" | "left" | "center" | "right"
                       | "justified")
                     | "align-y=" ("top" | "center" | "bottom")
                     | "shape=" ("auto" | "basic" | "advanced")
canvas_path    = "path" canvas_paint+ INDENT canvas_path_segment+
canvas_group   = "group" canvas_transform* INDENT canvas_command*
canvas_if      = "if" expr INDENT canvas_command*
canvas_for     = "for" name "in" expr INDENT canvas_command*
canvas_radius  = ("r=" | "r-tl=" | "r-tr=" | "r-br=" | "r-bl=") expr
canvas_paint   = "fill=" background_value
               | "fill-rule=" ("non-zero" | "even-odd")
               | canvas_stroke
canvas_stroke  = "stroke=" background_value ("stroke-w=" expr)?
                 ("cap=" ("butt" | "square" | "round"))?
                 ("join=" ("miter" | "round" | "bevel"))?
                 ("dash=" "(" expr_list ")")? ("dash-offset=" expr)?
canvas_transform = ("x=" | "y=" | "rotate=" | "scale="
                   | "scale-x=" | "scale-y=") expr
                 | "clip=(" expr "," expr "," expr "," expr ")"
canvas_path_segment = "move" point | "line" point
                    | "arc" point "r=" expr "start=" expr "end=" expr
                    | "arc-to" "ax=" expr "ay=" expr "bx=" expr "by=" expr "r=" expr
                    | "ellipse" point "r-x=" expr "r-y=" expr
                      "rotate=" expr "start=" expr "end=" expr
                    | "bezier" "ax=" expr "ay=" expr "bx=" expr "by=" expr point
                    | "quadratic" "cx=" expr "cy=" expr point
                    | "rect" point size
                    | "rounded" point size canvas_radius+
                    | "circle" point "r=" expr | "close"
```

`extern` items are the declaration kinds in section 5: a struct signature, a
bare/`pure`/`sync` function, or one of the typed adapters (`component`,
`selector`, `shader`, `task`, `stream`, `sip`, `recipe`, `event-filter`,
`subscription`, `theme`, `themer`, `window`, `markdown-viewer`, the `editor-*`
kinds, and the per-widget `*-style` kinds).

## Extern widgets in wasm view modules

For the `tree` target, an `extern` widget names a surface registered by the
embedding host; it does not call the declaration's native Rust function.
Positional `unit`, `bool`, `i64`, `f64`, and `str` arguments cross as owned,
tagged values. Lists, options and nonempty declared records of those types
also cross by value, including borrowed parameters. A route accepts the
same result types, with non-payload arguments snapshotted while the
view is built. The guest ignores a result tagged with a different type and
nonfinite floating-point events. No route means no guest event.

A provider receives the node key and arguments and returns an element whose
messages are tagged values. The renderer attaches the node's route. Unknown
surface names render a visible placeholder. Surface arguments are bounded to
256 values; text shares the frame text budget and nonfinite argument numbers
sanitize to zero. Returned strings are truncated on a UTF-8 boundary to the
wire string limit before entering the guest. Each wire decode permits at most
4096 surface values in total and nesting depth 32, rejecting the whole
message on overflow before building excessive nested values. Event values
share one text budget; nonfinite numbers anywhere reject the event. Record
names and field names are never truncated: an identifier that cannot fit
rejects the value. Frame sanitization replaces an argument that exceeds its
structural budget with `Unit`, preserving argument positions up to that
point; remaining arguments are omitted when the total value budget is spent.

Records carry the Ice declaration name and field names in declaration order.
A returned record must match that name, order, field count and every nested
type exactly. Native resource types, empty opaque declarations, recursive
record definitions, enums and other non-data types remain E190. No Rust
pointer or resource is serialized by treating it as an empty record.

## Partial border styles on the tree target

A tree-target border retains colour, width and corner radii as independent
optional values. Omission preserves the host widget's base style or a value
applied by an earlier state face. Explicit transparent colour, zero width,
and zero radii overwrite those fields. A partial border on a plain container
is applied over the default container border. Sanitization bounds present
values without creating absent ones. This matches the native emitter's
field-by-field style updates.

## Clipboard Tasks in wasm views

`task clipboard read`, `read-primary`, `write` and `write-primary` use the
host's `clipboard` capability on the tree target. Native declarations and
handler syntax are unchanged. The guest sends `clipboard.read` with an
encoded `ClipboardTarget`, or `clipboard.write` with an encoded
`(ClipboardTarget, String)`. Read replies encode `Option<String>`; successful
writes reply with empty bytes. `None` and an empty string remain distinct.

The example host checks the installed manifest and queues permitted work
until the mounted widget supplies its platform clipboard. Standard and
primary clipboards remain distinct. Text is limited to the wire string limit
on a UTF-8 boundary; malformed or oversized explicit write requests are
refused. A denied/invalid read logs its error and completes the original Task
with `None`; write errors are logged. Aborting a pending read cancels its
host request. Canceled requests and faulted/unmounted instances do not retain
queued clipboard work. Clipboard access remains host-owned and requires
capability consent at installation. Platform clipboard time participates in
the host redraw governor; the byte budget is checked before each queued
operation so exhaustion suppresses subsequent platform access.

## Shader surfaces on the tree target

A `shader name(args) w=… h=… -> handler _` call becomes a named host
surface. It uses the same scalar/list/option/record argument and event
validation as an extern component. The guest neither calls the native shader
function nor emits its native Rust probe. Unsupported native argument types
remain E190. A call without a route has no event handler.

A wire container carries the declared dimensions, defaulting each omitted
dimension to 100 pixels, matching Iced Shader. Explicit `shrink` becomes
zero on that axis because Shader has no intrinsic content size. The provider
fills that region (e.g. its native Shader uses `width(Fill).height(Fill)`) and owns its renderer,
state and redraw schedule. An unknown name shows the existing placeholder
inside the same bounds. Surface names share the extern component registry.
This establishes a host rendering boundary, not guest GPU execution.


## Declarative canvas on the tree target

A canvas sends copied rectangle, circle, line and path commands to the host.
Path segments, solid fills, even-odd fill rules, stroke caps/joins/dashes,
transforms and clips use native Iced geometry. Guest `if` and `for` choose the
commands; the host paints them. Dimensions and coordinates are widget-local.
A clipped group starts a fresh drawing frame, matching native `with_clip`:
its clip rectangle and child geometry use canvas coordinates, and transforms
outside that frame do not carry into it.

Commands, path segments and dash entries share a 4096-part decode/frame budget.
The host bounds numbers and group depth before rendering. Wrap geometry in
`mouse` for opt-in local pointer routes. Native canvas state/events/cache and
interaction options, host `canvas_width`/`canvas_height` bindings, gradient
paint, canvas text and raster/SVG drawing remain E190 on this target.


## Declarative editors on the tree target

An `editor` carries its document binding, hint, disabled state and native dimensions,
plus copied `size=`, `p=`, relative or absolute `line-h=`, `wrap=` and `font=`.
Omitted size and font use the guest application defaults. Declarative faces
copy background, partial border/radius, value, placeholder and selection colors.
The native status default is followed by active and then the applicable hovered,
focused or disabled face; focused-hovered applies after focused. Native focus and
editing stay in the host. The guest receives text, active caret and optional
selection anchor whenever text or cursor changes, including pointer and key
selection without an edit. Positions use zero-based lines and UTF-8 byte columns;
external positions clamp backward to extended grapheme boundaries. `None` means
no selection, not an unchanged selection. The owning host wrapper derives paint status from the
native focus state and cursor, preserving it between native widget calls.

Typography, colors and font names share wire sanitization and frame budgets.
`EditorOptions` changes the wire layout; rebuild hosts and guests together.
Opaque Rust editor style/action callbacks and `highlight=` remain E190. Tree
`editor-binding` uses the transaction contract, and `editor-highlighter` receives
borrowed `EditorStateView` plus declared arguments and returns `EditorPresentation`.
Sparse UTF-8 spans, caret menus, gutter/drop boundaries, margins and tagged hits
are validated against the exact document reference. Interaction decisions use the
same atomic transaction lane as keys; accepted commits retain their original
request input. See [editor presentation](docs/editor-presentation.md). `editor_cursor_line`, `editor_cursor_column`, `editor_has_selection`,
`editor_text`, `editor_copy`, `editor_line` and `editor_line_count` read the copied
document state. Explicit assignment increments an authoritative reset revision,
even for identical text. Observations carry that revision and a host sequence;
old observations and old echoed frames cannot rewind newer state. Sibling editor
keys bound to the same document synchronize newer observations. Snapshot restore
retains text/caret/anchor/revisions; a fresh host seeds its sequence from them.
Exhausted host observation counters reject edits before mutation and render an
explicit limit placeholder. Rebuild hosts and guests together for this wire change.
This does not provide structural pre-edit callbacks, undo grouping, or preserve
native word/line selection modes across an authoritative document replacement.

## Native editor surfaces in the example host

A named terminal surface may retain a host-owned PTY independently from its
mounted widget. The example host's `terminal` capability selects the session;
no surface argument selects a process or injects input. Its `terminal.events`
stream reports copied title/running/attention data. Native view teardown clears
focus and clipboard work without ending the session. These are example-host
capabilities, not portable native session handles in the wire format.

A named surface may keep a native editor document behind an instance-scoped
view lease and publish ordinary record values. The app-store `rich_composer`
example demonstrates this with `RichTextEditor`: guest text echoes preserve
native input state; changed text or an explicit reset generation replaces it.
Composition, native actions and history never cross as Rust values. See the
[fixture contract](examples/app-store/README.md#rich-composer-fixture) for its
semantic notice, byte/history bounds and lifecycle. This is an example provider,
not a new Ice keyword or an encoding for arbitrary native editor callbacks.

Canvas preparation also shares a 16,384-part host budget across the tree for
flattened segments and estimated dash expansion. Curves are flattened once
and those line paths are painted; excess draws and unstable arc-to tangents
are omitted before native tessellation. This bounds work beyond wire size.

## Container rules on the tree target

`responsive size=(width, height)` retains its native length bounds. Measured
locals may appear in child `if` conditions as numeric arithmetic, comparisons
and Boolean combinations. Independent subexpressions, including state-derived
thresholds, are evaluated by the guest and copied into the rule. Measured
operands name their responsive node; nested rules can read their own container
and ancestors. The host evaluates the rule during layout, with no guest call or
window-coordinate exposure. Selected children splice into the surrounding
row/column/grid without introducing another layout widget.

Each condition is bounded to 64 postfix operations. Unknown container keys,
malformed stacks and nonfinite copied constants select no branch. Native calls
of measurements and measurements in ordinary widget properties remain E190.
Size-independent control flow stays in the guest. Hidden branches retain copied
wire data but do not instantiate native surface providers. Provider closures are
shared `Arc`s so deferred layout can own its rendering context safely.

Within a measured condition, copied independent operands are restricted to data
reads, literals and comparisons/Boolean combinations of those values. Independent
calls, arithmetic and lazy `derived` reads are E190: native short-circuit evaluation could skip them,
whereas copying would execute them before layout. Precompute such thresholds
explicitly in guest state. Arithmetic involving a measurement runs in the host.

### Wasm bundle optimization

`cargo ice bundle --target wasm32-unknown-unknown --no-wasm-opt -p PACKAGE`
skips `wasm-opt` discovery and execution, even when an optimizer is on PATH.
Without the flag, the existing optional optimization behavior is unchanged.
The flag requires `wasm32-unknown-unknown` as the sole target and is not forwarded
to Cargo. Native or mixed-target requests fail before invoking build tools.
This option alone does not guarantee reproducible bytes across toolchains or paths.

### Desktop bundle resources

`[package.metadata.ice.bundle].resources` is an array of explicit file or directory
paths relative to the Cargo package manifest. Desktop bundles preserve each
entry's basename and nested files beside the executable, before signing.
See [tooling](docs/tooling.md#resources) for platform locations and validation.
This packaging metadata does not alter the Ice language or wire protocol.

### Tree layered layouts

Tree `stack`, `hover`, and `overlay` lower copied dimensions, style values,
children, and route slots to host-native widgets. Stack dimensions remain
inferred unless explicitly set, and responsive structural conditions splice
into the stack's child list. Hover uses native cursor presence and a copied
`open` flag. An overlay carries its base and an optional modal child; the host
blocks base keyboard/focus operations while the modal is present and forwards
its native overlay events to the same guest.

Tree `float` copies one child, scale, shadow and radius. Translation arithmetic
(`+`, `-`, `*`, `/`, `%`, unary minus) runs on the host against current original
and viewport bounds; geometry-independent expressions are evaluated by the guest
and copied as numbers. Each axis is limited to 64 postfix operations. Calls that
depend on host geometry remain E190. Invalid or nonfinite wire arithmetic yields
zero translation. A floated modal guards its visible translated panel against
outside-click dismissal. Wire child, numeric, depth, and frame limits apply.

## Tree text presentation

The tree target copies plain-text wrapping/shaping, named font descriptors,
relative/absolute line height, height, vertical alignment and grapheme tracking
to native host widgets. Hosts register trusted family names through
`ui_lang_runtime::view_tree::register_font_family` and load the matching font
bytes. Unregistered names use native sans-serif. Tracking uses a non-selectable
grapheme row and shares the host node budget. Boxes support maximum dimensions
and clipping; columns support `max-w=` and non-virtual rows/columns support `clip=`.
Linear clipping uses the native paint viewport and does not change native
pointer routing. The surrounding fill surface retains its native width while
the column bounds its child content. Buttons support padding utilities.
The Linear wire layout changed; these wire fields require
host and guest rebuilds together.

### Tree box shadows

Boxes copy `shadow=`, `shadow-x=`, `shadow-y=` and `shadow-blur=` into the
host's native container shadow. Signed offsets, blur and color alpha preserve
native paint outside the box without changing layout or pointer bounds.
Omitted fields retain native defaults. Boxes and tooltips share the shadow
value and sanitization: offsets are finite and bounded in either direction,
blur is nonnegative and bounded, and color channels are clamped. Other widget
shadow options retain their existing support boundaries.

The Container wire layout changed; rebuild hosts and guest bundles together.

### Tree button accessibility

Tree buttons preserve optional `checked=`, `expanded=` and `description=`.
The host forwards them to the native accessible wrapper: `false` is distinct
from omission, and descriptions share the frame text budget. This changes the
Button wire layout; rebuild hosts and guests together.
Native AccessKit snapshot tests cover true/false/absent states;
the widget wasm fixture verifies copied state after a native focus-button click.

### Tree button recipes

The tree target copies resolved button preset, recipe colors, border, label
size/relative line height/font and focus-visible ring color. Hosts resolve the
native preset, recipe base/status colors, typed active face, typed status face,
then recipe disabled treatment unless a typed disabled face exists. Guest
default font and text size apply to compact labels; a fixed dimension centers
content on that axis, and a fill or fill-portion dimension centers a compact
label the same way, in the content box the padding leaves. Written-out child
content keeps its own layout under a fill dimension, and `shrink` keeps the
button hugging its content. Explicit zero padding overrides native
defaults, including `@p-0px`. Font names share the frame text budget and trusted
host registry; numeric recipe values are sanitized. Unsupported utility
properties and Rust style callbacks remain E190. ButtonStyle changes require
host and guest rebuilds together.

### Tree wrapping rows and columns

Tree `row wrap` and `col wrap` use the native host wrapping widgets. They carry
`wrap-gap=` and `wrap-align=` alongside ordinary spacing, dimensions, padding
and child alignment. The host reflows when its available size changes; guests
provide children and copied values without measuring pixels. Optional wrapping
settings distinguish an ordinary layout from a wrapping layout with defaults.
Inter-line spacing uses the existing wire and child-count spacing limits.
Rebuild hosts and guests together for the Linear wire field.

### Tree tooltips

Tree tooltips use the host's native overlay, with copied position, gap, padding,
delay and viewport snapping. Native container presets and concrete solid
background/text/border/shadow/snap values cross; Rust style callbacks and
gradients remain refused. The tip's visible text supplies the first eligible
content node's accessible description, preserving an explicit description.
Tooltip children share tree limits, numeric style values are sanitized and
delay is capped at 60 seconds. Rebuild hosts and guests together for the wire
variant.

### Tree SVG button colors

Memory SVG children support `color=inherit` through the nearest host button's
resolved text color, including hover on button padding, pressed and disabled
states. Explicit SVG palette colors stay independent. The guest copies only
the inheritance flag; the host owns the existing native button ink cell.
Rust SVG style callbacks remain refused. Rebuild host and guests together
for the SVG wire field.

### Tree input presentation

Tree inputs preserve native label semantics: the positional string (or
`label=` override) is the accessible name, while `hint=` supplies the visible
placeholder. Description and disabled state reach the native accessibility
wrapper; disabled inputs do not produce edits or submit events.

Padding, text size, relative line height, horizontal alignment and named or
default/mono fonts are copied. Absent typography options use guest app defaults.
Input recipes preserve utility padding and fill width, with explicit options
winning, and utility colors/borders precede active and status overrides. Focus
ring color applies after active and before explicit focused/focused-hovered
styles. Text metadata shares frame budgets and layout numbers are bounded.
Rebuild hosts and guests together for InputOptions and the extended InputStyle.
Secret handles, input icons, paste routes and Rust style callbacks remain
refused.

## Tree keyed and virtual columns

`keyed` emits a wire KeyedColumn with copied bool/i64/f64 keys and bounded layout
properties. Ordinary keys retain native numeric PartialEq semantics; virtual
keys retain their lossless bit representation. Duplicate keys match old
occurrences in order. Hosts move widget state with those occurrences on arbitrary
permutations and discard removed rows. `virtual-row=` on keyed and ordinary
columns uses native host viewport observation, measured heights and deferred
offscreen diffing. The guest supplies children and an estimate, never coordinates
or layout callbacks. `scroll-to-key` targets the named scroll and lands the
first virtual column containing that key at the measured row top.

## Tree lazy subtrees

Tree `lazy` uses the checked native dependency/revision and `by` lowering. The
guest caches the resulting Node and callable route snapshots per Driver, keyed
by expression and reconciliation scope. Cache hits restore only the current
frame's routes; removed routes expire. Cache entries keep the latest dependency
revision and are bounded to 1024 entries. Retained SVG and raster image nodes keep hashes only;
new payload bytes cross in the first returned frame and are not replayed on hits. An eviction rebuild receives a
fresh generation even when dependency values match an older entry.

Wire Lazy carries a stable key, generation and copied child. Hosts retain native
view/layout state in a module-owned parking lot, with weak ownership from parked
children. Inner unmount/remount can reuse state; replacing or dropping the module
releases it, including nested native resources. A fresh Inputs value identifies
a new module generation; render snapshots preserve that identity.

Host memo invalidation includes sanitized child content, prepared canvas admission,
ancestor container measurements, inherited button ink, admitted images, surface
providers and live input/editor values. A different layout limit reflows the
child. Lazy snapshots cannot re-admit assets rejected by shared host budgets.
Existing lazy purity/type restrictions and Tree widget restrictions still apply.
Host and guest must be rebuilt together for the Lazy wire variant.


## Tree flex layouts

Tree flex sends layout rules and a parallel vector of per-child item rules to
the host's existing native flex engine. It preserves direction/reversal,
nowrap/wrap/wrap-reverse, justify/items/content alignment, independent gaps,
padding, dimensions/maxima and clipping. Item order, grow/shrink, fixed/content/
percentage basis, self alignment and fixed/percentage/auto margins use the same
checked lowering and if/for/match expansion. Minimum-cell grids currently use
ordinary growing, non-shrinking flex items on Tree; they do not yet carry the
native equal-column sizing described above. Utility sizing
on the painted outer container remains separate from explicit inner dimensions.

Metadata is decode-bounded and normalized to surviving child count after the
shared node budget. Numeric values follow existing wire size bounds; margins
retain finite negative values. The guest supplies neither measured coordinates
nor a layout callback. Native layout, input, hit testing and clipping stay host
responsibilities. Flex can be retained inside a Tree lazy boundary and reflows
when the host's layout limits change. Rebuild hosts and guests together for the
Flex wire variant. Other Tree widget/style restrictions continue to apply.

## Tree pinned children

The Tree target carries `pin` as one child plus evaluated local `x`/`y` and
optional `w`/`h`. The host uses native Pin layout, drawing, hit testing and overlay
forwarding. Omitted dimensions retain native Fill defaults. Nested offsets are
relative to each pin, never window coordinates; the guest does not measure them.
Signed offsets are bounded to ±8192 logical pixels (NaN becomes zero), and
lengths use the existing wire sanitizer. Native `length` values remain refused.
Host and guest must be rebuilt together for the Pin wire variant.

## Tree rich text

The Tree target copies literal spans and `for`-expanded spans into one native
rich text paragraph. Each span carries text, optional size/line height/font/color,
String link, solid background, border/radius/padding and underline/strike. Text
layout options retain the existing copied width/height, wrapping and alignment
contract. The host performs shaping, layout, painting and link hit testing.
A click invokes the snapshotted String handler through the existing guest route
store; cached lazy paragraphs preserve that handler across cache hits.

Spans share the frame's decoded node allowance before allocation and its rendered
node/text budgets before layout. Link and font strings spend that text budget too.
Native gradients and custom Rust styles remain refused; a gradient span reports
its own source location. Tooltip descriptions concatenate adjacent spans within
a paragraph before separating distinct text nodes. Rebuild hosts and guests
together for the RichText wire variant.

## Tree component lifetime

`lifetime retained` keeps guest component state when its identity is absent.
`lifetime mounted` now uses the same per-scope state storage as native, with
synchronous pruning after the complete owned Tree render. First sighting queues
the component's boot message with the current props; the next driver tick drains
those messages before external input. Rendering never executes an update.

After pruning, the driver reconciles subscriptions and marks the frame busy when
a deferred boot or cancellation task needs another tick. This allows an idle host
to deliver cancellation without input. Removed scopes lose state and boot marks;
reappearing scopes initialize and boot again. Queues belong to their driver.

Mounted descendants of explicit lazy and host-evaluated container conditions
report E190 at the component call, including through wrapper components and
slots. Those boundaries do not replay guest mount sightings or report host branch
activation. An unconditional responsive child has ordinary guest-known lifetime.
No new wire node or host lifecycle event is introduced.

## Tree QR codes

Tree `qr` carries an owned UTF-8 or byte payload, optional correction level,
normal/micro version, cell/total size, and cell/background colors. The host
encodes it with the same QR widget as native Ice; guests do not draw matrices.
Omitted options retain native defaults. Unencodable payloads have zero layout.

Each payload is limited to 8192 bytes, shares the frame text-byte budget, and
spends one of 32 frame encoding slots even when empty. Invalid versions and
excess payloads are dropped whole, never truncated into a different code.
Sizes and colors are bounded before rendering. Rebuild hosts and guests
together for the new `Qr` wire variant.

Native `log_timeline` and `virtual_list`, including the themed wrappers in
`ui-lang-components`, separate the source-slice lifetime from
the returned element lifetime. An owned row view permits a static surface from
a temporary slice (including an `Arc<[T]>`); a borrowing row view still ties its
elements to the source naturally. Only mounted row views are constructed.

## Tree keyboard subscriptions

A host delivers `wire::Event::Keyboard` after its mounted native widgets process
the event, including its captured/ignored status. The guest broadcasts to its
existing Iced subscription tracker and settles after each event, so filters,
`when` conditions and subscription removal keep their native ordering and a
burst cannot overflow the tracker's queue before it is polled. Key, modified
key, physical/native code, location, modifiers, text and repeat cross as data.

`ice:view` initializes with `init(macos: bool)`. The host supplies platform
semantics before boot. Tree `key.command_modifiers()` and modifier `.command`,
`.jump` and `.macos_command` projections use this per-instance setting. Guest
Rust externs must use `ui_lang_guest::keyboard` helpers for these meanings;
Iced's own Rust methods still use the compiler target OS. Other modifier bits
remain unchanged. Hosts and guests must rebuild together for the new ABI/event.

A shared-window host must route keyboard input only to the selected module,
never broadcast another module's or host input's text. This is a host routing
contract. Tree mouse subscriptions opt in through `Frame.mouse_interest`;
only active branches request host mouse observations. Coordinates are signed
guest-local logical pixels and must be finite. Preserve captured status, button
identity and wheel units. Deliver at most the latest move per redraw, leaving
that move and all discrete events in their original relative order. Generic
`event`/`event raw` listeners expose keyboard and mouse only; window/global,
IME and touch subscription transport remain unsupported. This changes the wire
encoding; rebuild native/Wasm guests and hosts together.


## Tree guest state snapshots

`ice:view` exports `snapshot() -> result<list<u8>, string>` and
`restore(state: list<u8>, macos: bool) -> result<_, string>` alongside `init` and
`tick`. Restore can initialize a fresh instance without `init`; it decodes all
persistent fields before constructing the app and runs neither state initializers
nor boot tasks. Errors preserve an already initialized driver. Native exports
provide `snapshot_native` and `restore_native` with the same behavior.

The envelope carries a SHA-256 state schema and complete root state, component
initial state, retained/mounted instance maps and mounted boot markers. Schema
identity covers state names/types and reachable record/enum shapes plus component
storage modes, excluding view layout and initializer values. Restored untouched
components read and materialize from saved initials. Memo revisions, handlers,
task lanes and subscription trackers start fresh. A new render prunes removed
mounted scopes; subsequent reappearance boots normally.

Supported values are unit, bool, i64, finite f64, strings, bytes, editor text,
markdown source, lists, options, results, declared data records/enums, palettes
and keyboard modifiers. Recursive or opaque/native state and secret stores reject
the entire snapshot; no fields are silently dropped. There are no migrations.
The codec bounds the entire snapshot to 8 MiB, depth 32 and 65,536 values and
rejects trailing bytes, duplicate instance scopes and malformed nested types.

Hosts must deliver pending UI events first. Busy frames, deferred component boot
and any live Task (including long-running Task streams) reject capture. Tracker
subscriptions restart from restored state on the next tick. The replacement's
first tick sends a complete tree and new routes. Rebuild hosts and guests for the
extended component interface. The app-store host polls its local catalog on the
executor and replaces an explicitly approved running artifact in the same Surface
and window. It stages snapshot/restore and a complete first frame before the UI
thread checks request serial, current window/instance and unchanged guest ticks.
Failed or stale candidates preserve the old guest and consent hash. Keyed native
focus/scroll and permitted host resources survive; removed terminal permission
removes its provider. Staged requests dispatch before their cancellations and
platform effects, and old instance input routes are refused. This host policy is
separate from the guest exports; it does not migrate incompatible state schemas.

### Tree scroll offsets

A Tree `scroll` supports `scroll=` with the native four-argument route:
absolute X/Y offsets in logical pixels and anchor-relative X/Y fractions.
The host emits the route only when the native viewport changes, including
end-anchored scrollback. Offsets are local to the scrollable; window coordinates
are not exposed. Missing routes produce no guest events. The full `viewport=`
route and scroll status styles remain refused. Hosts and guests must rebuild
for the added Scroll field and ScrollOffset event.

## Tree pick-list options

Tree pick lists copy padding, text size/relative line height, shaping, named or
built-in font, menu height and native arrow/static/dynamic/no-handle options.
Opening and outside-click dismissal use snapshotted message routes; choosing an
option uses the existing typed selection route. The host owns menu layout,
hit testing and the native distinction between selection and dismissal.
Active styles apply before status overrides; opened-hovered inherits opened.
Menu color, border/radius and shadow overrides retain native theme defaults for
unspecified fields. Custom Rust style callbacks and gradients remain refused.
Text metrics, padding, menu heights, shadow offsets/blur and named font strings
are sanitized before native rendering. Hosts and guests must rebuild together
for the extended PickList wire data.

### Guest preferred window size

Tree compilation carries the primary `window size` into the versioned
`ice.manifest.v2` custom section without another `export_app!` argument. The
strict six fields are version, name, description, comma-terminated capabilities
(or empty), `none` or `width,height`, and the canonical positive wire epoch. There is no legacy fallback.
The app-store catalog accepts finite positive f32 dimensions up to 8192 logical
pixels; Tree rejects declarations outside that range at the `size` source line,
without changing native-target limits. Saved placement takes precedence over the declaration, then the host
560×420 default. The initial native open uses the final size; restart preserves
the existing window. Hosts and guest modules must rebuild together.

Evidence: Core `tree_manifest_preferred_window_size_is_static_and_preserves_f32`,
host `manifest_format_and_preferred_size_are_strict`,
`preferred_window_settings_choose_saved_declared_then_default`, and the actual
wasm `bundled_preferred_size_reaches_initial_native_open` test documented in
[the app-store guide](examples/app-store/README.md#preferred-window-size-evidence).


Hosts can consume the Ice view contract without a graphics dependency through
`ui-lang-wire`. `WIT` exposes the canonical text; `with_view_wit!(callback)`
passes that same literal to a local macro for Wasmtime or wit-bindgen's `inline`
option. `export_app!` retains its four arguments and the same `ice:view` ABI.
`manifest::Manifest::parse` reads the strict six-line metadata;
`manifest::PreferredSize::dimensions` returns `[f32; 2]`. The optional `manifest`
feature adds `manifest::read_manifest` for extracting exactly one manifest from
component bytes, including nested core modules. Neither the default dependency
set nor this feature enables iced or a renderer.

Metadata extraction does not validate executable code. Hosts can compile the
component, resolve imports with `Linker::instantiate_pre`, and check exports
with the generated `ViewPre::new` without creating a store or running the guest.
This checks required ABI types; it is not proof that instantiation, init, or boot
will succeed. Import policy remains the host's responsibility.

`WIRE_EPOCH` identifies the exact serialized tree/event/frame protocol. After
verifying artifact bytes, call `Manifest::check_wire_protocol()` before any
instantiation, native child launch, init, or restore. A mismatch reports
`wire epoch guest N, host M`; app-store rejects the candidate and keeps a running
instance unchanged. The epoch is independent of the manifest format and WIT
signatures. Request capability declarations remain permissions, not renderer
feature negotiation. Any serialized shape change requires a new epoch; no old
decoder or implicit compatibility range is supported.

### Identified shared Tree structures

An identified keyed column uses its full authored scope as its wire node key;
its row keys extend that scope. An identified lazy boundary likewise carries
its own scope, with nested boundaries extending it. Native identity wrappers
are emitted only for native elements, never for Tree nodes.

## Native Tree guest execution

`export_app!` exposes `run_native()` on native targets. Its `--manifest` mode
writes the existing strict manifest without booting the app. `--ice-native`
serves length-prefixed wire-encoded init, tick, snapshot and restore requests
on stdin/stdout. Each packet is bounded to `MAX_SNAPSHOT_BYTES + 64`; snapshots
retain their existing schema and validation. Tick roots are omitted under the
same unchanged/patch rules as the wasm component exports. This transport is not
an operating-system sandbox or a replacement for the native language target.

The example store scans `<id>.native/{app[.exe],manifest}` without execution,
binds both byte sequences to consent and launches the verified executable copy.
All exchanges have a host deadline. Hosted capabilities and native widgets are
shared with wasm; arbitrary native OS access is explicitly trusted.

### Tree sensor continuity

Sensor `key=` is copied independently of the node identity and compared by the
host's native sensor. A changed key re-arms its show notification, preserving
its delay, local measurement and child state. The language's `bool`, `i64`,
`f64` and `str` keys use the bounded SurfaceValue codec; opaque extern keys
remain E190. Reset data shares frame text/value budgets. The existing per-frame sensor
loop limit still applies. Host and guest must rebuild for the new reset field.

### Authored Tree host tests

`ui_lang_build::compile_tree_tests(root)` analyzes the same imported Ice graph
and generates a Rust test file in `OUT_DIR`, returning its path. The host includes
it under `cfg(test)` in a module supplying `__ice_tree_test_driver(Config, test_id, fingerprint)`, which
returns the existing runtime semantic `Driver` for a mounted guest Program.
Typed steps additionally call
`__ice_tree_test_step(&mut Driver<P>, test_id: u32, step_id: u32, Location) -> ()`.
This adapter forwards the checked IDs to the same guest instance and reports any
guest error as a test failure at the supplied source `Location`. It redraws before
and after the step, publishing dispatch changes to mounted widgets before the
next rendered UI assertion. The app-store host's authored test adapter implements
these hooks. Targeted actions additionally call
`__ice_tree_test_target(&mut Driver<P>, test_id: u32, step_id: u32, Location) -> String`
after redraw, returning the checked guest-generated widget path.
Generated tests are ignored by default because they require separately built
guest packages; host CI must explicitly execute them. No test command or export
is added to production guest artifacts.

The Tree host-test subset accepts named presets, typed state expressions and
direct dispatch, target paths with checked live-state key expressions, click steps,
focus/next/previous/blur, type/clear/replace, select/select-all, cursor/front/end,
key/key-down/key-up/modifiers/chord/repeat,
`exists`/`missing` and literal text expectations, including `within` and
negation, with viewport and timeout configuration. Before every target use, the
host redraws to settle widget events, then requests the checked target path from
the guest using its test and step IDs. The read-only guest callback evaluates
keys against that rendered state, including aliases and nested keys. The reply
is bounded by the existing string limit; no app state is serialized. Keys retain
the native bool/i64/f64 restriction. Input text, selection/cursor indices and repeat
counts currently require literal arguments; state-derived values produce E190
rather than evaluating against the host Surface. These actions use the existing
native semantic Driver and deliver mounted widget events, without direct dispatch
as a substitute for user input. Other steps,
mounts, environment overrides and daemon windows produce E190 at their authored source origin;
the generator never silently drops an unsupported test. Direct `cfg(test)` builds
of a Tree guest containing authored tests explain the host harness requirement
at the test origin. Tree stack contracts remain generated. The Native target's
existing authored test generation and semantics are unchanged.

Explicit authored test artifacts enable the guest `authored-tests` feature, include
`compile_tree_guest_tests(root)` beside the generated app, and use
`export_test_app!`. Their `ice.test.manifest.v3` header and test-only `authored`
export are never accepted as production packages. The host and guest compare a
source-graph fingerprint before beginning a selected test. Previous test-manifest
versions are rejected before initialization or target-resolution commands. Presets use the same
generated boot function and Driver initialization; dispatch constructs the checked
message and runs ordinary update/task settling. Predicates read live typed state
inside the guest and return success or an error, without serializing application
state or requiring snapshot quiescence. The production view/native protocol and
Snapshot contracts do not change.

### Scoped guest window effects

For Tree applications, direct `task window focus`, `task window resize width height`,
`task window close`, and `exit` send the closed `WindowCommand` vocabulary through
`Request { kind: "host.window", ... }`. No window ID crosses this boundary;
explicit `target=` is E190. `exit` means close this guest's host window, never
terminate the host process. Native-target code generation is unchanged.

Resize dimensions must satisfy `PreferredSize::new`; invalid inputs are refused
instead of clamped. The app-store limits each guest to 32 pending commands and
rejects malformed/trailing payloads. The UI update validates Surface identity,
current instance token and current Running entry before submitting an Iced window
Task. Cancellation and replacement invalidate prepared commands. Completion is an
acknowledgement of runtime submission, not OS success, and is rechecked before a
response can reach a guest. Rejections are explicit `RequestError` replies or logs;
no old request ID is delivered to a replacement instance.

Other direct window operations and arbitrary native Task actions are outside this
support. No OS-theme/subscription semantics change with this boundary.

App-store persistent theme, Activity bus-feed and Clock tick streams are
subscription recipes. Snapshot restoration restarts each recipe once without
replaying finite mount tasks; pending finite work continues to reject snapshots.

### Tree raster image transport

`image` accepts an embedded relative literal asset or an `image` value produced
by `encoded(bytes)` / `rgba(width, height, bytes)`. Tree lowering preserves
`w`, `h`, `fit`, `rotate`, `opacity`, `filter` and `label`. The guest sends copied
encoded bytes or dimensions plus RGBA bytes once per typed content hash.
Runtime filesystem `Handle::Path` values report through `host.log` and produce
an empty node; statically visible nonembedded paths are E190. No host filesystem
fallback exists. Dynamic filesystem sources and image allocation operations
remain unsupported Tree features.

`viewer` accepts the same copied image sources and reuses the same raster cache.
Its native Iced widget receives `w`, `h`, `fit`, `filter`, `p`, `min-scale`,
`max-scale`, `scale-step` and `label`. Scale bounds use the shared native positive,
finite, ordered normalization; omitted options retain native defaults. Native
widget state retains zoom and pan across frames and keyed moves, but is not
serialized into guest snapshots. Missing or rejected pixels draw an empty area
with the requested dimensions. The host never reads an image path.

SVG and raster payloads share a 1 MiB frame allowance and an 8 MiB host-session
copied-byte allowance. Raster vector headers above 8 MiB are rejected before
allocation. RGBA dimensions must be nonzero and match the exact byte length.
The host admits at most 4,194,304 pixels per image, 8,388,608 attempted decoded pixels
per frame, and 16,777,216 attempted raster pixels per session. Combined SVG/raster
cache entries are capped at 8192; raster failures consume an entry and their
copied-byte allowance. Failed pixel decoding also spends its admitted pixel allowance. Excess or malformed pictures draw empty space with their
specified dimensions. Cache entries are retained without eviction because guests
send each payload once. Lazy subtrees use only already admitted cache entries;
Resync retains host pictures, and a replacement guest starts its own send history.

Encoded data uses the existing image decoder formats and orientation handling.
Decoder limits request at most 64 MiB allocation and bound dimensions before
pixel decoding. These are admission and decoder limits, not a guarantee on total
process memory or every codec's temporary allocations. Animations use their
static image decoding, as the native image widget does.

The tiny-skia renderer translates raster destinations in floating-point widget
coordinates before scaling their source pixels. This preserves fractional
origins under the existing parent transform and rotation, rather than rounding
the destination to whole source-pixel steps.

### Tree searchable combo boxes

Tree `combo` uses the host's actual Iced ComboBox and native searchable overlay.
Options and selected values remain typed guest data; selection and hover return
original option indices through the guest's typed route tables. Input, open and
close callbacks, declarative input/menu faces, typography, padding and icons cross
as copied data. An omitted width keeps the native Fill default. Rust style
callbacks and combo value parameters without an owned App state binding produce
source-origin E190. Component-owned `combo` state remains the existing common
E103 restriction, including for native compilation.

The App binding identifies shared search state; widget identities keep focus
separate. Assigning even the same options resets search, while pushing an option
preserves it. Temporarily hiding every widget for the binding preserves search.
Reload keeps it only when identity, reset revision and options match exactly;
hidden bindings are checked when shown again. Stale instance overlays cannot
send events to a replacement. Retained identities are capped by `MAX_NODES`,
and their keys, option labels and search text share `MAX_TEXT_BYTES_PER_FRAME`.
A combo that exceeds the retained inventory budget displays an explicit rejection
instead of using a different or outdated state. Rebuild hosts and guests together.

### Tree slider handle shapes

Tree slider faces carry `handle=circle(radius)` or `handle=rect(width)` with
`handle-r=` corner radii. The active face overlays the host theme; hovered and
dragged faces overlay the active face. Omitted fields preserve that inheritance;
an explicit zero circle radius hides the handle. Hosts bound copied radii to their pixel budget and use the native Iced handle geometry.
Rebuild hosts and guests together: the copied slider face wire layout changes.

### Tree container gradients

Container `bg=linear(angle, color@offset, ...)` uses native Iced construction,
then copies its angle and eight stop slots to the host. This preserves native
stop insertion, duplicate offsets and overflow behavior. The host paints a
native Iced linear gradient against the current container bounds. Stop colors
include their authored alpha. Nonfinite angles become zero; malformed wire
stops with nonfinite/out-of-range or nonincreasing offsets are discarded, and
colors use the shared RGBA bounds. Control faces, other layout surfaces, rich
spans and canvas gradients remain separate Tree gaps. Rebuild hosts and guests
together because Container background now carries a color-or-gradient tag.

## Default form composition

The standard component library exposes `Form`, `FormSection`, and `TextField`
through its existing `default.ice` import. These use ordinary components,
props, slots, recipes and native input binding; they introduce no Core syntax.
Form owns vertical scrolling and bounded content width. Field accepts optional
help and error text without allocating empty rows; TextField supplies a native
input while keeping padding and radius customizable. The exact props and
ownership contract are documented in `crates/ui-lang-components/README.md`.

## Default header composition

The shared Ice `PageHeader` and `Panel` components default `description` to the
empty string. Empty descriptions render no node or inter-child gap; supplied
text wraps inside the available width. Their `root/title` and optional
`root/description` IDs identify real text nodes. This is ordinary component
composition, not a new Core construct. Public signatures are tracked in the
component API baseline.

## Tree editor transactions

The existing `editor-binding` declaration has a target-specific Rust boundary.
Tree factories return `ui_lang_guest::EditorBinding<Payload>`; Native factories
retain the native Iced key-binding signature. The authored route still receives
its declared `Payload`. See [the transaction contract](docs/editor-transactions.md)
for admission, ordering, cancellation and guest-owned history.

## Default list item sizing

The standard library's `Item(title, description, meta)` preserves its leading
slot's chosen size. Its primary text and metadata use content-based flex sizing:
they shrink together under constraint, while metadata retains its intrinsic
width when space is available. Text wraps in the allocated columns. This is
ordinary library composition over existing `flex` and `box` behavior; it does
not change primitive row fill/shrink semantics or add a layout property.

## Default page insets

The standard component source also exports `Alert` and its `Success`,
`Warning`, `Destructive` variants, plus `EmptyState`, with inputs
`title:str, description:str=""`. Empty descriptions omit the text node and
its gap. Alert copy wraps within its content width; EmptyState copy fills that
width and centers each line. Both use word-or-glyph wrapping. These are
ordinary component defaults and conditional views, not additional syntax.

The standard component library exports `Page(padding:f64=24.0)`. Page fills its
available bounds, uses the semantic background, and insets one content root on
all four sides. Padding is customizable, including explicit zero for full-bleed
content. Page does not own scrolling or maximum content width. Form already
provides its own outer padding and should not be nested inside Page merely to
obtain a screen inset. This is ordinary library composition, not implicit margin
on every widget or a new Core construct.

### Display text budget diagnostics

The tree wire keeps its 64 KiB per-string limit and 64 KiB aggregate shaped-text
budget. Display strings may be shortened to fit; an `Editor` document exceeding
its per-document limit or losing bytes to the aggregate budget rejects the frame
instead. Editor placeholders are display strings. Document occurrences still
consume the aggregate budget; shared document storage is a separate feature.

`sanitize(Frame)` and `apply(Node, patches)` return a fixed-size `SanitizeReport`
from the actual before/after sanitizer pass. `display_text_truncated` covers
visible text, rich spans, control labels/options/placeholders, accessibility
labels, and textual host-surface arguments, which share the same budget (for
example a code preview). Surface record/type names and font family names are
metadata rather than textual payloads. Intentional
patch removal happens before measurement and does not produce a truncation report.
A producer that sanitizes before encoding retains its report in
`Frame.upstream_sanitization`. Receivers independently validate full frames and
applied patch results and distinguish their own observations from this advisory
producer report.

A host retains loss reports alongside its accepted tree through patches and
unchanged frames: freeing budget cannot reconstruct bytes already removed.
A new complete, untruncated tree clears that tree's loss status. The app-store
host logs `module`, installation `generation`, the fixed reason
`display_text_truncated`, and `origin=host|producer-reported`, without message or
document contents. Each origin is logged once per successful instance installation;
a failed reload candidate does not change that generation or warning state.

The report field changes the frame schema: wire epoch 2 requires matching hosts
and guests; epoch 1 modules are refused before execution.

### Hosted Tree window and input-method observations

Hosted Tree execution supports window focus/unfocus, close-request/closed,
file-hover/drop/leave and input-method opened/preedit/commit/closed subscriptions.
Interest is recomputed from active recipes by category; generic event recipes
opt into all supported categories. Observations retain captured status and
never replay input into host widgets. Preedit ranges use UTF-8 byte offsets.
Window geometry, scale and frame clocks are not Tree events and produce E190.
A host may deliver `closed` in one bounded final tick after removing a window;
that tick must not execute guest effects or delay the native close decision.

### Hosted OS theme and own-window controls

Tree `task system theme` returns the host OS mode and `subscribe system theme`
observes its current known value and changes, independently of application palette
selection. The values remain `none`, `light`, and `dark`. Queries wait for a real
host answer; malformed responses do not become successful default values.
Tree supports own-window maximize, minimize and resizable boolean controls via
the existing host effect lane. Other unsupported window and system-information,
font-load and image-allocation tasks fail with E190. Raw native toolkit actions
are not a portable effect API.

### Hosted resize handles

Tree `resize-handle` carries its stable identity, child, declarative cursor and
press/release/drag routes. The host uses the native resize widget, retaining
pointer grab outside the child's bounds until left release. Drag values are
logical-pixel deltas, independent of window position. Consecutive deltas are
accumulated without crossing a discrete route event. Removed/replaced widget
state never transfers an active gesture. This shape requires wire epoch 7.

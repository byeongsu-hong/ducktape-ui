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
from generated filenames back to source roots and content digests.

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

On the tree target, checked `task widget` focus, focused-query, input cursor/
selection and scroll/snap statements emit `host.widget` requests containing
`ui_lang_wire::WidgetCommand`. Qualified widget paths are copied exactly;
no native widget Id or operation object crosses. The host waits for the
requesting frame to be mounted and traverses only that guest's widgets and
overlays. Focused queries return bool (false for a missing target); mutations
acknowledge unit, preserving sequential Task ordering. Widget selectors and
virtual-row scrolling are rejected with E190 on this target. Native extern
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

### Accessibility

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
routing are deterministic across platforms. Native screen-reader export is a
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
                   ("status=" event_status)? ("when" expr)? "->" route
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
its native overlay events to the same guest. Native callback-based float
positioning remains refused. Wire child, numeric, depth, and frame limits apply.

## Tree text presentation

The tree target copies plain-text wrapping/shaping, named font descriptors,
relative/absolute line height, height, vertical alignment and grapheme tracking
to native host widgets. Hosts register trusted family names through
`ui_lang_runtime::view_tree::register_font_family` and load the matching font
bytes. Unregistered names use native sans-serif. Tracking uses a non-selectable
grapheme row and shares the host node budget. Boxes support maximum dimensions
and clipping; buttons support padding utilities. These wire fields require
host and guest rebuilds together.

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
default font and text size apply to compact labels; fixed dimensions center
content on the corresponding axes. Explicit zero padding overrides native
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

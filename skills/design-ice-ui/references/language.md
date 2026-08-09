# Ice language reference

Use this reference for the source model, declarations, types, expressions,
state, handlers, routes, and components. The repository implements language
revision 2.0; package version `0.1.0` and Iced version `0.14.x` are separate
version axes.

## Contents

- [Program model](#program-model)
- [Source and import rules](#source-and-import-rules)
- [Minimal application](#minimal-application)
- [Application settings](#application-settings)
- [Types](#types)
- [State and expressions](#state-and-expressions)
- [Handlers, routes, and effects](#handlers-routes-and-effects)
- [Components and slots](#components-and-slots)
- [Control flow and identity](#control-flow-and-identity)
- [First-class tests](#first-class-tests)
- [Common invalid translations](#common-invalid-translations)

## Program model

The compilation path is:

```text
UTF-8 .ice source graph
  -> relative import resolution
  -> indentation-aware parser
  -> name/type/semantic checker
  -> nominal CheckedDocument
  -> generated Iced Rust
  -> rustc
```

`ui_lang_build::compile_dir` runs this path from `build.rs` and writes ordinary
Rust below Cargo's `OUT_DIR`. `ui_lang::include_app!` includes one generated
root during Rust compilation. The build helper tells Cargo to track the root
and every imported `.ice` fragment. There is no runtime parser.

Ice owns:

- transient and display state;
- view structure, layout, style, and semantic identity;
- interaction routing and UI-local transitions;
- calls across typed Rust boundaries.

Rust owns:

- authoritative input validation and domain invariants;
- persistence, networking, authentication, and security;
- observability and platform-specific behavior;
- custom native widgets, tasks, subscriptions, and render programs.

## Source and import rules

- Use UTF-8 `.ice` files.
- Indent with two spaces. Tabs are errors.
- Let a deeper indent make a line a child of the previous line. Dedent only to
  a previously established level.
- Use full-line `//` comments. Inline and block comments are unsupported.
- Use ASCII identifiers. Do not begin one with a digit or `__`.
- Treat `_`, `none`, and Rust keywords as reserved.
- Prefer `PascalCase` for apps, extern structs, and components.
- Prefer `snake_case` for state, fields, functions, handlers, and parameters.
- Use kebab case after static IDs such as `#task-list`.
- Use double-quoted strings. Supported escapes are `\n`, `\r`, `\t`, `\"`,
  and `\\`.
- Import with `use "relative/path.ice"`. Paths are relative to the importing
  file, must use `/`, must end in `.ice`, and cannot be absolute.
- Allow nested imports. Canonical duplicate imports are idempotent; cycles and
  missing files are errors.

Top-level declarations are order-independent, but keep canonical order:

```text
app | daemon
use
extern
theme contract
palette
recipe
font / qr
enum
state
preset
component
on
subscribe
view
test
```

A source graph has exactly one app/daemon root and one view. Declarations from
all imports share one checked namespace. Imported fragments cannot declare a
second app or view, but may declare graph-unique tests.

## Minimal application

```ice
app Tasks
  title "Tasks"
  window
    size 960 720
    min-size 480 360
    position centered

extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  list_tasks() -> [Task] ! AppError

theme contract ProductTheme
  bg
  fg
  primary
  danger
  surface
  muted

palette dark for ProductTheme
  bg #0f172a
  fg #f8fafc
  primary #7c3aed
  danger #dc2626
  surface #111827
  muted #94a3b8

state
  tasks:[Task] = []
  loading = false
  error = ""

on mount
  loading = true
  run list_tasks() -> loaded _ | failed _

on loaded(next)
  tasks = next
  loading = false

on failed(cause)
  loading = false
  error = cause.message

view
  col w=fill h=fill p=24.0 gap=16.0 @bg-bg
    text "Tasks" size=24.0 @text-fg font-bold
    if loading
      text "Loading…" @text-muted
    if error != ""
      text error @text-danger
    for task in tasks
      text task.title @text-fg
```

Required theme tokens are `bg`, `fg`, `primary`, and `danger`; additional
tokens are allowed. The checker validates every token use.

## Application settings

Put settings under `app Name`:

```ice
app Tasks
  title window_title
  theme app_theme
  bg app_background
  fg app_text
  id "dev.example.tasks"
  executor iced::executor::Default
  renderer crate::backend::AppRenderer
  font "assets/Inter-Regular.ttf"
  text-size 16
  antialiasing true
  vsync true
  scale ui_scale
  window
    icon-rgba "assets/app.rgba" 32 32
    size 960 720
    min-size 480 360
    max-size 1920 1080
    position centered
    level normal
    maximized false
    fullscreen false
    visible true
    resizable true
    closeable true
    minimizable true
    decorations true
    transparent false
    blur false
    exit-on-close true
```

`title`, `theme`, `bg`, `fg`, and `scale` may be state expressions and are
recomputed by Iced callbacks. A named `window child` block declares a template
opened later by a window task. Platform blocks may coexist:

```ice
window
  platform linux
    app-id "dev.example.tasks"
    override-redirect false
  platform windows
    drag-and-drop true
    skip-taskbar false
    undecorated-shadow true
    corner round-small
  platform macos
    title-hidden false
    titlebar-transparent true
    fullsize-content-view true
  platform wasm
    target "iced"
```

Use `daemon Name` for a windowless application. A daemon rejects an unnamed
initial window; declare named templates and open them from `on mount`. The
read-only `window:window-id` binding names the currently rendered window.

## Types

Common Ice types:

| Ice | Rust boundary |
| --- | --- |
| `bool` | `bool` |
| `i64` | `i64` |
| `f64` | `f64` |
| `str` | `String` |
| `bytes` | `Vec<u8>` |
| `[T]` | `Vec<T>` |
| `T?` | `Option<T>` |
| `result[T,E]` | `Result<T, E>` |
| declared UI enum `Name` | generated cloneable Rust enum `Name` |
| named `Task` | declared Rust struct |
| `unit` | `()` |

Frequently used native types include `image`, `markdown`, `editor`, `instant`,
`window-id`, `task-handle`, `event`, `event-status`, keyboard/pointer/geometry
types, color/background/gradient, font/text types, and animation types. Read
`SPEC.md` section 5 or run `cargo ice schema` before using an unfamiliar type.

Extern struct fields are readable with dot projection:

```ice
extern crate::backend
  Task(id:i64, title:str, done:bool)

on selected(task)
  selected_id = task.id
```

Do not destructure, mutate a field, or call methods in Ice.

## State and expressions

Infer state from non-empty literals:

```ice
state
  draft = ""
  loading = false
  retries = 0
  opacity = 1.0
  modes = ["List", "Board"]
```

Annotate empty and optional state:

```ice
tasks:[Task] = []
selection:str? = none
search_modes:combo[str] = ["List", "Board"]
request:task-handle? = none
```

Name repeated pure view or handler expressions with top-level read-only
derived values:

```ice
derived
  normalized_draft = trim(draft)
  can_submit = !loading && !empty(normalized_draft)
```

Derived values may depend on app state and other derived values and may call a
declared `pure` extern. They cannot be assigned or bound with `<->`; dependency
cycles are errors. They are pure read-only computations, not signals, persistent
caches, runtime dependency nodes, or state mirrors that handlers must update.
Their observable evaluation count is not guaranteed: the compiler may coalesce
equivalent safe reads within one eager view build, but retains nothing across
frames. They reject `sync` externs and recomputation-unsafe built-ins:
`window_id.unique`,
`aborted`, `debug.time_with`, `image.upgrade`, the unqualified `encoded` and
`rgba` image constructors, and animation queries whose instant is omitted. The
category covers both runtime reads and calls that create a fresh retained
identity. The checker still permits the set in top-level app state initializers,
handlers, and views; capture the needed value or identity in state when it must
remain stable across view passes.

The expression language is deliberately closed:

- literals: strings, booleans, `i64`, `f64`, `none`, homogeneous lists, and
  hexadecimal `bytes(00 ff ...)`;
- paths: state, parameters, loop bindings, and extern struct fields;
- unary `!` and `-`;
- arithmetic `* / % + -`;
- comparisons `== != < <= > >=`;
- booleans `&& ||`;
- parentheses;
- built-ins such as `len`, `empty`, `trim`, `some`, `encoded`, `rgba`, and
  `aborted`;
- declared `pure` extern calls in every expression context;
- declared `sync` extern calls only in top-level app state initializers and
  immediately evaluated app/component/preset handler expressions, including
  arguments inside nested task statements; async completion route expressions
  are evaluated when the callback runs and are pure-only;
- checked native constructor/query families documented by the specification.

Examples:

```ice
disabled=(loading || empty(trim(draft)))
visible=(len(tasks) > 0)
selected=(task.id == selected_id)
selection = some(task.title)
```

There are no arbitrary Rust expressions, closures, method calls, null,
truthiness, object literals, tuple destructuring, or general allocation.
Create a typed Rust `pure` function when a deterministic, side-effect-free
domain conversion is absent. Reserve `sync` for an immediate effect,
environment read, or retained identity; component state initializers cannot
call it because rendering may initialize them again.

Only handlers assign state:

```ice
on draft_cleared
  draft = ""
```

Do not assign in a view, component call, `if`, or event route.

## Handlers, routes, and effects

Declare a handler with inferred payload parameters:

```ice
on submit
  let title = trim(draft)
  return if loading || empty(title)
  loading = true
  run create_task(title) -> created _ | failed _

on created(next)
  tasks = next
  draft = ""
  loading = false

on failed(cause)
  loading = false
  error = cause.message
```

Rules:

- Assign only declared state with a matching checked type.
- Use immutable `let name = expression` locals for repeated handler values;
  locals cannot shadow state, derived values, parameters, or earlier locals.
- Use `return if <bool>` as an early guard.
- Use `sync` externs only in expressions evaluated while the handler runs, such
  as `let` initializers, assignment right-hand sides, guards, and nested task
  arguments. Completion route expressions run later in callbacks and may call
  only `pure` externs.
- Put a task-producing statement last; it returns one Iced `Task`.
- Route fallible externs to both success and failure handlers.
- Route infallible externs only to success.
- Let all incoming uses of a handler agree on payload arity and type.

The punctuation is semantic:

| Form | Meaning |
| --- | --- |
| `input "Title" <-> draft` | two-way binding to supported state |
| `button "Save" -> submit` | send a unit interaction to `submit` |
| `checkbox task.title ... -> toggle(task.id, _)` | pass an expression and emitted bool |
| `run save() -> saved _` | forward async output |
| `_` | current route's emitted payload |
| `#row` / `#row(task.id)` | static/dynamic scoped identity |
| `@bg-surface` | checked semantic utility |

`_` is not a general wildcard or variable. Use it only where the current route
offers a payload. A parameterless route may intentionally discard a payload.

Available effect families include:

- bare async extern + `run`;
- `task` extern + `task`;
- `stream` extern + `stream`;
- `sip` extern + `sip`;
- `flow` task composition;
- `parallel` and `sequential` groups;
- `abortable` and `abort`;
- native clipboard, font, system, widget, window, pane, image, time, and debug
  operations.

Ordinary `run` delivers every completion. Use a named request lane when later
work supersedes earlier work: `run latest lane=search` filters stale success
and failure messages without stopping the old Future, while `run replace
lane=preview` also aborts the prior Iced task. Equal fully qualified lane names
join calls across handlers for one state owner. A fragment imported `as
catalog` may contribute an aliased component whose internal lane is likewise
qualified, but that lane remains owned by each component instance. Unaliased
app and preset fragments remain in the root namespace and may share root lanes.
The app owns one
scope, a daemon shares one scope across its windows, and each component instance
is independent. Names are static qualified identifiers and therefore finite per
owner; one owner cannot mix `latest` and `replace` for a name.

`latest` leaves stale Futures and their captured values live until completion.
`replace` drops work owned by the aborted task but cannot roll back effects
already performed or stop detached or blocking Rust work. Choose a backend
boundary with cancellation semantics that match the lane. Generated bookkeeping
is fixed per declared lane for each state owner; component-owner count follows
the retained/mounted lifetime contract. If an outer abort prevents the matching completion
from reaching update, one current replacement handle can remain until the next
replacement or owner drop; it does not accumulate.

Read [rust-boundary.md](rust-boundary.md) before adding one.

## Components and slots

Components are typed view templates, not runtime classes:

```ice
component TaskRow(task:Task, loading:bool)
  row p=16.0 gap=12.0 @w-full bg-surface
    checkbox task.title checked=task.done disabled=loading -> toggle(task.id, _)
```

They:

- have one view root;
- receive checked named props;
- do not capture app state;
- reject unknown, missing, duplicate, or wrongly typed props;
- expand at compile time when they have no local state/handlers.

A component may own small instance-scoped UI state:

```ice
component Counter(label:str)
  state
    count = 0
  on increment
    count = count + 1
  col
    text label
    text count
    button "Increment" -> increment
```

Local component state is keyed by hierarchical instance scope. It persists for
the app lifetime by default, so use stable explicit IDs for repeated retained
instances. Rendering may evaluate its initializer again, so it may call `pure`
externs but not `sync` externs or recomputation-unsafe built-ins, including the
unqualified `encoded` and `rgba` image constructors. Choose mounted lifetime
when disappearance should drop local state and abort replacement work:

```ice
component SearchDialog()
  lifetime mounted
  state
    query = ""
  on search
    run replace lane=search fetch(query) -> loaded _ | failed _
  input "Search" <-> query submit=search
```

There is no unmount handler or lifecycle effect.

Declare one conventional child slot with bare `slot`:

```ice
component Panel(title:str)
  col p=16.0 gap=12.0 @bg-surface
    text title
    slot

Panel title="Tasks"
  text "Content"
```

Declare named slots for structured content:

```ice
component Dialog()
  col
    slot header
    slot body
    slot actions

Dialog
  header:
    text "Delete?"
  body:
    text "This cannot be undone."
  actions:
    button "Cancel" -> cancel
```

Every slot is required and accepts one root; wrap siblings in `row`, `col`,
`grid`, or `stack`.

Qualified components form checked compound components:

```ice
component Dialog.Header()
  row
    slot

Dialog
  Dialog.Header
    text "Delete?"
```

All direct children of a compound call must belong to that family.

Components may declare one output:

```ice
component Toggle(checked:bool) -> bool
  extern native_toggle(checked) -> emit(_)

Toggle checked=enabled -> enabled_changed _
```

Every non-`unit` call requires a route. `emit` must forward the declared type.

## Control flow and identity

Use checked view control flow:

```ice
if loading
  text "Loading"

match status
  "ready"
    text "Ready"
  "failed"
    text error
  _
    text "Waiting"

for task in tasks
  TaskRow task=task loading=loading

keyed task in tasks by=task.id w=fill gap=8.0
  TaskRow task=task loading=loading

lazy loading as busy
  if busy
    text "Working"
```

Literal `match` uses first-match semantics; `_` is an optional final fallback.
Option and result patterns are exhaustive:

```ice
match choice
  some(value)
    text value
  none
    text "Not selected"

match outcome
  ok(value)
    text value
  err(error)
    text error
```

Top-level, non-generic UI enums use zero or one cloneable payload per variant.
They cannot be recursive. Constructors and exhaustive patterns share the same
spelling; a payload binding exists only inside its arm:

```ice
enum RequestState
  idle
  ready([Task])
  failed(AppError)

match request
  RequestState.idle
    text "Idle"
  RequestState.ready(tasks)
    TaskList tasks=tasks
  RequestState.failed(error)
    text error.message
```

`for` renders a checked list. `keyed` provides stable reconciliation identity.
`lazy dependency as name` rebuilds only when its hashable dependency changes;
it exposes only the dependency alias as a value but keeps the enclosing
component's routing context (local handlers, `forward`, `emit`).

IDs are scoped through components and dynamic structures:

```ice
scroll #task-list
box #task(task.id)
```

Use IDs for widget operations, component-local state identity, panes, and
accessibility routing. Every concrete rendered built-in accepts a direct ID.
Do not use DOM selectors or assume global uniqueness. A component call ID
introduces a scope, not a synthetic rendered box; select an identified
descendant for layout tests.

## First-class tests

Declare generated behavior tests in the same source graph:

```ice
test counter_contract
  viewport 320 240
  mount
    Counter #counter
  target root = #counter/root
  target increment = #counter/increment
  expect root.width ~= 240.0
  click increment
  expect text "1" within root
```

Optional `preset`, `viewport`, `timeout`, `mount`, and `target` declarations
must precede actions. Target aliases are scoped to one test and may be reused
in another. A later target may use an earlier alias as a path prefix, while a
`#` path is always absolute. Actions drive the real rendered widgets and
generated update/task/subscription path. Assertions can read app state, exact
visible text/input content, post-layout geometry, and structured paint fields. Rust externs are
real; deterministic variants belong behind a preset or `cfg(test)`, not an Ice
mock. Finite tasks settle before the next step; subscriptions are re-established
around simulated events, while intentionally infinite timer/I/O subscriptions
are sampled rather than awaited to global quiescence. Run `cargo ice test`;
ordinary Cargo discovers the same generated tests.

## Common invalid translations

| React/CSS instinct | Ice form |
| --- | --- |
| `<Button onClick={save}>` | `button "Save" -> save` |
| `value={draft} onChange={setDraft}` | `input "Title" <-> draft` |
| `useState(false)` | `state` block + assignment in `on` handler |
| `useEffect(() => load(), [])` | `on mount` ending in `run`/`task` |
| component closure over app state | explicit typed prop and route |
| `children` prop | declared `slot` |
| conditional JSX | indented `if` or `match` |
| `.map(...)` | `for item in items` or `keyed` |
| CSS class string | checked `@` utility or typed style property |
| DOM `id` | scoped `#id` |
| fetch in component render | typed Rust extern called from handler |
| arbitrary JS/Rust expression | closed expression or typed `pure` extern; `sync` only at its effect boundary |
| thrown async error | declared `! Error` and mandatory failure route |

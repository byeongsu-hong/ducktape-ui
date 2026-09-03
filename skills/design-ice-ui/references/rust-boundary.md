# Rust boundary, effects, and design integration

Use this reference when connecting `.ice` to Rust, changing Cargo setup,
running work, consuming native Iced capabilities, or testing generated apps.

## Contents

- [Cargo and entry point](#cargo-and-entry-point)
- [Extern namespace and structs](#extern-namespace-and-structs)
- [Pure functions and immediate sync calls](#pure-functions-and-immediate-sync-calls)
- [Async futures and result routes](#async-futures-and-result-routes)
- [Native task adapters](#native-task-adapters)
- [Streams, sippers, groups, and cancellation](#streams-sippers-groups-and-cancellation)
- [Subscriptions](#subscriptions)
- [Typed native adapters](#typed-native-adapters)
- [Cargo features and compatibility](#cargo-features-and-compatibility)
- [Testing](#testing)
- [Boundary checklist](#boundary-checklist)

## Cargo and entry point

The repository's reference app declares Iced plus the runtime and proc-macro
crates, with the generator as a build dependency:

```toml
[dependencies]
iced = { version = "=0.14.0", default-features = false, features = ["advanced", "tokio", "wgpu", "x11"] }
ui-lang = { path = "../../crates/ui-lang" }
ui-lang-runtime = { path = "../../crates/ui-lang-runtime", version = "=0.1.0" }

[build-dependencies]
ui-lang-build = { path = "../../crates/ui-lang-build", version = "=0.1.0" }
```

Preserve the consuming workspace's existing feature set. Add an Iced feature
only when the chosen widget or operation requires it.

Declare `ui-lang-runtime` directly. Generated Rust refers to the public
`::ui_lang_runtime` path; a transitive dependency is insufficient. Declare
`ui-lang-build` directly under `[build-dependencies]`; its exact version and the
runtime version are part of the backend contract.

Generate every app or daemon root below the source directory with the standard
Cargo build-script phase:

```rust
// build.rs
fn main() {
    ui_lang_build::compile_dir("src/ui").expect("compile Ice sources");
}
```

The helper writes generated Rust below `OUT_DIR/ui-lang-generated` and emits
Cargo dependency tracking for every root and imported fragment. Do not write a
custom target-directory resolver or generate into the system temporary
directory.

Include one manifest-relative app root:

```rust
ui_lang::include_app!("src/ui/tasks.ice");

fn main() -> iced::Result {
    Tasks::run()
}
```

Paths:

- must be one string literal;
- are relative to `CARGO_MANIFEST_DIR`;
- must use `/`;
- cannot be absolute or contain escape sequences.

The macro only includes the generated root from `OUT_DIR`; it performs no
filesystem writes. Generated code contains compile-time probes for declared
extern structs, fields, functions, and typed adapters even when they are not
reached at runtime. Do not include generated Rust manually.

## Extern namespace and structs

Declare a Rust module namespace once:

```ice
extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  list_tasks() -> [Task] ! AppError
  create_task(title:str) -> [Task] ! AppError
```

This resolves to:

```rust
crate::backend::Task
crate::backend::AppError
crate::backend::list_tasks
crate::backend::create_task
```

One source graph may declare several extern namespaces, but extern type and
function names remain graph-global. Keep declarations close to the `.ice`
feature that consumes them.

Rust structs should expose the declared fields with compatible types:

```rust
#[derive(Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub done: bool,
}

#[derive(Clone)]
pub struct AppError {
    pub message: String,
}
```

Generated Iced messages generally require crossing payload values to be
`Clone`. Rustc reports missing/private items and incompatible shapes.

Common mappings:

| Ice | Rust |
| --- | --- |
| `bool` | `bool` |
| `i64` | `i64` |
| `f64` | `f64` |
| `str` | `String` |
| `bytes` | `Vec<u8>` |
| `[T]` | `Vec<T>` |
| `T?` | `Option<T>` |
| `result[T,E]` | `Result<T,E>` |
| `unit` | `()` |

Use the complete type table in `SPEC.md` section 5 for native Iced types.

## Pure functions and immediate sync calls

Use `pure` for a deterministic, side-effect-free conversion the closed
expression language does not provide:

```ice
extern crate::backend
  NetworkError(message:str)
  pure normalize_error(error:NetworkError) -> str

on network_failed(cause)
  error = normalize_error(cause)
```

Rust:

```rust
pub fn normalize_error(error: NetworkError) -> String {
    error.message
}
```

A `pure` extern:

- may be called in checked Ice expressions;
- may be reevaluated in derived values, views, settings, component defaults and
  state initializers, subscription filters, easing, handlers, and tests;
- returns its value directly;
- cannot declare `! Error`;
- promises the same result for the same arguments and no observable side
  effects;
- must not become a back door for arbitrary rendering logic.

The compiler checks the signature but cannot inspect the Rust body. Treat
`pure` as a trusted backend contract, not an optimization hint.

Use `sync` when the call must run immediately and may perform an effect, read
the environment, or create retained identity. It is valid in a top-level app
state initializer or an immediately evaluated app, component, or preset handler
expression, including `let` initializers, assignment right-hand sides, guards,
and arguments inside nested task statements. A component state initializer
cannot call it because component rendering may initialize local state again.
Derived values, component defaults, and component state initializers also reject
recomputation-unsafe built-ins: `window_id.unique`, `aborted`,
`debug.time_with`, `image.upgrade`, the unqualified `encoded` and `rgba` image
constructors, and animation queries with an omitted instant. This category
covers both runtime reads and fresh retained identities. The checker permits
these built-ins in top-level app state initializers, handlers, and views. Capture
the needed value or identity in app state or an immediately evaluated handler
expression when it must remain stable, then pass it to `pure` functions used by
recomputed expressions.

Declared `pure` and `sync` calls take precedence over built-ins with the same
name. Neither kind may declare `! Error`.

## Async futures and result routes

A bare function declaration is asynchronous:

```ice
extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  list_tasks() -> [Task] ! AppError
  save_task(title:str) -> Task ! AppError
```

Rust:

```rust
pub async fn list_tasks() -> Result<Vec<Task>, AppError> {
    // Domain validation and I/O stay here.
}

pub async fn save_task(title: String) -> Result<Task, AppError> {
    // Validate again even if the UI disabled an empty submit.
}
```

Call it only from a handler:

```ice
on submit
  return if loading || empty(trim(draft))
  loading = true
  run every save_task(trim(draft)) -> saved _ | failed _
```

Every handler Future names one delivery mode: `run every`, `run latest`, or
`run replace`; all three lower through `iced::Task::perform`. A fallible
declaration requires both routes; an infallible declaration forbids the failure
route. The task statement must be last. Its arguments are evaluated immediately
and may call a declared `sync` extern, including inside nested task groups. Each
explicit expression in the success and failure routes becomes an owned snapshot
when the statement launches; both branches are materialized then, while `_` is
supplied only by the delivered completion. Route expressions remain pure-only
so an unused branch cannot perform an effect; their results must be ordinary
cloneable Ice data, and direct recomputation-unsafe builtins are rejected.
Evaluate either runtime value once in a preceding handler `let` and route that
local instead. The same snapshot rule applies to every `task` statement
completion route, including built-in tasks; stream, sip, flow, and native query
route timing is unchanged.

Use a named delivery lane when starts from one or more handlers supersede the
same logical work:

```ice
component Search()
  state
    query = ""
    result:str? = none
  on search
    run latest lane=search fetch(query) -> loaded _ | failed _
  on loaded(value)
    result = some(value)
  on failed(error)
    result = none
  col
    input "Query" <-> query
    button "Search" -> search
```

`run every` delivers every Future completion and owns no lane. Equal fully
qualified lane names share one lifecycle across handlers for an app, across all
windows of a daemon, or within one component instance; component instances
remain independent. A component imported under an alias qualifies its internal
lane into that namespace, but the lane remains component-instance-owned.
Unaliased app and preset fragments remain in the root namespace and may share
root lanes. `run latest` does not stop stale Futures, so their captures remain
live until completion. Use `run replace lane=<name>` when the prior Iced task
should be aborted. Abort cannot undo an effect already performed or stop work
detached by the Rust backend. Lane names are static and finite per owner, and
one lane cannot mix effect kinds or delivery modes.
Component-owner count follows the retained/mounted lifetime contract.

Bare handler `run` and `stream`, plus `stream latest`, are rejected.
Subscription `run` remains a long-lived stream source, and task-flow
`from`/`then` Future or stream sources remain Task adapters without a direct
completion route, so those distinct constructs do not select a delivery mode.

Use `invalidate lane=<name>` directly in an app, daemon, preset, or component
handler when an immediate state transition supersedes an existing lane without
starting replacement work. The lane may be declared later in the source graph
but must already exist for the same owner; invalidation does not allocate a lane
or return a task. It advances the generation before aborting, so every earlier
Future completion or queued stream item is stale. It leaves `latest` work
running and releases the current handle for a `replace` lane.
A matching Future replacement completion normally releases that handle first.
If an outer `abortable` suppresses the completion, one fixed current handle
remains until replacement, invalidation, or owner drop; it does not accumulate.
A component affects only its runtime instance. `parallel`, `sequential`, and
`abortable` task composition do not accept invalidation because it produces no
task.

## Native task adapters

Declare `task` when Rust already returns `iced::Task`:

```ice
extern crate::backend
  task copy_text(text:str) -> unit

on copy
  task copy_text(value) -> copied
```

Rust:

```rust
pub fn copy_text(text: String) -> iced::Task<()> {
    iced::clipboard::write(text)
}
```

Use built-in Ice task statements for already-covered native operations, such
as clipboard, font, system, widget, window, pane, time, image allocation, and
debug work. Use a typed `task` extern for a native operation that has no
canonical source form.

Do not wrap an ordinary async function in `iced::Task` merely to use the
`task` spelling; use the simpler bare extern and `run every` when every
completion matters.

## Streams, sippers, groups, and cancellation

Use an explicit stream delivery mode for repeated task output:

```ice
extern crate::backend
  stream progress(total:i64) -> i64
  stream checked_progress() -> i64 ! AppError

on start
  parallel
    stream every progress(100) -> progressed _
    stream replace lane=checked checked_progress() -> progressed _ | failed _
```

Rust returns `impl Stream<Item = T> + Send + 'static`. A fallible stream yields
`Result<T,E>` items and requires both routes; an error item does not terminate
the stream. `stream every` starts independent work and delivers every item.
`stream replace lane=<name>` aborts the previous task and filters all items from
older generations, including items already queued before replacement. Its one
handle stays live across ordinary items and is released only after natural
termination, replacement, invalidation, or owner drop. Component handlers allow
only the replacement form. Handler streams may be direct or in `parallel` and
`sequential`, but are rejected anywhere under `abortable`; use lane invalidation
for replacement-stream cancellation or a subscription for view-derived
long-lived activation.

Use `sip` when repeated progress and one final output are different types:

```ice
extern crate::backend
  sip download(url:str) progress=f64 -> bytes ! AppError

on start
  sip download(url)
    progress -> downloading _
    done -> downloaded _
    error -> failed _
```

Rust returns an Iced `Straw<Output, Progress, Error>`-compatible value. Enable
Iced's `sipper` feature.

Group independent tasks in parallel:

```ice
on refresh
  parallel
    run every load_tasks() -> tasks_loaded _ | failed _
    run every load_profile() -> profile_loaded _ | failed _
```

Group ordered runtime actions sequentially:

```ice
on save_then_refresh
  sequential
    run every save_draft() -> saved _ | failed _
    run every load_tasks() -> tasks_loaded _ | failed _
```

Sequential construction reads inputs and state before the tasks run. Use a
result handler when task two needs state produced by task one's result.

Make a task cancelable:

```ice
state
  request:task-handle? = none

on start
  abortable request abort-on-drop
    run every load_tasks() -> loaded _ | failed _

on cancel
  abort request
```

Use `aborted(request)` to query the handle. `abort` keeps the handle so its
status remains readable.

Use `flow` only when native task combinators must transform or depend on prior
task output before the final UI route. Reuse a
tested flow fixture; its typed stages are intentionally stricter than promise
chaining.

## Subscriptions

Declare app-level sources in one `subscribe` block:

```ice
subscribe
  every 500ms when auto_refresh -> tick _
  keyboard press when shortcuts_enabled -> key_pressed _
  mouse moved -> pointer_moved _ _
  window close-request status=any -> close_requested
  system theme -> theme_changed _
```

Rules:

- Routes may use only `_` payload placeholders; read current state in the
  destination handler.
- Add `when <bool>` to stop an inactive subscription.
- Use `status=any|captured|ignored` only on native event sources that carry
  Iced dispatch status.
- Use `with=<hashable expression>` to make context part of identity and prepend
  it to route payloads.
- Use `filter=<pure function>` for a typed `filter_map`.
- Use `event raw` only with deliberate filtering/routing; redraw events can
  otherwise create a loop.

Extern long-lived sources:

```ice
extern crate::backend
  stream worker() -> str
  recipe counter(id:i64) -> i64
  event-filter runtime_event() -> str
  subscription app_events() -> bool

subscribe
  run worker() -> received _
  recipe counter(generation) -> counted _
  events generation using=runtime_event -> received _
  app_events() -> active_changed _
```

Choose the smallest adapter matching the native Iced contract. Do not create a
custom recipe for an ordinary timer or stream.

## Typed native adapters

The language exposes typed extern adapter kinds instead of arbitrary Rust in a
view. Important families:

| Declaration | Rust responsibility |
| --- | --- |
| `component` | return an `iced::Element` |
| `selector` | return an Iced widget selector |
| `shader` | return a shader program |
| `task` | return `iced::Task` |
| `stream` | return a futures stream |
| `sip` | return an Iced straw/sipper |
| `recipe` | return a subscription recipe |
| `event-filter` | map a raw runtime event to `Option<T>` |
| `subscription` | return an Iced subscription |
| `theme` | return `iced::Theme` |
| `themer` | return alternate theme + subtree tuple |
| `window` | execute against a raw native window handle |
| `markdown-viewer` | provide a Markdown viewer |
| `*-style` | provide typed native widget status style |
| editor adapters | binding, highlighter, or style callbacks |

Example custom element:

```ice
extern crate::backend
  component native_help(active:bool) -> bool

view
  extern native_help(help_open) #help -> help_changed _
```

Rust:

```rust
pub fn native_help(active: bool) -> iced::Element<'static, bool> {
    iced::widget::button(if active { "Close" } else { "Help" })
        .on_press(!active)
        .into()
}
```

Extern components own their native style and reject `@` utilities. A direct
`#id` identifies the bounds of the returned native element for first-class
tests. Their declared output controls whether a route is required.

Read the exact Rust signatures in `SPEC.md` section 5 and copy the nearest
fixture. Generated probes are the final compatibility check.

## Cargo features and compatibility

Feature requirements follow Iced:

- image handles/widgets need the appropriate image feature;
- Markdown needs `markdown`;
- sippers need `sipper`;
- canvas needs `canvas`;
- selectors need `selector`;
- native time operations need `tokio` or `smol`, with `repeat` requiring
  `tokio`;
- custom advanced widgets generally need `advanced`;
- the renderer/backend needs its selected graphics features.

Do not guess the feature name. Inspect the existing app manifest and the
corresponding checked fixture.

Run `cargo ice compat` when changing backend/runtime integration. It verifies:

- exact Iced and iced_widget lockfile baselines;
- direct `ui-lang-runtime` dependency and version;
- AccessKit target-specific pins;
- reference app/runtime manifest contracts;
- reference app tests.

Normal feature work usually needs only `cargo ice check` and focused tests.

## Testing

Application behavior uses first-class top-level Ice `test` declarations. They
compile to Rust's built-in test runner, so no Rust registration or separate
case grammar is needed. Follow:

```text
examples/showcase/tests/cases/ui/component_state.ice
examples/showcase/src/ui/app.ice
```

Use named presets or Rust `cfg(test)` implementations when extern behavior must
be deterministic. The Ice test still calls the real typed Rust boundary; there
is no DSL mock layer. Run `cargo ice test`, or the narrow package's ordinary
Cargo tests.

Compiler fixtures are auto-discovered:

```text
crates/ui-lang-core/tests/cases/<suite>/<case>/
├── as-is.ice
└── to-be.*
```

Use:

- `format` fixture for canonical formatting;
- `diagnostic` fixture for an exact language error;
- `compile` fixture for expected generated Rust fragments;
- nearby Rust unit tests for parser/checker/codegen edge cases.

When changing an app:

1. Run `cargo ice fmt --check`.
2. Run `cargo ice check`.
3. Run `cargo ice test` or the narrow package/test target covering the behavior.

When changing the language implementation, run the `ui-lang-core` suite and
the relevant app compilation/tests.

## Boundary checklist

- Keep authoritative validation in Rust.
- Match every Ice extern declaration to a public Rust item.
- Preserve direct `ui-lang-runtime` dependency.
- Preserve the direct `ui-lang-build` build dependency and standard `build.rs`
  call.
- Use a bare extern plus an explicit `run every`/`latest`/`replace` delivery
  mode for ordinary futures.
- Use `stream every` for independent repeated output or a named
  `stream replace` lane when later work supersedes the stream.
- Use the matching typed adapter for native Iced return types.
- Route every fallible effect to success and failure.
- Keep task-producing statements final.
- Add only required Iced features.
- Let `cargo ice check` and rustc verify the boundary.

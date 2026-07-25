# Design with the extended native surface

Use this reference only after Core layout/widgets, components, handlers, and
typed async externs are insufficient. Ice's extended surface is implemented,
but it is not a promise to mirror every Iced API.

## Contents

- [Choose the smallest surface](#choose-the-smallest-surface)
- [Extended widget inventory](#extended-widget-inventory)
- [Overlays and compound layout](#overlays-and-compound-layout)
- [Pane grids](#pane-grids)
- [Canvas](#canvas)
- [Native task operations](#native-task-operations)
- [Native subscriptions and values](#native-subscriptions-and-values)
- [Typed escape hatches](#typed-escape-hatches)
- [How to verify an advanced construct](#how-to-verify-an-advanced-construct)

## Choose the smallest surface

Stop at the first row that meets the need:

| Need | Use |
| --- | --- |
| common layout/control/content | Core `.ice` construct |
| reusable view structure | Ice component + explicit props/slots |
| pure conversion | Rust `sync` extern |
| domain/I/O future | bare async extern + `run` |
| covered native operation | built-in `task`/widget/window/pane statement |
| existing native task/stream/subscription | matching typed extern |
| custom visual or widget | shader/component/canvas typed surface |
| genuinely common missing authoring concept | language design + revision |

Do not add syntax just because Iced exposes a method. The language's stable Core
is deliberately smaller than its backend.

## Extended widget inventory

The implemented view grammar includes more than the schema's Core list. Locate
the exact syntax in `SPEC.md` section 4 and a compiling fixture before use.

### Layout and composition

- `grid`: fixed or fluid grid with gaps, padding, alignment, and sizing.
- `overlay`: content/layer structure with `when`, dismiss route, backdrop,
  padding, and alignment.
- `panes`: declarative PaneGrid configuration and dynamic operations.
- `float`: scale/translate a child with optional shadow/radius.
- `pin`: place a child at explicit coordinates inside checked bounds.
- `sensor`: route show/resize/hide lifecycle observations.
- `responsive`: build from current limits or expose width/height bindings.
- `tooltip`: exactly content plus tooltip nodes with position/style.
- `mouse`: wrap a node in pointer routes and cursor behavior.
- nested `theme` / `themer`: select a native theme for a subtree.

### Text and documents

- `rich-text` with checked `span` children and link routing.
- `markdown` viewer over `markdown` state.
- `editor` with `editor` state, actions, binding/highlighter/style adapters.
- `table` with typed rows, declared columns, cell/header roots, and optional
  resizers.

### Controls

- `toggler`;
- `radio`;
- `slider`;
- `progress`;
- `pick`;
- `combo`.

Each uses typed routes and structured native status blocks. Do not transfer
properties between widget families; the checker rejects ineffective style.

### Content and media

- `space`;
- horizontal/vertical `rule`;
- declared `qr`;
- `svg`;
- zoom/pan `viewer`;
- `shader`;
- `canvas`.

Cargo features may be required. Match the reference app's exact manifest and
fixture rather than enabling an Iced default feature set wholesale.

## Overlays and compound layout

Use `overlay` for modal content:

```ice
overlay when=about_open dismiss=close_about backdrop=black/60 p=24.0 align-x=center align-y=center
  content
    MainPage
  layer
    Dialog
      Dialog.Header
        text "About"
      Dialog.Body
        text "Details"
      Dialog.Actions
        button "Close" -> close_about
```

The first `content` tree remains the base. The `layer` tree appears only while
`when` is true. Use `dismiss=` for the overlay's native dismissal path. Keep
accessible focus order and labels inside the modal tree.

Use slots/qualified component families for structural APIs. Do not model them
with strings naming regions.

## Pane grids

`panes #id` owns a checked PaneGrid. A pane declaration may be open, closed, or
templated for dynamic keys. The root ID is required for operations.

Representative view:

```ice
panes #workspace w=fill h=fill gap=8.0 min-size=120.0 resize=8.0
  split vertical ratio=0.7
    pane tasks
      title
        text "Tasks"
      controls
        button "Inspect" -> inspect
      col
        text "Task list"
    pane details
      title
        text "Details"
      col
        text "Selection"
  pane preview closed
    text "Preview"
```

Representative handlers:

```ice
on maximize_details
  pane #workspace maximize details

on restore_workspace
  pane #workspace restore

on swap_workspace
  pane #workspace swap tasks details

on open_preview
  pane #workspace split details preview horizontal ratio=0.35

on close_preview
  pane #workspace close preview

on inspect_workspace
  pane #workspace maximized -> pane_observed _
```

Additional operations cover adjacent lookup, move, resize, drop, and dynamic
pane targets. Copy the exact syntax from
`examples/iced-app/src/ui/resizable_panes.ice` or `tasks.ice`; operation payloads
and edge names are checked.

## Canvas

Canvas is its own checked sublanguage, not arbitrary Rust drawing code:

```ice
canvas w=fill h=160.0 cache=mode capture=true cursor=crosshair
  state
    hits = 0
  event mouse pressed as button
    set hits = hits + 1
    emit canvas_pressed button
    redraw
    capture
  rect x=0.0 y=0.0 w=canvas_width h=canvas_height fill=bg stroke=border
  circle x=48.0 y=48.0 r=28.0 fill=primary stroke=fg stroke-w=2.0
  text mode x=16.0 y=136.0 color=fg size=14.0 font=default
```

Canvas supports:

- local canvas `state`;
- typed input-method, keyboard, mouse, touch, and window event sources;
- actions `set`, `emit`, `redraw`, and `capture`;
- rectangles, circles, lines, text, images, SVG, paths, and transform groups;
- canvas-local `if` and `for`;
- fill/stroke, cap/join/dash, clipping, translation, rotation, and scale;
- paths with move, line, arc, arc-to, ellipse, cubic/quadratic Bézier,
  rectangle, rounded rectangle, circle, and close segments.

Rules:

- Mutate canvas-local state only with `set`.
- Emit into app handlers instead of reaching into app state.
- Keep event sources unique within one canvas.
- Put root-only event/capture/redraw directives at the canvas root.
- Use `cache=<hashable dependency>` only when cached drawing is useful.
- Use `capture=true` and event-level `capture` deliberately; they affect Iced
  dispatch.
- Use `redraw ... after=<duration>` carefully to avoid unnecessary frame loops.

Read the canvas grammar and
`examples/iced-app/src/ui/canvas_events.ice` before editing.

## Native task operations

Handlers can end in checked native task statements. Families include:

- time (`task time now`);
- clipboard read/write, including primary selection where supported;
- system information;
- font loading;
- image allocation;
- debug spans/timing;
- widget focus, scroll, text input, and selector operations;
- window open/close/move/resize/mode/level/focus/screenshot/raw handle
  operations;
- pane mutations and queries;
- application `exit`.

Examples:

```ice
on focus_search
  task widget focus #search

on open_child
  task window open child -> child_opened _

on capture_window
  task window screenshot -> window_captured _

on quit
  exit
```

Widget targets are hierarchical IDs and may be selected by exact ID, text,
point, or declared selector adapters where supported. Operations may be scoped
inside a stateful component only when the checker can restrict them to that
component subtree.

Never guess operation payload order. Find the operation in `SPEC.md` section 7
and its focused `.ice` fixture.

## Native subscriptions and values

Subscription families include:

- timers: `every`, `repeat`;
- generic `event` and `event raw`;
- input-method;
- keyboard;
- mouse;
- touch;
- window;
- system theme;
- extern stream, recipe, event-filter, and subscription sources.

Native structured value types preserve Iced semantics across state, handlers,
and externs:

- keyboard key, physical key, location, modifiers, press/release;
- mouse button, cursor, click, interaction, and scroll delta;
- touch finger;
- point, vector, size, rectangle, and transformation;
- pixels, padding, degrees, radians, and rotation;
- length and alignment families;
- color/background/gradient;
- font and text metrics/style values;
- event status;
- window ID, position, direction, level, mode, attention, screenshot, and
  redraw request;
- instant and animation values.

Use the named constructor/query built-ins documented in `SPEC.md` section 6.
Do not replace a native typed value with a string unless the UI only needs a
display label.

Some native values intentionally reject equality, ordering, or lazy identity
because the underlying Iced type does not implement the needed trait. Let the
checker enforce that boundary.

## Typed escape hatches

Prefer typed adapters over arbitrary syntax:

```ice
extern crate::backend
  component native_help(active:bool) -> bool
  selector by_kind(kind:str) -> str
  shader status_shader(speed:f64) -> bool
  task copy_text(text:str) -> unit
  stream task_steps(count:i64) -> i64
  sip download(url:str) progress=f64 -> bytes ! AppError
  recipe events(channel:i64) -> str
  event-filter runtime_event() -> str
  subscription app_events() -> bool
  theme app_theme(dark:bool)
  themer alternate_panel(active:bool) -> bool
  window describe_window(prefix:str) -> str
```

Also use the dedicated Markdown, editor, and per-widget style adapter kinds
where applicable. Their Rust return signatures are specific and compile-time
probed. Copy the exact declaration/signature pair from `SPEC.md` or the focused
reference fixture.

Use a borrowed extern-component parameter (`&str`, `&bool`, and so on) only
when the returned element's lifetime genuinely benefits. Default to owned
values; borrowed custom widgets introduce real lifetime constraints.

## How to verify an advanced construct

1. Search `SPEC.md` for the construct's grammar and semantic section.
2. Search `examples/iced-app/src/ui/` for the exact spelling.
3. Search `crates/ui-lang-core/src/check/tests/` and
   `crates/ui-lang-core/src/codegen/tests/` for edge behavior.
4. Inspect `COVERAGE.md` to distinguish implemented reachability from a future
   request.
5. Confirm required Iced Cargo features in the reference app.
6. Keep the live LSP attached to the app root while editing.
7. Run `cargo ice fmt --check` and `cargo ice check`.
8. Run the focused Rust test/fixture.

If no compiled example, grammar, checker path, or coverage entry supports the
proposed spelling, treat it as nonexistent rather than improvising it.

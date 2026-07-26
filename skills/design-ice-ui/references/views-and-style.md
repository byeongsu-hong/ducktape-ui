# Design views, layout, styling, and accessibility

Use this reference when authoring an Ice view. Run `cargo ice schema` for the
exact current Core property table; this guide explains how the pieces compose.

## Contents

- [Tree rules](#tree-rules)
- [Lengths and numeric values](#lengths-and-numeric-values)
- [Core layout](#core-layout)
- [Core widgets](#core-widgets)
- [View control flow](#view-control-flow)
- [Theme tokens and colors](#theme-tokens-and-colors)
- [Semantic recipes](#semantic-recipes)
- [Typed properties and utilities](#typed-properties-and-utilities)
- [Native status styles](#native-status-styles)
- [Scoped IDs](#scoped-ids)
- [Accessibility contract](#accessibility-contract)
- [Design checklist](#design-checklist)

## Tree rules

A `view` and every component contain exactly one root node:

```ice
view
  col w=fill h=fill
    text "Header"
    row
      button "Cancel" -> cancel
      button "Save" -> save
```

Indentation determines parentage. If two nodes need to occupy one slot or root,
wrap them in a layout node. Do not use JSX fragments or sibling roots.

Most lines follow:

```text
construct positional-values #optional-id property=value ... @utility ...
```

The `@` marker switches the rest of that line to utility tokens. Put typed
properties before it.

## Lengths and numeric values

Every length position accepts:

- `fill`;
- `fill(N)` for a positive `u16` fill portion;
- `shrink`;
- a numeric expression, checked and lowered through Ice's `f64` boundary.

Examples:

```ice
row w=fill h=48.0
text title w=fill(2)
space w=fill h=shrink
```

Use `w=`/`h=` for direct native builder size. Use the few wrapper utilities
only where the compiler documents wrapper ownership.

## Core layout

### Row and column

Use `row` and `col` for ordinary one-axis layout:

```ice
col w=fill h=fill gap=16.0 p=24.0 align=start clip=false
  row w=fill gap=12.0 align=center
    text "Title" w=fill
    button "Save" -> save
```

Common properties:

| Property | Meaning |
| --- | --- |
| `w`, `h` | checked length |
| `gap` | child spacing |
| `align` | cross-axis `start`, `center`, or `end` |
| `wrap` | enable wrapping |
| `wrap-gap`, `wrap-align` | wrapped-line behavior |
| `p`, `px`, `py`, `pt`, `pr`, `pb`, `pl` | numeric padding |
| `max-w` | supported on column |
| `clip` | clip contents |

### Flex

Use `flex` when CSS-like flex behavior is actually required:

```ice
flex w=fill gap=8.0 justify=space-between items=center
  box grow=1.0 p=12.0 @bg-surface
    text "Sidebar"
  box grow=2.0 p=12.0 @bg-bg
    text "Content"
```

Container properties include:

- `dir=row|row-reverse|column|column-reverse`;
- `wrap=nowrap|wrap|wrap-reverse` or combined `flow=...`;
- `justify`, `items`, and `content`;
- `gap`, `gap-x`, and `gap-y`;
- size, maximum size, padding, and clipping.

Direct `box` children may set flex-item behavior:

- `order=<i64 expression>`;
- `grow`, `shrink`;
- `basis=auto|content|number|percent(number)`;
- `flex=none|auto|initial|grow[,shrink[,basis]]`;
- `self=auto|start|end|flex-start|flex-end|center|baseline|stretch`;
- `m`, `mx`, `my`, `mt`, `mr`, `mb`, `ml` with `auto`, a number, or
  `percent(number)`.

Do not emulate flex with nested rows and state logic.

### Box

Use `box` as a single-child container for padding, alignment, surface styling,
clipping, and flex-item properties:

```ice
box w=fill h=120.0 p=16.0 align-x=center align-y=center bg=surface r=8.0
  text "Centered"
```

It requires exactly one child. Surface properties include `bg`, `text`,
`border`, `border-w`, corner radii, shadow color/offset/blur, and
`px-snap`.

### Stack

Use `stack` for overlapping children:

```ice
stack w=320.0 h=180.0
  image cover w=320.0 h=180.0 fit=cover
  box w=fill h=fill p=16.0 bg=black/40
    text title @text-white
```

`under=<u16>` controls how many children render below the final child. Use
`overlay` for modal interaction rather than rebuilding it from stack state.

### Scroll

Use `scroll` with exactly one child:

```ice
scroll #task-list dir=vertical w=fill h=fill bar=hidden
  keyed task in tasks by=task.id w=fill gap=8.0
    TaskRow task=task loading=loading
```

Properties include direction, width/height, scrollbar visibility and metrics,
axis anchors, auto-scroll, typed style adapters, and routes for scroll or
viewport changes. Consult `cargo ice schema` for route payload fields.

### Grid and keyed

Use `grid min-cell=280.0` for CSS-like wrapping that never shrinks a cell below
the requested width. Use `max-cell=` only when iced's native behavior of adding
columns to cap cell width is intended, or `cols=` for a fixed count. These
three modes are mutually exclusive. Use `keyed` when list identity must remain
stable. Prefer `for` for a simple unkeyed list. Do not manually index a list;
Ice has no arbitrary indexing expression.

## Core widgets

### Text

```ice
text title w=fill size=24.0 line-h=1.2 font=default @text-fg font-bold
```

Available Core properties include width/height, size, relative or pixel line
height, font, horizontal/vertical alignment, shaping, wrapping, and a typed
Rust text-style call.

Text expressions may be `str`, numbers, booleans, or other checker-supported
display values. Do not call `.toString()`.

### Input

```ice
input "New task" #new-task label="Task title" description="Required" <-> draft hint="What needs doing?" disabled=loading secure=false submit=submit w=fill p=12.0 @bg-surface border border-border rounded-lg focus:border-primary
```

Nested input lines, when present, are native status/icon blocks rather than
property continuation.

Core behavior:

- The positional string is the default accessible name.
- `<-> state` is required and targets compatible state.
- `change=handler _`, `submit=handler`, and `paste=handler _` add routes.
- `secure=true` creates a password input and suppresses its value from the
  accessibility tree.
- `disabled`, sizing, padding, text metrics, alignment, font, and style are
  checked properties.

### Button

Compact button:

```ice
button "Save" disabled=loading p=12.0 style=primary -> save
```

Child-content button:

```ice
button #help label="Open help" description="Keyboard shortcuts" -> open_help
  row gap=8.0
    text "?"
    text "Help"
```

A route is required. A child-content button must declare `label=` because the
compiler cannot derive a stable accessible name from arbitrary children.

### Checkbox

```ice
checkbox task.title #done checked=task.done disabled=loading -> toggle(task.id, _)
```

Keep it on one line in source. The route emits a `bool`, forwarded by `_`.
Properties cover accessible label/description, size, width, gap, text
metrics, font, icon, disabled state, and native/extern style.

### Image

```ice
image cover label=album.title description=album.artist w=160.0 h=160.0 fit=cover r=8.0
```

Sources may be a path string or an `image` handle. Core properties include size,
content fit, rotation, opacity, filtering, scale, expansion, per-corner radius,
and integer crop rectangle.

An unlabeled image is decorative and omitted from the semantic tree.
`description=` without `label=` is rejected.

## View control flow

`if` renders its children only when a checked bool is true:

```ice
if error != ""
  text error @text-danger
```

`match` chooses the first equal arm. The optional `_` fallback must be last:

```ice
match status
  "ready"
    ReadyPanel
  "failed"
    ErrorPanel message=error
  _
    text "Waiting"
```

`for` exposes a typed item:

```ice
for task in tasks
  TaskRow task=task loading=loading
```

`keyed` adds stable identity:

```ice
keyed task in tasks by=task.id w=fill gap=8.0
  TaskRow task=task loading=loading
```

`lazy dependency as local` caches the subtree against a hashable dependency:

```ice
lazy loading as busy
  if busy
    text "Working"
```

Use `lazy` only around a measurably expensive subtree. Do not use it as React
`useMemo`; it changes Iced widget construction and has hashability constraints.

## Theme tokens and colors

Declare semantic colors:

```ice
theme
  bg #0f172a
  surface #111827
  fg #f8fafc
  muted #94a3b8
  primary #7c3aed
  danger #dc2626
  border #334155
```

`bg`, `fg`, `primary`, and `danger` are required. `white`, `black`, and
`transparent` are built in and cannot be redeclared. Add opacity with
`token/0..100`, such as `black/40`.

Use `#RRGGBB` or `#RRGGBBAA` in theme declarations. Dynamic application colors
also accept 3/4/6/8 digit strings, but prefer checked theme tokens in views.

Gradients use checked background forms, for example:

```ice
box bg=linear(1.57, primary@0.0, surface@1.0)
  text "Gradient"
```

## Semantic recipes

Use a top-level recipe for a repeated visual role:

```ice
recipe panel for box
  @w-full p-5 bg-surface border border-border rounded-lg overflow-hidden

recipe primary_action for button
  @px-4 py-2 bg-primary text-primary_fg rounded-md
  @hover:bg-primary/90 pressed:bg-primary/80 disabled:opacity-50

view
  box @panel
    button "Save" @primary_action -> save
```

Targets are `col`, `row`, `flex`, `grid`, `stack`, `box`, `text`, `input`, and
`button`. Recipe names are graph-global, so an imported design-system fragment
can supply the defaults. Recipes expand in place; later utilities win and
typed node properties override recipe defaults. Put a local exception before
the `@`, such as `box p=24.0 @panel`. Recipe bodies are checked even when
unused, and the LSP can follow or safely rename imported recipe names.

Recipes only group checked utilities and do not compose other recipes. Keep
one role per recipe; use a component when the repeated thing has structure,
state, slots, or behavior.

## Typed properties and utilities

There is no CSS engine, class string, selector matching, cascade, or runtime
utility parser. Utilities are checked and lowered at compile time.

Prefer typed properties for direct builder fields:

```ice
row w=fill gap=12.0 p=16.0
```

Use `@` utilities for semantic styling or documented wrapper gaps:

```ice
row @w-full bg-surface border border-border rounded-lg
text "Title" @text-fg font-bold
input "Name" <-> name @px-4 py-3 bg-surface border border-border rounded-md
button "Save" @px-4 py-2 bg-primary text-white rounded-md
```

Accepted utility families:

| Family | Forms |
| --- | --- |
| wrapper size | `w-full`, `h-full` |
| max width | `max-w-sm` through `max-w-2xl` |
| alignment | `items-center`, `self-center` on documented targets |
| spacing | `gap-N`, `p-N`, `px-N`, `py-N` on documented targets |
| semantic colors | `bg-TOKEN`, `text-TOKEN`, `border-TOKEN` |
| border | `border`, `border-2` |
| radius | `rounded-sm`, `rounded`, `rounded-md`, `rounded-lg`, `rounded-full` |
| text | `text-xs` through `text-2xl`, `leading-*`, `font-bold` |
| button state | `hover:bg-*`, `pressed:bg-*`, `disabled:opacity-*` |
| input focus | `focus:border-*` |

Spacing `N` is one of `0 1 2 3 4 5 6 8 10 12 16 20 24` and maps to four
logical pixels per unit. Opacity utility values are `0 25 50 75 100`.

Do not specify the same owned field twice through a typed property and a direct
utility. The checker reports an ownership conflict. A typed property may
intentionally override a recipe default. A rounded layout wrapper also needs a
background or border; otherwise there is no rendered surface to round.

## Native status styles

Interactive widgets may contain structured status blocks:

```ice
button "Add" -> submit
  active bg=primary text=white r=8.0
  hovered bg=primary/80
  pressed bg=primary/70
  disabled bg=surface text=muted
```

`active` is the base inherited by every native status. Declare only deltas in
`hovered`, `pressed`, `focused`, `dragged`, `opened`, or `disabled`.

Selection controls use paired base states:

```ice
checkbox "Done" checked=done -> changed _
  active checked bg=primary icon=white border=primary
  active unchecked bg=surface icon=primary border=border
  hovered checked bg=primary/80
  disabled unchecked bg=bg icon=muted
```

Inheritance rules:

- checked/selected states inherit their matching `active checked|unchecked` or
  `active selected|unselected`;
- `focused-hovered` inherits `active`, then `focused`;
- `opened-hovered` inherits `active`, then `opened`;
- the later, more specific field wins.

Use an existing `style=` preset for fixed native appearance. Use a typed Rust
style adapter for reusable or state-dependent appearance too complex for
tokens and status blocks.

## Scoped IDs

IDs are typed hierarchical identity, not DOM IDs:

```ice
scroll #task-list
box #task(task.id)
```

- Use `#kebab-name` for static identity.
- Use `#name(expression)` for dynamic identity.
- Let component calls extend the hierarchy.
- Use IDs for widget operations, pane roots, stateful component instances, and
  stable accessibility routing.
- Do not assume a global CSS selector namespace.

## Accessibility contract

Core mappings:

| Ice | Semantic role and state |
| --- | --- |
| `text` | Label with visible text |
| `input` | TextInput with name, description, value, disabled/focus |
| secure input | PasswordInput without exported value |
| `button` | Button with name, description, disabled/focus/click |
| `checkbox` | CheckBox with name, description, checked/disabled/focus/click |
| labeled `image` | Image with name and optional description |

Requirements:

- Supply `label=` on every child-content button.
- Label meaningful images; leave decorative images unlabeled.
- Never put `description=` on media without `label=`.
- Keep controls in meaningful source order; semantic read and keyboard focus
  order follow the view tree.
- Do not create numeric tab order; the language has none.
- Expect disabled controls to be skipped by Tab and click actions.
- Preserve keyboard behavior: Enter/Space activates focused buttons; Space
  activates focused checkboxes.

Native screen-reader export is currently narrower than tree construction:
single-window Linux uses AT-SPI and single-window Windows uses UI Automation.
Named windows and other targets retain deterministic semantic behavior without
claiming the same native adapter coverage.

## Design checklist

Before finishing a view:

- Confirm one root per view, component, and slot.
- Confirm all component props and slots are explicit.
- Use `for`/`keyed` instead of duplicated nodes.
- Use typed properties before utilities.
- Use only declared theme tokens.
- Label child-content buttons and meaningful images.
- Give repeated stateful components stable IDs.
- Run LSP formatting, then `cargo ice fmt` and `cargo ice check`.

# Spacing and typography

Start from the default Ice components and semantic recipes. The
[executable example](../../../examples/showcase/tests/cases/ui/design_metrics.ice)
uses the same workspace form in its view and standard/compact tests, including
Korean content and an actual edit-and-save route.

## Choose density through existing inputs

Page inset, space between sections, space within a field, and control padding
have different jobs. The example uses a 24px page inset and 16px between sections
normally, then 12px for both in its compact composition. Shared `section` and
`field` recipes retain 12px and 8px internal gaps. Readable body text stays at
13.5px; density does not mean shrinking every label.

Inherit semantic control recipes to change only repeated padding:

```ice
recipe compact_action for button extends secondary_action
  @px-12px py-8px

recipe compact_control for input extends control
  @px-12px py-8px
```

Use the input's existing `text-size=12.5` property with `@compact_control`.
Set `label="Workspace name"` on the input too: a Field's visible text label
does not automatically become its slotted input's accessible name.
Text-size utilities belong to supported text/button targets; the checker
rejects them on input recipes. These two controls keep at least 32 logical
pixels of height in the desktop example. That is the example's tested minimum,
not a universal touch-target guarantee. Validate the actual label line box,
vertical center and pointer route after changing size or padding. Prefer
natural height for labels that can wrap; a fixed small height cannot contain
arbitrarily long text.

The existing component inputs, recipe inheritance and typed text properties
cover this composition. A new global density token system is unnecessary.
Rust consumers can change the public `Theme::spacing`, `Theme::controls` and
`Theme::typography` fields; those theme fields do not rewrite an Ice source graph.

## Load the font before selecting its family

Font bytes belong to the application. A family declaration selects a face;
it does not load a font file. The test example explicitly loads:

```ice
app Workspace
  font "assets/IBMPlexSansKR-Regular.ttf"
  font "assets/IBMPlexSansKR-SemiBold.ttf"
  font "assets/IBMPlexSansKR-Bold.ttf"
  font "assets/GeistMono-Regular.ttf"

font body family="IBM Plex Sans KR" default=true
```

Paths are relative to the declaring Ice source; use the paths appropriate to
your app. The repository ships these assets and their licenses under
`assets/fonts`. IBM Plex Sans KR provides the Korean glyphs used in the example;
its regular and semibold faces cover body and heading roles. Load the required
script coverage rather than assuming a generic sans-serif fallback contains it.
The `machine` recipe selects the monospace family; load a monospace font too.

Rust defaults use generic `Font::DEFAULT` and `Font::MONOSPACE`. After loading
font bytes through the Iced application builder, bind named roles explicitly:

```rust
let theme = LIGHT.with_fonts(
    iced::Font::with_name("IBM Plex Sans KR"),
    iced::Font::with_name("Geist Mono"),
);
```

The named Rust font fields do not load bytes either. The showcase uses Geist;
a Korean app can choose IBM Plex Sans KR without copying component source.

## Keep role metrics consistent

Ice recipes and Rust `TextRole` use the same line-height multipliers:

| Roles | Multiplier |
| --- | --- |
| Display, screen title | 1.2 |
| Section title, pane header, list, nav label, badge | 1.35 |
| Body, caption, machine, meta, compact meta, field label | 1.5 |

Line height is the font size times this multiplier, not the height of the
visible ink. Different scripts and fonts have different ascenders and descenders.
Compare baselines for the same role and face; do not force a heading and caption
onto identical line boxes. The native role comparisons render two lines through
both APIs and check size, line height, total height, first-baseline offset,
font and semantic color.
The workspace checks longer wrapped content, explicit Korean family selection,
control label centering and interaction at both tested widths.

The comparison covers the bare `machine` recipe and Rust `TextRole::Machine`.
`Typography.Machine` additionally draws an accent chip with 7px/3px padding;
Rust `inline_code` is a separate compact muted-surface helper. Their wrapper
padding and background remain distinct; text-role parity does not imply every
convenience container has the same geometry.

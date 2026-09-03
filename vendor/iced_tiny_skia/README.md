# iced_tiny_skia 0.14.0, patched

The published `iced_tiny_skia 0.14.0` source with one upstream commit and a
handful of local changes. Every headless capture in this workspace renders
with this backend, so each of them reaches every `canvas`, every capture and
every `tiny-skia` window.

- **A canvas group's clip is translated once.** The release translated it by
  the group's own transformation twice, so geometry drawn at an offset was
  clipped to a rectangle displaced by that offset. Applied as
  [iced-rs/iced@76b32d4906](https://github.com/iced-rs/iced/commit/76b32d4906),
  which no 0.14 release carries.
- **A quad's shadow asks the clip mask its own question**, so a quad well
  inside its clip can no longer paint outside it through an offset or a wide
  blur.
- **A canvas text is recorded with the frame's rectangle, not an infinite
  one.** An infinite clip becomes NaN when the layer multiplies it by a
  transformation, and a NaN rectangle loses no comparison, so a changed label
  asked for the whole window to be repainted.
- **`mod layer` is public**, so a renderer that records instead of presenting
  can walk the items it produced.
- **A present hands the display only what changed** — nothing at all when the
  frame is already on screen.
- **A quad crossing the repainted region is filled over that region** rather
  than over its whole area — a rounded one too, when the part being filled
  holds none of the corners it curves inside of.
- **A glyph that rasterises to nothing is cached** instead of being hinted
  again on every draw.
- **A glyph decides for itself whether the clip mask applies to it**, instead
  of the whole text deciding, so a text clipped to a whole window no longer
  blits every glyph through the mask on a partial repaint.
- **A text damages the pixels its alignment puts it on**, and a canvas label
  damages its own line. The release measured every text rightwards and
  downwards from its position and gave a canvas text no bounded height, so a
  right-aligned or centred label left its old glyphs on screen.
- **The clip mask is filled when it is read, not when it is named.** Filling
  it clears the whole window whatever the rectangle is, and the draw loop
  names one far more often than anything reads one: once per layer per damaged
  region, most of which draw nothing there, twice more around every group of
  primitives, and once per clipped run of glyphs that then skips the mask for
  every glyph inside the rectangle anyway.

- **A quad's shadow is repainted when the quad changes.** The release
  measured the damage from the quad's bounds, so a card that lost its shadow
  kept the old one on screen.

- **A stroke is repainted over the width it was drawn with.** The release
  measured a changed primitive by the bounds of its path, so a horizontal
  rule — a rectangle of zero height — asked for no repaint at all.

- **The SVG cache looks a handle up before it builds a parser.** `usvg`'s
  options carry a `fontdb::Database`, and they were built ahead of the lookup,
  so every cache hit constructed one and dropped it to find the tree already
  there. Measuring an `svg` happens during layout, so an app with icons paid it
  once per icon per frame.

The six defects are pinned by tests in `crates/ui-lang-runtime/tests`
(`canvas_offset_clip.rs`, `shadow_layer_clip.rs`, `canvas_text_damage.rs`,
`text_anchor_damage.rs`, `shadow_damage.rs`, `stroke_damage.rs`).

## Found in the same sweep, deliberately not fixed

Four of those six are one defect wearing different clothes: the renderer draws
one rectangle and measures another. Sweeping the rest of `Layer::damage` turns
up a seventh instance, in `Text::Raw`, and it is left alone on purpose.

`Layer::draw_text_raw` stores the active transformation inside the item rather
than folding it into the position, and pushes it as `Item::Live`, whose
`transformation()` is the identity. `Text::visible_bounds` compensates for
exactly that in the `Paragraph` and `Editor` arms — it multiplies by the stored
transformation — and does not in the `Raw` arm, which returns `raw.clip_bounds`
untouched. `engine.rs` meanwhile draws the buffer at `raw.position` sized by the
buffer itself, under `transformation * local_transformation`. So the rectangle
measured is neither the rectangle drawn nor in the space it is drawn in.

It stays unfixed because nothing produces a `Text::Raw`. `fill_raw` is declared
on `iced_graphics`' text renderer trait and implemented here, and no widget in
`iced_widget`, no code in `iced_core`, and nothing in this workspace calls it —
it is an escape hatch for a custom text renderer. A fix could only be exercised
by a test that constructs the variant itself, which would pin the arithmetic
without demonstrating a repaint anyone can see. Upstream also marks the variant
unsound in the same area: `impl PartialEq for Raw` returns `false`
unconditionally, with a TODO saying raw buffers cannot be compared, so every
frame carrying one damages it regardless.

Reach for this entry the moment something calls `fill_raw`. The fix is to give
the `Raw` arm the same treatment the `Paragraph` arm already has, and the test
becomes writable at that point because a widget produces the input.

Wired in through `[patch.crates-io]` in the workspace `Cargo.toml` and in
`examples/app-store/Cargo.toml`. Delete this directory and both patch entries
once a release carries all of it.

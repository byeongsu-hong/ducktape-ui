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

The four defects are pinned by tests in `crates/ui-lang-runtime/tests`
(`canvas_offset_clip.rs`, `shadow_layer_clip.rs`, `canvas_text_damage.rs`,
`text_anchor_damage.rs`).

Wired in through `[patch.crates-io]` in the workspace `Cargo.toml` and in
`examples/app-store/Cargo.toml`. Delete this directory and both patch entries
once a release carries all of it.

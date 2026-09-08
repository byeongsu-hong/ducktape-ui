# iced_widget 0.14.2, patched

Published crates.io source, recorded by `.cargo_vcs_info.json` at upstream commit
`54abf81d13fec06d4d9ac754b03ce2c3313e8a1f` (`widget/` in
[iced-rs/iced](https://github.com/iced-rs/iced/tree/54abf81d13fec06d4d9ac754b03ce2c3313e8a1f/widget)).
The package omitted its license file; `LICENSE` is copied from that same
upstream commit's repository root.

Only `src/row.rs`, `src/column.rs` and `src/text/rich.rs` change upstream
behavior. Wrapping layouts record their actual line ranges, resolve their
assigned size before alignment, and align each line against the inner content
area. This fixes center/end alignment in filling or fixed-size layouts and with
leading padding. `rich_text` offsets its span decorations and its link hit test
by the anchored paragraph origin `text::draw` paints at, instead of the widget
bounds' corner, so an aligned label's highlights, borders, underlines and
strikethroughs land on its glyphs and its links follow where they are drawn;
the hit test is confined to that anchored area as well as to the widget, so
empty box is no longer the first span. The remaining source, generated manifest
and assets match the published package. See
`examples/showcase/tests/cases/ui/wrap_alignment.ice`,
`crates/ui-lang-runtime/tests/rich_text_alignment.rs` and the wrapping
alignment and rich text alignment sections in `COVERAGE.md` for executable
Red/Green evidence.

Both the root workspace and the separate `examples/app-store` workspace select
this crate through `[patch.crates-io]`. Cargo does not propagate workspace patch
entries through published Ice packages: an external application needs the same
`iced_widget` path patch to use this fix. Package compilation checks alone do
not establish patched rendering behavior. Remove the vendor copy and both
patch entries once the selected upstream release includes these fixes.

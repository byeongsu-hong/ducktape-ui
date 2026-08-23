# iced_tiny_skia 0.14.0, patched

The published `iced_tiny_skia 0.14.0` source with one upstream commit applied:
[iced-rs/iced@76b32d4906](https://github.com/iced-rs/iced/commit/76b32d4906)
— "Fix transformation of `canvas` primitives in `tiny_skia`". The release
translated a canvas group's clip rectangle by the group's own transformation
twice, so geometry drawn at an offset was clipped to a rectangle displaced by
that offset. Every headless capture in this workspace renders with this
backend, so the bug reached every `canvas` in every capture.

Wired in through `[patch.crates-io]` in the workspace `Cargo.toml`. Delete
this directory and that patch entry once a release carries the commit.

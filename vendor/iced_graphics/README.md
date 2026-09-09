# Local iced_graphics patch

This is the published `iced_graphics` 0.14.0 source, manifest and bundled fonts.
The MIT license and upstream repository are recorded in `Cargo.toml`.

`text::align` keeps the finite available shaping width for justified paragraphs,
then measures the aligned result. Previously it reduced that width to the
longest natural line before distributing spaces. Single lines, explicit hard
breaks with no soft wrapping, and unbounded text retain natural measurements.

The owner regression is `text::alignment_tests::justified_width_follows_expanded_lines`.
The native Ice regression is `examples/showcase/tests/wrapped_text_alignment.rs`;
it reads actual line ink and covers both plain and rich text.

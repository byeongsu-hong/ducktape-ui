# Tree SVG button ink

Ducktape buttons use direct memory SVG children with color=inherit. Preserve
the native button ink cell: the host button resolves its final status style
then writes text color before its child SVG draws. Only copied intent crosses
the wire; arbitrary SVG Rust style callbacks remain refused.

Add a boolean SVG inheritance field, emit it for checked color=inherit and
carry the nearest button ink through host render/responsive contexts. Allocate
a cell only for buttons whose own subtree needs one; nested buttons own theirs.
Use the existing ButtonInk implementation. Keep explicit/non-inheriting SVG
colors and ordinary glyphs unchanged.

Bundle an actual guest with a glyph button. Raster assertions must cover idle,
hover with cursor on button padding (not glyph), pressed and disabled colors,
and an explicitly tinted sibling. Prove the intended assertion fails without
ink propagation, restore, then run wire/host/workspace checks and review.
Rebase onto merged wrapping/tooltip support before the final real-node probe.
Do not edit Ducktape source.

Implementation and actual wasm raster assertions pass. Black-ink mutation
fails the intended pixel assertion, restoration passes all six mounted widget
tests. Scoped the existing ring oracle to its own button after the new red
SVG exposed its whole-frame pixel count. Core Tree and wire tests pass; root
clippy is clean. Independent final diff review has no remaining findings.

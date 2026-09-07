# Tree text and kit presentation

Priority raised by the actual Ducktape port: the shared node kit cannot build with current Tree text refusals. Work
only in Ice. Read the downstream six-file source graph without modifying it:
app/src/ui/screens/node.ice, components/node.ice, components/kit.ice,
components/patterns.ice, theme.ice and ducktape-ui/log-timeline.ice.

Support copied text wrapping/shaping, named family and face metadata, relative or
absolute line height, dimensions and vertical alignment, tracking and the kit's
bounded container max-width/clipping/padding. Reuse native text/grapheme widgets;
preserve native selection, accessibility and per-widget bounds. The actual kit
uses Geist/Geist Mono families, medium/semibold faces, wrap=none/word-or-glyph,
line-h around 1.2–1.6, and box max-w 620/640. Do not substitute monospace or flatten
the component graph to claim named-font support. Host font loading remains host
owned; copy family/face declarations and resolve them to host-native fonts with
bounded guest input and no guest-controlled permanent string leaks.

Implementation must include wire bounds/patching, Tree emitters, host rendering,
real wasm fixture and native layout/paint/interaction evidence. Check copied face metadata and compare native wrapping, height, padding, maximum
width and clipped raster bounds,
and demonstrate assertion Red/Green. Keep unknown native callback styles refused.
Run the downstream node/kit source validation with Claude before merge; the six
fragments need a proper typed app root, not six standalone compile calls.

Current status: implementation and actual wasm bundle pass. Imported E190
origin regression reproduced (line 1 instead of 2), then passed after avoiding
second remap. Actual downstream typed root now first refuses button checked/
expanded at screens/node.ice:40. Claude confirms button accessibility metadata,
recipes, tooltip and an opaque activity-log surface slot are followup needs.
Native wasm layout, clipping and interaction tests pass; the height mutation
fails its intended assertion (24 versus 44) and passes after restoration. Core
tests, wire unit/hostile tests, workspace check and both workspace/host clippy
pass. Independent review found finite tracking overflow; it is fixed and covered
by a regression. PR CI is the remaining delivery gate.
The keyed worktree and its real reorder failure remain intact.

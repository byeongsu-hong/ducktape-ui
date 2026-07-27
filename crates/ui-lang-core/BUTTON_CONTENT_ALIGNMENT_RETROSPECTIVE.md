# Fixed-size button content alignment retrospective

Date: 2026-07-27

Status: root cause fixed in the shared Ice button code generator; repository-wide audit remains a follow-up task.

## Incident

The Apple Music example used a 36 × 36 px play/pause button with 8 px padding and a 14 × 14 px SVG child. The circle was correctly positioned, but the SVG inside it appeared above and to the left of center.

This was a `ui-lang` code-generation bug, not an Apple Music layout or SVG-authoring bug. Fixed-size buttons containing a string label and fixed-size buttons containing a child node followed different layout paths:

- A generated string label was wrapped in a fill-sized container and centered on each fixed axis.
- A custom child such as `svg`, `row`, or a component was passed directly to `iced::widget::button`.

Iced's button uses `layout::padded`, which places child content at the padding origin; it does not center a smaller child in the remaining space. In the reported case, padding left a 20 × 20 px content area around a 14 × 14 px SVG. The unused 6 px stayed entirely on the trailing edges, shifting the SVG center 3 px up and 3 px left.

## Root cause and fix

`crates/ui-lang-core/src/codegen/view/controls.rs` treated centering as label-specific behavior. The fix makes it button-content behavior: regardless of whether content came from a label or child node, it is wrapped in a fill-sized container and centered on every axis where the button has a fixed length.

The behavior for `fill` and intrinsic/shrink axes is unchanged. The fix adds no syntax or runtime abstraction.

## What went wrong during diagnosis

The first attempted response removed the emphasized pink play/pause background and changed icon sizing. That altered the design without addressing the user's stated problem. Those edits were reverted once the complaint was clarified as internal content alignment.

The better diagnostic sequence was:

1. Preserve the reported design and reproduce both play and pause states.
2. Compare the button bounds, content viewport, and painted glyph.
3. Trace button code generation into Iced's native layout implementation.
4. Do the padding arithmetic before changing styles or SVG paths.

## Why existing tests missed it

- The code-generation test covered centering for a fixed-size string-label button, but not a fixed-size button with a child node.
- Its custom-child button used `w=fill`; assertions checked generation and styling, not the child's position on the fixed height axis.
- The Apple Music semantic test compared component bounds with button bounds. Component or semantic bounds can represent an allocated slot rather than the painted SVG viewport, so equal reported centers did not prove pixel alignment.
- There was no rendered regression check for the play state; the default screenshot showed pause only.

The fix adds a focused code-generation assertion for custom content in a fixed-size button. Follow-up work should decide whether the semantic test API needs a way to distinguish allocated, layout, and painted bounds.

## Repository-wide audit handoff

Audit all controls where generated label content and explicit child content can take different layout paths. Start with button-like controls, then expand only when the same pattern is present.

### Search targets

- Every `ViewNode` code-generation branch that accepts either a label or child node.
- Every use of fixed width or height combined with padding and custom content.
- Every wrapper built only for synthesized text while explicit children bypass it.
- Native Iced widgets whose layout primitive positions at the padding origin instead of centering.
- Semantic geometry assertions that observe allocated component slots instead of leaf layout or painted bounds.

Useful starting commands:

```sh
git grep -n "if let Some(content)" crates/ui-lang-core/src/codegen
git grep -nE "button .*w=[0-9]|button .*h=[0-9]" -- '*.ice'
git grep -nE "center_x|center_y|align_x|align_y" crates/ui-lang-core/src/codegen
git grep -nE "center_x.*button|center_y.*button" -- '*.ice' '*.rs'
```

### Required behavior matrix

For each affected control, cover the smallest matrix that exercises the divergent paths:

| Content | Axis length | Expected behavior |
| --- | --- | --- |
| String label | fixed | centered within the padded content area |
| Child node | fixed | same centering as the string label |
| String label | fill or intrinsic | existing behavior preserved |
| Child node | fill or intrinsic | existing behavior preserved |

Exercise at least `text`, `svg`, a layout node (`row` or `col`), and a nested component. Test width and height independently so a fix does not accidentally center a fill axis.

### Completion criteria

- No label/custom-child split produces different alignment for the same declared dimensions.
- Focused code-generation tests cover each shared lowering rule.
- At least one runtime or rendered check proves the leaf content position, not only its component slot.
- Existing fill and intrinsic sizing behavior remains unchanged.
- The audit reports unaffected branches as checked; it does not add speculative abstractions or new syntax.

### Copy-ready follow-up prompt

> Use this retrospective to audit the full repository for label-versus-custom-child layout divergence. Trace each candidate through generated Rust into the native Iced layout primitive. Add only focused regressions for confirmed shared bugs, preserve fill/intrinsic behavior, render representative fixed-size controls, and deliver through a worktree, PR, review, and merge according to `AGENTS.md`.

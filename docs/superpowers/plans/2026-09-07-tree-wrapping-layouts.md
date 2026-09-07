# Tree wrapping layouts

The actual Ducktape typed node root, after button recipe support, first fails
at components/node.ice:708 (`row wrap`, gap 8, wrap-gap 3, align center).
Use the native row/column wrapping widgets; the guest sends copied layout data
and children and never measures or paints pixels.

Extend Linear with optional wrapping settings (inter-line spacing and line
alignment). Absence preserves ordinary linear layout; presence selects native
wrap, even when both settings are omitted. Forward existing main-axis spacing,
padding, dimensions and child alignment before wrapping, then wrap-gap and
wrap-align with the same axis mapping as native codegen. Bound spacing using
the existing sanitizer and child-count-aware host helper. Keep other unsupported
properties explicitly refused.

Add native layout assertions for rows and columns at narrow and wide available
sizes, spacing/alignment, and child routes through actual bundled wasm. Prove
the wrap assertion fails when wrapping is temporarily disabled, restore and
rerun. Update hostile frame generation and all direct Linear constructors.
Rebase onto the merged recipe rev before probing the real node root again;
run workspace check with tests before completing any conflicted rebase.
Deliver a focused PR after checks and review; no Ducktape source edits.

Implemented on 72a90af4 while #962 CI runs. Workspace check with tests passes.
Core 992 unit tests plus integration tests, wire 47 unit/6 hostile tests and
both actual text wasm host tests pass. Disabling host wrapping fails the native
72px assertion with 20px; restoration passes. Independent review found no
actionable issue. Workspace/host clippy and Rust/Ice formatting pass.

Rebased onto recipe main c7145f2e; workspace check with tests passed before
rebase continuation. The combined real node root passes wrapping and next
refuses Icon's Rust `icon_tint(tone)` SVG style callback at components/icon.ice:7.
This remains an intentional refusal; declarative downstream colors and host
SVG button-ink inheritance are separate requirements.

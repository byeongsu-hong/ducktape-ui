# Tree combo implementation

Approved scope: real Iced combo search/edit/filter/menu/keyboard, typed guest selection/hover routes, input/open/close routes, declarative input/menu style and metrics. Rust style callbacks remain explicit E190. No copied native widget implementation or Pick substitution.

1. Lifetime spike in runtime view_tree/combo.rs: own shared native State; delegate Widget and owned Overlay calls without unsafe. Real UI input/filter/selection oracle; counterexample drops overlay or rebuilds State.
2. Wire Combo node and finite/bounded sanitization. Guest Tree combo state tracks reset revision for assignments, preserving query for push. Typed option index slots preserve original values and duplicate labels.
3. Inputs adoption keyed by App-owned binding identity (widget focus remains separate); unchanged revision/options retain native State, append updates preserve search, reset revision or incompatible options reset. Reload transfers retained state only when identity/revision/options match. Hidden App bindings retain search across hide/readd and exact matching reload; cumulative identities/options/query storage is bounded with explicit rejection. Old generation events remain rejected.
4. Core emitter/typed state integration, declarative styles and metrics, explicit unsupported Rust style and unowned parameter diagnostics. Component Combo state retains the common native E103 restriction. Focused Core representation and typed native/wasm builds.
5. Actual native child and Wasm fixture input/filter/keyboard/menu selection, set/push/resync/reload/removal/stale overlay tests; minimal behavioral mutations Red then exact restoration Green.
6. Rust/Ice checks, full relevant tests, strict app Clippy, README/SPEC/COVERAGE and CI. Rebase after root coordinates sensor/allocation merges, commit/push focused main PR; root merges.

# Keep native layout memoization out of Tree components

Use the Ducktape port's supplied component-in-state-loop reproduction. Disable
native memo selection before deriving scope bindings, retaining native target
memoization. Prove the no-wrapper regression assertion fails before the fix,
then build the exact imported-palette fixture with cargo ice bundle for wasm.
Run core tests, workspace checks, format and lint; review and deliver a focused
PR. Report the merged revision to Claude after resources and this fix land.

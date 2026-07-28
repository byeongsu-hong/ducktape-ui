# Visual conformance

The approved React design system is the source of truth for Ducktape visuals.
Its design QA remains in that project; this directory only records its rendered
contract and compares Ice against it.

Refresh the committed web contract after an approved design-system change:

```bash
cd conformance
npm ci
npx playwright install chromium
DUCKTAPE_DESIGN_SYSTEM=/path/to/ducktape-design-system npm run capture
```

Run the Ice comparison:

```bash
cargo test -p ice-ui-conformance
```

The first suite covers the visual roles currently present in the React
reference: typography, button states, inputs, and cards. `SHADCN_LIGHT` and
`SHADCN_DARK` are separate theme-swap profiles; they are not golden references
for the Ducktape design.

# Ducktape Design System equivalence

Reference: `~/dev/ducktape-design-system/src`.

Status on 2026-07-26: the Ice showcase covers the complete reference catalog.

| Reference layer | Reference count | Ice home | Status |
| --- | ---: | --- | --- |
| Catalog root | 1 | `examples/showcase/src/ui/showcase.ice` | Equivalent |
| Chrome | 4 | Parameterized in `crates/ui/src/ice/components.ice` | 4 / 4 |
| Sections | 12 | Concrete composition in `examples/showcase/src/ui/components.ice` | 12 / 12 |
| Blocks | 48 | Concrete composition in `examples/showcase/src/ui/components.ice` | 48 / 48 |
| Primitives | 22 | Parameterized in `crates/ui/src/ice/components.ice` | 22 / 22 |
| Icons | 21 | Parameterized SVG components in `crates/ui/src/ice/components.ice` | 21 / 21 |

Equivalence means the same visible hierarchy, copy, tokens, typography,
geometry, icon paths, represented states, and component boundaries. All 12
sections and all 48 blocks keep their reference component names in the
showcase. Compound Ice names replace reference prop variants for `StateCard`,
`StatusPill`, `ButtonStateSwatch`, `LogLine`, and `LangTag`; icon components use
the `Icon.*` namespace.

## Ownership rule

`crates/ui` owns only reusable variables, recipes, components, icon paths, and
retained control behavior. `examples/showcase` owns catalog copy, demo state,
sample people, repositories, channels, endpoints, hashes, and code snippets.
Moving concrete examples into the package is a boundary violation.

## Verification

```bash
cargo ice fmt --check
cargo fmt --all -- --check
cargo check -p showcase
cargo test -p showcase
cargo test -p ducktape-ui --all-features
```

The scrolled showcase snapshots are the visual regression check. The UI crate
tests cover retained semantic and accessibility contracts; the showcase tests
enforce the source boundary.

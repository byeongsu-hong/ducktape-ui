# Feature evidence contracts

Every feature proposal names its primary semantic owner under decision 0002
before implementation and identifies any adjacent integration layers. A
feature or epic is complete only when it supplies the applicable evidence for
its owner and integrations and updates `SPEC.md`, `README.md`, and
`COVERAGE.md` wherever their public claims change. Evidence may be split across
stacked pull requests; each pull request states which rows it supplies, and the
epic cannot be declared complete while a required row is missing.

## Ice Core

| Evidence | Required result |
| --- | --- |
| Syntax | one canonical form |
| Parser | accepted and malformed fixtures |
| Formatter | idempotent canonical output |
| Checker | type, scope, capability, lifecycle, and invalid cases |
| Normalized HIR | all sugar and semantic choices resolved |
| Backend | native lowering with no checker decision repeated |
| Schema/LSP | completion, hover, actions, and signature behavior as applicable |
| Source map | root and imported origins survive lowering and generated diagnostics |
| Runtime | a real generated program executes the behavior |
| Documentation | specification and readable application example |
| Performance | new complexity stays inside an explicit measured budget |

## Runtime widget

| Evidence | Required result |
| --- | --- |
| Typed API | no unchecked dynamic payload or hidden capability |
| Lifecycle | state ownership, reconciliation, stable identity, and removal rules |
| Interaction | applicable mouse, keyboard, focus, and scrolling behavior; unsupported inputs are explicit |
| Unicode/IME | required for every text-editing surface |
| Accessibility | native semantics or an explicit supported-limit statement |
| Headless | inspectable geometry and semantics |
| Native renderer | WGPU first draw or renderer-specific smoke |
| Performance | realistic large fixture and budget |

## `ducktape-ui` component

| Evidence | Required result |
| --- | --- |
| Contract | applicable typed props, events, slots, output, and lifetime are explicit |
| Visual state | active, hover, pressed, disabled, and focus where applicable |
| Accessibility | name, role, actions, and keyboard behavior |
| Theme | application font and semantic token inheritance |
| Conformance | comparison with the intended reference contract |
| Responsive | wide and narrow evidence |
| Example | showcase consumes the public interface, not private helpers |

## Product-local Rust boundary

Product-local work still requires a typed interface, lifecycle ownership,
invalid/error cases, and product tests. It does not need Core parser, formatter,
HIR, schema, or generic component evidence unless it changes those surfaces.

Reviewers reject claims based only on generated Rust string snapshots, a
workspace-internal build, or a visual screenshot when the relevant contract
also requires semantics, source mapping, packaging, or performance evidence.

## Delivery roles

Each epic separates three responsibilities, even when one person coordinates
the work:

| Role | Responsibility |
| --- | --- |
| Design | choose the owning layer; define the typed contract, invalid cases, lifecycle, and performance budget |
| Implementation | deliver the smallest complete vertical slice and remove the superseded path |
| Adversarial review | independently probe edge cases, source maps, property/performance bounds, packaged consumers, and platform behavior |

Task changes live in dedicated worktrees and focused branches. The implementer
does not provide the final adversarial approval for the same change; every
actionable finding is resolved and the affected evidence is rerun before merge.

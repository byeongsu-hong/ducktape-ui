# Ice tooling and live LSP

Ice ships executable Cargo tooling and a live stdio language server. Use them
while editing; do not treat the LSP as a planned feature or replace it with
syntax guesses.

## Contents

- [Find the command](#find-the-command)
- [Command reference](#command-reference)
- [Use the live LSP](#use-the-live-lsp)
- [LSP capabilities](#lsp-capabilities)
- [Open-buffer and workspace behavior](#open-buffer-and-workspace-behavior)
- [Rename rules](#rename-rules)
- [Use the generated schema](#use-the-generated-schema)
- [Diagnose generated Rust](#diagnose-generated-rust)
- [Verification recipes](#verification-recipes)
- [Troubleshooting](#troubleshooting)

## Find the command

In this repository, `.cargo/config.toml` declares:

```toml
[alias]
ice = "run -p cargo-ice --"
```

Run `cargo ice ...` from the workspace root. The alias builds and executes the
workspace's `cargo-ice` binary. In a consuming repository, first inspect its
`.cargo/config.toml` or documented installation; do not assume an unrelated
global `cargo-ice` binary.

Confirm the command surface:

```bash
cargo ice help
```

Expected form:

```text
cargo ice <fmt [--check] | check | test | clippy | compat | expand <file.ice> | dev <-p package | file.ice [-- cargo-build-args...]> [-- app-args...] | inspect <file.ice> [options] | diff <baseline.json> <current.json> [options] | schema | lsp>
```

## Command reference

| Command | Use |
| --- | --- |
| `cargo ice fmt` | format Rust and every discovered `.ice` file, then analyze app graphs |
| `cargo ice fmt --check` | check Rust and Ice formatting without rewriting `.ice` |
| `cargo ice check` | analyze app graphs, then run workspace `cargo check` |
| `cargo ice test` | analyze app graphs, then run `cargo test --workspace` including generated Ice tests |
| `cargo ice clippy` | analyze app graphs, then lint all workspace targets |
| `cargo ice compat` | verify backend/runtime pins and run reference app tests |
| `cargo ice expand FILE` | print generated Rust for one app root |
| `cargo ice inspect FILE [options]` | render the actual app headlessly and write a PNG plus structured JSON manifest |
| `cargo ice diff BASE.json CURRENT.json [options]` | compare manifests and pixels, write `report.json` plus `diff.png`, and fail on unexplained deltas |
| `cargo ice schema` | print generative Core/LSP/backend JSON |
| `cargo ice lsp` | run the live stdio language server, including the `Run Ice lint` source action |

Discovery recursively scans below the current directory, skipping `.git`,
worktree metadata, `target`, and `tests/cases` fixture trees. Files with a
top-level `app` or `daemon` are roots. Formatting covers both roots and imported
fragments.

`cargo ice check` gives two layers of evidence:

1. parser/checker diagnostics with `.ice` source paths, lines, columns, and
   hints;
2. rustc diagnostics for extern paths/signatures, generated types, selected
   renderer, and Cargo features.

`cargo ice inspect` accepts a root `.ice` file and deterministic inputs:

```bash
cargo ice inspect path/to/app.ice \
  --viewport 1440x900 \
  --theme light \
  --system-theme light \
  --preset populated \
  --scale 1 \
  --locale en-US \
  --platform linux \
  --reduced-motion \
  --name populated_light
```

Use `--output DIR` to control the artifact directory and `--package NAME` when
the root is included from outside its containing Cargo package. The JSON
records viewport and environment, target geometry, visible/content bounds,
scroll state, structured tiny-skia paint, rendered text/font/baseline data,
accessibility, and the originating `.ice` path/line/column for identified
nodes. The command runs the generated app `Program`; it does not substitute a
separate mock view.

`inspect` captures the initial or selected-preset state. When the required
state is reached through interaction, use a first-class Ice test to perform
semantic actions and `capture` after them, then pass that manifest to `diff`.

Compare two captures with:

```bash
cargo ice diff baseline.json current.json \
  --pixel-threshold 0 \
  --max-changed-ratio 0 \
  --value-tolerance 0 \
  --output target/ice-diff/example
```

The command exits unsuccessfully when structured fields differ or the changed
pixel ratio exceeds the allowed maximum. Read both `report.json` and
`diff.png`; tolerance is an explicit review decision, not a way to hide an
unknown change.

## Use the live LSP

Configure any LSP client with:

```text
language/file type: ice
file pattern:        *.ice
transport:           stdio
command:             cargo
arguments:           ice, lsp
working directory:   Cargo workspace root
workspace folder:    Cargo workspace root
```

The equivalent process command is:

```bash
cargo ice lsp
```

The server advertises `ice.lint` through `workspace/executeCommand`. Invoking
the corresponding `Run Ice lint` source action runs workspace Clippy and
publishes error-level generated Rust findings at their mapped `.ice` URI and
range. Warning-level backend findings remain available through `cargo ice
clippy` without flooding editor diagnostics.

It intentionally waits for Content-Length-framed JSON-RPC on stdin. A quiet,
long-running process means the server is ready; it is not a hung checker.
Launch it through an LSP client rather than typing into its terminal.

Minimal client pseudoconfiguration:

```json
{
  "languageId": "ice",
  "extensions": [".ice"],
  "command": "cargo",
  "args": ["ice", "lsp"],
  "cwd": "<workspace-root>",
  "transport": "stdio"
}
```

If an editor requests a single executable plus arguments, keep `cargo` as the
executable. Do not configure `cargo ice lsp` as a literal executable name.

## LSP capabilities

The server advertises:

- full-document text synchronization;
- UTF-16 positions;
- publish diagnostics;
- whole-document formatting;
- schema-driven Core and test-mode completion;
- cross-file definition;
- prepare-rename and workspace rename.

The exact initialization capability object includes:

```json
{
  "positionEncoding": "utf-16",
  "textDocumentSync": {
    "openClose": true,
    "change": 1
  },
  "documentFormattingProvider": true,
  "completionProvider": {
    "resolveProvider": false
  },
  "definitionProvider": true,
  "renameProvider": {
    "prepareProvider": true
  }
}
```

Formatting calls the same `ui_lang_core::format_fragment` implementation as
`cargo ice fmt`. Completion items come from the generated Core construct table.
Handler completion offers `run every`, `run latest lane=...`,
`run replace lane=...`, `stream every`, `stream replace lane=...`, and
`invalidate lane=...` snippets. Extern-aware completion safely defaults a
selected stream function to `stream replace lane=<qualified-function-name>`;
`stream every` remains an intentional generic-snippet opt-in. It never proposes
bare `stream` or `stream latest`. The error-route quick fix recognizes all
canonical routed Future/stream modes and does not treat lane invalidation,
subscription `run`, or task-flow sources as routed handler statements.
Current completion is intentionally vocabulary-wide rather than
cursor-context-aware; let diagnostics reject a construct in the wrong context.

## Open-buffer and workspace behavior

For an existing file URI, the server analyzes the complete import graph with
every open buffer overlaid on disk. This makes unsaved edits participate in
real checking.

Use it correctly:

1. Open the app/daemon root as well as the imported fragment being edited.
2. Initialize the client with the Cargo workspace folder.
3. Save or keep buffers open normally; opening, changing, or closing any buffer
   reanalyzes all open app roots.
4. Read an imported error at the imported file URI. The root owns the report,
   but reports are aggregated by the actual diagnostic URI.
5. When a fragment buffer closes, expect analysis to fall back to its disk
   contents.

An isolated imported fragment is not a complete app and is not analyzed as a
standalone root. If diagnostics appear absent, open its importing app root.

The server uses:

- current buffers for open roots and imports;
- disk for closed imports;
- every closed app root under the initialized workspace when checking
  workspace-wide navigation/rename safety.

## Rename rules

Definition and rename operate on checked component, app-handler, semantic
recipe, and test-target declarations and references.

Supported:

- cross-file go-to-definition for components and app handlers;
- collision-checked rename for plain component names;
- collision-checked rename for app handlers;
- definition and collision-checked rename for recipes;
- definition and rename for a target alias within its one test scope;
- compound-family root rename that updates dotted descendants.

Definition-only:

- direct dotted component descendants such as `Dialog.Header`;
- implicit `mount`;
- component-local handlers, which are lexical implementation details.

Rename proceeds only when:

- the new name is a valid identifier;
- it does not collide with a declaration of the same kind;
- every reference has an exact retained source span;
- every app root under the initialized workspace checks;
- imported-symbol rename has an initialized workspace root.

If rename is refused, fix all workspace diagnostics first. Do not bypass a
collision by text replacement.

## Use the generated schema

`cargo ice schema` is the quickest source of exact Core facts:

```bash
cargo ice schema
```

It reports:

- language name, revision, encoding, extension, and indentation;
- required document prelude and theme tokens;
- Core constructs with valid contexts, canonical syntax, child cardinality,
  typed properties, binding, and route shape;
- utility ownership and status inheritance;
- first-class test configuration, actions, assertions, target fields, and
  runtime/paint contract;
- LSP behavior;
- Iced/runtime compatibility contract.

Useful read-only queries when `jq` is available:

```bash
cargo ice schema | jq '.language'
cargo ice schema | jq '.lsp'
cargo ice schema | jq '.core.documentPrelude'
cargo ice schema | jq '.core.types'
cargo ice schema | jq '.core.style'
cargo ice schema | jq '.core.testMode'
cargo ice schema | jq '.core.constructs[] | {label, contexts, syntax, children, properties, binding, route}'
```

Treat schema output as authoritative for Core. Treat `SPEC.md` as authoritative
for the complete implemented language, and `COVERAGE.md` as the exact Iced
surface ledger.

## Diagnose generated Rust

Use:

```bash
cargo ice expand examples/iced-app/src/ui/tasks.ice
```

Expansion is for:

- seeing how a construct lowers;
- locating the generated type behind a rustc error;
- comparing output in compiler fixtures;
- diagnosing renderer or lifetime boundaries.

Do not:

- edit or commit the expanded output;
- copy generated message/state internals into hand-written code;
- use expansion instead of fixing the `.ice` source or extern signature.

The `ui-lang-build` build script emits source dependency tracking; generated
Rust contains the extern probes, and the proc macro only includes that output
from Cargo's `OUT_DIR`.

## Verification recipes

### Edit an application

```bash
cargo ice fmt --check
cargo ice check
cargo ice test
cargo test -p iced-app <focused-test-filter>
```

Use `cargo ice fmt` before `--check` when formatting changes are expected.

### Verify a visual change

```bash
cargo ice inspect path/to/app.ice --viewport 1440x900 --theme light --name current
cargo ice diff path/to/baseline/current.json target/ice-inspect/path_to_app/current.json
```

Open the emitted PNG. Read the JSON for exact layout, paint, text, a11y, and
source provenance. Repeat with the same input tuple after each correction and
cover every materially different preset or responsive breakpoint.

### Edit the language parser/checker/code generator

```bash
cargo test -p ui-lang-core
cargo ice check
```

Add one auto-discovered fixture or one nearby unit test for non-trivial new
logic.

### Edit the LSP

```bash
cargo test -p cargo-ice
cargo ice check
```

The `cargo-ice` tests exercise JSON-RPC lifecycle, UTF-16 ranges, open-buffer
overlays, imported diagnostics, formatting, completion, definition, and rename.

### Edit compatibility or accessibility integration

```bash
cargo ice compat
scripts/a11y-smoke.sh
scripts/a11y-windows-check.sh
```

Run platform gates only when the change touches their contract and the host has
the required environment/toolchain.

## Troubleshooting

### `cargo: no such command: ice`

Run from the repository/workspace root and inspect `.cargo/config.toml`. The
public repository defines a local alias; another workspace must provide or
install its own command path.

### LSP starts but shows no fragment diagnostics

Open the importing app/daemon root. Fragments are checked in a root graph, not
as standalone programs.

### Cross-file rename is unavailable

Initialize the workspace folder, open the relevant root/import buffers, and
make every workspace app root pass checking.

### Completion suggests an invalid construct

Current completion is schema-derived but not context-aware. Use the completion
as vocabulary and let live diagnostics identify invalid placement.

### Rustc reports a missing extern item

Compare the `.ice` namespace/name/type declaration with the public Rust path
and signature. Use `cargo ice expand` only if the generated probe is unclear.

### The formatter changed indentation

Accept the canonical two-space tree, then verify that parent/child meaning is
still intended. `cargo ice fmt` does not migrate removed vocabulary.

### `cargo ice inspect` reports no matching generated test

Confirm that the requested file contains the top-level `app` or `daemon` and is
included by `ui_lang::include_app!` in the selected package. For an external
include, pass the owning Cargo package with `--package`.

### An editor assumes JavaScript, JSX, or CSS for `.ice`

Override the file association to the `ice` language ID and attach only the Ice
server. Disable unrelated JSX/CSS formatting for `*.ice`.

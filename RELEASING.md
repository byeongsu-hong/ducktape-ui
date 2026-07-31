# Releasing Ducktape UI and Ice crates

The Ice language revision and Cargo package version are separate contracts.
Ice remains a 2.0 Preview candidate while every publishable crate currently
uses the pre-1.0 package version `0.1.0`.

## Package contract

All manifests below `crates/` release in lockstep. Internal Cargo dependencies
use an exact version requirement and a workspace path:

```toml
ui-lang-core = { path = "crates/ui-lang-core", version = "=0.1.0" }
```

The path keeps repository development local. Cargo removes it from a published
manifest, leaving the exact registry version. Generated Rust is an internal
interface between the same-version `ui-lang-build`, `ui-lang`, and
`ui-lang-runtime` crates; it is not a cross-version ABI.

Before 1.0, removed behavior is removed outright and all workspace callers are
updated in the same change. Releases do not carry deprecated syntax,
compatibility shims, migrations, or fallback implementations.

## Package evidence

Run the normal workspace checks, then:

```bash
scripts/package-smoke.sh
```

The smoke script creates the exact `.crate` archives, extracts them outside the
workspace, patches registry resolution only to those extracted archives, and
checks every package with all features. This catches missing package files,
workspace-only manifest inheritance, absent internal versions, and path
dependencies that would disappear after publication.

For local validation of uncommitted release changes only:

```bash
ICE_PACKAGE_ALLOW_DIRTY=1 scripts/package-smoke.sh
```

The default remains strict so the release command refuses a dirty checkout.

## Registry order

The first crates.io publication must respect this dependency graph:

1. `ui-lang-core`, `ui-lang-runtime`, and `ducktape-ui`
2. `ui-lang-build`
3. `ui-lang` and `cargo-ice`

Wait until each preceding version is visible in the registry index before
publishing its dependents. A release tag is `v<package-version>` and is created
only after every crate at that version is available. The tag workflow validates
that all `crates/*/Cargo.toml` versions match, builds the supported `cargo-ice`
binaries, attests the archives, and publishes the GitHub release.

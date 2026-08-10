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
The downstream smoke then builds the extracted `cargo-ice` package and uses
that binary to emit the same declaration-only fingerprint twice, verify a zero
JSON diff, reject a named event added to an existing component, and reject a
hash-corrupt artifact. This proves the public API workflow from packaged crates
rather than from workspace paths.

The default component library also keeps a reviewed semantic API baseline at
`api/baselines/ducktape-ui.json`. Before release, regenerate and compare it:

```bash
cargo ice api crates/ui/src/ice/default.ice > target/ducktape-ui-api.json
cargo ice api diff api/baselines/ducktape-ui.json target/ducktape-ui-api.json
```

Breaking changes exit nonzero. Update the committed baseline only when the
corresponding breaking/additive/behavioral report is intentional and reviewed;
formatting and file relocation alone do not change its SHA-256 fingerprint.
Pull requests always compare against the target branch's baseline, so updating
the baseline in the same change cannot hide a breaking diff. The committed
baseline must also match the command above byte-for-byte. After reviewing both
the semantic diff and regenerated baseline, a maintainer explicitly accepts an
intentional breaking change by applying the `api-breaking-approved` label. Label
addition and removal rerun the gate. The approval applies only to the current
head: any later push makes the synchronize run fail until a maintainer removes
and reapplies the label. Ordinary contributors and fork pull requests receive
no write token or baseline override. Retargeting a pull request also reruns the
gate against the new target commit and requires a fresh breaking approval. The
approval label records review of a pull-request change; it does not bypass
release evidence. The tag workflow regenerates the committed artifact again,
requires byte equality, and runs the canonical reader through an exact zero
JSON diff before publishing. Because the package version participates in the
fingerprint payload, an intentional release version bump must regenerate and
review the baseline too.

## macOS application

A tag also publishes the showcase application as a signed, notarized disk
image. The `Showcase disk image` job builds `aarch64-apple-darwin` and
`x86_64-apple-darwin`, joins them into one universal binary, and runs

```bash
cargo ice bundle -p showcase \
  --target aarch64-apple-darwin --target x86_64-apple-darwin
```

The job verifies the result before it is uploaded: both architectures are
present, the icon and bundle identifier are in place, and `codesign --verify
--strict` passes. When notarization ran, `stapler validate` and `spctl
--assess` confirm the ticket is stapled, which is what lets a first launch
succeed on a machine that cannot reach Apple. The disk image and its SHA-256
join the attested release assets.

Signing is driven entirely by repository secrets, so the job is runnable before
they exist — an unset secret arrives as an empty string, and the bundle is then
signed ad hoc and never sent to Apple. Set all six to publish a distributable
build:

| Secret | Contents |
| --- | --- |
| `MACOS_CERTIFICATE` | base64 of the Developer ID Application `.p12` |
| `MACOS_CERTIFICATE_PASSWORD` | that `.p12`'s export password |
| `MACOS_SIGNING_IDENTITY` | `Developer ID Application: NAME (TEAMID)` |
| `MACOS_NOTARY_KEY` | base64 of the App Store Connect API `.p8` |
| `MACOS_NOTARY_KEY_ID` | that key's ID |
| `MACOS_NOTARY_ISSUER` | the issuer UUID for that key |

Certificates expire. A signature made before expiry stays valid because it
carries a trusted timestamp, but the next release fails at `codesign` — renew
the Developer ID certificate and replace the first three secrets.

`docs/tooling.md` documents the command itself, including how another package
declares its own bundle.

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

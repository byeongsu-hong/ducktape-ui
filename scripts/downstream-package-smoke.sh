#!/usr/bin/env bash
set -euo pipefail

package_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
fixture_root="$package_root/tests/downstream-app"
package_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$package_root/crates/ui-lang-core/Cargo.toml")
packages=(
  ui-lang-core
  ui-lang-runtime
  ducktape-ui
  ui-lang-build
  ui-lang
  cargo-ice
)
consumer_packages=(
  ui-lang-core
  ui-lang-runtime
  ducktape-ui
  ui-lang-build
  ui-lang
)

downstream_scratch=$(mktemp -d -t ducktape-downstream.XXXXXX)
cleanup() {
  if [[ -n "$downstream_scratch" && -d "$downstream_scratch" ]]; then
    rm -rf -- "$downstream_scratch"
  fi
}
trap cleanup EXIT

consumer="$downstream_scratch/app"
package_sources="$downstream_scratch/packages"
target_dir="$downstream_scratch/target"
mkdir -p "$consumer" "$package_sources"
cp -R "$fixture_root/." "$consumer/"

for package in "${packages[@]}"; do
  archive="$package_root/target/package/$package-$package_version.crate"
  if [[ ! -f "$archive" ]]; then
    echo "missing $archive; run scripts/package-smoke.sh first" >&2
    exit 1
  fi
  tar -xzf "$archive" -C "$package_sources"
  mv "$package_sources/$package-$package_version" "$package_sources/$package"
done

mkdir -p "$consumer/.cargo"
{
  echo '[patch.crates-io]'
  for package in "${consumer_packages[@]}"; do
    printf '%s = { path = "%s/%s" }\n' "$package" "$package_sources" "$package"
  done
} > "$consumer/.cargo/config.toml"

export CARGO_TARGET_DIR="$target_dir"
export ICED_TEST_BACKEND=tiny-skia

cd "$consumer"
cargo generate-lockfile
cargo metadata --locked --format-version 1 > "$downstream_scratch/metadata.json"
if grep -Fq "$package_root/crates/" "$downstream_scratch/metadata.json"; then
  echo "downstream dependency graph reached into the source workspace" >&2
  exit 1
fi
for package in "${consumer_packages[@]}"; do
  if ! grep -Fq "$package_sources/$package/Cargo.toml" "$downstream_scratch/metadata.json"; then
    echo "downstream dependency graph did not select extracted $package archive" >&2
    exit 1
  fi
done

cargo check --locked
cargo build --manifest-path "$package_sources/cargo-ice/Cargo.toml"
cargo_ice="$target_dir/debug/cargo-ice"
if [[ ! -x "$cargo_ice" ]]; then
  echo "packaged cargo-ice binary was not built at $cargo_ice" >&2
  exit 1
fi
"$cargo_ice" ice check
"$cargo_ice" ice test packaged_consumer_contract -- --nocapture
cargo test --locked packaged_runtime_is_a_direct_dependency

mapfile -t generated_manifests < <(
  find "$target_dir/debug/build" -path '*/out/ui-lang-generated/manifest.json' -type f
)
if [[ ${#generated_manifests[@]} -eq 0 ]]; then
  echo "no downstream generated manifest was written" >&2
  exit 1
fi
generated_name="$(printf '%s' 'src/ui/app.ice' | sha256sum | cut -d ' ' -f 1).rs"
for generated_manifest in "${generated_manifests[@]}"; do
  generated_directory=$(dirname "$generated_manifest")
  if ! grep -Fq '"schemaVersion": 1' "$generated_manifest" ||
    ! grep -Fq "\"$generated_name\": \"src/ui/app.ice\"" "$generated_manifest"; then
    echo "downstream generated manifest does not contain the canonical SHA mapping" >&2
    cat "$generated_manifest" >&2
    exit 1
  fi
  if [[ ! -f "$generated_directory/$generated_name" ]]; then
    echo "downstream generated Rust is missing at $generated_directory/$generated_name" >&2
    exit 1
  fi
done

cp "$consumer/diagnostics/backend-wrong-signature.rs" "$consumer/src/backend.rs"
set +e
diagnostic=$("$cargo_ice" ice check 2>&1)
diagnostic_status=$?
set -e
if [[ $diagnostic_status -eq 0 ]]; then
  echo "packaged cargo-ice accepted an invalid downstream extern signature" >&2
  exit 1
fi
if ! grep -Fq 'src/ui/extern/backend.ice:2:1:' <<< "$diagnostic" ||
  ! grep -Fq 'sync greeting(name:str) -> str' <<< "$diagnostic" ||
  ! grep -Fq 'note: generated Rust location:' <<< "$diagnostic"; then
  echo "packaged cargo-ice did not source-map the extern failure" >&2
  printf '%s\n' "$diagnostic" >&2
  exit 1
fi

echo "packaged downstream Ice consumer passed for $package_version"

#!/usr/bin/env bash
set -euo pipefail

package_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$package_root"

package_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' crates/ui-lang-core/Cargo.toml)
packages=(
  ui-lang-core
  ui-lang-runtime
  ducktape-ui
  ui-lang-build
  ui-lang
  cargo-ice
)

for manifest in crates/*/Cargo.toml; do
  manifest_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$manifest")
  if [[ "$manifest_version" != "$package_version" ]]; then
    echo "$manifest has version $manifest_version; expected $package_version" >&2
    exit 1
  fi
done

dirty_args=()
if [[ "${ICE_PACKAGE_ALLOW_DIRTY:-0}" == "1" ]]; then
  dirty_args+=(--allow-dirty)
fi

core_patch=(
  --config "patch.crates-io.ui-lang-core.path=\"$package_root/crates/ui-lang-core\""
)
build_patch=(
  --config "patch.crates-io.ui-lang-build.path=\"$package_root/crates/ui-lang-build\""
)
runtime_patch=(
  --config "patch.crates-io.ui-lang-runtime.path=\"$package_root/crates/ui-lang-runtime\""
)

cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-core
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-runtime
cargo package --locked --no-verify "${dirty_args[@]}" -p ducktape-ui "${runtime_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-build "${core_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang \
  "${core_patch[@]}" "${build_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p cargo-ice "${core_patch[@]}"

package_scratch=$(mktemp -d -t ducktape-package.XXXXXX)
cleanup() {
  if [[ -n "$package_scratch" && -d "$package_scratch" ]]; then
    rm -rf -- "$package_scratch"
  fi
}
trap cleanup EXIT

for package in "${packages[@]}"; do
  tar -xzf "target/package/$package-$package_version.crate" -C "$package_scratch"
  if awk '
    /^\[/ { dependency = ($0 ~ /dependencies(\.|])/); next }
    dependency && /^[[:space:]]*path[[:space:]]*=/ { found = 1 }
    END { exit(found ? 0 : 1) }
  ' "$package_scratch/$package-$package_version/Cargo.toml"; then
    echo "$package still contains a path dependency after packaging" >&2
    exit 1
  fi
done

packaged_patches=(
  --config "patch.crates-io.ui-lang-core.path=\"$package_scratch/ui-lang-core-$package_version\""
)
packaged_build_patch=(
  --config "patch.crates-io.ui-lang-build.path=\"$package_scratch/ui-lang-build-$package_version\""
)
packaged_runtime_patch=(
  --config "patch.crates-io.ui-lang-runtime.path=\"$package_scratch/ui-lang-runtime-$package_version\""
)

check_package() {
  local package=$1
  shift
  CARGO_TARGET_DIR="$package_root/target" cargo check \
    --manifest-path "$package_scratch/$package-$package_version/Cargo.toml" \
    --all-features \
    "$@"
}

check_package_features() {
  local package=$1
  local features=$2
  shift 2
  CARGO_TARGET_DIR="$package_root/target" cargo check \
    --manifest-path "$package_scratch/$package-$package_version/Cargo.toml" \
    --no-default-features \
    --features "$features" \
    "$@"
}

check_package ui-lang-core
check_package ui-lang-runtime
check_package_features ui-lang-runtime data-grid,x11
check_package ducktape-ui "${packaged_runtime_patch[@]}"
check_package ui-lang-build "${packaged_patches[@]}"
check_package ui-lang "${packaged_patches[@]}" "${packaged_build_patch[@]}"
check_package cargo-ice "${packaged_patches[@]}"

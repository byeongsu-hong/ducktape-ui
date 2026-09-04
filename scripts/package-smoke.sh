#!/usr/bin/env bash
set -euo pipefail

package_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$package_root"

package_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' crates/ui-lang-core/Cargo.toml)
packages=(
  ui-lang-template
  ui-lang-wire
  ui-lang-core
  ui-lang-runtime
  ui-lang-guest
  ui-lang-components
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
# The published view format. Both the generator and the runtime depend on it,
# so every package below it needs the patch as well as its direct one.
template_patch=(
  --config "patch.crates-io.ui-lang-template.path=\"$package_root/crates/ui-lang-template\""
)
# The tree a view module ships. The runtime renders it and the guest builds
# it, so both and everything above the runtime need the patch.
wire_patch=(
  --config "patch.crates-io.ui-lang-wire.path=\"$package_root/crates/ui-lang-wire\""
)

cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-template
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-wire
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-core "${template_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-runtime \
  "${template_patch[@]}" "${wire_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-guest "${wire_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-components \
  "${runtime_patch[@]}" "${template_patch[@]}" "${wire_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang-build \
  "${core_patch[@]}" "${template_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p ui-lang \
  "${core_patch[@]}" "${build_patch[@]}" "${template_patch[@]}"
cargo package --locked --no-verify "${dirty_args[@]}" -p cargo-ice \
  "${core_patch[@]}" "${template_patch[@]}"

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
packaged_template_patch=(
  --config "patch.crates-io.ui-lang-template.path=\"$package_scratch/ui-lang-template-$package_version\""
)
packaged_wire_patch=(
  --config "patch.crates-io.ui-lang-wire.path=\"$package_scratch/ui-lang-wire-$package_version\""
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

check_package ui-lang-template
check_package ui-lang-wire
check_package ui-lang-core "${packaged_template_patch[@]}"
check_package ui-lang-runtime "${packaged_template_patch[@]}" "${packaged_wire_patch[@]}"
check_package_features ui-lang-runtime data-grid,x11 \
  "${packaged_template_patch[@]}" "${packaged_wire_patch[@]}"
check_package ui-lang-guest "${packaged_wire_patch[@]}"
check_package ui-lang-components "${packaged_runtime_patch[@]}" "${packaged_template_patch[@]}" \
  "${packaged_wire_patch[@]}"
check_package ui-lang-build "${packaged_patches[@]}" "${packaged_template_patch[@]}"
check_package ui-lang "${packaged_patches[@]}" "${packaged_build_patch[@]}" \
  "${packaged_template_patch[@]}"
check_package cargo-ice "${packaged_patches[@]}" "${packaged_template_patch[@]}"

#!/usr/bin/env bash
set -euo pipefail

package_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
fixture_root="$package_root/tests/downstream-app"
package_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$package_root/crates/ui-lang-core/Cargo.toml")
packages=(
  ui-lang-template
  ui-lang-core
  ui-lang-runtime
  ducktape-ui
  ui-lang-build
  ui-lang
  cargo-ice
)
consumer_packages=(
  ui-lang-template
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

api_baseline="$downstream_scratch/api-baseline.json"
api_repeat="$downstream_scratch/api-repeat.json"
api_event_added="$downstream_scratch/api-event-added.json"
api_zero_report="$downstream_scratch/api-zero-diff.json"
api_breaking_report="$downstream_scratch/api-breaking-diff.json"
api_breaking_error="$downstream_scratch/api-breaking-diff.stderr"
api_corrupt="$downstream_scratch/api-corrupt.json"
api_corrupt_error="$downstream_scratch/api-corrupt.stderr"

"$cargo_ice" ice api tests/cases/api/baseline.ice > "$api_baseline"
"$cargo_ice" ice api tests/cases/api/baseline.ice > "$api_repeat"
if ! cmp -- "$api_baseline" "$api_repeat"; then
  echo "packaged cargo-ice emitted a non-deterministic API fingerprint" >&2
  diff -u -- "$api_baseline" "$api_repeat" >&2 || true
  exit 1
fi

"$cargo_ice" ice api diff "$api_baseline" "$api_repeat" --format json > "$api_zero_report"
if ! grep -Fq '"breaking": 0' "$api_zero_report" ||
  ! grep -Fq '"behavioral_review": 0' "$api_zero_report" ||
  ! grep -Fq '"additive": 0' "$api_zero_report" ||
  ! grep -Fq '"changes": []' "$api_zero_report"; then
  echo "packaged cargo-ice did not report an empty API diff for identical fingerprints" >&2
  cat "$api_zero_report" >&2
  exit 1
fi

"$cargo_ice" ice api tests/cases/api/event-added.ice > "$api_event_added"
set +e
"$cargo_ice" ice api diff "$api_baseline" "$api_event_added" --format json \
  > "$api_breaking_report" 2> "$api_breaking_error"
api_breaking_status=$?
set -e
if [[ $api_breaking_status -eq 0 ]]; then
  echo "packaged cargo-ice accepted a named event added to an existing component" >&2
  cat "$api_breaking_report" >&2
  exit 1
fi
if ! grep -Fq '"breaking": 1' "$api_breaking_report" ||
  ! grep -Fq '"additive": 0' "$api_breaking_report" ||
  ! grep -Fq '"classification": "breaking"' "$api_breaking_report" ||
  ! grep -Fq '"code": "event_added"' "$api_breaking_report" ||
  ! grep -Fq '"path": "components.PackagedCard.events.opened"' "$api_breaking_report" ||
  ! grep -Fq 'Ice API diff contains 1 breaking change(s)' "$api_breaking_error"; then
  echo "packaged cargo-ice did not emit the named-event breaking API contract" >&2
  cat "$api_breaking_report" >&2
  cat "$api_breaking_error" >&2
  exit 1
fi

awk '
  !changed && /"fingerprint": "sha256:/ {
    sub(/sha256:/, "sha256:0")
    changed = 1
  }
  { print }
' "$api_baseline" > "$api_corrupt"
set +e
"$cargo_ice" ice api diff "$api_corrupt" "$api_baseline" --format json \
  > /dev/null 2> "$api_corrupt_error"
api_corrupt_status=$?
set -e
if [[ $api_corrupt_status -eq 0 ]] ||
  ! grep -Fq 'corrupt API fingerprint' "$api_corrupt_error"; then
  echo "packaged cargo-ice accepted a corrupt API fingerprint" >&2
  cat "$api_corrupt_error" >&2
  exit 1
fi

"$cargo_ice" ice check
"$cargo_ice" ice test packaged_consumer_contract -- --nocapture
review_dir="$downstream_scratch/review"
"$cargo_ice" ice review src/ui/app.ice \
  --test packaged_consumer_contract \
  --output "$review_dir"
if ! grep -Fq '"success": true' "$review_dir/report.json" ||
  ! grep -Fq '"executions": 1' "$review_dir/report.json" ||
  ! grep -Fq '"key": "packaged_consumer_contract/packaged_consumer.json"' "$review_dir/report.json" ||
  [[ ! -f "$review_dir/report.html" ]]; then
  echo "packaged cargo-ice did not publish complete downstream review evidence" >&2
  cat "$review_dir/report.json" >&2
  exit 1
fi
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
  generated_rust="$generated_directory/$generated_name"
  if [[ ! -f "$generated_rust" ]]; then
    echo "downstream generated Rust is missing at $generated_rust" >&2
    exit 1
  fi
  generated_digest=$(sha256sum "$generated_rust" | cut -d ' ' -f 1)
  if ! grep -Fq '"schemaVersion": 3' "$generated_manifest" ||
    ! grep -Fq "\"$generated_name\": {" "$generated_manifest" ||
    ! grep -Fq '"source": "src/ui/app.ice"' "$generated_manifest" ||
    ! grep -Fq "\"contentSha256\": \"$generated_digest\"" "$generated_manifest"; then
    echo "downstream generated manifest does not contain the canonical SHA mapping and content digest" >&2
    cat "$generated_manifest" >&2
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
  ! grep -Fq 'pure greeting(name:str) -> str' <<< "$diagnostic" ||
  ! grep -Fq 'note: generated Rust location:' <<< "$diagnostic"; then
  echo "packaged cargo-ice did not source-map the extern failure" >&2
  printf '%s\n' "$diagnostic" >&2
  exit 1
fi

echo "packaged downstream Ice consumer passed for $package_version"

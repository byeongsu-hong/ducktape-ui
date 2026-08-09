#!/usr/bin/env bash
set -euo pipefail

workspace=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
scratch=$(mktemp -d -t ice-generation-overlap.XXXXXX)
dev_log="$scratch/dev.log"
check_log="$scratch/check.log"
dev_pid=
dev_group=
check_pid=

cleanup() {
  for pid in "$check_pid" "$dev_group"; do
    if [[ -n "$pid" ]] && kill -0 -- "-$pid" 2>/dev/null; then
      kill -TERM -- "-$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  rm -rf -- "$scratch"
}
trap cleanup EXIT

cd "$workspace"

# Remove only the disposable iced-app package artifacts so both commands must
# publish their own generated cache during this run. Without this boundary,
# two stale manifests from an earlier build could make the cache assertions a
# false positive.
cargo clean -p iced-app

setsid cargo ice dev examples/iced-app/src/ui/tasks.ice -- -p iced-app >"$dev_log" 2>&1 &
dev_pid=$!
dev_group=$dev_pid
setsid cargo check --locked -p iced-app >"$check_log" 2>&1 &
check_pid=$!

deadline=$((SECONDS + 300))
while kill -0 "$check_pid" 2>/dev/null; do
  if ! kill -0 "$dev_pid" 2>/dev/null; then
    echo "cargo ice dev exited before publishing its watcher state" >&2
    cat "$dev_log" >&2
    exit 1
  fi
  if ((SECONDS >= deadline)); then
    echo "cargo check did not complete during the generated-output overlap budget" >&2
    cat "$dev_log" >&2
    cat "$check_log" >&2
    exit 1
  fi
  sleep 0.1
done
if ! wait "$check_pid"; then
  check_pid=
  echo "the concurrent cargo check failed" >&2
  cat "$check_log" >&2
  exit 1
fi
check_pid=

while ! grep -Fq "cargo_ice::dev: watching" "$dev_log"; do
  if ! kill -0 "$dev_pid" 2>/dev/null; then
    echo "cargo ice dev exited before publishing its watcher state" >&2
    cat "$dev_log" >&2
    exit 1
  fi
  if ((SECONDS >= deadline)); then
    echo "cargo ice dev did not publish its watcher state during the overlap budget" >&2
    cat "$dev_log" >&2
    exit 1
  fi
  sleep 0.1
done

python3 - "$workspace" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

workspace = Path(sys.argv[1])
roots = list((workspace / "target/debug/build").glob("iced-app-*/out/ui-lang-generated/manifest.json"))
if not roots:
    raise SystemExit("no iced-app generated manifest was published")

matching = []
for manifest_path in roots:
    document = json.loads(manifest_path.read_text())
    if any(entry.get("source") == "src/ui/tasks.ice" for entry in document.get("outputs", {}).values()):
        matching.append((manifest_path, document))
if len(matching) < 2:
    raise SystemExit(
        "expected distinct dev-fingerprint and normal-check generated caches; "
        f"found {len(matching)}"
    )

for manifest_path, document in matching:
    if document.get("schemaVersion") != 3:
        raise SystemExit(f"unexpected generated manifest schema in {manifest_path}")
    directory = manifest_path.parent
    for output, entry in document["outputs"].items():
        generated = directory / output
        if not generated.is_file():
            raise SystemExit(f"generated output is missing: {generated}")
        digest = hashlib.sha256(generated.read_bytes()).hexdigest()
        if digest != entry.get("contentSha256"):
            raise SystemExit(f"generated output digest mismatch: {generated}")
    for child in directory.iterdir():
        if child.name.startswith((".ui-lang-transaction-", ".atomicwrite")):
            raise SystemExit(f"stale generated transaction remains: {child}")
PY

kill -INT "$dev_pid"
stop_deadline=$((SECONDS + 15))
while kill -0 "$dev_pid" 2>/dev/null; do
  if ((SECONDS >= stop_deadline)); then
    echo "cargo ice dev did not stop after SIGINT" >&2
    cat "$dev_log" >&2
    exit 1
  fi
  sleep 0.1
done
if ! wait "$dev_pid"; then
  dev_pid=
  echo "cargo ice dev reported failure after the overlap run" >&2
  cat "$dev_log" >&2
  exit 1
fi
dev_pid=

if kill -0 -- "-$dev_group" 2>/dev/null; then
  echo "cargo ice dev left a process in its session after clean shutdown" >&2
  cat "$dev_log" >&2
  exit 1
fi
dev_group=

grep -Fq "cargo_ice::dev: stopping" "$dev_log"
echo "separate cargo ice dev and cargo check processes published valid generated caches"

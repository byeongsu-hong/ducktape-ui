#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
artifact=${1:-target/performance/rich-text-editor.jsonl}
if [[ "$artifact" != /* ]]; then
  artifact="$repo_root/$artifact"
fi

mkdir -p -- "$(dirname -- "$artifact")"
: > "$artifact"
export ICE_EDITOR_PERF_JSONL="$artifact"

cd -- "$repo_root"
cargo test --locked -p ui-lang-runtime --lib performance_contract -- \
  --ignored --nocapture --test-threads=1
cargo test --release --locked -p ui-lang-runtime \
  --test rich_text_editor_allocations allocation_contract_100k_total_allocations -- \
  --ignored --exact --nocapture --test-threads=1
python3 scripts/validate-editor-performance.py "$artifact"

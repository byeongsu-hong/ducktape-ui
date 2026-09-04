#!/usr/bin/env bash
# Turns every app module under target/wasm32-unknown-unknown/release into an
# `ice:view` component in the catalog directory.
#
# The core module still links wasm-bindgen's placeholder imports — iced's
# wasm target pulls `web-time` and `web-sys` in, and their `#[wasm_bindgen]`
# glue is exported, so the linker keeps it — and a component may not import
# what its world does not name. Each of those import modules is satisfied by
# a stub adapter whose every function traps: nothing on a guest's frame path
# calls them, and a call is a bug worth a trap. The stubs are generated from
# the module's own import list, so a new wasm-bindgen hash needs no edit.
set -euo pipefail

cd "$(dirname "$0")"
release=target/wasm32-unknown-unknown/release
catalog=${APP_STORE_CATALOG:-target/app-store-catalog}
mkdir -p "$catalog"

stub_for() {
  # $1: module wat, $2: import module name → a wat on stdout exporting each
  # function that module imports from `$2`, with the imported signature.
  awk -v module="$2" '
    /^  \(type \(;[0-9]+;\) \(func/ {
      match($0, /\(;[0-9]+;\)/); id = substr($0, RSTART + 2, RLENGTH - 4)
      sig = $0; sub(/^.*\(func/, "", sig); sub(/\)\)$/, "", sig); types[id] = sig
    }
    $0 ~ "^  \\(import \"" module "\" " {
      match($0, /"[^"]*" "[^"]*"/); pair = substr($0, RSTART, RLENGTH)
      split(pair, names, "\" \""); name = names[2]; sub(/"$/, "", name)
      match($0, /\(type [0-9]+\)/); id = substr($0, RSTART + 6, RLENGTH - 7)
      print "  (func (export \"" name "\")" types[id] " unreachable)"
    }' "$1"
}

for module in "$release"/app_store_*.wasm; do
  name=$(basename "$module" .wasm)
  wat=$(mktemp); wasm-tools print "$module" > "$wat"
  adapters=()
  for import in $(grep -o '^  (import "[^"]*"' "$wat" | cut -d'"' -f2 | sort -u); do
    stub=$(mktemp --suffix=.wasm)
    { echo "(module"; stub_for "$wat" "$import"; echo ")"; } | wasm-tools parse -o "$stub"
    adapters+=(--adapt "$import=$stub")
  done
  wasm-tools component new "$module" "${adapters[@]}" -o "$catalog/$name.wasm"
  # The glue's own metadata is a third of the file and nothing reads it.
  wasm-tools strip --delete __wasm_bindgen_unstable "$catalog/$name.wasm" -o "$catalog/$name.wasm"
  rm -f "$wat"
  echo "$catalog/$name.wasm"
done

#!/bin/sh
set -eu

target=${UI_LANG_WINDOWS_TARGET:-x86_64-pc-windows-gnu}

if ! rustup target list --installed | grep -qx "$target"; then
  echo "missing Rust target: $target" >&2
  exit 1
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$(dirname -- "$script_dir")"

cargo check --locked --target "$target" \
  -p ui-lang-runtime \
  -p showcase

cargo check --locked --target "$target" --tests \
  -p ui-lang-runtime \
  -p ui-lang-core \
  -p showcase

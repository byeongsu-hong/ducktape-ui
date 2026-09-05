#!/bin/sh
set -eu

# The macOS half of `scripts/a11y-windows-check.sh`: the NSAccessibility
# adapter only compiles for an Apple target, so this needs a Mac and no CI job
# runs it. It builds the runtime, the reference app and the two-window daemon
# in both their production and test forms, runs the bridge tests that pin the
# adapter's main-thread refusal, its window-focus handling, and the per-window
# scoping a daemon's export rests on, then the in-process smoke that attaches
# the subclass to a real `NSView` and reads the tree back the way VoiceOver
# would.

if [ "$(uname -s)" != "Darwin" ]; then
  echo "requires macOS; the NSAccessibility adapter has no cross-compile" >&2
  exit 1
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$(dirname -- "$script_dir")"

cargo check --locked \
  -p ui-lang-runtime \
  -p showcase \
  -p two-windows-example

cargo check --locked --tests \
  -p ui-lang-runtime \
  -p ui-lang-core \
  -p showcase \
  -p two-windows-example

cargo test --locked -p ui-lang-runtime --lib -- macos_ window_bridges scoped_snapshot
cargo test --locked -p ui-lang-runtime --test macos_native_smoke

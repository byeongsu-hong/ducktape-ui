#!/usr/bin/env python3
"""Measure the Ice edit->run loop: codegen cost, rustc cost, and their sum.

Three numbers per package, each the median of `--runs` samples:

  noop     `cargo build` with nothing changed. Cargo's own overhead.
  script   the package's build-script binary run directly, regenerating every
           root into its real OUT_DIR. The Ice compiler alone, no cargo, no
           rustc. (Bumping ICE_DEV_BUILD_FINGERPRINT does NOT isolate this —
           cargo marks the whole crate dirty on an env change and reruns rustc.)
  edit     one byte changed in a root `.ice`, so build.rs reruns AND rustc
           recompiles the crate. This is what an author actually waits for;
           `edit - script` is rustc's share.
  handler  the same, but the byte lands in a handler fragment instead. Where
           an edit lands decides what rustc re-checks, so one anchor is not
           the edit loop: on showcase these two have differed by 30%. Skipped
           for apps with no handler fragment.

Run it before and after a change; `--json out.json` writes a baseline that
`--compare baseline.json` reads back.

    scripts/build_bench.py --packages showcase trading-example --runs 3
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# package -> root .ice relative to ROOT, the literal edited to force a rebuild
# (None discovers a `title`/`id` line), and the handler fragment plus its own
# literal, where the app has one.
PACKAGES = {
    "showcase": {
        "root": "examples/showcase/src/ui/app.ice",
        "anchor": 'title "ducktape-ui · Ice"',
        "handler": ("examples/showcase/src/ui/handlers/app.ice", '"cancelled"'),
    },
    "trading-example": {"root": "examples/trading/src/ui/app.ice"},
    "music-example": {
        "root": "examples/apple-music/src/ui/app.ice",
        "handler": ("examples/apple-music/src/ui/handlers/app.ice", '"Sign In"'),
    },
    "markdown-example": {
        "root": "examples/markdown-editor/src/ui/app.ice",
        "handler": ("examples/markdown-editor/src/ui/handlers/app.ice", '"Untitled.md"'),
    },
    "candles-example": {"root": "examples/candles/src/ui/app.ice"},
    "terminal-example": {"root": "examples/terminal/src/ui/app.ice"},
}

PHASES = ("noop", "script", "edit", "handler")


def anchor(source: Path, configured: str | None) -> str:
    """The literal whose trailing space we toggle to force a real rebuild."""
    if configured:
        return configured
    for line in source.read_text().splitlines():
        stripped = line.strip()
        if stripped.startswith(('title "', 'id "')) and stripped.endswith('"'):
            return stripped
    raise SystemExit(f"{source}: no `title`/`id` literal to edit; add one to PACKAGES")


def build_script(package: str) -> float:
    """Time the package's own build script: the Ice compiler with no cargo around it."""
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "debug" / "build"
    # By mtime, not by name: a package matches several build directories and
    # the newest is the live one. Sorting by name reads a stale build script
    # against a stale OUT_DIR, which times a build nobody is running.
    newest = lambda path: path.stat().st_mtime
    binaries = sorted(target.glob(f"{package}-*/build-script-build"), key=newest)
    out_dirs = sorted(target.glob(f"{package}-*/out/ui-lang-generated"), key=newest)
    if not binaries or not out_dirs:
        raise SystemExit(f"{package}: no build script or OUT_DIR under {target}; build it first")
    manifest = next(
        path.parent for path in ROOT.glob("examples/*/Cargo.toml") if f'name = "{package}"' in path.read_text()
    )
    started = time.perf_counter()
    result = subprocess.run(
        [str(binaries[-1])],
        cwd=manifest,
        env={
            **os.environ,
            "CARGO_MANIFEST_DIR": str(manifest),
            "OUT_DIR": str(out_dirs[-1].parent),
        },
        capture_output=True,
        text=True,
    )
    elapsed = time.perf_counter() - started
    if result.returncode != 0:
        sys.stderr.write(result.stderr)
        raise SystemExit(f"{package} build script failed")
    return elapsed


def build(package: str, env: dict[str, str] | None = None) -> float:
    started = time.perf_counter()
    result = subprocess.run(
        ["cargo", "build", "-p", package],
        cwd=ROOT,
        env={**os.environ, **(env or {})},
        capture_output=True,
        text=True,
    )
    elapsed = time.perf_counter() - started
    if result.returncode != 0:
        sys.stderr.write(result.stderr)
        raise SystemExit(f"cargo build -p {package} failed")
    return elapsed


def edit_pair(path: Path, literal: str) -> tuple[str, str]:
    """The file's text with and without a trailing space inside `literal`."""
    original = path.read_text()
    if literal not in original:
        raise SystemExit(f"{path}: anchor {literal!r} not found")
    return original, original.replace(literal, literal[:-1] + ' "', 1)


def measure(package: str, runs: int) -> dict[str, float]:
    entry = PACKAGES[package]
    source = ROOT / entry["root"]
    original, edited = edit_pair(source, anchor(source, entry.get("anchor")))

    handler = entry.get("handler")
    if handler:
        handler_source = ROOT / handler[0]
        handler_original, handler_edited = edit_pair(handler_source, handler[1])

    build(package)  # warm: leave nothing dirty behind
    samples: dict[str, list[float]] = {phase: [] for phase in PHASES}
    try:
        for run in range(runs):
            samples["noop"].append(build(package))
            samples["script"].append(build_script(package))
            source.write_text(edited if run % 2 == 0 else original)
            samples["edit"].append(build(package))
            if handler:
                handler_source.write_text(
                    handler_edited if run % 2 == 0 else handler_original
                )
                samples["handler"].append(build(package))
    finally:
        source.write_text(original)
        if handler:
            handler_source.write_text(handler_original)
        build(package)
    return {
        phase: statistics.median(values)
        for phase, values in samples.items()
        if values
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", nargs="+", default=["showcase"])
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--json", type=Path, help="write results here")
    parser.add_argument("--compare", type=Path, help="read a baseline to diff against")
    arguments = parser.parse_args()

    baseline = json.loads(arguments.compare.read_text()) if arguments.compare else {}
    results = {}
    for package in arguments.packages:
        if package not in PACKAGES:
            raise SystemExit(f"unknown package {package}; add it to PACKAGES")
        results[package] = measure(package, arguments.runs)
        report(package, results[package], baseline.get(package))

    if arguments.json:
        arguments.json.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n")


def report(package: str, result: dict[str, float], before: dict[str, float] | None) -> None:
    print(f"\n{package}")
    for phase in PHASES:
        if phase not in result:
            continue
        line = f"  {phase:8s} {result[phase]:6.2f}s"
        if before and phase in before:
            ratio = result[phase] / before[phase] if before[phase] else float("inf")
            line += f"   was {before[phase]:6.2f}s  ({ratio:.2f}x)"
        print(line)


if __name__ == "__main__":
    main()

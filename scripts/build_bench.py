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

# package -> (root .ice relative to ROOT, literal edited to force a rebuild)
PACKAGES = {
    "showcase": ("examples/showcase/src/ui/app.ice", 'title "ducktape-ui · Ice"'),
    "iced-app": ("examples/iced-app/src/ui/tasks.ice", 'id "dev.ducktape.ice.tasks"'),
    "trading-example": ("examples/trading/src/ui/app.ice", None),
    "music-example": ("examples/apple-music/src/ui/app.ice", None),
    "markdown-example": ("examples/markdown-editor/src/ui/app.ice", None),
    "candles-example": ("examples/candles/src/ui/app.ice", None),
    "terminal-example": ("examples/terminal/src/ui/app.ice", None),
}


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
    binaries = sorted(target.glob(f"{package}-*/build-script-build"))
    out_dirs = sorted(target.glob(f"{package}-*/out/ui-lang-generated"))
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


def measure(package: str, runs: int) -> dict[str, float]:
    source = ROOT / PACKAGES[package][0]
    original = source.read_text()
    literal = anchor(source, PACKAGES[package][1])
    if literal not in original:
        raise SystemExit(f"{source}: anchor {literal!r} not found")
    edited = original.replace(literal, literal[:-1] + ' "', 1)

    build(package)  # warm: leave nothing dirty behind
    samples: dict[str, list[float]] = {"noop": [], "script": [], "edit": []}
    try:
        for run in range(runs):
            samples["noop"].append(build(package))
            samples["script"].append(build_script(package))
            source.write_text(edited if run % 2 == 0 else original)
            samples["edit"].append(build(package))
    finally:
        source.write_text(original)
        build(package)
    return {phase: statistics.median(values) for phase, values in samples.items()}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", nargs="+", default=["showcase", "iced-app"])
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
    for phase in ("noop", "script", "edit"):
        line = f"  {phase:8s} {result[phase]:6.2f}s"
        if before and phase in before:
            ratio = result[phase] / before[phase] if before[phase] else float("inf")
            line += f"   was {before[phase]:6.2f}s  ({ratio:.2f}x)"
        print(line)


if __name__ == "__main__":
    main()

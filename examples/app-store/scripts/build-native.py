#!/usr/bin/env python3
"""Build trusted Tree executables and package their static manifests for the store."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--out", type=Path, default=Path("target/app-store-native-catalog"))
parser.add_argument("-p", "--package", action="append", help="trusted local package to build (default: the five shipped apps)")
args = parser.parse_args()
workspace = Path(__file__).resolve().parents[1]
packages = args.package or [f"app-store-{name}" for name in ("counter", "todo", "clock", "activity", "chaos")]
command = ["cargo", "build", "--release", "--locked", "--bins", "--message-format=json-render-diagnostics"]
for package in packages:
    command += ["-p", package]
artifacts = {}
with subprocess.Popen(command, cwd=workspace, stdout=subprocess.PIPE, text=True) as build:
    for line in build.stdout:
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message.get("executable"):
            artifacts[message["target"]["name"]] = Path(message["executable"])
    if build.wait():
        raise SystemExit("native build failed")
if set(artifacts) != set(packages):
    raise SystemExit(f"expected every requested native app; got {sorted(artifacts)}")
output = args.out.resolve()
output.mkdir(parents=True, exist_ok=True)
for package, executable in sorted(artifacts.items()):
    # This is an explicitly requested build of trusted local source, never a
    # catalog scan. --manifest returns before boot and does not evaluate Ice.
    manifest = subprocess.run([executable, "--manifest"], check=True, capture_output=True, timeout=10).stdout
    directory = output / (package.removeprefix("app-store-") + ".native")
    directory.mkdir(exist_ok=True)
    shutil.copyfile(executable, directory / ("app.exe" if os.name == "nt" else "app"))
    (directory / "manifest").write_bytes(manifest)
    print(directory)

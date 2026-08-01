#!/usr/bin/env python3
import json
import pathlib
import sys


SCHEMA = "ice.rich-text-editor.performance.v1"
SCENARIOS = {
    "caret_1000": 1_000,
    "selection_drag_1000": 1_000,
    "one_char_insertion": 1,
    "hangul_ime_sequence": 3,
    "viewport_resize": 1,
    "format_key_only": 1,
}
METRICS = {
    "full_text_materializations",
    "materialized_source_bytes",
    "parsed_line_strings",
    "parsed_line_bytes",
    "composition_display_strings",
    "composition_display_bytes",
    "composition_line_strings",
    "composition_line_bytes",
    "mapping_line_comparisons",
    "styled_signature_comparisons",
    "newly_owned_styled_texts",
    "newly_owned_styled_text_bytes",
    "line_vector_slots_prepared",
    "rebuilt_lines",
    "shaped_paragraphs",
    "highlighted_lines",
    "accepted_change_hints",
    "rejected_change_hints",
}
COMMON = {"schema", "kind", "scenario", "document_lines", "iterations"}
OPERATION = COMMON | {"elapsed_ns", "wall_time_budget_ns", "metrics"}
HEAP = COMMON | {
    "collector",
    "scope",
    "allocation_count",
    "allocated_bytes",
    "allocation_count_budget",
    "allocated_bytes_budget",
}


def fail(message: str) -> None:
    raise SystemExit(f"editor performance artifact: {message}")


def integer(value: object, label: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        fail(f"{label} must be an integer >= {minimum}, got {value!r}")
    return value


if len(sys.argv) != 2:
    fail("usage: validate-editor-performance.py <artifact.jsonl>")

path = pathlib.Path(sys.argv[1])
try:
    raw_lines = path.read_text(encoding="utf-8").splitlines()
except OSError as error:
    fail(f"cannot read {path}: {error}")
if not raw_lines:
    fail(f"{path} is empty")

records = []
for line_number, raw_line in enumerate(raw_lines, 1):
    try:
        record = json.loads(raw_line)
    except json.JSONDecodeError as error:
        fail(f"line {line_number} is not JSON: {error}")
    if not isinstance(record, dict):
        fail(f"line {line_number} must contain one JSON object")
    records.append(record)

seen = set()
for line_number, record in enumerate(records, 1):
    if record.get("schema") != SCHEMA:
        fail(f"line {line_number} has unsupported schema {record.get('schema')!r}")
    kind = record.get("kind")
    scenario = record.get("scenario")
    if kind not in {"operation", "heap"}:
        fail(f"line {line_number} has invalid kind {kind!r}")
    if scenario not in SCENARIOS:
        fail(f"line {line_number} has unknown scenario {scenario!r}")
    identity = (kind, scenario)
    if identity in seen:
        fail(f"duplicate {kind} record for {scenario}")
    seen.add(identity)
    expected_fields = OPERATION if kind == "operation" else HEAP
    if set(record) != expected_fields:
        fail(
            f"line {line_number} fields differ: "
            f"missing={sorted(expected_fields - set(record))}, "
            f"extra={sorted(set(record) - expected_fields)}"
        )
    if integer(record["document_lines"], "document_lines", 1) != 100_001:
        fail(f"{kind}/{scenario} must exercise exactly 100001 logical lines")
    if integer(record["iterations"], "iterations", 1) != SCENARIOS[scenario]:
        fail(f"{kind}/{scenario} has the wrong iteration count")

    if kind == "operation":
        elapsed = integer(record["elapsed_ns"], "elapsed_ns", 1)
        budget = integer(record["wall_time_budget_ns"], "wall_time_budget_ns", 1)
        if elapsed >= budget:
            fail(f"{scenario} took {elapsed}ns; budget is {budget}ns")
        metrics = record["metrics"]
        if not isinstance(metrics, dict) or set(metrics) != METRICS:
            fail(f"operation/{scenario} has an invalid metrics object")
        for name, value in metrics.items():
            integer(value, f"operation/{scenario}.metrics.{name}")
    else:
        if record["collector"] != "dhat-0.3.3":
            fail(f"heap/{scenario} has an unsupported collector")
        if record["scope"] != "operation-only":
            fail(f"heap/{scenario} has an unsupported measurement scope")
        count = integer(record["allocation_count"], "allocation_count")
        count_budget = integer(
            record["allocation_count_budget"], "allocation_count_budget", 1
        )
        allocated = integer(record["allocated_bytes"], "allocated_bytes")
        allocated_budget = integer(
            record["allocated_bytes_budget"], "allocated_bytes_budget", 1
        )
        if count > count_budget:
            fail(f"{scenario} allocated {count} blocks; budget is {count_budget}")
        if allocated > allocated_budget:
            fail(f"{scenario} allocated {allocated} bytes; budget is {allocated_budget}")

expected = {(kind, scenario) for kind in ("operation", "heap") for scenario in SCENARIOS}
if seen != expected:
    fail(f"record set differs: missing={sorted(expected - seen)}, extra={sorted(seen - expected)}")

print(f"validated {len(records)} rich-text editor performance records in {path}")

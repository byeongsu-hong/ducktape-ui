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
    "composition_display_strings",
    "composition_display_bytes",
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


class ArtifactError(Exception):
    pass


def fail(message: str) -> None:
    raise ArtifactError(message)


def integer(value: object, label: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        fail(f"{label} must be an integer >= {minimum}, got {value!r}")
    return value


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    value = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate object key {key!r}")
        value[key] = item
    return value


def reject_constant(value: str) -> None:
    fail(f"non-finite JSON constant {value}")


def parse_records(raw_lines: list[str], source: object) -> list[dict[str, object]]:
    if not raw_lines:
        fail(f"{source} is empty")

    records = []
    for line_number, raw_line in enumerate(raw_lines, 1):
        try:
            record = json.loads(
                raw_line,
                object_pairs_hook=unique_object,
                parse_constant=reject_constant,
            )
        except (json.JSONDecodeError, ArtifactError) as error:
            fail(f"line {line_number} is not strict JSON: {error}")
        if not isinstance(record, dict):
            fail(f"line {line_number} must contain one JSON object")
        records.append(record)
    return records


def validate_records(records: list[dict[str, object]]) -> None:
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
            if record["collector"] != "stats_alloc-0.1.10":
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
                fail(
                    f"{scenario} allocated {allocated} bytes; budget is {allocated_budget}"
                )

    expected = {
        (kind, scenario) for kind in ("operation", "heap") for scenario in SCENARIOS
    }
    if seen != expected:
        fail(
            f"record set differs: missing={sorted(expected - seen)}, "
            f"extra={sorted(seen - expected)}"
        )


def operation_record(scenario: str) -> dict[str, object]:
    return {
        "schema": SCHEMA,
        "kind": "operation",
        "scenario": scenario,
        "document_lines": 100_001,
        "iterations": SCENARIOS[scenario],
        "elapsed_ns": 1,
        "wall_time_budget_ns": 2,
        "metrics": dict.fromkeys(METRICS, 0),
    }


def heap_record(scenario: str) -> dict[str, object]:
    return {
        "schema": SCHEMA,
        "kind": "heap",
        "scenario": scenario,
        "document_lines": 100_001,
        "iterations": SCENARIOS[scenario],
        "collector": "stats_alloc-0.1.10",
        "scope": "operation-only",
        "allocation_count": 1,
        "allocated_bytes": 1,
        "allocation_count_budget": 1,
        "allocated_bytes_budget": 1,
    }


def expect_failure(raw_lines: list[str], expected: str) -> None:
    try:
        validate_records(parse_records(raw_lines, "self-test input"))
    except ArtifactError as error:
        if expected not in str(error):
            raise AssertionError(f"expected {expected!r}, got {error!r}") from error
        return
    raise AssertionError(f"expected validation failure containing {expected!r}")


def self_test() -> None:
    records = [operation_record(scenario) for scenario in SCENARIOS]
    records.extend(heap_record(scenario) for scenario in SCENARIOS)
    raw_lines = [json.dumps(record, separators=(",", ":")) for record in records]
    validate_records(parse_records(raw_lines, "self-test input"))

    operation = raw_lines[0]
    duplicate_identity = operation.replace(
        "{", f'{{"scenario":"{records[0]["scenario"]}",', 1
    )
    expect_failure([duplicate_identity, *raw_lines[1:]], "duplicate object key 'scenario'")

    duplicate_metric = operation.replace(
        '"metrics":{', '"metrics":{"full_text_materializations":0,', 1
    )
    expect_failure([duplicate_metric, *raw_lines[1:]], "full_text_materializations")

    for constant in ("NaN", "Infinity", "-Infinity"):
        invalid = operation.replace('"elapsed_ns":1', f'"elapsed_ns":{constant}')
        expect_failure([invalid, *raw_lines[1:]], f"non-finite JSON constant {constant}")

    negative = operation.replace('"elapsed_ns":1', '"elapsed_ns":-1')
    expect_failure([negative, *raw_lines[1:]], "elapsed_ns must be an integer >= 1")
    negative_metric = operation.replace(
        '"full_text_materializations":0', '"full_text_materializations":-1'
    )
    expect_failure([negative_metric, *raw_lines[1:]], "full_text_materializations")

    wall_failure = operation.replace('"elapsed_ns":1', '"elapsed_ns":2')
    if '"elapsed_ns":2' not in wall_failure:
        raise AssertionError("wall failure injection lost the actual elapsed value")
    expect_failure([wall_failure, *raw_lines[1:]], "took 2ns; budget is 2ns")

    heap_failure = raw_lines[len(SCENARIOS)].replace(
        '"allocation_count":1', '"allocation_count":2'
    )
    if '"allocation_count":2' not in heap_failure:
        raise AssertionError("heap failure injection lost the actual allocation count")
    expect_failure(
        [*raw_lines[: len(SCENARIOS)], heap_failure, *raw_lines[len(SCENARIOS) + 1 :]],
        "allocated 2 blocks; budget is 1",
    )

    expect_failure(raw_lines[:-1], "record set differs")
    expect_failure([raw_lines[0], *raw_lines], "duplicate operation record")
    print("validated strict rich-text editor performance artifact self-test")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate-editor-performance.py <artifact.jsonl>|--self-test")
    if sys.argv[1] == "--self-test":
        self_test()
        return

    path = pathlib.Path(sys.argv[1])
    try:
        raw_lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        fail(f"cannot read {path}: {error}")
    records = parse_records(raw_lines, path)
    validate_records(records)
    print(f"validated {len(records)} rich-text editor performance records in {path}")


try:
    main()
except ArtifactError as error:
    raise SystemExit(f"editor performance artifact: {error}") from None

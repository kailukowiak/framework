"""Verifies a timesheet-scenario document by what it IS and what it DOES.

Run 2 of the hand-driven smoke test taught this file its founding rule:
never grade the agent on its own report. That run's agent declared the
CSV import broken while the import was fine and the reading tool was
lying — self-reports inherit every bug in the tools they were made with.
So the verifier reads the produced document for structural facts, then
drives the behavior itself through the same MCP tools: flip the
parameter, watch the frame regrow, flip it back, and demand the entered
values come back with it. Everything is found by shape, never by name —
agents name things differently every run, and the contract is about
shapes.

Environment: FRAMEWORK_MCP_BINARY, FRAMEWORK_DOCUMENT.
Exit 0 with an "ok:" line per passed assertion; first failure raises.
"""

import calendar
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def month_bounds(anchor):
    year, month, day = (int(part) for part in anchor.split("-"))
    return day, calendar.monthrange(year, month)[1]


def main():
    document_path = os.environ["FRAMEWORK_DOCUMENT"]
    with open(document_path) as file:
        document = json.load(file)["document"]
    frames = [o for o in document["objects"] if o["kind"] == "frame"]

    # Structure: a generator frame whose rule reads some document value.
    generators = [f for f in frames if f.get("generator")]
    assert generators, "no generator frame — the dates were hand-written"
    generator = generators[0]
    rule = json.dumps(generator["generator"])
    assert '"kind": "value"' in rule or '"value"' in rule, (
        "the generator's bounds name no value, so the period cannot follow "
        "the parameter"
    )
    print("ok: period dates are generated, bound to a document value")

    # Structure: some frame expands against the generator.
    def steps_of(frame):
        derivation = frame.get("derivation") or {}
        return list(derivation.get("steps", [])) + list(frame.get("steps", []))

    expanded = [
        f
        for f in frames
        if any(
            step.get("kind") == "expand" and step.get("frameId") == generator["id"]
            for step in steps_of(f)
        )
    ]
    assert expanded, "no frame expands against the generator — no cross join"
    sheet = expanded[0]
    print(f"ok: a frame expands against the generator ({len(frames)} frames)")

    # Structure: the expanded frame carries a keyed entry column with the
    # two entries the prompt dictates (8 and 4 hours).
    entry_columns = sheet.get("entryColumns") or []
    assert entry_columns, "no entry column — typed hours cannot survive regrowth"
    entries = entry_columns[0].get("entries", [])
    raws = sorted(entry["raw"] for entry in entries)
    assert raws == ["4", "8"], f"expected entries 4 and 8, found {raws}"
    print("ok: hours live as keyed entries (4 and 8)")

    # The anchor value the generator reads, found through the rule itself.
    anchor_ids = set()

    def collect_value_ids(node):
        if isinstance(node, dict):
            if node.get("kind") == "value" and "object_id" in node:
                anchor_ids.add(node["object_id"])
            for child in node.values():
                collect_value_ids(child)
        elif isinstance(node, list):
            for child in node:
                collect_value_ids(child)

    collect_value_ids(generator["generator"])
    assert anchor_ids, "the generator names no value id"

    def anchor_name_and_raw():
        for candidate in document["objects"]:
            if candidate["kind"] == "value" and candidate["id"] in anchor_ids:
                return candidate["name"], candidate["raw"]
            if candidate["kind"] == "block":
                for line in candidate.get("lines", []):
                    if line["id"] in anchor_ids:
                        return line["name"], line["source"].split("=", 1)[-1].strip()
        raise AssertionError("the generator's value id resolves to nothing")

    anchor_name, anchor_raw = anchor_name_and_raw()
    day, _ = month_bounds(anchor_raw)
    print(f"ok: the parameter is '{anchor_name}' = {anchor_raw}")

    # Behavior, through the same tools an agent uses.
    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], document_path)
    try:
        sheet_name = sheet["name"]
        before = client.call("get_frame", {"frame": sheet_name, "limit": 1})
        rows_before = before["totalRowCount"]
        assert rows_before > 0, "the sheet reads as empty"

        # A different month with a different length, so a hardcoded month
        # cannot pass by luck.
        year, month, _ = (int(part) for part in anchor_raw.split("-"))
        next_month = month % 12 + 1
        next_year = year + (1 if month == 12 else 0)
        next_length = calendar.monthrange(next_year, next_month)[1]
        flipped = f"{next_year:04d}-{next_month:02d}-{next_length:02d}"
        client.call("set_value", {"value": anchor_name, "raw": flipped})
        during = client.call("get_frame", {"frame": sheet_name, "limit": 1})
        assert during["totalRowCount"] != rows_before, (
            "the sheet did not regrow when the parameter moved"
        )
        expected_ratio = next_length / day
        actual_ratio = during["totalRowCount"] / rows_before
        assert abs(actual_ratio - expected_ratio) < 0.01, (
            f"the sheet regrew by {actual_ratio:.3f}, expected "
            f"{expected_ratio:.3f} — the period is not the full month range"
        )
        print(
            f"ok: parameter flip regrew the sheet "
            f"({rows_before} -> {during['totalRowCount']} rows)"
        )

        client.call("set_value", {"value": anchor_name, "raw": anchor_raw})
        after = client.call("get_frame", {"frame": sheet_name, "limit": 1000})
        assert after["totalRowCount"] == rows_before, "the sheet did not grow back"
        hours_column = entry_columns[0]["columnId"]
        visible = [
            cell["display"]
            for row in after["rows"]
            for cell in row["cells"]
            if cell["columnId"] == hours_column and cell["display"]
        ]
        assert sorted(visible) == ["4", "8"], (
            f"entries did not survive the round trip: {visible}"
        )
        print("ok: entered hours survived the period round trip")
    finally:
        client.close()

    # Structure: the wide view is on, spreading dates over the entry column.
    crosstab = (sheet.get("display") or {}).get("crosstab")
    assert crosstab, "the crosstab view is not set"
    assert crosstab["valuesColumnId"] == hours_column, (
        "the crosstab does not spread the entry column"
    )
    print("ok: crosstab view is on over the entry column")
    print("PASS")


if __name__ == "__main__":
    main()

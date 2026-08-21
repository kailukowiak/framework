"""Budget scenario verifier: summarize, a validated join, a percentage
column, and live recomputation through the whole chain.

Found by shape, never by name; behavior driven through the same MCP tools
an agent uses. See ../timesheet/verify.py for the founding rules.
"""

import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def main():
    document_path = os.environ["FRAMEWORK_DOCUMENT"]
    with open(document_path) as file:
        document = json.load(file)["document"]
    frames = {f["id"]: f for f in document["objects"] if f["kind"] == "frame"}

    def steps_of(frame):
        derivation = frame.get("derivation") or {}
        return list(derivation.get("steps", [])) + list(frame.get("steps", []))

    summarized = [
        f
        for f in frames.values()
        if any(step.get("kind") == "summarize" for step in steps_of(f))
    ]
    assert summarized, "no frame summarizes — totals were not derived"
    print("ok: a frame summarizes actuals")

    # The join, either flat or as a step, with a keyed lookup side.
    joined = None
    lookup_id = None
    for frame in frames.values():
        derivation = frame.get("derivation") or {}
        join = derivation.get("join")
        if join:
            joined, lookup_id = frame, join["lookupFrameId"]
            break
        for step in steps_of(frame):
            if step.get("kind") == "join":
                joined, lookup_id = frame, step["join"]["lookupFrameId"]
                break
    assert joined, "no frame joins the budget on"
    lookup = frames[lookup_id]
    assert lookup.get("uniqueKeys"), "the join's lookup side carries no unique key"
    assert lookup.get("rows"), "the budget frame does not own its rows"
    print("ok: a validated join brings the budget in")

    percentage = [
        column
        for column in joined["columns"]
        if column.get("dataType") == "percentage"
        or (column.get("format") or {}).get("style") == "percent"
    ]
    assert percentage, "no percentage-shown column on the joined frame"
    used_column = percentage[0]["id"]
    print("ok: a Used column reads as a percentage")

    # Live propagation: double one budget cell, the Used value must halve.
    budget_column = next(
        column
        for column in lookup["columns"]
        if column["dataType"] in ("number", "integer", "currency")
    )
    target_row = lookup["rows"][0]
    old_raw = target_row["cells"][budget_column["id"]]["raw"]

    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], document_path)
    try:

        def used_values():
            snapshot = client.call("get_frame", {"frame": joined["name"], "limit": 100})
            return {
                row["cells"][0]["display"]: cell["numericValue"]
                for row in snapshot["rows"]
                for cell in row["cells"]
                if cell["columnId"] == used_column
            }

        before = used_values()
        client.call(
            "set_cell",
            {
                "frame": lookup["name"],
                "row": target_row["id"],
                "column": budget_column["id"],
                "raw": (
                    str(int(float(old_raw) * 2))
                    if float(old_raw) * 2 == int(float(old_raw) * 2)
                    else str(float(old_raw) * 2)
                ),
            },
        )
        after = used_values()
        moved = [
            (key, before[key], after[key])
            for key in before
            if key in after
            and before[key]
            and after[key]
            and abs(after[key] - before[key] / 2) < 1e-6
        ]
        assert moved, (
            f"no Used value halved after doubling a budget cell: "
            f"before={before}, after={after}"
        )
        print(f"ok: doubling a budget halved its Used value {moved[0][1:]}")
    finally:
        client.close()
    print("PASS")


if __name__ == "__main__":
    main()

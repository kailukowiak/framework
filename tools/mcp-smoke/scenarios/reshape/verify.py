"""Reshape scenario verifier: a real unpivot, the crosstab reading, and a
source edit flowing through to the long frame.

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

    unpivoted = [
        (frame, step)
        for frame in frames.values()
        for step in steps_of(frame)
        if step.get("kind") == "unpivot"
    ]
    assert unpivoted, "no unpivot step — the reshape was faked or skipped"
    long_frame, unpivot = unpivoted[0]
    assert len(unpivot["columns"]) == 6, (
        f"the unpivot melts {len(unpivot['columns'])} columns, expected the 6 months"
    )
    print("ok: a frame unpivots the six month columns")

    source_id = (long_frame.get("derivation") or {}).get("sourceFrameId")
    source = frames.get(source_id) or long_frame
    assert source.get("rows"), "the source frame does not own editable rows"
    print("ok: the source frame is editable (typed rows)")

    crosstab = (long_frame.get("display") or {}).get("crosstab")
    assert crosstab, "no crosstab view on the long frame"
    print("ok: the long frame carries the wide reading")

    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], document_path)
    try:
        snapshot = client.call("get_frame", {"frame": long_frame["name"], "limit": 100})
        assert snapshot["totalRowCount"] == 48, (
            f"expected 48 long rows, found {snapshot['totalRowCount']}"
        )
        print("ok: 48 long rows (8 products × 6 months)")

        # A source edit must land in the long frame. First data row, second
        # column (the first month) of the source.
        target_row = source["rows"][0]
        month_column = source["columns"][1]
        product = target_row["cells"][source["columns"][0]["id"]]["raw"]
        client.call(
            "set_cell",
            {
                "frame": source["name"],
                "row": target_row["id"],
                "column": month_column["id"],
                "raw": "9999",
            },
        )
        snapshot = client.call("get_frame", {"frame": long_frame["name"], "limit": 100})
        landed = any(
            any(cell["display"] == product for cell in row["cells"])
            and any(cell["display"] == "9999" for cell in row["cells"])
            for row in snapshot["rows"]
        )
        assert landed, "the source edit did not reach the long frame"
        print(f"ok: editing {product}'s {month_column['name']} flowed through")
    finally:
        client.close()
    print("PASS")


if __name__ == "__main__":
    main()

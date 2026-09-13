"""NRF calendar scenario verifier.

Grades the *document*, never the agent's self-report:
  - the calendar exists with the exact retail-4-5-4 shape, including
    `weekend: [6, 7]` — this is the field that used to be documented
    backwards (as the days worked rather than the days off), so a
    regression there is exactly what this scenario is meant to catch.
  - it is set as the document's default calendar.
  - `networkdays`/`fiscal_week` computed columns exist and hold the values
    those functions must produce for a Thursday-holiday, Saturday/Sunday
    weekend NRF calendar (values below reconciled against
    crates/framework-core/tests/retail_calendars.rs's own NRF anchors).

Found by shape, never by name, per ../timesheet/verify.py's founding rules.
"""

import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def calls(node, names):
    if isinstance(node, dict):
        name = node.get("name", "")
        if name in names:
            yield name
        for child in node.values():
            yield from calls(child, names)
    elif isinstance(node, list):
        for child in node:
            yield from calls(child, names)


def main():
    path = os.environ["FRAMEWORK_DOCUMENT"]
    with open(path) as file:
        document = json.load(file)["document"]

    calendars = document.get("calendars", [])
    calendar = next(
        (c for c in calendars if c.get("fyStart") == 2 and c.get("pattern") == "fourFiveFour"),
        None,
    )
    assert calendar is not None, f"no February-start 4-5-4 calendar found among {calendars}"

    year_end = calendar.get("yearEnd", {})
    nearest = year_end.get("nearestWeekday") if isinstance(year_end, dict) else None
    assert nearest == {"weekday": 6, "month": 1, "day": 31}, (
        f"year end must be the Saturday nearest January 31, got {year_end}"
    )
    assert calendar.get("yearLabel") == "start", f"NRF labels by the start year, got {calendar.get('yearLabel')}"

    # The exact defect this scenario exists to catch: `weekend` names the
    # days NOT worked. A build that flipped its meaning (documenting it as
    # the days worked) would still write [6, 7] here for a calendar built
    # to look right, but would compute weekday-based functions backwards —
    # caught below by the networkdays/fiscal_week values, not by this
    # field alone.
    assert calendar.get("weekend") == [6, 7], f"expected weekend [6, 7], got {calendar.get('weekend')}"
    assert calendar.get("holidays") == ["2026-01-01"], f"expected one holiday 2026-01-01, got {calendar.get('holidays')}"
    print("ok: NRF calendar shape (Feb start, 4-5-4, Saturday nearest Jan 31, weekend [6, 7], one holiday)")

    assert document.get("defaultCalendarId") == calendar["id"], "the NRF calendar must be set as the document default"
    print("ok: NRF calendar is the document default")

    frames = [o for o in document["objects"] if o["kind"] == "frame"]
    frame = next(
        (f for f in frames if set(calls(f.get("steps", []), {"networkdays", "fiscal_week"})) == {"networkdays", "fiscal_week"}),
        None,
    )
    assert frame is not None, "no frame carries both a networkdays and a fiscal_week computed column"

    outputs = {}
    for step in frame.get("steps", []):
        for column in step.get("columns", []):
            found = list(calls(column, {"networkdays", "fiscal_week"}))
            if found:
                outputs[found[0]] = column["outputColumnId"]
    assert len(outputs) == 2, f"expected exactly one networkdays and one fiscal_week column, found {outputs}"

    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], path)
    try:
        page = client.call("get_frame", {"frame": frame["id"], "limit": 10})
        rows = [{cell["columnId"]: cell["numericValue"] for cell in row["cells"]} for row in page["rows"]]
        assert len(rows) == 3, f"expected the three Start/End rows, found {len(rows)}"

        # Jan 1 2026 (Thursday) is the holiday; Jan 3-4 is the weekend.
        # networkdays: Thu-Wed inclusive minus the holiday Thursday and the
        # weekend counts 4 the first row, -4 reversed, 0 across a bare
        # weekend.
        expected_days = [4, -4, 0]
        for row, expected in zip(rows, expected_days):
            assert row[outputs["networkdays"]] == expected, (
                f"networkdays row mismatch: got {row[outputs['networkdays']]}, expected {expected}"
            )
        print("ok: networkdays excludes the weekend and the 2026-01-01 holiday")

        # fiscal_week under the NRF calendar: FY2025 (labelled by its
        # start) runs 2025-02-02 to 2026-01-31, 52 weeks. Jan 1 and Jan 3
        # 2026 fall in week 48, Jan 7 in week 49.
        expected_weeks = [48, 49, 48]
        for row, expected in zip(rows, expected_weeks):
            assert row[outputs["fiscal_week"]] == expected, (
                f"fiscal_week row mismatch: got {row[outputs['fiscal_week']]}, expected {expected}"
            )
        print("ok: fiscal_week lands in the NRF weeks the default calendar implies")

        print("PASS: NRF calendar is well-formed, defaulted, and drives networkdays/fiscal_week correctly")
    finally:
        client.close()


if __name__ == "__main__":
    main()

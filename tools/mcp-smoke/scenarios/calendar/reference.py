"""Build the NRF calendar scenario through the public MCP surface."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def main():
    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"])
    try:
        client.call("add_calendar", {
            "name": "NRF",
            "fyStart": 2,
            "pattern": "fourFiveFour",
            "yearEnd": {"nearestWeekday": {"weekday": 6, "month": 1, "day": 31}},
            "yearLabel": "start",
            "weekend": [6, 7],
            "holidays": ["2026-01-01"],
        })
        client.call("set_default_calendar", {"calendar": "NRF"})
        client.call("create_frame", {"name": "Spans", "grid": [
            ["Start", "End"],
            ["2026-01-01", "2026-01-07"],
            ["2026-01-07", "2026-01-01"],
            ["2026-01-03", "2026-01-04"],
        ]})
        client.call("add_calculated_column", {
            "frame": "Spans", "name": "Days", "formula": "networkdays(`Start`, `End`)",
        })
        client.call("add_calculated_column", {
            "frame": "Spans", "name": "Week", "formula": "fiscal_week(`Start`)",
        })
    finally:
        client.close()


if __name__ == "__main__":
    main()

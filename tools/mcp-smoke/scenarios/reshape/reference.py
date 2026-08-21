"""Builds the reshape scenario's reference solution through the MCP tools —
no model involved. See ../budget/reference.py for why this exists.
"""

import csv
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def main():
    fixtures = os.path.dirname(os.environ["FRAMEWORK_DOCUMENT"])
    with open(os.path.join(fixtures, "sales_wide.csv")) as file:
        grid = [list(row) for row in csv.reader(file)]
    client = McpClient(
        os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"]
    )
    try:
        client.call("create_frame", {"name": "Sales wide", "grid": grid})
        wide_id = frame_id(client, "Sales wide")
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "addLinkedFrame",
                    "sourceFrameId": wide_id,
                    "name": "Sales long",
                    "x": 0.0,
                    "y": 0.0,
                }
            },
        )
        months = ", ".join(f"`{month}`" for month in grid[0][1:])
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "setFramePipeline",
                    "frameId": frame_id(client, "Sales long"),
                    "steps": [
                        {
                            "kind": "unpivot",
                            "columns": months,
                            "nameColumnId": "month~reference",
                            "nameColumnName": "Month",
                            "valueColumnId": "sales~reference",
                            "valueColumnName": "Sales",
                        }
                    ],
                }
            },
        )
        client.call(
            "set_crosstab",
            {
                "frame": "Sales long",
                "namesColumn": "Month",
                "valuesColumn": "Sales",
            },
        )
    finally:
        client.close()


def frame_id(client, name):
    summary = client.call("inspect_document")
    for candidate in summary["objects"]:
        if candidate["kind"] == "frame" and candidate["name"] == name:
            return candidate["id"]
    raise AssertionError(f"frame {name} not found")


if __name__ == "__main__":
    main()

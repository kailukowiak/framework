"""Build the timesheet scenario's known-good solution through MCP.

The selfcheck is deliberately the same public path a cold agent receives:
named tools for the parameter, generator, expansion, keyed entries, and
crosstab, with only the file import using the canonical operation union.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def main():
    fixtures = os.path.dirname(os.environ["FRAMEWORK_DOCUMENT"])
    client = McpClient(
        os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"]
    )
    try:
        client.call("create_block", {"name": "Parameters"})
        client.call(
            "set_block_source",
            {
                "block": "Parameters",
                "source": "Timesheet date = 2026-09-30",
            },
        )
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "importFrameFromFile",
                    "name": "Entry lines",
                    "path": os.path.join(fixtures, "entry_lines.csv"),
                    "x": 0.0,
                    "y": 0.0,
                }
            },
        )
        client.call(
            "create_generator_frame",
            {
                "name": "Period",
                "formula": (
                    "sequence(`Parameters`.`Timesheet date`.dt.month_start(), "
                    "`Parameters`.`Timesheet date` + 1)"
                ),
                "columnName": "Date",
            },
        )
        client.call(
            "expand_frame",
            {
                "frame": "Entry lines",
                "against": "Period",
                "name": "Sheet",
            },
        )
        client.call(
            "add_entry_column",
            {
                "frame": "Sheet",
                "name": "Hours",
                "dataType": "number",
                "keyColumns": ["Line", "Date"],
            },
        )
        client.call(
            "set_entry_value",
            {
                "frame": "Sheet",
                "column": "Hours",
                "key": ["1", "2026-09-15"],
                "raw": "8",
            },
        )
        client.call(
            "set_entry_value",
            {
                "frame": "Sheet",
                "column": "Hours",
                "key": ["2", "2026-09-03"],
                "raw": "4",
            },
        )
        client.call(
            "set_crosstab",
            {
                "frame": "Sheet",
                "namesColumn": "Date",
                "valuesColumn": "Hours",
            },
        )
    finally:
        client.close()


if __name__ == "__main__":
    main()

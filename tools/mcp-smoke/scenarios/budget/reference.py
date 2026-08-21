"""Builds the budget scenario's reference solution through the MCP tools —
no model involved. Selfcheck runs this, then the scenario's verifier: a
pass proves the task is achievable through the surface AND the verifier
recognizes a correct answer, all for zero tokens.
"""

import csv
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient

BUDGETS = {
    "Travel": 2000,
    "Software": 1500,
    "Hardware": 3000,
    "Training": 1200,
    "Marketing": 2500,
    "Facilities": 3500,
}


def main():
    fixtures = os.path.dirname(os.environ["FRAMEWORK_DOCUMENT"])
    client = McpClient(
        os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"]
    )
    try:
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "importFrameFromFile",
                    "name": "Actuals",
                    "path": os.path.join(fixtures, "actuals.csv"),
                    "x": 0.0,
                    "y": 0.0,
                }
            },
        )
        grid = [["Category", "Monthly budget"]] + [
            [category, str(amount)] for category, amount in BUDGETS.items()
        ]
        client.call("create_frame", {"name": "Budget", "grid": grid})
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "addDerivedFrame",
                    "sourceFrameId": frame_id(client, "Actuals"),
                    "name": "Totals",
                    "groupKeys": [{"name": "Category", "formula": "`Category`"}],
                    "aggregates": [{"name": "Total", "formula": "`Amount`.sum()"}],
                    "maintainOrder": True,
                    "x": 0.0,
                    "y": 0.0,
                }
            },
        )
        client.call("set_unique_key", {"frame": "Budget", "columns": ["Category"]})
        totals = client.call("get_frame", {"frame": "Totals", "limit": 1})
        budget = client.call("get_frame", {"frame": "Budget", "limit": 1})
        column_id = lambda snapshot, name: next(
            column["id"] for column in snapshot["columns"] if column["name"] == name
        )
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "addJoinFrame",
                    "primaryFrameId": frame_id(client, "Totals"),
                    "lookupFrameId": frame_id(client, "Budget"),
                    "primaryKeyColumnIds": [column_id(totals, "Category")],
                    "lookupKeyColumnIds": [column_id(budget, "Category")],
                    "joinType": "left",
                    # The outputs, both sides spelled out: a join's columns
                    # are declared, not inherited.
                    "columns": [
                        {
                            "sourceFrameId": frame_id(client, "Totals"),
                            "sourceColumnId": column_id(totals, "Category"),
                            "name": "Category",
                        },
                        {
                            "sourceFrameId": frame_id(client, "Totals"),
                            "sourceColumnId": column_id(totals, "Total"),
                            "name": "Total",
                        },
                        {
                            "sourceFrameId": frame_id(client, "Budget"),
                            "sourceColumnId": column_id(budget, "Monthly budget"),
                            "name": "Monthly budget",
                        },
                    ],
                    "name": "Budget check",
                    "x": 0.0,
                    "y": 0.0,
                }
            },
        )
        client.call(
            "add_calculated_column",
            {
                "frame": "Budget check",
                "name": "Used",
                "formula": '(`Total` / (`Monthly budget` * 5)).show("percent")',
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

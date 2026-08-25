"""Build the vectors-and-joins reference solution through MCP only."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def frame_id(client, name):
    summary = client.call("inspect_document")
    for candidate in summary["objects"]:
        if candidate["kind"] == "frame" and candidate["name"] == name:
            return candidate["id"]
    raise AssertionError(f"frame {name} not found")


def object_id(client, name, kind):
    summary = client.call("inspect_document")
    for candidate in summary["objects"]:
        if candidate["kind"] == kind and candidate["name"] == name:
            return candidate["id"]
    raise AssertionError(f"{kind} {name} not found")


def columns(client, name):
    snapshot = client.call("get_frame", {"frame": name, "limit": 1})
    return {column["name"]: column["id"] for column in snapshot["columns"]}


def main():
    client = McpClient(
        os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"]
    )
    try:
        client.call(
            "create_frame",
            {
                "name": "Launch inputs",
                "grid": [
                    ["Line", "SKU", "Units", "Launch month"],
                    ["1", "A-100", "120", "2026-09-01"],
                    ["2", "B-200", "90", "2026-10-01"],
                    ["3", "C-300", "75", ""],
                    ["4", "A-100", "140", ""],
                    ["5", "D-400", "60", ""],
                    ["6", "B-200", "110", ""],
                ],
            },
        )
        source_id = frame_id(client, "Launch inputs")
        source_columns = columns(client, "Launch inputs")
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "addLinkedFrame",
                    "sourceFrameId": source_id,
                    "name": "Launch plan",
                    "x": 0.0,
                    "y": 0.0,
                }
            },
        )
        plan_id = frame_id(client, "Launch plan")
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "setFramePipeline",
                    "frameId": plan_id,
                    "steps": [
                        {
                            "kind": "sort",
                            "keys": [
                                {
                                    "columnId": source_columns["Line"],
                                    "descending": False,
                                }
                            ],
                        },
                        {
                            "kind": "withColumns",
                            "columns": [
                                {
                                    "outputColumnId": source_columns["Launch month"],
                                    "name": "Launch month",
                                    "formula": "sequence(2026-09-01, periods=frame.len(), step=1mo)",
                                }
                            ],
                        },
                    ],
                }
            },
        )

        client.call(
            "create_frame",
            {
                "name": "Product catalog",
                "grid": [
                    ["SKU", "Product", "Region", "Unit price"],
                    ["A-100", "Aurora", "West", "250"],
                    ["B-200", "Boreal", "East", "180"],
                    ["C-300", "Cedar", "North", "320"],
                    ["D-400", "Delta", "South", "210"],
                ],
            },
        )
        client.call(
            "set_unique_key", {"frame": "Product catalog", "columns": ["SKU"]}
        )
        plan_columns = columns(client, "Launch plan")
        catalog_columns = columns(client, "Product catalog")
        catalog_id = frame_id(client, "Product catalog")
        join_columns = [
            {
                "sourceFrameId": plan_id,
                "sourceColumnId": column_id,
                "name": name,
            }
            for name, column_id in plan_columns.items()
        ] + [
            {
                "sourceFrameId": catalog_id,
                "sourceColumnId": catalog_columns[name],
                "name": name,
            }
            for name in ("Product", "Region", "Unit price")
        ]
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "addJoinFrame",
                    "primaryFrameId": plan_id,
                    "lookupFrameId": catalog_id,
                    "primaryKeyColumnIds": [plan_columns["SKU"]],
                    "lookupKeyColumnIds": [catalog_columns["SKU"]],
                    "joinType": "left",
                    "columns": join_columns,
                    "name": "Scheduled launches",
                    "x": 0.0,
                    "y": 0.0,
                }
            },
        )
        client.call(
            "add_calculated_column",
            {
                "frame": "Scheduled launches",
                "name": "Revenue",
                "formula": "`Units` * `Unit price`",
            },
        )

        client.call(
            "create_generator_frame",
            {
                "name": "Scenarios",
                "formula": '["Base", "Upside", "Downside"]',
                "columnName": "Scenario",
            },
        )
        scenario_id = frame_id(client, "Scenarios")
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "setFramePipeline",
                    "frameId": scenario_id,
                    "steps": [
                        {
                            "kind": "zipVector",
                            "outputColumnId": "scenario-multiplier",
                            "name": "Multiplier",
                            "vector": "[1, 1.15, 0.85]",
                        }
                    ],
                }
            },
        )

        variables = [
            ("Scenario", '["Base", "Upside", "Downside"]'),
            ("Multiplier", "[1, 1.15, 0.85]"),
            ("Quarter", '["Q1", "Q2", "Q3", "Q4"]'),
            ("Base revenue", "[100, 110, 120, 130]"),
        ]
        for index, (name, formula) in enumerate(variables):
            client.call(
                "apply_operation",
                {
                    "operation": {
                        "type": "addVariable",
                        "name": name,
                        "formula": formula,
                        "x": 800.0,
                        "y": 100.0 + index * 70.0,
                    }
                },
            )
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "addCalculationMatrix",
                    "name": "Scenario × Quarter",
                    "x": 800.0,
                    "y": 400.0,
                }
            },
        )
        matrix_id = object_id(client, "Scenario × Quarter", "calculationMatrix")
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "setCalculationMatrix",
                    "objectId": matrix_id,
                    "rows": [
                        {"name": "Scenario", "formula": "`Scenario`"},
                        {"name": "Multiplier", "formula": "`Multiplier`"},
                    ],
                    "columns": [
                        {"name": "Quarter", "formula": "`Quarter`"},
                        {
                            "name": "Base revenue",
                            "formula": "`Base revenue`",
                        },
                    ],
                    "body": "(`Base revenue` * `Multiplier`).round(2).cast(\"string\") + \" {}\".format(`Quarter`)",
                }
            },
        )
    finally:
        client.close()


if __name__ == "__main__":
    main()

"""Build the scenarios-and-sensitivity model through the public MCP surface.

Container and value creation have no dedicated tool yet, so this reaches
them through `apply_operation` — the same escape hatch the finance and
timesheet twins use for CSV import. Scenario creation itself goes through
the named `add_scenario` tool this phase adds.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def main():
    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"])
    try:
        # The demo document already carries a block named "Assumptions" (its
        # one hard-coded tax rate); this build reuses that name for its own
        # container, so the demo's block has to go first — and "Orders"
        # reads that block's tax line, so it has to go before the block does.
        client.call("delete_object", {"object": "Orders"})
        client.call("delete_object", {"object": "Assumptions"})
        client.call("apply_operation", {
            "operation": {"type": "addContainer", "name": "Assumptions", "x": 0.0, "y": 0.0},
        })
        summary = client.call("inspect_document", {})
        container_id = next(o["id"] for o in summary["objects"] if o["name"] == "Assumptions")
        for name, raw in [
            ("Price", "120"), ("Annual units", "8000"),
            ("Unit cost", "70"), ("Fixed costs", "250000"),
        ]:
            client.call("apply_operation", {
                "operation": {
                    "type": "addValue", "name": name, "raw": raw,
                    "x": 0.0, "y": 0.0, "containerId": container_id,
                },
            })

        client.call("add_scenario", {
            "name": "Upside",
            "values": [
                {"value": "Price", "raw": "125"},
                {"value": "Annual units", "raw": "9500"},
            ],
        })
        client.call("add_scenario", {
            "name": "Downside",
            "values": [
                {"value": "Price", "raw": "115"},
                {"value": "Annual units", "raw": "6500"},
            ],
        })

        client.call("create_frame", {"name": "Plan", "grid": [
            ["Weight"],
            ["6"], ["6"], ["7"], ["8"], ["9"], ["9"],
            ["9"], ["9"], ["8"], ["9"], ["10"], ["10"],
        ]})
        for name, formula in [
            ("Units", "`Annual units` * `Weight` / 100"),
            ("Revenue", "`Units` * `Price`"),
            ("Cost", "`Units` * `Unit cost`"),
        ]:
            client.call("add_calculated_column", {"frame": "Plan", "name": name, "formula": formula})

        client.call("create_block", {"name": "Model"})
        client.call("set_block_source", {
            "block": "Model",
            "source": "\n".join([
                "revenue = `Plan`.`Revenue`.sum()",
                "gross = revenue - `Plan`.`Cost`.sum()",
                "ebitda = gross - `Fixed costs`",
                "margin = ebitda / revenue",
                "upside ebitda = under(`Upside`, ebitda)",
                "downside ebitda = under(`Downside`, ebitda)",
                "target price = solve(ebitda == 300000, by=`Price`, within=[100, 200])",
            ]),
        })
    finally:
        client.close()


if __name__ == "__main__":
    main()

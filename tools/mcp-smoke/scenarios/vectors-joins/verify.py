"""Verify generated vectors, a row-count date series, and a live keyed join."""

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
    variables = {
        result["name"]: result
        for result in document["objects"]
        if result["kind"] == "result" and result.get("variable")
    }

    def steps_of(frame):
        derivation = frame.get("derivation") or {}
        return list(derivation.get("steps", [])) + list(frame.get("steps", []))

    paired = [
        frame
        for frame in frames.values()
        if frame.get("generator")
        and any(step.get("kind") == "zipVector" for step in steps_of(frame))
    ]
    assert paired, "no generated frame retains a paired-vector/zip step"
    print("ok: a generated vector is paired with a second vector")

    joined = None
    join = None
    for frame in frames.values():
        candidate = (frame.get("derivation") or {}).get("join")
        if candidate:
            joined, join = frame, candidate
            break
    assert joined and join, "no joined launch result exists"
    assert join["joinType"] == "left", "the launch join does not keep every source row"
    lookup = frames[join["lookupFrameId"]]
    assert lookup.get("rows"), "the lookup is not an editable literal table"
    assert lookup.get("uniqueKeys"), "the lookup SKU is not declared unique"
    plan = frames[(joined.get("derivation") or {})["sourceFrameId"]]
    source_id = (plan.get("derivation") or {}).get("sourceFrameId")
    assert source_id in frames, "the dated launch plan is not live from an upstream table"
    source = frames[source_id]
    assert source.get("rows") and len(source["rows"]) == 6, (
        "the editable launch source must remain at six rows for the growth check"
    )
    serialized_steps = json.dumps(steps_of(plan), separators=(",", ":"))
    assert "sequence" in serialized_steps and "frame" in serialized_steps, (
        "the launch date is not a sequence tied to frame length"
    )
    assert any(step.get("kind") == "sort" for step in steps_of(plan)), (
        "the row-wise date series has no declared sort"
    )
    print("ok: the launch calendar is an ordered, row-count-aware derivation")

    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], document_path)
    try:
        paired_snapshot = client.call(
            "get_frame", {"frame": paired[0]["name"], "limit": 20}
        )
        assert len(paired_snapshot["rows"]) == 3, "the scenario table is not three rows"
        scenario_values = {
            cell["columnName"]: cell["display"]
            for cell in paired_snapshot["rows"][1]["cells"]
        }
        assert scenario_values == {"Scenario": "Upside", "Multiplier": "1.15"}, (
            f"the paired values are wrong: {scenario_values}"
        )
        print("ok: the paired-vector table contains the intended row-wise values")

        expected_variables = {
            "Scenario": '["Base", "Upside", "Downside"]',
            "Multiplier": "[1, 1.15, 0.85]",
            "Quarter": '["Q1", "Q2", "Q3", "Q4"]',
            "Base revenue": "[100, 110, 120, 130]",
        }
        assert set(expected_variables).issubset(variables), (
            "the tutorial's four standalone vector Variables do not all exist"
        )
        for name in expected_variables:
            assert variables[name]["formula"]["expression"], f"{name} has no formula"
            assert variables[name].get("variable"), f"{name} is not a Variable"

        matrices = [
            item for item in document["objects"] if item["kind"] == "calculationMatrix"
        ]
        assert len(matrices) == 1, "expected one Calculation Matrix"
        matrix = matrices[0]
        assert [axis["name"] for axis in matrix["rows"]] == [
            "Scenario",
            "Multiplier",
        ], "the Matrix row axis is not the tutorial's zipped pair"
        assert [axis["name"] for axis in matrix["columns"]] == [
            "Quarter",
            "Base revenue",
        ], "the Matrix column axis is not the tutorial's zipped pair"
        expected_body = "(`Base revenue` * `Multiplier`).round(2).cast(\"string\") + \" {}\".format(`Quarter`)"
        assert matrix["body"]["source"] == expected_body, (
            "the Matrix does not use the chained-format body formula"
        )
        summary = client.call("inspect_document")
        matrix_summary = next(
            item for item in summary["objects"] if item["id"] == matrix["id"]
        )
        assert matrix_summary["value"] is None, (
            f"the Calculation Matrix reports an error: {matrix_summary['value']}"
        )
        assert matrix_summary["rowCount"] == 12, (
            "the Calculation Matrix is not the expected live 3 × 4 result"
        )

        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "setResultFormula",
                    "objectId": variables["Quarter"]["id"],
                    "formula": '["Q1", "Q2"]',
                }
            },
        )
        client.call(
            "apply_operation",
            {
                "operation": {
                    "type": "setResultFormula",
                    "objectId": variables["Base revenue"]["id"],
                    "formula": "[100, 110]",
                }
            },
        )
        shortened = client.call("inspect_document")
        shortened_matrix = next(
            item for item in shortened["objects"] if item["id"] == matrix["id"]
        )
        assert shortened_matrix["rowCount"] == 6, (
            "the Matrix did not shrink live when both column vectors changed"
        )
        client.call("undo")
        client.call("undo")
        restored = client.call("inspect_document")
        restored_matrix = next(
            item for item in restored["objects"] if item["id"] == matrix["id"]
        )
        assert restored_matrix["rowCount"] == 12, (
            "undo did not restore the Matrix's 3 × 4 result"
        )
        print("ok: the tutorial Calculation Matrix is live and chained formatting runs")

        plan_before = client.call("get_frame", {"frame": plan["name"], "limit": 20})
        join_before = client.call("get_frame", {"frame": joined["name"], "limit": 20})
        assert len(plan_before["rows"]) == 6 and len(join_before["rows"]) == 6
        last_before = cell_display(plan_before["rows"][-1], "Launch month")
        assert last_before == "2027-02-01", f"unexpected sixth date: {last_before}"
        assert revenue_sum(join_before) == 137600, "initial joined revenue is wrong"

        client.call(
            "add_row",
            {
                "frame": source["name"],
                "values": {"Line": "7", "SKU": "C-300", "Units": "50"},
            },
        )
        plan_after = client.call("get_frame", {"frame": plan["name"], "limit": 20})
        join_after = client.call("get_frame", {"frame": joined["name"], "limit": 20})
        assert len(plan_after["rows"]) == 7, "the date frame did not grow upstream"
        assert cell_display(plan_after["rows"][-1], "Launch month") == "2027-03-01", (
            "the seventh date was not calculated from frame.len()"
        )
        assert len(join_after["rows"]) == 7, "the keyed join did not grow upstream"
        last = join_after["rows"][-1]
        assert cell_display(last, "Product") == "Cedar"
        assert cell_display(last, "Region") == "North"
        assert revenue_sum(join_after) == 153600, "joined revenue did not recompute"
        print("ok: adding one source row grows the date series and joined result live")
    finally:
        client.close()
    print("PASS")


def cell_display(row, column_name):
    return next(
        cell["display"] for cell in row["cells"] if cell["columnName"] == column_name
    )


def revenue_sum(snapshot):
    return sum(
        cell["numericValue"]
        for row in snapshot["rows"]
        for cell in row["cells"]
        if cell["columnName"] == "Revenue"
    )


if __name__ == "__main__":
    main()

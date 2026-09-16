"""Scenarios-and-sensitivity verifier.

Grades the *document*, never the agent's self-report:
  - two scenarios, Upside and Downside, override exactly Price and Annual
    units to the right numbers, and the document is still on Base.
  - a Plan frame carries the twelve seasonality weights and the Units /
    Revenue / Cost calculated columns the tutorial's section 1 builds.
  - a block carries `under(...)` calls against both scenarios and a
    `solve(...)` goal seek, found by shape (the functions it calls), never
    by the object's name, per ../timesheet/verify.py's founding rule — and
    its computed answers, read back through inspect_document the way
    ../returns/verify.py reads a block's answers, match the tutorial's
    checkpoints: 272500 (Upside), 42500 (Downside), 138.75 (target price).
"""
import json
import math
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


def block_answers(client, block_id):
    summary = client.call("inspect_document", {})
    value = next(o["value"] for o in summary["objects"] if o["id"] == block_id)
    answers = {}
    for entry in value.split("; "):
        if "→" not in entry:
            continue
        left, right = entry.rsplit("→", 1)
        answers[left.split("=", 1)[0].strip()] = right.strip()
    return answers


def main():
    path = os.environ["FRAMEWORK_DOCUMENT"]
    with open(path) as file:
        document = json.load(file)["document"]

    assert document.get("activeScenario") is None, (
        "the document must stay on Base — under()/solve() must never activate a scenario"
    )

    scenarios = document.get("scenarios", [])
    scenario_names = {scenario["name"] for scenario in scenarios}
    assert scenario_names == {"Upside", "Downside"}, (
        f"expected exactly Upside and Downside scenarios, found {scenario_names}"
    )

    values = {o["name"]: o for o in document["objects"] if o["kind"] == "value"}
    for name in ["Price", "Annual units", "Unit cost", "Fixed costs"]:
        assert name in values, f"missing assumption value '{name}'"
    price_id = values["Price"]["id"]
    units_id = values["Annual units"]["id"]

    upside = next(s for s in scenarios if s["name"] == "Upside")
    downside = next(s for s in scenarios if s["name"] == "Downside")
    assert upside["values"].get(price_id) == "125", f"Upside Price mismatch: {upside['values']}"
    assert upside["values"].get(units_id) == "9500", f"Upside Annual units mismatch: {upside['values']}"
    assert downside["values"].get(price_id) == "115", f"Downside Price mismatch: {downside['values']}"
    assert downside["values"].get(units_id) == "6500", f"Downside Annual units mismatch: {downside['values']}"
    print("ok: Upside and Downside override exactly Price and Annual units")

    frames = [o for o in document["objects"] if o["kind"] == "frame"]
    plan = next(f for f in frames if any(c["name"] == "Weight" for c in f["columns"]) and len(f["rows"]) == 12)
    weight_column = next(c for c in plan["columns"] if c["name"] == "Weight")
    weights = [row["cells"][weight_column["id"]]["raw"] for row in plan["rows"]]
    assert weights == ["6", "6", "7", "8", "9", "9", "9", "9", "8", "9", "10", "10"], (
        f"Plan's Weight column should hold the tutorial's seasonality weights, got {weights}"
    )
    column_names_by_id = {c["id"]: c["name"] for c in plan["columns"]}
    output_names = set()
    for step in plan.get("steps", []):
        if step.get("kind") != "withColumns":
            continue
        for column in step.get("columns", []):
            name = column_names_by_id.get(column["outputColumnId"])
            if name:
                output_names.add(name)
    assert {"Units", "Revenue", "Cost"} <= output_names, (
        f"Plan needs Units, Revenue and Cost calculated columns, found {output_names}"
    )
    print("ok: Plan carries the twelve weights and the Units/Revenue/Cost columns")

    blocks = [o for o in document["objects"] if o["kind"] == "block"]
    block = next(
        (b for b in blocks if set(calls(b.get("lines", []), {"under", "solve"})) == {"under", "solve"}),
        None,
    )
    assert block is not None, "no block carries both an under() and a solve() line"

    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], path)
    try:
        answers = block_answers(client, block["id"])
        expected = {
            "upside ebitda": 272500.0,
            "downside ebitda": 42500.0,
            "target price": 138.75,
        }
        for name, target in expected.items():
            assert name in answers, f"no block line named '{name}', found {list(answers)}"
            actual = float(answers[name])
            assert math.isclose(actual, target, abs_tol=0.01), (
                f"'{name}' should read {target}, got {actual}"
            )
        print("ok: under(Upside/Downside, ebitda) and the price goal seek match the tutorial's checkpoints")
        print("PASS: scenarios override correctly, under() and solve() compute the section-5 checkpoints, and Base stays active")
    finally:
        client.close()


if __name__ == "__main__":
    main()

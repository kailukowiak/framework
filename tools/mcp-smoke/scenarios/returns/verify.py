"""Inspect formula shape, then change persisted cash flows and check both returns."""
import json
import math
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def calls(node):
    if isinstance(node, dict):
        name = node.get("name", "").lower().removeprefix("finance.")
        path = node.get("path", [])
        if len(path) == 2 and path[0].lower() == "finance":
            name = path[1].lower()
        if name in {"irr", "xirr"}:
            yield name
        for child in node.values():
            yield from calls(child)
    elif isinstance(node, list):
        for child in node:
            yield from calls(child)


def main():
    path = os.environ["FRAMEWORK_DOCUMENT"]
    with open(path) as file:
        objects = json.load(file)["document"]["objects"]
    block = next(o for o in objects if o["kind"] == "block" and set(calls(o)) == {"irr", "xirr"})
    frame = next(o for o in objects if o["kind"] == "frame" and len(o["rows"]) == 2
                 and any(c["raw"] == "121" for r in o["rows"] for c in r["cells"].values()))
    row = next(r for r in frame["rows"] if any(c["raw"] == "121" for c in r["cells"].values()))
    column = next(k for k, c in row["cells"].items() if c["raw"] == "121")
    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], path)
    try:
        def check(expected):
            summary = client.call("inspect_document", {})
            value = next(o["value"] for o in summary["objects"] if o["id"] == block["id"])
            answers = []
            for line in value.split("; "):
                if "→" in line:
                    answer = line.rsplit("→", 1)[1].strip()
                    answers.append(float(answer.rstrip("%")) / (100 if answer.endswith("%") else 1))
            for target in expected:
                assert any(math.isclose(a, target, abs_tol=1e-6) for a in answers), (expected, value)

        check([0.1, 0.21])
        client.call("set_cell", {"frame": frame["id"], "row": row["id"], "column": column, "raw": "144"})
        check([0.2, 0.44])
        client.call("undo", {})
        check([0.1, 0.21])
        print("PASS: persisted IRR and XIRR follow cash-flow edits and undo")
    finally:
        client.close()


if __name__ == "__main__":
    main()

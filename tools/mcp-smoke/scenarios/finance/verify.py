"""Find formulas by shape; verify amounts and live source changes over MCP."""
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
        if name in {"pmt", "ipmt", "ppmt"}:
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
    frame = next(o for o in objects if o["kind"] == "frame" and set(calls(o)) == {"pmt", "ipmt", "ppmt"})
    assert len(frame["rows"]) == 2, "two editable loans are required"
    outputs = {}
    for step in frame.get("steps", []):
        for column in step.get("columns", []):
            found = list(calls(column))
            if found:
                outputs[found[0]] = column["outputColumnId"]
    assert len(outputs) == 3, "each financial formula needs a calculated column"
    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], path)
    try:
        def values():
            page = client.call("get_frame", {"frame": frame["id"], "limit": 10})
            return [{cell["columnId"]: cell["numericValue"] for cell in row["cells"]} for row in page["rows"]]

        before = values()
        expected = [(7733.120611, 2000, 5733.120611), (10000, 0, 10000)]
        for row, targets in zip(before, expected):
            for function, target in zip(["pmt", "ipmt", "ppmt"], targets):
                assert math.isclose(row[outputs[function]], target, abs_tol=0.005)
        principal_id = next(key for key, cell in frame["rows"][0]["cells"].items() if cell["raw"] == "400000")
        client.call("set_cell", {"frame": frame["id"], "row": frame["rows"][0]["id"], "column": principal_id, "raw": "200000"})
        after = values()
        for column in outputs.values():
            assert math.isclose(after[0][column], before[0][column] / 2, abs_tol=0.005)
        client.call("undo", {})
        for column in outputs.values():
            assert math.isclose(values()[0][column], before[0][column], abs_tol=0.005)
        print("PASS: native finance formulas reconcile and follow editable assumptions and undo")
    finally:
        client.close()


if __name__ == "__main__":
    main()

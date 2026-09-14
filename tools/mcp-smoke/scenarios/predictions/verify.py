"""Verify a scored frame that carries its source, paired live beside a sibling."""

import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def steps_of(frame):
    derivation = frame.get("derivation") or {}
    return list(derivation.get("steps", [])) + list(frame.get("steps", []))


def main():
    document_path = os.environ["FRAMEWORK_DOCUMENT"]
    with open(document_path) as file:
        document = json.load(file)["document"]
    frames = {f["id"]: f for f in document["objects"] if f["kind"] == "frame"}

    models = [item for item in document["objects"] if item["kind"] == "model"]
    assert any(model.get("fitted") for model in models), "no fitted model exists"
    print("ok: a fitted model exists")

    scored_frames = [frame for frame in frames.values() if frame.get("prediction")]
    assert len(scored_frames) == 1, "expected exactly one prediction frame"
    scored = scored_frames[0]
    binding = scored["prediction"]
    source = frames[scored["derivation"]["sourceFrameId"]]
    assert source.get("rows") and len(source["rows"]) == 8, (
        "the editable source must remain at eight rows for the growth check"
    )
    scored_ids = [column["id"] for column in scored["columns"]]
    assert all(column["id"] in scored_ids for column in source["columns"]), (
        "the prediction frame does not carry its source's columns"
    )
    assert all(output in scored_ids for output in binding["outputColumnIds"]), (
        "the prediction frame is missing the model's outputs"
    )
    assert scored.get("materialization") is None, (
        "the prediction frame was materialized; it has to be read live"
    )
    output_name = next(
        column["name"]
        for column in scored["columns"]
        if column["id"] == binding["outputColumnIds"][0]
    )
    print("ok: the prediction frame is a scored table, read live")

    paired, pair_step = None, None
    for frame in frames.values():
        if frame["id"] == scored["id"]:
            continue
        for step in steps_of(frame):
            if step.get("kind") == "zipVector" and scored["id"] in json.dumps(step):
                paired, pair_step = frame, step
    assert paired and pair_step, "no frame pairs the prediction column with a zip step"
    assert (paired.get("derivation") or {}).get("sourceFrameId") == source["id"], (
        "the paired frame is not a live frame built from the scoring source"
    )
    paired_column = next(
        column["name"]
        for column in paired["columns"]
        if column["id"] == pair_step["outputColumnId"]
    )
    print("ok: a live frame from the source pairs the prediction column")

    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], document_path)
    try:

        def column_values(frame_name, column_name):
            snapshot = client.call("get_frame", {"frame": frame_name, "limit": 50})
            return [
                next(
                    cell["display"]
                    for cell in row["cells"]
                    if cell["columnName"] == column_name
                )
                for row in snapshot["rows"]
            ]

        before = column_values(scored["name"], output_name)
        assert len(before) == 8, "the prediction frame does not score every source row"
        assert column_values(paired["name"], paired_column) == before, (
            "the paired column does not show the prediction row for row"
        )
        print("ok: the paired column matches the predictions row for row")

        client.call(
            "add_row",
            {"frame": source["name"], "values": {binding["featureColumnIds"][0]: "90"}},
        )
        after_scored = column_values(scored["name"], output_name)
        after_paired = column_values(paired["name"], paired_column)
        assert len(after_scored) == 9 and len(after_paired) == 9, (
            "the ninth source row was not scored and paired live"
        )
        assert after_scored[:8] == before, "existing predictions changed when a row was added"
        assert after_paired == after_scored, "the paired column drifted from the predictions"
        snapshot = client.call("get_frame", {"frame": scored["name"], "limit": 50})
        newest = next(
            cell for cell in snapshot["rows"][8]["cells"] if cell["columnName"] == output_name
        )
        value = newest.get("numericValue")
        assert value is not None and abs(value - 230) < 15, (
            f"the ninth prediction is not near the fitted line: {value}"
        )
        print("ok: a ninth source row is scored and paired live")
    finally:
        client.close()


if __name__ == "__main__":
    main()

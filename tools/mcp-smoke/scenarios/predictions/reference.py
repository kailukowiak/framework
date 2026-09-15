"""Build the predictions reference solution through MCP only."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def object_id(client, name, kind):
    summary = client.call("inspect_document")
    for candidate in summary["objects"]:
        if candidate["kind"] == kind and candidate["name"] == name:
            return candidate["id"]
    raise AssertionError(f"{kind} {name} not found")


def columns(client, name):
    snapshot = client.call("get_frame", {"frame": name, "limit": 1})
    return {column["name"]: column["id"] for column in snapshot["columns"]}


def apply(client, operation):
    client.call("apply_operation", {"operation": operation})


def main():
    client = McpClient(
        os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"]
    )
    try:
        spend = [10, 20, 30, 40, 50, 60, 70, 80]
        sales = [72, 88, 112, 131, 149, 172, 190, 211]
        client.call(
            "create_frame",
            {
                "name": "Campaigns",
                "grid": [["Spend", "Sales"]]
                + [[str(x), str(y)] for x, y in zip(spend, sales)],
            },
        )
        source_id = object_id(client, "Campaigns", "frame")
        source_columns = columns(client, "Campaigns")
        apply(
            client,
            {
                "type": "addModel",
                "name": "Sales model",
                "x": 400.0,
                "y": 0.0,
                "spec": {
                    "sourceFrameId": source_id,
                    "targetColumnId": source_columns["Sales"],
                    "featureColumnIds": [source_columns["Spend"]],
                    "method": "ols",
                    "covariance": "classical",
                    "confidenceLevel": 0.95,
                    "holdoutFraction": 0.0,
                    "seed": 42,
                },
            },
        )
        model_id = object_id(client, "Sales model", "model")
        apply(client, {"type": "fitModel", "modelId": model_id})
        apply(
            client,
            {
                "type": "addModelPredictions",
                "modelId": model_id,
                "sourceFrameId": source_id,
                "featureColumnIds": [source_columns["Spend"]],
                "name": "Predictions",
                "x": 400.0,
                "y": 400.0,
            },
        )
        apply(
            client,
            {
                "type": "addLinkedFrame",
                "sourceFrameId": source_id,
                "name": "Scored",
                "x": 0.0,
                "y": 400.0,
            },
        )
        scored_id = object_id(client, "Scored", "frame")
        apply(
            client,
            {
                "type": "setFramePipeline",
                "frameId": scored_id,
                "steps": [
                    {
                        "kind": "zipVector",
                        "outputColumnId": "predicted",
                        "name": "Predicted",
                        "vector": "`Predictions`.`Prediction`",
                    }
                ],
            },
        )
    finally:
        client.close()


if __name__ == "__main__":
    main()

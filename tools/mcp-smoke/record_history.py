"""Append one compact, reviewable agent-run result to the tracked history."""

import json
import os
import sys
from datetime import datetime, timezone


def main():
    scenario, model, verdict, run_directory, metrics_path = sys.argv[1:]
    with open(metrics_path) as file:
        metrics = json.load(file)
    metrics.pop("final_report", None)
    record = {
        "recordedAt": datetime.now(timezone.utc).isoformat(),
        "scenario": scenario,
        "model": model,
        "verdict": verdict,
        "run": os.path.basename(run_directory),
        **metrics,
    }
    history_path = os.path.join(os.path.dirname(__file__), "history.jsonl")
    with open(history_path, "a") as history:
        history.write(json.dumps(record, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()

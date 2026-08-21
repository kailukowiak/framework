"""Distills an agent transcript (claude -p --output-format stream-json)
into the numbers worth trending across runs.

The friction score of a run is not what the agent says about itself — it
is how much work the run took: calls made, errors hit, escape hatches
reached for. A run that "succeeds" through forty errors and an
apply_operation crutch scores worse than one that walks through six named
tools, and only these numbers make that visible across model versions and
tool-surface changes.
"""

import json
import sys
from collections import Counter


def metrics(transcript_path):
    calls = Counter()
    errors = 0
    result = {}
    with open(transcript_path) as transcript:
        for line in transcript:
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("type") == "assistant":
                for block in event.get("message", {}).get("content", []):
                    if block.get("type") == "tool_use":
                        calls[block.get("name", "?")] += 1
            elif event.get("type") == "user":
                content = event.get("message", {}).get("content", [])
                if isinstance(content, list):
                    for block in content:
                        if block.get("type") == "tool_result" and block.get("is_error"):
                            errors += 1
            elif event.get("type") == "result":
                result = event
    framework_calls = {
        name: count for name, count in calls.items() if "framework" in name
    }
    return {
        "tool_calls_total": sum(calls.values()),
        "framework_calls": sum(framework_calls.values()),
        "escape_hatch_calls": sum(
            count
            for name, count in framework_calls.items()
            if name.endswith("apply_operation")
        ),
        "tool_errors": errors,
        "calls_by_tool": dict(sorted(framework_calls.items())),
        "duration_ms": result.get("duration_ms"),
        "total_cost_usd": result.get("total_cost_usd"),
        "final_report": result.get("result", ""),
    }


if __name__ == "__main__":
    print(json.dumps(metrics(sys.argv[1]), indent=2))

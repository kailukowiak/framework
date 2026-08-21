#!/bin/bash
# Agent-lane MCP smoke test: hand a fresh model nothing but a scenario
# prompt and the framework-mcp server, then grade the run on the document
# it produced — never on what it says about itself.
#
#   tools/mcp-smoke/run.sh <scenario> [model]
#
# Produces tools/mcp-smoke/runs/<scenario>-<model>-<timestamp>/ holding the
# transcript, the built document, metrics.json, report.md, and verify.txt.
# Exit code is the verifier's. Requires the claude CLI (a signed-in
# session; runs cost model usage) and the machine-local MCP switch the
# desktop app writes when MCP is enabled in Settings.
#
# The deterministic lane lives in `cargo test -p framework-mcp` and runs
# free on every build; this lane exists to measure what that one cannot —
# whether a cold agent can *discover* the way through the tool surface.
set -euo pipefail

SELFCHECK=""
if [ "${1:-}" = "--selfcheck" ]; then
	SELFCHECK=1
	shift
fi
SCENARIO="${1:?usage: run.sh [--selfcheck] <scenario|all> [model]}"
MODEL="${2:-sonnet}"
ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"

build_server() {
	if [ -n "${FRAMEWORK_MCP_BINARY:-}" ] && [ -x "$FRAMEWORK_MCP_BINARY" ]; then
		BINARY="$FRAMEWORK_MCP_BINARY"
		return
	fi
	echo "building framework-mcp..."
	cargo build -p framework-mcp --manifest-path "$REPO/Cargo.toml" >/dev/null
	BINARY="$REPO/target/debug/framework-mcp"
}

# `all` fans out over every scenario — the occasional broad sweep. With
# --selfcheck it uses no model tokens and belongs after any surface change;
# without, each scenario is a model run, so save it for milestones.
if [ "$SCENARIO" = "all" ]; then
	build_server
	status=0
	results=""
	for candidate in "$ROOT"/scenarios/*/; do
		name="$(basename "$candidate")"
		if [ -n "$SELFCHECK" ]; then
			if FRAMEWORK_MCP_BINARY="$BINARY" "$0" --selfcheck "$name" "$MODEL"; then
				verdict="PASS"
			else
				verdict="FAIL"
				status=1
			fi
		else
			if FRAMEWORK_MCP_BINARY="$BINARY" "$0" "$name" "$MODEL"; then
				verdict="PASS"
			else
				verdict="FAIL"
				status=1
			fi
		fi
		results="${results}${name}\t${verdict}\n"
	done
	echo
	echo "SUMMARY"
	printf "%b" "$results"
	exit $status
fi

SCENARIO_DIR="$ROOT/scenarios/$SCENARIO"
[ -d "$SCENARIO_DIR" ] || {
	echo "unknown scenario: $SCENARIO" >&2
	exit 2
}

STAMP="$(date +%Y%m%d-%H%M%S)"
RUN_KIND="$MODEL"
if [ -n "$SELFCHECK" ]; then RUN_KIND="selfcheck"; fi
RUN_DIR="$ROOT/runs/$SCENARIO-$RUN_KIND-$STAMP"
mkdir -p "$RUN_DIR"

build_server

cp "$SCENARIO_DIR"/fixtures/* "$RUN_DIR"/ 2>/dev/null || true
DOCUMENT="$RUN_DIR/document.fw"
cat >"$RUN_DIR/mcp.json" <<CONFIG
{
  "mcpServers": {
    "framework": {
      "command": "$BINARY",
      "args": ["--document", "$DOCUMENT"]
    }
  }
}
CONFIG

if [ -n "$SELFCHECK" ]; then
	# No model: the scenario's reference script builds a correct solution
	# through the same MCP tools, and the verifier grades that. Proves the
	# task is achievable through the surface AND the verifier recognizes a
	# right answer — the pair that keeps agent-lane failures meaningful.
	[ -f "$SCENARIO_DIR/reference.py" ] || {
		echo "scenario '$SCENARIO' has no reference.py; selfcheck is incomplete" >&2
		exit 3
	}
	echo "selfcheck: building the reference solution..."
	FRAMEWORK_MCP_BINARY="$BINARY" FRAMEWORK_DOCUMENT="$DOCUMENT" \
		python3 "$SCENARIO_DIR/reference.py"
else
	echo "running $MODEL on scenario '$SCENARIO'..."
	(
		cd "$RUN_DIR"
		claude -p "$(cat "$SCENARIO_DIR/prompt.md")" \
			--model "$MODEL" \
			--mcp-config mcp.json --strict-mcp-config \
			--allowedTools "mcp__framework" \
			--output-format stream-json --verbose \
			>transcript.jsonl 2>claude-stderr.log
	)

	python3 "$ROOT/metrics.py" "$RUN_DIR/transcript.jsonl" >"$RUN_DIR/metrics.json"
	python3 - "$RUN_DIR" <<'REPORT'
import json, sys
run_dir = sys.argv[1]
with open(f"{run_dir}/metrics.json") as f:
    metrics = json.load(f)
with open(f"{run_dir}/report.md", "w") as f:
    f.write(metrics.pop("final_report", "") or "(no final report)")
print(json.dumps(metrics, indent=2))
REPORT
fi

echo "verifying the document..."
if FRAMEWORK_MCP_BINARY="$BINARY" FRAMEWORK_DOCUMENT="$DOCUMENT" \
	python3 "$SCENARIO_DIR/verify.py" | tee "$RUN_DIR/verify.txt"; then
	if [ -z "$SELFCHECK" ]; then
		python3 "$ROOT/record_history.py" \
			"$SCENARIO" "$MODEL" PASS "$RUN_DIR" "$RUN_DIR/metrics.json"
	fi
	echo "VERDICT: PASS  ($RUN_DIR)"
else
	status=$?
	if [ -z "$SELFCHECK" ]; then
		python3 "$ROOT/record_history.py" \
			"$SCENARIO" "$MODEL" FAIL "$RUN_DIR" "$RUN_DIR/metrics.json"
	fi
	echo "VERDICT: FAIL  ($RUN_DIR)" >&2
	exit $status
fi

# MCP agent smoke tests

Two lanes keep the MCP surface honest as the app grows:

**Deterministic lane** — `cargo test -p framework-mcp`. Scripted tool-call
sequences asserting engine behavior (`the_timesheet_flow_runs_on_named_tools_alone`
is the model). Free, runs on every build, belongs in CI. Add one whenever a
capability gains a named tool.

**Agent lane** — this directory. A fresh model gets nothing but a scenario
prompt and the server, and has to *find* its way through the tool surface:

```bash
tools/mcp-smoke/run.sh --selfcheck all    # zero-token reference builds + verifiers
tools/mcp-smoke/run.sh timesheet          # one agent run, sonnet by default
tools/mcp-smoke/run.sh all                # broad sweep, one model run each
tools/mcp-smoke/run.sh timesheet haiku    # lower bar: if haiku passes, the
                                          # surface is genuinely discoverable
```

## Cadence — spend tokens where they buy signal

- **Every MCP-surface or formula-vocabulary change**: first
  `cargo test -p framework-mcp`, then `--selfcheck all`. The selfcheck uses
  zero model tokens; each scenario's required `reference.py` rebuilds the
  correct solution through the tools and the verifier grades it, so a broken
  tool or a drifted verifier fails here first. A missing reference is a
  failure, never a skipped success.
- **After a meaningful change lands**: one agent run of the scenario
  nearest the change (a few minutes of sonnet).
- **Milestones** (release, big vocabulary shifts): `run.sh all`. Full runs
  remain machine-local, while each agent verdict and its friction metrics
  append to tracked `history.jsonl` — the trend is the point. Three archived
  timesheet runs in Aug 2026 took the verdict from "No" to "worked
  essentially first-try"; that trajectory is only visible because the runs
  were kept.

Scenarios deliberately cover different surface areas — `timesheet`
(generators, expand, entry columns, crosstab), `budget` (summarize,
validated joins, percentage columns, live recomputation), `reshape`
(unpivot, editable sources, wide readings) — so `all` is a broad sweep,
not five runs of the same muscle.

This lane measures what deterministic tests cannot: discoverability. The
history that motivated it — three hand-driven runs of the timesheet
scenario in Aug 2026 — moved from "agent hand-writes a CSV of dates and
fakes data entry with a when() formula" to "every step first-try through
named tools" purely through tool-surface fixes the runs themselves exposed
(see `runs/` archives and the git history around `expand_frame`).

## Grading rules

1. **Never grade the agent on its self-report.** Run 2's agent declared
   the CSV import broken while the import was fine and `get_frame` was
   lying. Self-reports inherit every bug in the tools they were written
   with. The verifier reads the *document* the run produced, then drives
   the behavior itself through the same MCP tools.
2. **Find things by shape, never by name.** Agents name objects
   differently every run. The timesheet verifier locates the generator by
   its `generator` field, the sheet by its `expand` step, the parameter by
   chasing the value id out of the generator's own rule.
3. **Metrics are the friction score.** `metrics.json` counts tool calls,
   errors hit, and `apply_operation` escape-hatch uses. A run that passes
   through forty errors is a worse surface than one that passes through
   six named calls — trend these across runs, not the prose verdicts. A PASS
   therefore means the produced document met the scenario, not that the tool
   surface was already good enough.

## Adding a scenario

```
scenarios/<name>/
  prompt.md    # the task, written for a cold agent; include hard
               # requirements the verifier will actually check
  fixtures/    # files copied next to the document before the run
  verify.py    # shape-based assertions; env FRAMEWORK_MCP_BINARY and
               # FRAMEWORK_DOCUMENT; exit nonzero on the first failure
```

Write `prompt.md` and `verify.py` together: every hard requirement in the
prompt should map to an assertion, so a pass means the requirement was
met, not merely claimed.

## Requirements and cost

Needs the `claude` CLI signed in (agent runs cost model usage — a sonnet
timesheet run is a few minutes), and the machine-local MCP switch the
desktop app writes when MCP is enabled in Settings. Runs land in `runs/`
(gitignored): transcript, document, metrics, report, verify output. Compact
agent metrics also append to tracked `history.jsonl`; commit intentional
benchmark runs so comparisons survive beyond one machine.

import { useEffect, useState } from "react";
import { dependencyGraph, getFrameQueryPlan } from "./lib/api";
import type { DependencyNode, FrameQueryPlan } from "./lib/api";

/**
 * "How did I get this result": a value or result's dependency tree, walked
 * back to its inputs, with each stop's current value shown next to it.
 *
 * A frame node is a dead end here on purpose — its own wrangle chain is a
 * different kind of path, stepped through separately (see
 * `sampleFrameStep`), not something this tree reaches into.
 *
 * Loaded only after the disclosure opens. Diagnostics should be close to a
 * failed answer, but a healthy canvas should not pay a round trip for a tree
 * nobody asked to inspect.
 */
export function DebugTracePanel({ objectId }: { objectId: string }) {
  const [open, setOpen] = useState(false);
  const [root, setRoot] = useState<DependencyNode | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setRoot(null);
    setError(null);
  }, [objectId]);

  useEffect(() => {
    if (!open || root || error) return;
    let cancelled = false;
    dependencyGraph(objectId)
      .then((node) => {
        if (!cancelled) setRoot(node);
      })
      .catch((err) => {
        if (!cancelled) setError(String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [error, objectId, open, root]);

  return (
    <details
      className="debug-trace-panel"
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary>Trace dependencies</summary>
      {error && <p className="debug-trace-error">{error}</p>}
      {!error && !root && <p className="debug-trace-loading">Tracing…</p>}
      {root && <TraceNode node={root} depth={0} />}
    </details>
  );
}

function TraceNode({ node, depth }: { node: DependencyNode; depth: number }) {
  const [expanded, setExpanded] = useState(depth < 2);
  const hasChildren = node.children.length > 0;

  return (
    <div className="debug-trace-node" style={{ marginLeft: depth === 0 ? 0 : 14 }}>
      <button
        type="button"
        className="debug-trace-row"
        disabled={!hasChildren}
        onClick={() => hasChildren && setExpanded((value) => !value)}
      >
        <span className="debug-trace-caret">{hasChildren ? (expanded ? "−" : "+") : ""}</span>
        <span className="debug-trace-name">{node.name}</span>
        <span className="debug-trace-kind">{node.kind}</span>
        {node.formula && <span className="debug-trace-formula">{node.formula}</span>}
        {node.display && <span className="debug-trace-value">{node.display}</span>}
        {node.error && <span className="debug-trace-node-error">{node.error}</span>}
        {node.kind === "repeated" && (
          <span className="debug-trace-hint">already shown above — stopped to avoid a loop</span>
        )}
      </button>
      {node.kind === "frame" && <QueryPlanDetails frameId={node.objectId} />}
      {hasChildren && expanded && (
        <div className="debug-trace-children">
          {node.children.map((child, index) => (
            <TraceNode key={`${child.objectId}-${index}`} node={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
}

/** The existing logical and optimized plans, now reachable from every frame
 * leaf in a dependency trace as well as the frame inspector. */
export function QueryPlanDetails({ frameId }: { frameId: string }) {
  const [plan, setPlan] = useState<FrameQueryPlan | null>(null);
  const [stage, setStage] = useState<"optimized" | "logical">("optimized");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = (open: boolean) => {
    if (!open || loading) return;
    setLoading(true);
    setPlan(null);
    setError(null);
    void getFrameQueryPlan(frameId)
      .then(setPlan)
      .catch((reason) => setError(String(reason).replace(/^Error:\s*/, "")))
      .finally(() => setLoading(false));
  };

  return (
    <details
      className="query-plan-panel"
      onToggle={(event) => load(event.currentTarget.open)}
    >
      <summary>{loading ? "Reading query plan…" : "Query plan"}</summary>
      {error && <p className="debug-trace-error">{error}</p>}
      {plan && (
        <>
          <div className="query-plan-stage">
            <button
              type="button"
              className={stage === "optimized" ? "active" : ""}
              onClick={() => setStage("optimized")}
            >
              Optimized
            </button>
            <button
              type="button"
              className={stage === "logical" ? "active" : ""}
              onClick={() => setStage("logical")}
            >
              As written
            </button>
          </div>
          <pre>{stage === "optimized" ? plan.optimized : plan.logical}</pre>
        </>
      )}
    </details>
  );
}

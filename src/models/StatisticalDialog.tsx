import { useState } from "react";
import type { FrameObject } from "../lib/types";
import { useModels, type ModelDialogState } from "./context";
import { ModelDialogShell } from "./ModelDialogShell";

type Method = "meanConfidence" | "pearsonCorrelation" | "welchDifference";

export function StatisticalDialog({ state, onClose }: { state: ModelDialogState; onClose: () => void }) {
  const { document, onOperation } = useModels();
  const frames = document.objects.filter((object): object is FrameObject => object.kind === "frame");
  const [sourceId, setSourceId] = useState(state.sourceFrameId ?? frames[0]?.id ?? "");
  const source = frames.find(frame => frame.id === sourceId);
  const [method, setMethod] = useState<Method>("meanConfidence");
  const [name, setName] = useState("Statistics");
  const [xColumn, setXColumn] = useState("");
  const [yColumn, setYColumn] = useState("");
  const [confidence, setConfidence] = useState(95);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submit = async () => {
    setBusy(true); setError(null);
    try {
      const failure = await onOperation({ type: "addStatisticalAnalysis", name,
        sourceFrameId: sourceId, xColumnId: xColumn, yColumnId: method === "meanConfidence" ? null : yColumn,
        request: { method, confidenceLevel: confidence / 100 }, x: state.x, y: state.y }, { inlineError: true });
      if (failure) setError(failure); else onClose();
    } catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  };
  return <ModelDialogShell title="Statistical analysis" busy={busy} error={error} onClose={onClose}>
    <label>Name<input aria-label="Statistical result name" autoFocus value={name} onChange={event => setName(event.target.value)} /></label>
    <label>Analysis<select aria-label="Statistical method" value={method} onChange={event => setMethod(event.target.value as Method)}>
      <option value="meanConfidence">Estimate a mean and confidence interval</option>
      <option value="pearsonCorrelation">Measure association · Pearson correlation</option>
      <option value="welchDifference">Compare two means · Welch’s t-test</option>
    </select></label>
    <label>Source frame<select aria-label="Statistics source frame" value={sourceId}
      onChange={event => { setSourceId(event.target.value); setXColumn(""); setYColumn(""); }}>
      <option value="">Choose a frame</option>{frames.map(frame => <option key={frame.id} value={frame.id}>{frame.name}</option>)}
    </select></label>
    <StatisticalColumn source={source} label={method === "welchDifference" ? "Group A column" : "First column"} value={xColumn} onChange={setXColumn} />
    {method !== "meanConfidence" && <StatisticalColumn source={source} label={method === "welchDifference" ? "Group B column" : "Second column"} value={yColumn} onChange={setYColumn} />}
    <label>Confidence (%)<input type="number" min="50" max="99.9" step="0.1" value={confidence} onChange={event => setConfidence(event.target.valueAsNumber)} /></label>
    <p className="model-mapping">Results are a saved table you can reference in Scratchwork. Use Wrangle to filter or clean missing values first.</p>
    <div className="dialog-actions"><button className="secondary-action" onClick={onClose}>Cancel</button>
      <button className="primary-action" disabled={!xColumn || (method !== "meanConfidence" && !yColumn)} onClick={() => void submit()}>
        {busy ? "Calculating…" : "Create statistics table"}</button></div>
  </ModelDialogShell>;
}

function StatisticalColumn({ source, label, value, onChange }: {
  source: FrameObject | undefined; label: string; value: string; onChange: (value: string) => void;
}) {
  return <label>{label}<select aria-label={label} value={value} onChange={event => onChange(event.target.value)}>
    <option value="">Choose a column</option>
    {source?.columns.map(column => <option key={column.id} value={column.id}>{column.name}</option>)}
  </select></label>;
}

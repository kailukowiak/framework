import type { FrameObject } from "../lib/types";
import type { ForestSettings } from "../lib/bindings/ForestSettings";

export function isForestMethod(method: string): boolean {
  return method === "randomForestRegressor" || method === "randomForestClassifier";
}

export function ModelFitSettings({ source, method, target, onTarget, covariance, onCovariance,
  confidence, onConfidence, holdout, onHoldout, seed, onSeed, forest, onForest }: {
  source: FrameObject | undefined; method: string; target: string; onTarget: (value: string) => void;
  covariance: "classical" | "hc3"; onCovariance: (value: "classical" | "hc3") => void;
  confidence: number; onConfidence: (value: number) => void; holdout: number; onHoldout: (value: number) => void;
  seed: number; onSeed: (value: number) => void; forest: ForestSettings; onForest: (value: ForestSettings) => void;
}) {
  const boosted = method.startsWith("xgboost");
  const forestMethod = isForestMethod(method);
  const targetLabel = (method === "logistic" || method === "xgboostBinary") ? "Target (numeric 0 or 1)"
    : (method === "randomForestClassifier" || method === "xgboostMulticlass") ? "Target (numeric classes 0, 1, …)" : "Target";
  return <>
    <label>{targetLabel}<select aria-label="Model target column" value={target} onChange={event => onTarget(event.target.value)}>
      <option value="">Choose a target</option>
      {source?.columns.map(column => <option key={column.id} value={column.id}>{column.name}</option>)}
    </select></label>
    {!forestMethod && !boosted && <div className="model-settings-row">
      <label>Standard errors<select aria-label="Model standard errors" value={method === "logistic" ? "classical" : covariance}
        disabled={method === "logistic"} onChange={event => onCovariance(event.target.value as typeof covariance)}>
        <option value="classical">Classical</option><option value="hc3">Robust (HC3)</option>
      </select></label>
      <label>Confidence (%)<input type="number" min="50" max="99.9" step="0.1" value={confidence}
        onChange={event => onConfidence(event.target.valueAsNumber)} /></label>
    </div>}
    <div className="model-settings-row">
      <label>Random holdout (%)<input type="number" min="0" max="50" value={holdout}
        onChange={event => onHoldout(event.target.valueAsNumber)} /></label>
      <label>Split seed<input type="number" min="0" step="1" value={seed}
        onChange={event => onSeed(event.target.valueAsNumber)} /></label>
    </div>
    {forestMethod && <ForestControls forest={forest} onChange={onForest} />}
    <p className="model-mapping">Numeric features; missing values are reported as errors. Random holdout assumes independent rows; use 0 for training-only results.</p>
  </>;
}

function ForestControls({ forest, onChange }: { forest: ForestSettings; onChange: (value: ForestSettings) => void }) {
  return <details><summary>Forest settings</summary>
    <div className="model-settings-row">
      <label>Trees<input type="number" min="1" max="1000" value={forest.trees}
        onChange={event => onChange({ ...forest, trees: event.target.valueAsNumber })} /></label>
      <label>Maximum depth<input type="number" min="1" max="24" value={forest.maxDepth}
        onChange={event => onChange({ ...forest, maxDepth: event.target.valueAsNumber })} /></label>
    </div>
    <div className="model-settings-row">
      <label>Minimum rows per leaf<input type="number" min="1" value={forest.minSamplesLeaf}
        onChange={event => onChange({ ...forest, minSamplesLeaf: event.target.valueAsNumber })} /></label>
      <label>Features per split<input type="number" min="1" placeholder="Automatic" value={forest.maxFeatures ?? ""}
        onChange={event => onChange({ ...forest, maxFeatures: event.target.value === "" ? null : event.target.valueAsNumber })} /></label>
    </div>
    <label>Forest seed<input type="number" min="0" value={forest.seed}
      onChange={event => onChange({ ...forest, seed: event.target.valueAsNumber })} /></label>
  </details>;
}

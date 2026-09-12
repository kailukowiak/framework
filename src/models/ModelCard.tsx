import { useState } from "react";
import type { DataObject, Operation } from "../lib/types";
import { useModels } from "./context";
import { CoefficientTable, MetricsTable } from "./ModelSummaryTables";

export function ModelCard({ model }: { model: Extract<DataObject, { kind: "model" }> }) {
  const { document, onOperation, position: place } = useModels();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const computed = document.computedModels[model.id];
  const position = place();
  const run = async (operation: Operation) => {
    setBusy(true); setError(null);
    try {
      const failure = await onOperation(operation, { inlineError: true });
      setError(failure);
      const view = document.views.find(view => view.objectId === model.id);
      // A first fit introduces open-ended tables. Only expand the untouched
      // initial card; a user's chosen dimensions survive every retrain.
      if (!failure && operation.type === "fitModel" && !model.fitted && view?.height === 140) {
        setError(await onOperation({ type: "resizeView", viewId: view.id, width: view.width, height: 420 }, { inlineError: true }));
      }
    }
    catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  };
  const copy = (kind: "coefficients" | "trainingMetrics" | "evaluationMetrics") => void run({
    type: "addModelSummary", modelId: model.id, kind,
    name: `${model.name} ${kind === "coefficients" ? "coefficients" : kind === "evaluationMetrics" ? "evaluation" : "training"}`,
    ...position,
  });
  return <div className="model-card">
    <input className="object-name-input" aria-label="Model name" defaultValue={model.name} key={model.name}
      onBlur={event => { if (event.target.value !== model.name) void run({ type: "renameObject", objectId: model.id, name: event.target.value }); }}
      onKeyDown={event => { if (event.key === "Enter") event.currentTarget.blur(); }} />
    <p className="model-status">{modelStatus(model)}</p>
    {computed?.stale && <p className="model-status" role="status">Training data or settings changed. Fit again to update the model.</p>}
    <ModelActions model={model} busy={busy} position={position} run={run} copy={copy} />
    {(error || computed?.error) && <p className="model-error" role="alert">{error ?? computed?.error}</p>}
    <SavedModelSummary model={model} />
  </div>;
}

function ModelActions({ model, busy, position, run, copy }: {
  model: Extract<DataObject, { kind: "model" }>; busy: boolean; position: { x: number; y: number };
  run: (operation: Operation) => Promise<void>; copy: (kind: "coefficients" | "trainingMetrics" | "evaluationMetrics") => void;
}) {
  const { open } = useModels();
  const fitted = model.fitted;
  const summary = fitted?.result.summary;
  return (
    <div className="model-actions">
      {model.spec && <>
        <button className="secondary-action" disabled={busy} onClick={() => open({ kind: "edit", modelId: model.id, ...position })}>Edit specification…</button>
        <button className="secondary-action" disabled={busy} onClick={() => void run({ type: "fitModel", modelId: model.id })}>
          {busy ? "Fitting…" : fitted ? "Retrain" : "Fit model"}</button>
      </>}
      <button className="secondary-action" disabled={busy || !fitted}
        onClick={() => open({ kind: "predict", modelId: model.id, ...position })}>Predictions…</button>
      {fitted && Boolean(summary?.coefficients.length || summary?.trainingMetrics.rows || fitted.evaluationMetrics) && <details className="model-summary-actions"><summary>Copy stats to frame</summary>
        <div>{Boolean(summary?.coefficients.length) && <button disabled={busy} onClick={() => copy("coefficients")}>Coefficients</button>}
          {Boolean(summary?.trainingMetrics.rows) && <button disabled={busy} onClick={() => copy("trainingMetrics")}>Training metrics</button>}
          {fitted.evaluationMetrics && <button disabled={busy} onClick={() => copy("evaluationMetrics")}>Evaluation metrics</button>}</div>
      </details>}
    </div>
  );
}

function SavedModelSummary({ model }: { model: Extract<DataObject, { kind: "model" }> }) {
  const fitted = model.fitted;
  const summary = fitted?.result.summary;
  if (!summary) return null;
  return (
    <div className="model-summary">
      <CoefficientTable summary={summary} />
      {fitted?.evaluationMetrics && <MetricsTable metrics={fitted.evaluationMetrics} label="Holdout evaluation" />}
      <MetricsTable metrics={summary.trainingMetrics} label="Training" />
      {summary.warnings.map((warning, i) => <p className="model-status" key={i}>{warning}</p>)}
    </div>
  );
}

function modelStatus(model: Extract<DataObject, { kind: "model" }>): string {
  const kind = model.fitted?.result.payload.kind;
  const method = kind === "onnx" ? "Imported ONNX pipeline" : (kind === "randomForest" || model.spec?.method.startsWith("randomForest")) ? "Random forest" : kind === "xgboost" ? "XGBoost"
    : (kind ? kind === "binaryLogistic" : model.spec?.method === "logistic") ? "Logistic regression" : "Linear regression";
  const status = model.spec ? model.fitted ? "Fitted" : "Not fitted" : "Imported · inference only";
  const observations = model.fitted?.result.summary.observations;
  return `${method} · ${status}${observations == null ? "" : ` · ${observations} training rows`}`;
}

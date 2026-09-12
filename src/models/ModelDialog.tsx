import { useState } from "react";
import type { FrameObject, Operation } from "../lib/types";
import { invoke } from "../lib/invoke";
import { useModels, type ModelDialogState } from "./context";
import { ModelDialogShell } from "./ModelDialogShell";
import { ModelSourceFields } from "./ModelSourceFields";
import { resolveModelColumns } from "./columns";
import { modelDraft } from "./modelDraft";
import { ModelFitSettings, isForestMethod } from "./ModelFitSettings";
import { ModelImportFields } from "./ModelImportFields";
import type { Method } from "../lib/bindings/Method";
import type { ForestSettings } from "../lib/bindings/ForestSettings";

export function ModelDialog({ state, onClose }: { state: ModelDialogState; onClose: () => void }) {
  const { document, onOperation } = useModels();
  const [initial] = useState(() => modelDraft(document, state));
  const frames = document.objects.filter((object): object is FrameObject => object.kind === "frame");
  const [sourceId, setSourceId] = useState(initial.sourceId);
  const source = frames.find(frame => frame.id === sourceId);
  const [name, setName] = useState("Regression");
  const [method, setMethod] = useState<Method | "onnx">(initial.method);
  const [features, setFeatures] = useState(initial.features);
  const [target, setTarget] = useState(initial.target);
  const [covariance, setCovariance] = useState<"classical" | "hc3">(initial.covariance);
  const [confidence, setConfidence] = useState(initial.confidence);
  const [holdout, setHoldout] = useState(initial.holdout);
  const [seed, setSeed] = useState(initial.seed);
  const [json, setJson] = useState("");
  const [bytes, setBytes] = useState<number[] | null>(null);
  const [fileName, setFileName] = useState("");
  const [forest, setForest] = useState<ForestSettings>(initial.forest);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submit = async () => {
    setBusy(true); setError(null);
    try {
      const featureColumnIds = resolveModelColumns(source, features);
      const position = { x: state.x, y: state.y };
      let operation: Operation;
      if (method === "xgboost") {
        operation = { type: "importModel", name: name.trim(), sourceFrameId: sourceId,
          featureColumnIds, json, ...position };
      } else if (method === "onnx") {
        if (!bytes) throw new Error("Choose an ONNX model file.");
        operation = { type: "importOnnxModel", name: name.trim(), sourceFrameId: sourceId, featureColumnIds, bytes, ...position };
      } else {
        if (!target) throw new Error("Choose a target column.");
        if (featureColumnIds.includes(target)) throw new Error("The target cannot also be a feature.");
        const nextSpec = { sourceFrameId: sourceId, targetColumnId: target, featureColumnIds,
          method, covariance: method === "ols" ? covariance : "classical" as const,
          confidenceLevel: confidence / 100, holdoutFraction: holdout / 100, seed, ...(isForestMethod(method) ? { forest } : {}) };
        operation = state.kind === "edit" && state.modelId
          ? { type: "setModelSpec", modelId: state.modelId, spec: nextSpec }
          : { type: "addModel", name: name.trim(), spec: nextSpec, ...position };
      }
      const failure = await onOperation(operation, { inlineError: true });
      if (failure) setError(failure); else onClose();
    } catch (reason) { setError(String(reason).replace(/^Error:\s*/, "")); }
    finally { setBusy(false); }
  };
  const pick = async () => {
    setBusy(true); setError(null);
    try {
      const file = await invoke<{ name: string; contents: string; bytes: number[] | null } | null>("pick_model_file");
      if (file) {
        setJson(file.contents); setBytes(file.bytes); setFileName(file.name);
        setMethod(file.bytes ? "onnx" : "xgboost"); setName(file.name.replace(/\.(json|onnx)$/i, ""));
      }
    } catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  };
  return <ModelDialogShell title={state.kind === "edit" ? "Edit model specification" : "Create model"}
    busy={busy} error={error} onClose={onClose}>
    {state.kind !== "edit" && <label>Name<input autoFocus aria-label="Model name" value={name}
      onChange={event => setName(event.target.value)} /></label>}
    <label>Task and method<select aria-label="Model method" value={method}
      onChange={event => setMethod(event.target.value as typeof method)}>
      <option value="ols">Predict a number · Linear regression (OLS)</option>
      <option value="logistic">Predict a category · Logistic regression</option>
      <option value="randomForestRegressor">Predict a number · Random forest</option>
      <option value="randomForestClassifier">Predict a category · Random forest</option>
      {state.kind !== "edit" && <>
        <option value="xgboost">Import a trained XGBoost model</option>
        <option value="onnx">Import a fitted sklearn pipeline (ONNX)</option>
      </>}
    </select></label>
    <ModelSourceFields frames={frames} sourceId={sourceId} onSource={id => { setSourceId(id); setFeatures(""); setTarget(""); }}
      features={features} onFeatures={setFeatures} />
    {method === "xgboost" || method === "onnx"
      ? <ModelImportFields method={method} json={json} onJson={setJson} fileName={fileName} onPick={() => void pick()} />
      : <ModelFitSettings source={source} method={method} target={target} onTarget={setTarget}
        covariance={covariance} onCovariance={setCovariance} confidence={confidence} onConfidence={setConfidence}
        holdout={holdout} onHoldout={setHoldout} seed={seed} onSeed={setSeed} forest={forest} onForest={setForest} />}
    <div className="dialog-actions"><button className="secondary-action" onClick={onClose}>Cancel</button>
      <button className="primary-action" disabled={!sourceId || !features.trim() || (method === "xgboost" && !json.trim()) || (method === "onnx" && !bytes)}
        onClick={() => void submit()}>{busy ? "Working…" : state.kind === "edit" ? "Apply specification" : method === "xgboost" || method === "onnx" ? "Import model" : "Create model"}</button>
    </div>
  </ModelDialogShell>;
}

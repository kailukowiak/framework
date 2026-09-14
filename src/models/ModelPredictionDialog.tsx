import { useState } from "react";
import type { DataObject, FrameObject } from "../lib/types";
import { useModels, type ModelDialogState } from "./context";
import { ModelDialogShell } from "./ModelDialogShell";
import { ModelSourceFields } from "./ModelSourceFields";
import { resolveModelColumns } from "./columns";

export function ModelPredictionDialog({ state, onClose }: { state: ModelDialogState; onClose: () => void }) {
  const { document, onOperation } = useModels();
  const found = document.objects.find(object => object.id === state.modelId);
  const model = found?.kind === "model" ? found : null;
  const binding = fittedBinding(model);
  const frames = document.objects.filter((object): object is FrameObject => object.kind === "frame");
  const [sourceId, setSourceId] = useState(binding?.sourceFrameId ?? "");
  const source = frames.find(frame => frame.id === sourceId);
  const featureNames = model?.fitted?.result.featureNames.join("\n") ?? "";
  const [name, setName] = useState(`${model?.name ?? "Model"} predictions`);
  const [features, setFeatures] = useState(featureNames);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submit = async () => {
    if (!model) return;
    setBusy(true); setError(null);
    try {
      const failure = await onOperation({ type: "addModelPredictions", modelId: model.id,
        sourceFrameId: sourceId, featureColumnIds: resolveModelColumns(source, features), name,
        x: state.x, y: state.y }, { inlineError: true });
      if (failure) setError(failure); else onClose();
    } catch (reason) { setError(String(reason).replace(/^Error:\s*/, "")); }
    finally { setBusy(false); }
  };
  return <ModelDialogShell title="Create live predictions" busy={busy} error={error} onClose={onClose}>
    <label>Name<input aria-label="Prediction frame name" autoFocus value={name} onChange={event => setName(event.target.value)} /></label>
    <ModelSourceFields frames={frames} sourceId={sourceId}
      onSource={id => { setSourceId(id); setFeatures(featureNames); }} features={features} onFeatures={setFeatures}
      featureNames={model?.fitted?.result.featureNames} />
    <div className="dialog-actions"><button className="secondary-action" onClick={onClose}>Cancel</button>
      <button className="primary-action" disabled={!sourceId || !features.trim() || !model?.fitted}
        onClick={() => void submit()}>{busy ? "Creating…" : "Create predictions"}</button></div>
  </ModelDialogShell>;
}

function fittedBinding(model: Extract<DataObject, { kind: "model" }> | null) {
  return model?.fitted?.spec ?? model?.spec ?? model?.importedInput;
}

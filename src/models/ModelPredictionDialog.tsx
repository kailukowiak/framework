import { useState } from "react";
import type { DataObject, FrameObject } from "../lib/types";
import { useModels, type ModelDialogState } from "./context";
import { ModelDialogShell } from "./ModelDialogShell";
import { ModelSourceFields } from "./ModelSourceFields";
import { modelColumnNames, resolveModelColumns } from "./columns";

export function ModelPredictionDialog({ state, onClose }: { state: ModelDialogState; onClose: () => void }) {
  const { document, onOperation } = useModels();
  const found = document.objects.find(object => object.id === state.modelId);
  const model = found?.kind === "model" ? found : null;
  const binding = fittedBinding(model);
  const frames = document.objects.filter((object): object is FrameObject => object.kind === "frame");
  const [sourceId, setSourceId] = useState(binding?.sourceFrameId ?? "");
  const source = frames.find(frame => frame.id === sourceId);
  const featureNames = model?.fitted?.result.featureNames.join("\n") ?? "";
  // Scoring the frame the model was fitted or imported against, the bound
  // columns are listed by their own names, which always resolve — an
  // imported booster's fitted names are the file's, and a file that names
  // nothing gets "feature_1". Anywhere else the fitted names are the
  // starting point, since the new frame has to answer to them in order.
  const featuresFor = (frame: FrameObject | undefined) => {
    const bound = binding && frame?.id === binding.sourceFrameId
      ? modelColumnNames(frame, binding.featureColumnIds) : "";
    return bound && bound.split("\n").every(Boolean) ? bound : featureNames;
  };
  const [name, setName] = useState(`${model?.name ?? "Model"} predictions`);
  const [features, setFeatures] = useState(featuresFor(source));
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
      onSource={id => { setSourceId(id); setFeatures(featuresFor(frames.find(frame => frame.id === id))); }}
      features={features} onFeatures={setFeatures}
      featureNames={model?.fitted?.result.featureNames} />
    <div className="dialog-actions"><button className="secondary-action" onClick={onClose}>Cancel</button>
      <button className="primary-action" disabled={!sourceId || !features.trim() || !model?.fitted}
        onClick={() => void submit()}>{busy ? "Creating…" : "Create predictions"}</button></div>
  </ModelDialogShell>;
}

function fittedBinding(model: Extract<DataObject, { kind: "model" }> | null) {
  return model?.fitted?.spec ?? model?.spec ?? model?.importedInput;
}

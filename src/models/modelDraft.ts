import type { DocumentView } from "../lib/types";
import type { ModelDialogState } from "./context";
import { modelColumnNames } from "./columns";

/** Resolve a saved specification once; the open editor then owns its draft. */
export function modelDraft(document: DocumentView, state: ModelDialogState) {
  const existing = document.objects.find(object => object.id === state.modelId);
  const saved = existing?.kind === "model" ? existing.spec : null;
  const sourceId = saved?.sourceFrameId ?? state.sourceFrameId ?? document.objects.find(object => object.kind === "frame")?.id ?? "";
  const source = document.objects.find(object => object.id === sourceId);
  const spec = saved ?? { method: "ols" as const, featureColumnIds: [], targetColumnId: "",
    covariance: "classical" as const, confidenceLevel: 0.95, holdoutFraction: 0.2, seed: 42 };
  return {
    sourceId,
    method: spec.method,
    features: modelColumnNames(source?.kind === "frame" ? source : undefined, spec.featureColumnIds),
    target: spec.targetColumnId ?? "",
    covariance: spec.covariance,
    confidence: spec.confidenceLevel * 100,
    holdout: spec.holdoutFraction * 100,
    seed: spec.seed,
    forest: saved?.forest ?? { trees: 100, maxDepth: 8, minSamplesLeaf: 2, maxFeatures: null, seed: 0 },
  };
}

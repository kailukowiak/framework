import type { DataObject } from "./types";

/**
 * Every frame a chain's union steps stack onto, found by scanning a frame's
 * persisted steps.
 *
 * The steps array is typed `unknown[]` here, not `FrameStepInput[]` --
 * `FrameObject.steps`/`derivation.steps` hold parsed expressions in a shape
 * only the core writes, so the editor only ever reads them back rendered.
 * A union step's `frameId` survives serialization as a plain string
 * regardless, which is all a lineage cord needs.
 */
function unionSourceFrameIds(steps: unknown[] | undefined): string[] {
  if (!steps) return [];
  return steps.flatMap((step) => {
    if (
      step &&
      typeof step === "object" &&
      (step as { kind?: unknown }).kind === "union" &&
      typeof (step as { frameId?: unknown }).frameId === "string"
    ) {
      return [(step as { frameId: string }).frameId];
    }
    return [];
  });
}

/** Both a scoring table and its fitted model are visible dependencies. */
export function lineageSourceIds(object: DataObject): string[] {
  if (object.kind === "plot") return [object.sourceFrameId];
  if (object.kind === "model") {
    const source = object.spec?.sourceFrameId ?? object.importedInput?.sourceFrameId;
    return source ? [source] : [];
  }
  if (object.kind !== "frame") return [];
  return [...new Set([
    object.derivation?.sourceFrameId,
    object.derivation?.join?.lookupFrameId,
    object.prediction?.modelId,
    ...unionSourceFrameIds(object.derivation?.steps),
    ...unionSourceFrameIds(object.steps),
  ].filter((id): id is string => Boolean(id)))];
}

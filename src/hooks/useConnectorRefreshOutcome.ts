import { useSyncExternalStore } from "react";
import { refreshFrameConnector } from "../lib/api";
import type { SchemaDiff } from "../lib/bindings/SchemaDiff";
import type { DocumentView } from "../lib/types";

/**
 * What the last connector refresh did to each frame's schema.
 *
 * Kept outside React because the two ends of it are on opposite sides of
 * the application: the refresh is run from the Data sidebar, and the frame's
 * source panel in the Inspector is where a frame explains itself. Threading
 * a refresh result from one to the other would mean adding a prop to every
 * component between them to carry a sentence neither of them reads.
 *
 * Deliberately not in the document. The diff is true about one moment; the
 * document is what is true until someone changes it. Reloading the app
 * forgets this, which is correct — nothing here just refreshed.
 */
const outcomes = new Map<string, SchemaDiff | null>();
const listeners = new Set<() => void>();

function recordConnectorRefresh(
  frameId: string,
  schema: SchemaDiff | null
): void {
  outcomes.set(frameId, schema);
  for (const listener of listeners) listener();
}

/**
 * Reads a frame's source again and files the schema report the refresh came
 * back with, so the frame's source panel can say what changed.
 *
 * The recording lives here rather than at the call site because it is the
 * other half of this module: the report has exactly one producer, and a
 * refresh that threw produced none — the last one must not go on standing
 * as if it were this one's answer.
 */
export async function refreshConnectorAndRecord(
  frameId: string
): Promise<DocumentView> {
  try {
    const outcome = await refreshFrameConnector(frameId);
    recordConnectorRefresh(frameId, outcome.schema);
    return outcome.view;
  } catch (reason) {
    recordConnectorRefresh(frameId, null);
    throw reason;
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * The schema report from this frame's last refresh in this session, or null
 * when it has not been refreshed or the refresh changed nothing.
 */
export function useConnectorRefreshDiff(frameId: string): SchemaDiff | null {
  return useSyncExternalStore(
    subscribe,
    () => outcomes.get(frameId) ?? null,
    () => null
  );
}

/** Test seam: state a refresh report directly, and forget them all. */
export function recordConnectorRefreshForTest(
  frameId: string,
  schema: SchemaDiff | null
): void {
  recordConnectorRefresh(frameId, schema);
}

export function forgetConnectorRefreshes(): void {
  outcomes.clear();
  for (const listener of listeners) listener();
}

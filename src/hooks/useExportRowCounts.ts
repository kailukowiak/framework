import { useEffect, useState } from "react";
import { invoke } from "../lib/invoke";
import type { DocumentView } from "../lib/types";

export function useExportRowCounts(document: DocumentView, currentView: boolean) {
  const frames = document.objects.filter((object) => object.kind === "frame");
  const ids = JSON.stringify(frames.map((frame) => frame.id));
  const key = JSON.stringify([currentView, frames.map((frame) => [frame.id, document.computedFrames[frame.id]?.fingerprint, frame.display])]);
  const [answer, setAnswer] = useState<{ key: string; counts: Record<string, number>; error: string | null } | null>(null);
  useEffect(() => {
    let disposed = false;
    void invoke<Record<string, number>>("export_row_counts", { frameIds: JSON.parse(ids), currentView })
      .then((counts) => { if (!disposed) setAnswer({ key, counts, error: null }); })
      .catch((reason) => { if (!disposed) setAnswer({ key, counts: {}, error: String(reason) }); });
    return () => { disposed = true; };
  }, [currentView, ids, key]);
  return answer?.key === key ? answer : null;
}

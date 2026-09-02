import { useEffect, useState } from "react";
import { previewFramePipeline, type PipelineSchema } from "../lib/api";
import { reportIgnoredFailure } from "../lib/errorReporting";
import type { FrameStepInput } from "../lib/types";

/**
 * What the draft would actually produce, asked of the core rather than
 * worked out here. It answers from the query plan, so this costs no scan
 * — but it is a round trip, so it waits for typing to finish. `previewOf`
 * is the draft the answer describes, held by identity, which is all that
 * is needed: the inputs array is rebuilt whenever the chain changes, so
 * anything but the very array that was sent means the answer is about a
 * draft that no longer exists.
 */
export function usePipelineSchemaPreview(
  frameId: string,
  stepScopeInputs: FrameStepInput[]
) {
  const [preview, setPreview] = useState<PipelineSchema | null>(null);
  const [previewOf, setPreviewOf] = useState<FrameStepInput[] | null>(null);
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let disposed = false;
    const timer = window.setTimeout(() => {
      void previewFramePipeline(frameId, stepScopeInputs)
        .then((next) => {
          if (disposed) return;
          setPreview(next);
          setPreviewOf(stepScopeInputs);
        })
        // A preview that cannot be taken is not worth reporting: the
        // fallback still describes the chain, and saving reports the real
        // error against the real chain.
        .catch(reportIgnoredFailure("frame pipeline preview"));
    }, 250);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [frameId, stepScopeInputs]);
  return { preview, previewOf };
}

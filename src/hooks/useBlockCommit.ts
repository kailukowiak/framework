import { useCallback, useRef, useState } from "react";
import type { OperationHandler } from "../lib/handlers";

/** Text may come back rewritten while an earlier commit is still pending.
 * Settling that commit must trigger reconciliation too: a ref-only counter
 * could skip the new source forever, leaving an old assumption beside its
 * newly computed answer after a control changed it.
 */
export function useBlockCommit(blockId: string, initial: string, onOperation: OperationHandler) {
  const sent = useRef(initial);
  const sentEditing = useRef<number | null>(null);
  const pending = useRef<number | undefined>(undefined);
  const latest = useRef<Promise<void> | null>(null);
  const [inflight, setInflight] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const commit = useCallback((source: string, editing: number | null): Promise<void> => {
    window.clearTimeout(pending.current);
    if (source === sent.current && editing === sentEditing.current) return latest.current ?? Promise.resolve();
    sent.current = source;
    sentEditing.current = editing;
    setInflight((count) => count + 1);
    const promise = (async () => {
      try {
        setError(await onOperation({ type: "setBlockSource", blockId, source, editing }, { inlineError: true }));
      } finally { setInflight((count) => count - 1); }
    })();
    latest.current = promise;
    return promise;
  }, [blockId, onOperation]);
  return { sent, sentEditing, pending, inflight, error, commit };
}

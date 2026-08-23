import { useEffect, useState } from "react";
import {
  getJoinDiagnostics,
  type JoinDiagnostics,
} from "../lib/api";

export function useJoinDiagnostics(
  primaryFrameId: string | undefined,
  lookupFrameId: string | undefined,
  primaryKeyId: string | undefined,
  lookupKeyId: string | undefined,
  compatible: boolean
) {
  const [diagnostics, setDiagnostics] = useState<JoinDiagnostics | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setDiagnostics(null);
    setError(null);
    if (
      !compatible ||
      !primaryFrameId ||
      !lookupFrameId ||
      !primaryKeyId ||
      !lookupKeyId
    )
      return;
    let current = true;
    setLoading(true);
    void getJoinDiagnostics(
      primaryFrameId,
      lookupFrameId,
      [primaryKeyId],
      [lookupKeyId]
    )
      .then((answer) => {
        if (current) setDiagnostics(answer);
      })
      .catch((failure: unknown) => {
        if (current)
          setError(failure instanceof Error ? failure.message : String(failure));
      })
      .finally(() => {
        if (current) setLoading(false);
      });
    return () => {
      current = false;
    };
  }, [
    primaryFrameId,
    lookupFrameId,
    primaryKeyId,
    lookupKeyId,
    compatible,
  ]);

  return { diagnostics, error, loading };
}

import { useEffect, useState } from "react";
import { getFramePage } from "../lib/api";
import type { ComputedFrame, FrameObject } from "../lib/types";

/** A chart must never present rows fetched for an older source as current.
 * The lineage fingerprint changes for relevant calculations; the explicit
 * refresh revision also covers rereading an external file at the same path.
 * Fetches abandoned by either change cannot publish their late answers.
 */
export function usePlotRows(frame: FrameObject, computed: ComputedFrame, gated: boolean, refresh: number) {
  const columns = JSON.stringify(frame.columns.map(({ id, dataType }) => ({ id, dataType })));
  const key = JSON.stringify([frame.id, computed.fingerprint, frame.display, columns, refresh]);
  const [result, setResult] = useState<{ key: string; rows: Record<string, unknown>[]; error: string | null } | null>(null);
  useEffect(() => {
    if (!computed.paged || gated) return;
    let disposed = false;
    const fields = JSON.parse(columns) as FrameObject["columns"];
    async function load() {
      const rows: Record<string, unknown>[] = [];
      let total = Infinity;
      while (!disposed && rows.length < total) {
        const page = await getFramePage(frame.id, rows.length, 10000);
        total = page.totalRows;
        if (!page.rows.length && rows.length < total) throw new Error("The source changed while loading. Refresh the chart again.");
        rows.push(...page.rows.map((row) => Object.fromEntries(fields.map((column, index) => {
          const raw = row[index] ?? "";
          const value = !raw ? null : ["integer", "number", "currency", "percentage"].includes(column.dataType)
            ? Number(raw) : column.dataType === "boolean" ? raw.toLowerCase() === "true" : raw;
          return [column.id, typeof value === "number" && !Number.isFinite(value) ? null : value];
        }))));
      }
      if (!disposed) setResult({ key, rows, error: null });
    }
    void load().catch((reason) => {
      if (!disposed) setResult({ key, rows: [], error: String(reason).replace(/^Error:\s*/, "") });
    });
    return () => { disposed = true; };
  }, [columns, computed.paged, frame.id, gated, key]);
  const current = result?.key === key ? result : null;
  return { rows: current?.rows ?? [], loading: Boolean(computed.paged && !gated && !current), error: current?.error ?? null };
}

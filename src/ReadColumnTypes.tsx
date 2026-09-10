import { useState } from "react";
import { readColumnTypes, READ_TYPES, type ReadType } from "./lib/readColumnTypes";
import type { OperationHandler } from "./lib/handlers";
import type { Column, ComputedFrame, FrameObject } from "./lib/types";

/** One pair of controls serves every source column, regardless of table width.
 * The input schema matters here: a column dropped later in Wrangle still has a
 * read type, while a calculated output has no source type to customize. */
export function ReadColumnTypes({ frame, computed, columns, onOperation }: {
  frame: FrameObject;
  computed: ComputedFrame;
  columns: Column[];
  onOperation: OperationHandler;
}) {
  const model = readColumnTypes(frame, computed, columns);
  const [selected, setSelected] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const columnId = model.columns.some((column) => column.id === selected)
    ? selected : model.columns[0]?.id ?? "";
  if (!columnId || frame.disconnectedRead) return null;
  const change = async (type: ReadType | "") => {
    setBusy(true);
    setError(null);
    try {
      setError(await onOperation({ type: "setFramePipeline", frameId: frame.id,
        steps: model.change(columnId, type) }, { inlineError: true }));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  return <details>
    <summary>Column types</summary>
    <div className="read-type-controls">
      <select aria-label="Read column" value={columnId} disabled={busy}
        onChange={(event) => { setSelected(event.target.value); setError(null); }}>
        {model.columns.map((column) => <option key={column.id} value={column.id}>{column.name}</option>)}
      </select>
      <select aria-label="Read column type" value={model.types.get(columnId) ?? ""}
        disabled={busy} onChange={(event) => void change(event.target.value as ReadType | "")}>
        <option value="">From source</option>
        {READ_TYPES.map((type) => <option key={type} value={type}>
          {type === "string" ? "Text" : type[0].toUpperCase() + type.slice(1)}
        </option>)}
      </select>
    </div>
    {error && <p role="alert">{error}</p>}
  </details>;
}

import { useState } from "react";
import { isCalculatedFrameColumn, isEditableGridColumn, type TakeOwnershipHandler } from "./FrameGrid";
import { formulaToken } from "./lib/formulaReferences";
import type { Column, ComputedFrame, FrameObject, Row } from "./lib/types";

/**
 * Why the last edit attempt at the focused cell was refused, said in the
 * card's own status line rather than a window-level toast — errors go where
 * the thing is. The engine's `editing.reason` is the frame-level answer; a
 * calculated column beside editable inputs needs the column-level one.
 */
export function frameEditRefusalStatus(
  frame: FrameObject,
  computed: ComputedFrame,
  focus: { editRefused?: boolean; columnId: string; rowId: string } | null,
  actions: { rows: Row[]; rowOffset?: number; onEditCalculatedColumn: (frame: FrameObject, column: Column, row: number) => void; onTakeOwnership?: TakeOwnershipHandler; onTransformColumn: (frame: FrameObject, column: Column, formula: string) => void }
) {
  const refusedColumn = focus?.editRefused
    ? frame.columns.find((column) => column.id === focus.columnId)
    : undefined;
  if (!refusedColumn || isEditableGridColumn(computed, refusedColumn, frame)) return null;
  const text = isCalculatedFrameColumn(computed, refusedColumn)
    ? `${refusedColumn.name} is calculated. Edit its formula instead of typing over one result.`
    : computed.editing.reason ?? "This frame’s cells cannot be edited.";
  const rowIndex = actions.rows.findIndex((row) => row.id === focus?.rowId);
  const raw = actions.rows[rowIndex]?.cells[refusedColumn.id]?.raw;
  return (
    <div className="frame-page-controls frame-edit-refusal" role="status">
      <span>{text}</span>
      <RefusalActions
        key={JSON.stringify([focus?.rowId, refusedColumn.id, raw])}
        frame={frame}
        column={refusedColumn}
        raw={raw}
        onEditFormula={() => actions.onEditCalculatedColumn(
          frame, refusedColumn, Math.max(0, rowIndex) + (actions.rowOffset ?? 0)
        )}
        calculated={isCalculatedFrameColumn(computed, refusedColumn)}
        {...actions}
      />
    </div>
  );
}

function RefusalActions({ frame, column, raw, calculated, onEditFormula, onTakeOwnership, onTransformColumn }: {
  frame: FrameObject; column: Column; raw?: string; calculated: boolean;
  onEditFormula: () => void;
  onTakeOwnership?: TakeOwnershipHandler;
  onTransformColumn: (frame: FrameObject, column: Column, formula: string) => void;
}) {
  const [replacement, setReplacement] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const editableCopy = async () => {
    if (!confirming) { setConfirming(true); return; }
    setBusy(true);
    try { setError(await onTakeOwnership!(frame.id, { inlineError: true })); }
    catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  };
  return <>
    {calculated ? <button onClick={onEditFormula}>Edit column formula</button> : <>
      {raw !== undefined && raw !== "" && ["string", "categorical"].includes(column.dataType) && replacement === null &&
        <button onClick={() => setReplacement(raw)}>Replace all exact matches…</button>}
      {replacement !== null && <form onSubmit={(event) => {
        event.preventDefault();
        const token = formulaToken(column.name);
        onTransformColumn(frame, column, `when(${token} == ${JSON.stringify(raw)}).then(${JSON.stringify(replacement)}).otherwise(${token})`);
      }}>
        <label>Replace every {JSON.stringify(raw)} with <input aria-label="Replacement value" value={replacement} onChange={(event) => setReplacement(event.target.value)} /></label>
        <button type="submit">Open in Wrangle</button>
        <span>Applies to all matching rows, including future refreshes.</span>
      </form>}
      {onTakeOwnership && <button disabled={busy} onClick={() => void editableCopy()}>
        {busy ? "Making editable…" : confirming ? "Make editable — stop source refresh and recomputation" : "Make rows editable…"}
      </button>}
      {confirming && <span>These values become owned data. Undo restores the source.</span>}
    </>}
    {error && <span role="status">{error}</span>}
  </>;
}

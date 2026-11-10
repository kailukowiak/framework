import { useCallback } from "react";
import { nextColumnName } from "../FrameGrid";
import { appendColumnOperation } from "../lib/gridStructure";
import type { OperationHandler } from "../lib/handlers";
import type { DataType, FrameObject } from "../lib/types";

/**
 * What type a column born from one typed value should have: a plain number
 * makes a Number column, anything else is text. Deliberately narrow. The
 * engine has a fuller reader (the one a paste into an empty frame types
 * columns with), but it is not reachable from a single `addColumn`, and a
 * wrong guess here is worse than a modest one: "02134" or "1,5" read as
 * numbers would silently lose what was typed. So only the spelling that no
 * one means as text qualifies — optional minus, digits, an optional
 * fraction, and no leading zero before another digit.
 */
function typeForFirstValue(raw: string): DataType {
  return /^-?(0|[1-9]\d*)(\.\d+)?$/.test(raw.trim()) ? "number" : "string";
}

/**
 * Typing into the column that does not exist yet: one `addColumn` that
 * carries the typed value for this row in `values`, keyed by row id as
 * `addRow.values` is.
 *
 * It used to be `addColumn` and then, once the column had appeared, a
 * `setCell` — two undo steps for one gesture, because the engine mints the
 * new column's id in `prepare` and a `setCell` in the same `batch` had
 * nothing to name. Carrying the value on the operation itself removes the
 * need to name the column at all: the engine validates the value against the
 * new column's type, writes it, and undo takes the column and its value out
 * together. A value the type refuses refuses the whole add, so nothing
 * half-made is left behind.
 */
export function useGhostColumn({
  frame,
  enabled,
  onOperation,
}: {
  frame: FrameObject;
  enabled: boolean;
  onOperation: OperationHandler;
}) {
  const commit = useCallback(
    (rowId: string, raw: string) => {
      void onOperation(
        appendColumnOperation(frame, nextColumnName(frame), {
          dataType: typeForFirstValue(raw),
          values: { [rowId]: raw },
        })
      );
    },
    [frame, onOperation]
  );

  return enabled ? commit : null;
}

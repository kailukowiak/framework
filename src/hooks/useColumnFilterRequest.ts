import { useCallback, useEffect, useRef, useState } from "react";
import { appendColumnFilter } from "../PipelineColumnFilters";
import type { StepDraft } from "../lib/pipelineSteps";
import type { Column } from "../lib/types";

/**
 * What the column header's funnel asks of the Wrangle chain.
 *
 * The funnel's default answer is the value checklist -- Excel's most-used
 * filter gesture -- so a request opens the picker for that column and writes
 * nothing. Merely opening it must not remove rows, which is the same promise
 * the typed draft makes by staying local until Return.
 *
 * Dates are the exception and take the typed path. A date is not a string
 * literal in the formula language, so a checklist of dates would have to
 * know how to spell each one back as a predicate, and a list that cannot be
 * written back is a list that silently filters nothing. The typed filter
 * already knows the date spelling; it stays the way in for them, and for
 * anyone who picks "Edit in Wrangle" out of the picker.
 */
export function useColumnFilterRequest({
  request,
  onHandled,
  columns,
  setSteps,
}: {
  request?: { token: number; columnId: string };
  onHandled?: () => void;
  columns: Column[];
  setSteps: (update: (current: StepDraft[]) => StepDraft[]) => void;
}) {
  const handled = useRef<number | null>(null);
  const [picking, setPicking] = useState<{ column: Column; token: number } | null>(
    null
  );

  const editTyped = useCallback(
    (column: Column, token: number) =>
      setSteps((current) => appendColumnFilter(current, column, token)),
    [setSteps]
  );

  useEffect(() => {
    if (request === undefined || handled.current === request.token) return;
    handled.current = request.token;
    onHandled?.();
    const column = columns.find((candidate) => candidate.id === request.columnId);
    if (!column) return;
    if (column.dataType === "date") editTyped(column, request.token);
    else setPicking({ column, token: request.token });
  }, [columns, editTyped, onHandled, request]);

  const close = useCallback(() => setPicking(null), []);
  return { picking, close, editTyped };
}

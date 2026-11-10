import { useCallback, useState } from "react";
import { ValueFilterPicker } from "./ValueFilterPicker";
import { getDistinctValues } from "./lib/api";
import { stepInput, type StepDraft } from "./lib/pipelineSteps";
import type { Column } from "./lib/types";
import { writable, writeValueFilter, type LoadDistinctValues } from "./lib/valueFilter";

/**
 * Mounts the value checklist for one column of the chain being edited.
 *
 * The one decision made here is *which rows the list is read from*. The
 * picker writes its choice as a trailing Filter step, so on a second open the
 * saved chain already excludes every unchecked value; listing from it would
 * make those values vanish and never be checkable again. So the engine is
 * asked about the chain with the picker's own predicate taken off, fixed at
 * the moment the picker opens (re-reading as the draft changes would reload
 * the list under the person's hands).
 */
export function ColumnFilterPickerHost({
  frameId,
  column,
  steps,
  onChangeSteps,
  onEditInWrangle,
  onClose,
}: {
  frameId: string;
  column: Column;
  steps: StepDraft[];
  onChangeSteps: (steps: StepDraft[]) => void;
  onEditInWrangle: () => void;
  onClose: () => void;
}) {
  const [unfiltered] = useState(() => writeValueFilter(steps, column, null));
  const load = useCallback<LoadDistinctValues>(
    async (id, columnId, query) => {
      const found = await getDistinctValues(
        id,
        columnId,
        unfiltered.map(stepInput),
        undefined,
        query
      );
      // A number with more than four decimals reads back from the engine
      // rounded, so a predicate written from it would keep a different value
      // than the one ticked. Say so rather than filter wrongly.
      if (!writable(found.values.map((entry) => entry.value)))
        throw new Error(
          "Some values have more than four decimals and cannot be ticked. Use Edit in Wrangle."
        );
      return found;
    },
    [unfiltered]
  );
  return (
    <ValueFilterPicker
      frameId={frameId}
      column={column}
      steps={steps}
      loadDistinctValues={load}
      onChangeSteps={onChangeSteps}
      onEditInWrangle={onEditInWrangle}
      onClose={onClose}
    />
  );
}

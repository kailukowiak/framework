import { ListOrdered } from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import type { SequenceFillState } from "./ColumnAuthoringDialogs";
import type { OperationHandler } from "./lib/handlers";
import type { Column, FrameObject } from "./lib/types";
import type { ContextGeneratorInference } from "./lib/contextGeneratorInference";

/** The pointing gesture is only an author for the same formula a person types. */
export function InferredSeriesMenuAction({
  frame,
  column,
  inference,
  viewId,
  x,
  y,
  onClose,
  onFill,
  onOperation,
}: {
  frame: FrameObject;
  column: Column;
  inference: ContextGeneratorInference;
  viewId?: string;
  x: number;
  y: number;
  onClose: () => void;
  onFill: Dispatch<SetStateAction<SequenceFillState | null>>;
  onOperation: OperationHandler;
}) {
  const rule = inference.rule;
  if (!rule) return null;
  return (
    <button
      onClick={() => {
        onClose();
        if (inference.pattern) {
          onFill({
            frameId: frame.id,
            columnId: column.id,
            columnName: column.name,
            viewId,
            initialStart: inference.pattern.start,
            initialStep: inference.pattern.step,
            kind: inference.pattern.kind,
            dateUnit:
              inference.pattern.kind === "date" ? inference.pattern.unit : undefined,
          });
          return;
        }
        void onOperation({
          type: "addGeneratorFrame",
          name: column.name,
          formula: rule,
          columnName: column.name,
          x: x + 80,
          y: y + 40,
        });
      }}
    >
      <ListOrdered size={14} />
      <span>{inference.actionLabel}</span>
    </button>
  );
}

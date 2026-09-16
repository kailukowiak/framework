import { ChevronRight } from "lucide-react";
import { useContext } from "react";
import { NumberDisplayContext } from "./FrameGrid";
import { ParameterAnswer } from "./ParameterInputs";
import { scratchworkResultIsLong } from "./ScratchworkResultViewer";
import { takenWhen } from "./ScalarCards";
import { formatComputedScalar } from "./lib/columnFormatting";
import { beginVectorPointerDrag } from "./lib/vectorDrag";
import type { OperationHandler } from "./lib/handlers";
import type { ComputedBlockLine } from "./lib/types";

function gutterRowTitle(
  line: ComputedBlockLine,
  answered: boolean
): string | undefined {
  if (line.error) return line.error;
  if (line.frozen) return `Frozen ${takenWhen(line.frozen.takenAt)}`;
  return answered ? "Open and copy this answer" : undefined;
}

/** One answer beside its line, spanning as many rows as the line does. */
export function GutterRow({
  line,
  formula,
  span,
  banded,
  open,
  onToggle,
  onOperation,
}: {
  line: ComputedBlockLine;
  formula: string | null;
  span: number;
  banded: boolean;
  open: boolean;
  onToggle: () => void;
  onOperation: OperationHandler;
}) {
  const useGrouping = useContext(NumberDisplayContext);
  const answered = !line.blank && !line.comment && !line.error;
  const draggable = answered && formula !== null && line.valueCount > 0;
  const className = gutterRowClass(line, answered, banded, span);
  const spanStyle =
    span > 1 ? { height: `calc(var(--text-sm) * 1.7 * ${span})` } : undefined;
  const title = gutterRowTitle(line, answered);
  // A solved line says how it got there and offers to make it real. Both
  // ride in the row that already exists rather than a panel under it: the
  // gutter is one line per line, and a goal seek that pushed its neighbours
  // down the page would cost more than it told you.
  const solve = answered ? line.solve : undefined;
  const row = answered ? (
    <ParameterAnswer id={line.id} className={className} style={spanStyle}>
    <button
      type="button"
      className={`${className}${open ? " open" : ""}${
        draggable ? " draggable-formula" : ""
      }`}
      style={spanStyle}
      title={
        draggable
          ? "Drag this answer onto a table or empty canvas; click to open"
          : title
      }
      aria-label={`Open ${line.name || "scratchwork"} result`}
      aria-expanded={open}
      onClick={onToggle}
    >
      {draggable && (
        <span
          className="formula-drag-handle"
          aria-hidden="true"
          onClick={(event) => event.stopPropagation()}
          onPointerDown={(event) =>
            beginVectorPointerDrag(event.nativeEvent, {
              objectId: line.id,
              formula,
              name: line.name,
              length: line.valueCount,
            })
          }
        >
          ↗
        </span>
      )}
      <span>
        {formatComputedScalar(
          line.typedValue,
          line.dataType,
          line.display,
          useGrouping
        )}
        {solve && (
          <span className="block-solve-note">
            {` \u00b7 ${solve.iterations} steps \u00b7 residual ${residualText(
              solve.residual,
              useGrouping
            )}`}
          </span>
        )}
        {line.frozen && (
          <i className={line.frozen.stale ? "stale" : "frozen"}>
            {line.frozen.stale ? "*" : "·"}
          </i>
        )}
      </span>
      <ChevronRight size={10} aria-hidden />
    </button></ParameterAnswer>
  ) : (
    <div className={className} style={spanStyle} title={title}>
      {line.error ? "!" : ""}
    </div>
  );
  if (!solve) return row;
  // Applying the answer is the ordinary SetValue the person could have
  // typed, so undo reaches it like any other edit.
  return (
    <div className="block-gutter-solve" style={spanStyle}>
      {row}
      <button
        type="button"
        className="inline-action"
        aria-label={`Apply solved value to ${solve.targetName}`}
        onClick={() =>
          void onOperation({
            type: "setValue",
            objectId: solve.targetId,
            raw: solve.answerRaw,
          })
        }
      >
        Apply
      </button>
    </div>
  );
}

/**
 * How close the search got, in the same number format the answer beside it
 * uses. Bisection lands on float noise rather than on nothing, and a gutter
 * reading "residual 0.000000000000284" is telling you about arithmetic
 * rather than about your model, so anything under a billionth reads as 0.
 */
function residualText(residual: number, useGrouping: boolean): string {
  if (Math.abs(residual) < 1e-9) return "0";
  return formatComputedScalar(
    { type: "number", value: residual },
    "number",
    String(residual),
    useGrouping
  );
}

function gutterRowClass(line: ComputedBlockLine, answered: boolean, banded: boolean, span: number): string {
  return `block-gutter-row${line.error ? " failed" : ""}${
    line.frozen ? (line.frozen.stale ? " stale" : " frozen") : ""
  }${answered && scratchworkResultIsLong(line.display) ? " long" : ""}${
    banded ? " banded" : ""
  }${span > 1 ? " spanning" : ""}`;
}

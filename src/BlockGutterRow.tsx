import { ChevronRight } from "lucide-react";
import { useContext } from "react";
import { NumberDisplayContext } from "./FrameGrid";
import { ParameterAnswer } from "./ParameterInputs";
import { scratchworkResultIsLong } from "./ScratchworkResultViewer";
import { takenWhen } from "./ScalarCards";
import { formatComputedScalar } from "./lib/columnFormatting";
import { beginVectorPointerDrag } from "./lib/vectorDrag";
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
}: {
  line: ComputedBlockLine;
  formula: string | null;
  span: number;
  banded: boolean;
  open: boolean;
  onToggle: () => void;
}) {
  const useGrouping = useContext(NumberDisplayContext);
  const answered = !line.blank && !line.comment && !line.error;
  const draggable = answered && formula !== null && line.valueCount > 0;
  const className = gutterRowClass(line, answered, banded, span);
  const spanStyle =
    span > 1 ? { height: `calc(var(--text-sm) * 1.7 * ${span})` } : undefined;
  const title = gutterRowTitle(line, answered);
  return answered ? (
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
}

function gutterRowClass(line: ComputedBlockLine, answered: boolean, banded: boolean, span: number): string {
  return `block-gutter-row${line.error ? " failed" : ""}${
    line.frozen ? (line.frozen.stale ? " stale" : " frozen") : ""
  }${answered && scratchworkResultIsLong(line.display) ? " long" : ""}${
    banded ? " banded" : ""
  }${span > 1 ? " spanning" : ""}`;
}

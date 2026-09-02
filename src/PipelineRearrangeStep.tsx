import { reorderColumnIds } from "./lib/pipelineStepCommands";
import type { StepDraft, VisibleColumn } from "./lib/pipelineSteps";

type ColumnDrop = { columnId: string; after: boolean };

/** The draggable column order of one Rearrange columns step. */
export function PipelineRearrangeStep({
  step,
  visible,
  draggingColumn,
  setDraggingColumn,
  columnDrop,
  setColumnDrop,
  savePatch,
}: {
  step: Extract<StepDraft, { kind: "select" }>;
  visible: VisibleColumn[];
  draggingColumn: string | null;
  setDraggingColumn: (columnId: string | null) => void;
  columnDrop: ColumnDrop | null;
  setColumnDrop: (drop: ColumnDrop | null) => void;
  savePatch: (
    stepId: string,
    update: (step: StepDraft) => StepDraft
  ) => Promise<string | null>;
}) {
  return (
    <div className="pipeline-column-order" role="list">
      {step.columnIds.map((columnId) => {
        const column = visible.find((candidate) => candidate.id === columnId);
        if (!column) return null;
        const drop = columnDrop?.columnId === columnId ? columnDrop : null;
        return (
          <div
            role="listitem"
            tabIndex={0}
            aria-label={`${column.name}, position ${
              step.columnIds.indexOf(columnId) + 1
            } of ${step.columnIds.length}`}
            key={columnId}
            data-pipeline-column-id={columnId}
            className={`${draggingColumn === columnId ? "dragging" : ""} ${
              drop ? (drop.after ? "drop-after" : "drop-before") : ""
            }`}
            title="Drag to rearrange"
            onPointerDown={(event) => {
              if (event.button !== 0) return;
              event.preventDefault();
              event.currentTarget.focus();
              const start = { x: event.clientX, y: event.clientY };
              const list = event.currentTarget.closest(
                ".pipeline-column-order"
              );
              let moved = false;
              let latestDrop: { columnId: string; after: boolean } | null =
                null;
              const move = (moveEvent: PointerEvent) => {
                if (
                  !moved &&
                  Math.hypot(
                    moveEvent.clientX - start.x,
                    moveEvent.clientY - start.y
                  ) < 3
                )
                  return;
                moved = true;
                setDraggingColumn(columnId);
                const target = document
                  .elementFromPoint(moveEvent.clientX, moveEvent.clientY)
                  ?.closest<HTMLElement>("[data-pipeline-column-id]");
                if (
                  !target ||
                  target.closest(".pipeline-column-order") !== list
                ) {
                  latestDrop = null;
                  setColumnDrop(null);
                  return;
                }
                const bounds = target.getBoundingClientRect();
                latestDrop = {
                  columnId: target.dataset.pipelineColumnId!,
                  after: moveEvent.clientY >= bounds.top + bounds.height / 2,
                };
                setColumnDrop(latestDrop);
              };
              const end = () => {
                window.removeEventListener("pointermove", move);
                window.removeEventListener("pointerup", end);
                window.removeEventListener("pointercancel", end);
                if (moved && latestDrop) {
                  const ordered = reorderColumnIds(
                    step.columnIds,
                    columnId,
                    latestDrop.columnId,
                    latestDrop.after
                  );
                  if (ordered !== step.columnIds)
                    void savePatch(step.id, (current) =>
                      current.kind === "select"
                        ? { ...current, columnIds: ordered }
                        : current
                    );
                }
                setDraggingColumn(null);
                setColumnDrop(null);
              };
              window.addEventListener("pointermove", move);
              window.addEventListener("pointerup", end);
              window.addEventListener("pointercancel", end);
            }}
            onKeyDown={(event) => {
              const offset =
                event.key === "ArrowUp"
                  ? -1
                  : event.key === "ArrowDown"
                  ? 1
                  : 0;
              if (!offset) return;
              const from = step.columnIds.indexOf(columnId);
              const to = from + offset;
              if (to < 0 || to >= step.columnIds.length) return;
              event.preventDefault();
              const ordered = [...step.columnIds];
              [ordered[from], ordered[to]] = [ordered[to], ordered[from]];
              void savePatch(step.id, (current) =>
                current.kind === "select"
                  ? { ...current, columnIds: ordered }
                  : current
              );
            }}
          >
            {column.name}
          </div>
        );
      })}
    </div>
  );
}

import { CircleAlert, RefreshCw } from "lucide-react";
import { SelectionStatisticsStatus } from "./SelectionStatisticsStatus";
import { displayedSummaryRows } from "./FrameSummaryFooter";
import type { GridContext, GridFocus } from "./FrameGrid";
import { DEFAULT_CANVAS_ZOOM, formatCanvasZoom } from "./lib/canvasZoom";
import type { OperationHandler } from "./lib/handlers";

/** Readouts and exceptional actions that belong in the canvas corner. */
export function CanvasStatus({
  withInspector,
  context,
  focus,
  documentPath,
  staleCount,
  refreshing,
  zoom,
  onOperation,
  onSave,
  onRefresh,
  onZoom,
}: {
  withInspector: boolean;
  context: GridContext | null;
  focus: GridFocus | null;
  documentPath: string | null;
  staleCount: number;
  refreshing: boolean;
  zoom: number;
  onOperation: OperationHandler;
  onSave: () => void;
  onRefresh: () => void;
  onZoom: (zoom: number) => void;
}) {
  return (
    <div className={`canvas-status ${withInspector ? "with-inspector" : ""}`}>
      <SelectionStatisticsStatus
        context={context}
        focus={focus}
        onAddSummary={(operation) => {
          if (context) {
            const rows = displayedSummaryRows(context.frame);
            void onOperation({
              type: "setFrameSummaryRows",
              frameId: context.frame.id,
              summaryRows: rows.includes(operation) ? rows : [...rows, operation],
            });
          }
        }}
      />
      {!documentPath && (
        <button
          className="unsaved-canvas"
          onClick={onSave}
          title="This canvas has no file. Nothing on it survives quitting until it does."
        >
          <CircleAlert size={14} /> Unsaved canvas — save it
        </button>
      )}
      {staleCount > 0 && (
        <button
          className="toolbar-button stale-refresh"
          disabled={refreshing}
          onClick={onRefresh}
          title="Recompute every snapshot that has fallen behind, starting from the top of each chain"
        >
          <RefreshCw className={refreshing ? "spinning" : ""} size={14} />
          {refreshing ? "Refreshing…" : `Refresh ${staleCount} stale`}
        </button>
      )}
      {zoom !== DEFAULT_CANVAS_ZOOM && (
        <button
          className="canvas-zoom-readout"
          title="Reset the canvas to 100% (⌘0)"
          onClick={() => onZoom(DEFAULT_CANVAS_ZOOM)}
        >
          {formatCanvasZoom(zoom)}
        </button>
      )}
    </div>
  );
}

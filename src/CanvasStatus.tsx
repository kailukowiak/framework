import { CircleAlert, RefreshCw } from "lucide-react";
import { ScenarioSwitcher } from "./ScenarioSwitcher";
import { SelectionStatisticsStatus } from "./SelectionStatisticsStatus";
import { displayedSummaryRows } from "./FrameSummaryFooter";
import type { GridContext, GridFocus } from "./FrameGrid";
import { DEFAULT_CANVAS_ZOOM, formatCanvasZoom } from "./lib/canvasZoom";
import type { OperationHandler } from "./lib/handlers";
import type { DocumentView } from "./lib/types";

/** Readouts and exceptional actions that belong in the canvas corner. */
export function CanvasStatus({
  withInspector,
  withCollapsedInspector,
  document,
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
  withCollapsedInspector: boolean;
  /** Only for the scenario switcher, which is a statement about the whole
      document rather than about whatever card is selected. */
  document: Pick<DocumentView, "scenarios" | "activeScenario">;
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
    <div
      className={`canvas-status ${withInspector ? "with-inspector" : ""} ${
        withCollapsedInspector ? "with-inspector-collapsed" : ""
      }`}
    >
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
      <ScenarioSwitcher
        scenarios={document.scenarios ?? []}
        activeScenario={document.activeScenario ?? null}
        onOperation={onOperation}
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

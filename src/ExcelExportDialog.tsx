import { CircleAlert, FileSpreadsheet, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useExportRowCounts } from "./hooks/useExportRowCounts";
import { createPortal } from "react-dom";
import type { DocumentView, FrameObject } from "./lib/types";

function hasNamedValues(document: DocumentView): boolean {
  return document.objects.some((object) => {
    if (object.kind === "value" || object.kind === "result") return true;
    if (object.kind !== "block") return false;
    return document.computedBlocks[object.id]?.lines.some(
      (line) => !line.blank && !line.comment
    );
  });
}

export function ExcelExportDialog({
  document,
  onClose,
  onExport,
}: {
  document: DocumentView;
  onClose: () => void;
  onExport: (frameIds: string[], includeLineage: boolean, currentView: boolean) => Promise<boolean>;
}) {
  const frames = document.objects.filter(
    (object): object is FrameObject => object.kind === "frame"
  );
  const namedValues = hasNamedValues(document);
  const [selected, setSelected] = useState(() => new Set(frames.map((frame) => frame.id)));
  const [includeLineage, setIncludeLineage] = useState(true);
  const [currentView, setCurrentView] = useState(true);
  const counts = useExportRowCounts(document, currentView);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const allSelected = frames.length > 0 && selected.size === frames.length;

  // Escape closes, same as every other dialog — but not mid-export, when the
  // native save panel owns the keyboard and the work is already running.
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !exporting) onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [exporting, onClose]);

  const exportWorkbook = async () => {
    setExporting(true);
    setError(null);
    try {
      if (
        await onExport(
          frames.filter((frame) => selected.has(frame.id)).map((frame) => frame.id),
          includeLineage,
          currentView
        )
      ) {
        onClose();
      }
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
    } finally {
      setExporting(false);
    }
  };

  return createPortal(
    <div className="dialog-backdrop excel-export-backdrop" onPointerDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}>
      <div className="insert-dialog excel-export-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">EXCEL EXPORT</span>
            <h2>Choose worksheets</h2>
          </div>
          <button className="icon-button" aria-label="Close Excel export" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <label className="excel-export-row">
          <span>Rows to export</span>
          <select aria-label="Rows to export" value={currentView ? "view" : "all"} onChange={(event) => setCurrentView(event.target.value === "view")}>
            <option value="view">Current view — filtered and sorted rows</option>
            <option value="all">Entire table — ignore display filters and sorting</option>
          </select>
        </label>
        <p>Tables export as rows and columns; canvas layout and formatting are not included. Named answers keep their formula’s scope.</p>
        {counts?.error && <p role="status">Could not count export rows: {counts.error}</p>}
        {frames.length > 0 && (
          <div className="excel-export-list">
            <label className="excel-export-row excel-export-all">
              <input
                type="checkbox"
                checked={allSelected}
                onChange={(event) => setSelected(
                  event.target.checked ? new Set(frames.map((frame) => frame.id)) : new Set()
                )}
              />
              <strong>All tables</strong>
              <span>{selected.size} of {frames.length}</span>
            </label>
            {frames.map((frame) => (
              <label className="excel-export-row" key={frame.id}>
                <input
                  type="checkbox"
                  checked={selected.has(frame.id)}
                  onChange={(event) => {
                    setSelected((current) => {
                      const next = new Set(current);
                      if (event.target.checked) next.add(frame.id);
                      else next.delete(frame.id);
                      return next;
                    });
                  }}
                />
                <span>{frame.name}</span>
                <small>{counts?.counts[frame.id]?.toLocaleString() ?? "…"} rows</small>
              </label>
            ))}
          </div>
        )}

        {namedValues && (
          <div className="excel-export-values">
            <FileSpreadsheet size={14} />
            <span><strong>Values</strong> — named constants and current formula results</span>
          </div>
        )}
        {frames.length === 0 && !namedValues && (
          <p className="excel-export-empty">This document has no tables or named values to export.</p>
        )}
        <label className="excel-export-row">
          <input
            type="checkbox"
            checked={includeLineage}
            onChange={(event) => setIncludeLineage(event.target.checked)}
          />
          <span>Include a sheet explaining how each column was computed</span>
        </label>
        {error && (
          <div className="formula-editor-error">
            <CircleAlert size={12} />
            <span>{error}</span>
          </div>
        )}
        <div className="dialog-actions">
          <button className="secondary-action" onClick={onClose}>Cancel</button>
          <button
            className="primary-action"
            disabled={exporting || (selected.size === 0 && !namedValues)}
            onClick={() => void exportWorkbook()}
          >
            {exporting ? "Exporting…" : "Export .xlsx…"}
          </button>
        </div>
      </div>
    </div>,
    globalThis.document.body
  );
}

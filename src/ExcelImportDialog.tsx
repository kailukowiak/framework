import { CircleAlert, FileSpreadsheet, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  previewExcelRange,
  type ExcelRangePreview,
  type ExcelWorkbookInfo,
} from "./lib/api";

export interface ExcelRangeSelection {
  sheetName: string;
  cellRange: string;
  hasHeader: boolean;
  name: string;
}

interface ImportDraft extends ExcelRangeSelection {
  preview: ExcelRangePreview | null;
  error: string | null;
  busy: boolean;
}

function useExcelImport(
  workbook: ExcelWorkbookInfo,
  onImport: (selection: ExcelRangeSelection, another: boolean) => Promise<void>
) {
  const firstTable = workbook.tables[0];
  const firstRegion = workbook.suggestedRegions[0];
  const firstSheet = workbook.sheets[0];
  const [draft, setDraft] = useState<ImportDraft>({
    sheetName: firstTable?.sheetName ?? firstRegion?.sheetName ?? firstSheet?.name ?? "",
    cellRange: firstTable?.cellRange ?? firstRegion?.cellRange ?? firstSheet?.usedRange ?? "A1:A1",
    name: firstTable?.name ?? firstSheet?.name ?? "Imported data",
    hasHeader: true,
    preview: null,
    error: null,
    busy: false,
  });
  const tablesForSheet = useMemo(
    () => workbook.tables.filter((table) => table.sheetName === draft.sheetName),
    [draft.sheetName, workbook.tables]
  );
  const regionsForSheet = useMemo(
    () => workbook.suggestedRegions.filter((region) => region.sheetName === draft.sheetName),
    [draft.sheetName, workbook.suggestedRegions]
  );

  useEffect(() => {
    if (!draft.sheetName || !draft.cellRange.trim()) return;
    let disposed = false;
    const timer = window.setTimeout(() => {
      void previewExcelRange(
        workbook.path,
        draft.sheetName,
        draft.cellRange,
        draft.hasHeader
      )
        .then((preview) => {
          if (!disposed) setDraft((current) => ({ ...current, preview, error: null }));
        })
        .catch((reason) => {
          if (!disposed) {
            setDraft((current) => ({
              ...current,
              preview: null,
              error: String(reason).replace(/^Error:\s*/, ""),
            }));
          }
        });
    }, 220);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [draft.cellRange, draft.hasHeader, draft.sheetName, workbook.path]);

  const chooseSheet = (sheetName: string) => {
    const sheet = workbook.sheets.find((candidate) => candidate.name === sheetName);
    const table = workbook.tables.find((candidate) => candidate.sheetName === sheetName);
    const region = workbook.suggestedRegions.find(
      (candidate) => candidate.sheetName === sheetName
    );
    setDraft((current) => ({
      ...current,
      sheetName,
      cellRange: table?.cellRange ?? region?.cellRange ?? sheet?.usedRange ?? "A1:A1",
      name: table?.name ?? sheetName,
      preview: null,
      error: null,
    }));
  };
  const chooseRegion = (cellRange: string) => {
    if (!regionsForSheet.some((candidate) => candidate.cellRange === cellRange)) return;
    setDraft((current) => ({
      ...current,
      cellRange,
      name: current.sheetName,
      hasHeader: true,
      preview: null,
      error: null,
    }));
  };
  const chooseTable = (tableName: string) => {
    const table = workbook.tables.find((candidate) => candidate.name === tableName);
    if (!table) return;
    setDraft((current) => ({
      ...current,
      sheetName: table.sheetName,
      cellRange: table.cellRange,
      name: table.name,
      hasHeader: true,
      preview: null,
      error: null,
    }));
  };
  const change = (updates: Partial<ExcelRangeSelection>) => {
    const invalidatesPreview = updates.cellRange !== undefined || updates.hasHeader !== undefined;
    setDraft((current) => ({
      ...current,
      ...updates,
      preview: invalidatesPreview ? null : current.preview,
      error: invalidatesPreview ? null : current.error,
    }));
  };
  const submit = async (another: boolean) => {
    if (!draft.preview || draft.error || !draft.name.trim()) return;
    setDraft((current) => ({ ...current, busy: true }));
    try {
      const { sheetName, cellRange, hasHeader } = draft;
      await onImport({ sheetName, cellRange, hasHeader, name: draft.name.trim() }, another);
    } catch (reason) {
      setDraft((current) => ({
        ...current,
        error: String(reason).replace(/^Error:\s*/, ""),
      }));
    } finally {
      setDraft((current) => ({ ...current, busy: false }));
    }
  };
  return {
    draft,
    tablesForSheet,
    regionsForSheet,
    chooseSheet,
    chooseTable,
    chooseRegion,
    change,
    submit,
  };
}

function RangePreview({ preview }: { preview: ExcelRangePreview | null }) {
  if (!preview) return null;
  return (
    <>
      <div className="excel-preview-meta">
        <span>{preview.rowCount.toLocaleString()} data rows</span>
        <span>{preview.columns.length.toLocaleString()} columns</span>
        {preview.formulaCellCount > 0 && (
          <span>{preview.formulaCellCount.toLocaleString()} cached formula results</span>
        )}
      </div>
      <div className="excel-preview-scroll">
        <table>
          <thead><tr>{preview.columns.map((column, index) => (
            <th key={`${column}-${index}`}>{column}</th>
          ))}</tr></thead>
          <tbody>{preview.rows.map((row, rowIndex) => (
            <tr key={rowIndex}>{preview.columns.map((_, columnIndex) => (
              <td key={columnIndex}>{row[columnIndex] ?? ""}</td>
            ))}</tr>
          ))}</tbody>
        </table>
      </div>
      {preview.formulaCellCount > 0 && (
        <p className="excel-import-warning"><CircleAlert size={13} /> Formula cells are imported only as their last saved values. FrameWork does not retain or evaluate Excel formulas.</p>
      )}
      {preview.errorCellCount > 0 && (
        <p className="excel-import-warning"><CircleAlert size={13} /> {preview.errorCellCount} Excel error cell{preview.errorCellCount === 1 ? "" : "s"} will be imported as text.</p>
      )}
    </>
  );
}

export function ExcelImportDialog({
  workbook,
  onClose,
  onImport,
}: {
  workbook: ExcelWorkbookInfo;
  onClose: () => void;
  onImport: (selection: ExcelRangeSelection, another: boolean) => Promise<void>;
}) {
  const controller = useExcelImport(workbook, onImport);
  const { draft } = controller;
  const disabled = draft.busy || !draft.preview || Boolean(draft.error) || !draft.name.trim();
  return (
    <div className="dialog-backdrop">
      <div className="insert-dialog excel-import-dialog">
        <div className="dialog-header">
          <div><span className="eyebrow">EXCEL VALUES</span><h2>Import a worksheet range</h2></div>
          <button className="icon-button" onClick={onClose} aria-label="Close Excel import"><X size={18} /></button>
        </div>
        <div className="excel-import-source">
          <FileSpreadsheet size={15} />
          <span><strong>{workbook.fileName}</strong><small>{workbook.path}</small></span>
        </div>
        <div className="excel-import-fields">
          <label>Sheet<select value={draft.sheetName} onChange={(event) => controller.chooseSheet(event.target.value)}>
            {workbook.sheets.map((sheet) => <option key={sheet.name} value={sheet.name}>{sheet.name} · {sheet.usedRange ?? "empty"}</option>)}
          </select></label>
          <label>Range<input value={draft.cellRange} spellCheck={false} placeholder="A7:H3281" onChange={(event) => controller.change({ cellRange: event.target.value.toUpperCase() })} /></label>
          <label>Frame name<input value={draft.name} onChange={(event) => controller.change({ name: event.target.value })} /></label>
        </div>
        {controller.tablesForSheet.length > 0 && <label className="excel-table-picker">Excel table
          <select defaultValue="" onChange={(event) => controller.chooseTable(event.target.value)}>
            <option value="">Choose a defined table…</option>
            {controller.tablesForSheet.map((table) => <option key={table.name} value={table.name}>{table.name} · {table.cellRange}</option>)}
          </select>
        </label>}
        {controller.regionsForSheet.length > 0 && <label className="excel-table-picker">Suggested region
          <select defaultValue="" onChange={(event) => controller.chooseRegion(event.target.value)}>
            <option value="">Choose a detected rectangle…</option>
            {controller.regionsForSheet.map((region) => (
              <option key={region.cellRange} value={region.cellRange}>
                {region.cellRange} · {region.rowCount.toLocaleString()} rows × {region.columnCount.toLocaleString()} columns
              </option>
            ))}
          </select>
        </label>}
        <label className="plot-toggle excel-header-toggle">
          <input type="checkbox" checked={draft.hasHeader} onChange={(event) => controller.change({ hasHeader: event.target.checked })} /> First row contains column names
        </label>
        <RangePreview preview={draft.preview} />
        {draft.error && <div className="formula-editor-error">{draft.error}</div>}
        <div className="dialog-actions">
          <button className="secondary-action" onClick={onClose} disabled={draft.busy}>Cancel</button>
          <button className="secondary-action" onClick={() => void controller.submit(true)} disabled={disabled}>Import and add another</button>
          <button className="primary-action" onClick={() => void controller.submit(false)} disabled={disabled}>{draft.busy ? "Importing…" : "Import range"}</button>
        </div>
      </div>
    </div>
  );
}

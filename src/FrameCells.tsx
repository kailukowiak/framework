import { useContext, useEffect, useRef, useState } from "react";
import { NumberDisplayContext } from "./FrameGrid";
import { columnFormatBadge, formatCellValue } from "./lib/columnFormatting";
import type { GridDirection } from "./lib/gridNavigation";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  ColumnFormat,
  ComputedCell,
  DataType,
  Row,
  FrameObject,
} from "./lib/types";

export function GridCellContent({
  column,
  row,
  computedCell,
  isDerived,
  paged = false,
  readOnly = false,
  editable = false,
  readOnlyReason,
  editing,
  onNavigate,
  onEdit,
  onEditFormula,
  onCommit,
  onCancel,
  onFormulaKey,
}: {
  column: Column;
  row: Row;
  computedCell?: ComputedCell;
  isDerived: boolean;
  paged?: boolean;
  readOnly?: boolean;
  /**
   * Whether a keystroke in this cell can become a real operation — the
   * per-column answer (`isEditableGridColumn`), not the frame-level flag.
   * The editor renders only when this is true: an editor whose commit the
   * engine would refuse is an edit that appears to work and does nothing,
   * which is the exact failure the engine's `FrameEditing` contract exists
   * to prevent.
   */
  editable?: boolean;
  readOnlyReason?: string;
  editing: { seed: string | null } | null;
  /** Receives the click so shift-click can extend rather than reset. */
  onNavigate: (event: React.MouseEvent) => void;
  onEdit: () => void;
  onEditFormula?: () => void;
  onCommit: (raw: string, move: GridDirection | null) => void;
  onCancel: () => void;
  /** `=` typed into an empty editor: the column's formula editor opens instead. */
  onFormulaKey?: () => void;
}) {
  const cell = row.cells[column.id];
  const displayFormat = displayedColumnFormat(column);
  if (column.formula || computedCell?.isOverride) {
    return (
      <ComputedCellButton
        column={column}
        raw={cell?.raw}
        computedCell={computedCell}
        displayFormat={displayFormat}
        readOnlyReason={readOnlyReason}
        onNavigate={onNavigate}
        onEditFormula={onEditFormula}
      />
    );
  }
  // A paged frame -- an import, or anything derived from one, including a
  // grouped aggregate -- is read entirely through the core's lazy Polars
  // plan, which delivers evaluated values on the row itself. Those rows
  // have no entry in `computedFrames` (their ids are page-local), so the
  // row's own value is the only value there is, derived column or not.
  if (paged || (readOnly && !isDerived)) {
    return (
      <DisplayCell
        column={column}
        raw={cell?.raw ?? ""}
        displayFormat={displayFormat}
        editable={false}
        title={readOnlyReason ?? "This cell comes from the frame's source"}
        onNavigate={onNavigate}
        // Not an editor: the edit request is forwarded so the card can
        // refuse it visibly, with the engine's reason, instead of a
        // double-click doing nothing at all.
        onEdit={onEdit}
      />
    );
  }
  if (isDerived) {
    return (
      <ComputedCellButton
        column={column}
        raw={cell?.raw}
        computedCell={computedCell}
        displayFormat={displayFormat}
        readOnlyReason={readOnlyReason}
        onNavigate={onNavigate}
        onEditFormula={onEditFormula}
      />
    );
  }
  if (editing && editable) {
    if (column.dataType === "categorical") {
      return (
        <select
          className="categorical-cell"
          autoFocus
          defaultValue={cell?.raw ?? ""}
          onChange={(event) => onCommit(event.target.value, null)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              onCommit(event.currentTarget.value, event.shiftKey ? "up" : "down");
            } else if (event.key === "Tab") {
              event.preventDefault();
              onCommit(event.currentTarget.value, event.shiftKey ? "left" : "right");
            } else if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              onCancel();
            }
          }}
          onBlur={() => onCancel()}
        >
          <option value="">—</option>
          {(column.categories ?? []).map((category) => (
            <option key={category} value={category}>
              {category}
            </option>
          ))}
        </select>
      );
    }
    return (
      <GridCellEditor
        initial={editing.seed ?? cell?.raw ?? ""}
        placeholder={
          column.dataType === "date"
            ? "YYYY-MM-DD"
            : column.dataType === "boolean"
            ? "true / false"
            : ""
        }
        onCommit={onCommit}
        onCancel={onCancel}
        onFormulaKey={onFormulaKey}
      />
    );
  }
  // The catch-all display also serves the cell whose edit request was
  // refused above (`editing` set, `editable` false): rendering the value it
  // actually holds, never an editor, is what keeps a refused keystroke from
  // leaving a draft on screen that the document does not contain.
  return (
    <DisplayCell
      column={column}
      raw={cell?.raw ?? ""}
      displayFormat={displayFormat}
      editable={editable}
      title={editable ? undefined : readOnlyReason}
      onNavigate={onNavigate}
      onEdit={editable ? onEdit : undefined}
    />
  );
}

/** A value shown as a value: read-only unless a keystroke here can commit. */
function DisplayCell({
  column,
  raw,
  displayFormat,
  editable,
  title,
  onNavigate,
  onEdit,
}: {
  column: Column;
  raw: string;
  displayFormat: ColumnFormat | null;
  editable: boolean;
  title?: string;
  onNavigate: (event: React.MouseEvent) => void;
  onEdit?: () => void;
}) {
  return (
    <div
      className={`cell-display${editable ? "" : " read-only"}${
        displayFormat ? " formatted-numeric" : ""
      }`}
      title={
        title ??
        (editable ? "Double-click or press F2 to edit" : "This cell cannot be edited")
      }
      onClick={onNavigate}
      onDoubleClick={onEdit}
    >
      {displayFormat ? (
        <FormattedCellValue raw={raw} format={displayFormat} dataType={column.dataType} />
      ) : (
        raw
      )}
    </div>
  );
}

function ComputedCellButton({
  column,
  raw,
  computedCell,
  displayFormat,
  readOnlyReason,
  onNavigate,
  onEditFormula,
}: {
  column: Column;
  raw?: string;
  computedCell?: ComputedCell;
  displayFormat: ColumnFormat | null;
  readOnlyReason?: string;
  onNavigate: (event: React.MouseEvent) => void;
  onEditFormula?: () => void;
}) {
  return (
    <button
      className={`computed-cell${displayFormat ? " formatted-numeric" : ""}`}
      title={
        computedCell?.error ??
        (computedCell?.isOverride
          ? "Legacy cell formula · press F2 or use the formula bar to inspect"
          : onEditFormula
          ? "Double-click to edit the column formula"
          : readOnlyReason ?? "Select cell")
      }
      onClick={onNavigate}
      onDoubleClick={onEditFormula}
    >
      {displayFormat ? (
        <FormattedCellValue
          raw={formattedCellSource(computedCell, raw)}
          format={displayFormat}
          dataType={column.dataType}
        />
      ) : (
        computedCell?.display ?? raw ?? "—"
      )}
      {computedCell?.isOverride && <span className="override-mark">ƒ</span>}
    </button>
  );
}

function GridCellEditor({
  initial,
  placeholder,
  onCommit,
  onCancel,
  onFormulaKey,
}: {
  initial: string;
  placeholder?: string;
  onCommit: (raw: string, move: GridDirection | null) => void;
  onCancel: () => void;
  onFormulaKey?: () => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const settled = useRef(false);
  useEffect(() => {
    const element = inputRef.current;
    if (!element) return;
    element.focus();
    element.setSelectionRange(element.value.length, element.value.length);
  }, []);
  return (
    <input
      ref={inputRef}
      className="cell-editor"
      defaultValue={initial}
      placeholder={placeholder}
      spellCheck={false}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          settled.current = true;
          onCommit(event.currentTarget.value, event.shiftKey ? "up" : "down");
        } else if (event.key === "Tab") {
          event.preventDefault();
          settled.current = true;
          onCommit(event.currentTarget.value, event.shiftKey ? "left" : "right");
        } else if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          settled.current = true;
          onCancel();
        } else if (event.key === "=" && !event.currentTarget.value && onFormulaKey) {
          // A leading `=` is a doorway, not a character: this value editor
          // closes untouched and the column's formula editor opens.
          event.preventDefault();
          settled.current = true;
          onFormulaKey();
        }
      }}
      onBlur={(event) => {
        if (!settled.current) onCommit(event.target.value, null);
      }}
    />
  );
}

export function EditableColumnHeader({
  frame,
  column,
  onOperation,
}: {
  frame: FrameObject;
  column: Column;
  onOperation: OperationHandler;
}) {
  const [editing, setEditing] = useState(false);
  if (!editing) {
    return (
      <span
        title="Double-click to rename column"
        onDoubleClick={(event) => {
          event.stopPropagation();
          setEditing(true);
        }}
      >
        {column.name}
      </span>
    );
  }
  return (
    <input
      className="column-header-input"
      autoFocus
      defaultValue={column.name}
      onClick={(event) => event.stopPropagation()}
      onBlur={(event) => {
        setEditing(false);
        const name = event.target.value.trim();
        if (name && name !== column.name)
          onOperation({
            type: "renameColumn",
            frameId: frame.id,
            columnId: column.id,
            name,
          });
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter") event.currentTarget.blur();
        if (event.key === "Escape") setEditing(false);
      }}
    />
  );
}

export function displayedColumnFormat(column: Column): ColumnFormat | null {
  if (column.format) return column.format;
  if (column.dataType === "integer") return { style: "number", decimals: 0 };
  if (column.dataType === "number") return { style: "number", decimals: null };
  if (column.dataType === "currency") return { style: "currency", decimals: null };
  if (column.dataType === "percentage") return { style: "percent", decimals: null };
  return null;
}

function formattedCellSource(
  result: ComputedCell | undefined,
  fallback?: string
): string | number {
  return result?.typedValue.type === "number"
    ? result.typedValue.value
    : result?.display ?? fallback ?? "";
}

export function ColumnFormatBadge({ format }: { format?: ColumnFormat | null }) {
  const badge = format ? columnFormatBadge(format) : null;
  return badge ? (
    <span
      className="column-format-badge"
      title="Displayed unit; stored values are unscaled"
    >
      {badge}
    </span>
  ) : null;
}

export function FormattedCellValue({
  raw,
  format,
  dataType,
}: {
  raw: string | number | null | undefined;
  format: ColumnFormat;
  dataType: DataType;
}) {
  const useGrouping = useContext(NumberDisplayContext);
  const parts = formatCellValue(raw, format, { dataType, useGrouping });
  if (!parts.symbol) return <>{parts.value || "—"}</>;
  return (
    <span className="accounting-cell">
      <span className="accounting-symbol">{parts.symbol}</span>
      <span className="accounting-value">{parts.value || "—"}</span>
    </span>
  );
}

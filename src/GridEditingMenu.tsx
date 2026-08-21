import {
  ArrowDown,
  ArrowRight,
  Eraser,
  Pencil,
  type LucideIcon,
} from "lucide-react";
import { isEditableGridColumn, type GridContext, type GridFocus } from "./FrameGrid";
import {
  clearGridRangeUpdates,
  fillGridRangeUpdates,
  type GridCellUpdate,
} from "./lib/gridEditing";
import type { Column, ComputedFrame, Selection } from "./lib/types";

type CellEditRequest = {
  viewId: string;
  rowId: string;
  columnId: string;
};

type GridEditingAction = {
  label: string;
  shortcut: string;
  Icon: LucideIcon;
  edit?: CellEditRequest;
  updates?: GridCellUpdate[];
};

/** The click-accessible spelling of the grid's existing edit shortcuts. */
export function GridEditingMenu({
  column,
  computed,
  rowId,
  viewId,
  gridContext,
  gridFocus,
  frameId,
  onClose,
  onSelect,
  onGridFocus,
  onSetCells,
}: {
  column: Column | null;
  computed: ComputedFrame | undefined;
  rowId?: string;
  viewId?: string;
  gridContext: GridContext | null;
  gridFocus: GridFocus | null;
  frameId: string;
  onClose: () => void;
  onSelect: (selection: Selection) => void;
  onGridFocus: (focus: GridFocus) => void;
  onSetCells: (updates: GridCellUpdate[]) => void;
}) {
  const resolvedViewId = viewId ?? gridFocus?.viewId;
  const clear =
    gridContext && gridFocus
      ? clearGridRangeUpdates(gridContext, gridFocus)
      : [];
  const fillDown =
    gridContext && gridFocus
      ? fillGridRangeUpdates(gridContext, gridFocus, "down")
      : [];
  const fillRight =
    gridContext && gridFocus
      ? fillGridRangeUpdates(gridContext, gridFocus, "right")
      : [];
  const actions: GridEditingAction[] = [];
  if (
    column &&
    rowId &&
    resolvedViewId &&
    isEditableGridColumn(computed, column)
  ) {
    actions.push({
      label: "Edit cell",
      shortcut: "F2",
      Icon: Pencil,
      edit: { viewId: resolvedViewId, rowId, columnId: column.id },
    });
  }
  if (clear.length)
    actions.push({ label: "Clear contents", shortcut: "Delete", Icon: Eraser, updates: clear });
  if (fillDown.length)
    actions.push({ label: "Fill down", shortcut: "⌘D", Icon: ArrowDown, updates: fillDown });
  if (fillRight.length)
    actions.push({ label: "Fill right", shortcut: "⌘R", Icon: ArrowRight, updates: fillRight });
  if (!actions.length) return null;
  return (
    <>
      {actions.map(({ label, shortcut, Icon, edit, updates }) => (
        <button
          key={label}
          onClick={() => {
            onClose();
            if (edit) {
              onSelect({
                objectId: frameId,
                viewId: edit.viewId,
                rowId: edit.rowId,
                columnId: edit.columnId,
              });
              onGridFocus({
                viewId: edit.viewId,
                objectId: frameId,
                rowId: edit.rowId,
                columnId: edit.columnId,
                mode: "edit",
                editSeed: null,
                anchor: null,
                span: null,
              });
            } else if (updates) onSetCells(updates);
          }}
        >
          <Icon size={14} />
          <span>{label}</span>
          <kbd>{shortcut}</kbd>
        </button>
      ))}
      <span className="menu-separator" />
    </>
  );
}

import { ChevronRight, Copy, Database } from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import { GridEditingMenu } from "./GridEditingMenu";
import type {
  ContextMenuState,
  GridContext,
  GridFocus,
} from "./FrameGrid";
import type { InspectorPanelAction } from "./lib/inspectorPanel";
import type {
  Column,
  DocumentView,
  FrameObject,
  Operation,
  Selection,
} from "./lib/types";

export type ContextMenuGridItemsProps = {
  contextMenu: ContextMenuState;
  contextFrame: FrameObject | null;
  contextColumn: Column | null;
  contextGrid: GridContext | null;
  gridFocus: GridFocus | null;
  document: DocumentView;
  copyIncludesHeaders: boolean;
  copySelection: (includeHeaders: boolean) => Promise<void>;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
  setSelection: Dispatch<SetStateAction<Selection | null>>;
  setGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
  setInspectorSection: (action: InspectorPanelAction) => void;
  setNotice: Dispatch<SetStateAction<string | null>>;
};

/**
 * What the right-click menu offers about the cells themselves: copying the
 * selection, the editing actions for the cell under the pointer, and the
 * way out of a grid that will not take an edit.
 */
export function ContextMenuGridItems({
  contextMenu,
  contextFrame,
  contextColumn,
  contextGrid,
  gridFocus,
  document,
  copyIncludesHeaders,
  copySelection,
  run,
  setContextMenu,
  setSelection,
  setGridFocus,
  setInspectorSection,
  setNotice,
}: ContextMenuGridItemsProps) {
  return (
    <>
      {/* Copy comes first and applies to the selection, not to whatever
          the pointer happens to be over — right-clicking inside a range
          is how people reach for it, and the range is what they mean.
          The preference changes what this one ordinary Copy does; its
          inverse remains explicit without turning the menu into a
          preferences panel. */}
      {gridFocus && contextFrame && contextFrame.id === gridFocus.objectId && (
        <>
          <button
            onClick={() => {
              setContextMenu(null);
              void copySelection(copyIncludesHeaders);
            }}
          >
            <Copy size={14} />
            <span>Copy</span>
            <kbd>⌘C</kbd>
          </button>
          <button
            onClick={() => {
              setContextMenu(null);
              void copySelection(!copyIncludesHeaders);
            }}
          >
            <Copy size={14} />
            <span>
              {copyIncludesHeaders ? "Copy without headers" : "Copy with headers"}
            </span>
          </button>
          <span className="menu-separator" />
        </>
      )}
      {contextFrame && (
        <GridEditingMenu
          column={contextColumn}
          computed={document.computedFrames[contextFrame.id]}
          rowId={contextMenu.rowId}
          viewId={contextMenu.viewId}
          gridContext={contextGrid}
          gridFocus={gridFocus}
          frameId={contextFrame.id}
          onClose={() => setContextMenu(null)}
          onSelect={setSelection}
          onGridFocus={setGridFocus}
          onSetCells={(updates) => {
            void run({
              type: "setCells",
              frameId: contextFrame.id,
              cells: updates,
            });
          }}
        />
      )}
      {contextFrame &&
        contextMenu.rowId &&
        !document.computedFrames[contextFrame.id]?.editing.cells && (
          <details className="context-menu-submenu">
            <summary>
              <Database size={14} />
              <span>Data source</span>
              <ChevronRight className="submenu-chevron" size={14} />
            </summary>
            <div>
              <button
                onClick={() => {
                  const reason =
                    document.computedFrames[contextFrame.id]?.editing.reason;
                  setContextMenu(null);
                  setSelection({
                    objectId: contextFrame.id,
                    viewId: contextMenu.viewId,
                    rowId: contextMenu.rowId,
                    columnId: contextColumn?.id,
                  });
                  setInspectorSection("selection");
                  setNotice(reason ?? "Make an owned copy to edit this value.");
                }}
              >
                <Database size={14} />
                <span>Make these rows editable…</span>
              </button>
            </div>
          </details>
        )}
    </>
  );
}

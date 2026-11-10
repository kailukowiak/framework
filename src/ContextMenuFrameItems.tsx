import {
  ArrowDownToLine,
  BarChart3,
  FolderOpen,
  FunctionSquare,
  GitBranch,
  GitMerge,
  Plus,
  Save,
  Trash2,
} from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import {
  nextColumnName,
  type ContextMenuState,
  type GridContext,
  type GridFocus,
} from "./FrameGrid";
import {
  deleteRowsLabel,
  deleteRowsOperation,
  appendColumnOperation,
  appendRowOperation,
  framePermitsColumnEdits,
  gridPositionIsMeaningful,
  insertColumnOperation,
  insertRowOperation,
  selectedRowIds,
} from "./lib/gridStructure";
import { defaultPlotSpec, viewHolding } from "./lib/canvasCards";
import type { InspectorPanelAction } from "./lib/inspectorPanel";
import type { JoinState } from "./lib/joinState";
import type { ImportMode } from "./lib/preferences";
import type {
  Column,
  DataType,
  DocumentView,
  FrameObject,
  Operation,
  Selection,
} from "./lib/types";

/** The format an in-place update would write, read off the source path. Take
 *  the basename first: a directory may carry a dot the filename does not. */
function sourceFormatName(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toUpperCase() : "file";
}

export type ContextMenuFrameActionsProps = {
  contextMenu: ContextMenuState;
  contextFrame: FrameObject;
  contextColumn: Column | null;
  askOnImport: boolean;
  importMode: ImportMode;
  runAppendImport: (
    target: { frameId: string; x: number; y: number },
    mode: ImportMode
  ) => Promise<boolean>;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setAppendImport: Dispatch<
    SetStateAction<{ frameId: string; x: number; y: number } | null>
  >;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
  setInspectorSection: (action: InspectorPanelAction) => void;
  setJoin: Dispatch<SetStateAction<JoinState>>;
  setSelection: Dispatch<SetStateAction<Selection | null>>;
};

/** What this frame can become: joined, appended to, read from, rewritten. */
export function ContextMenuFrameActions({
  contextMenu,
  contextFrame,
  contextColumn,
  askOnImport,
  importMode,
  runAppendImport,
  run,
  setAppendImport,
  setContextMenu,
  setInspectorSection,
  setJoin,
  setSelection,
}: ContextMenuFrameActionsProps) {
  return (
    <>
      <button
        onClick={() => {
          setContextMenu(null);
          setJoin({
            primaryFrameId: contextFrame.id,
            x: contextMenu.canvasX + 60,
            y: contextMenu.canvasY + 40,
          });
        }}
      >
        <GitMerge size={14} />
        <span>Combine with…</span>
      </button>
      <button
        onClick={() => {
          const target = {
            frameId: contextFrame.id,
            x: contextMenu.canvasX + 54,
            y: contextMenu.canvasY + 54,
          };
          setContextMenu(null);
          // An append source has the same static-versus-refreshable
          // choice as an ordinary import. Keeping the prompt before
          // the picker avoids quietly creating a different kind of
          // data because this import began at a frame menu.
          if (askOnImport) {
            setAppendImport(target);
            return;
          }
          void runAppendImport(target, importMode);
        }}
      >
        <FolderOpen size={14} />
        <span>Import and append…</span>
      </button>
      <button
        onClick={() => {
          setContextMenu(null);
          run({
            type: "addLinkedFrame",
            sourceFrameId: contextFrame.id,
            name: `${contextFrame.name} frame`,
            x: contextMenu.canvasX + 28,
            y: contextMenu.canvasY + 28,
          });
        }}
      >
        <GitBranch size={14} />
        <span>Create frame from this</span>
      </button>
      {/* The period declaration is one choice from the frame's own date
          columns — never free text — so it is a select like the column
          type above rather than a dialog. Declaring is what lets prior
          read the period before instead of the row above. */}
      {contextFrame.columns.some((column) => column.dataType === "date") && (
        <label className="context-menu-field">
          <span>Period column</span>
          <select
            value={contextFrame.period?.columnId ?? ""}
            onChange={(event) => {
              setContextMenu(null);
              const columnId = event.target.value;
              run({
                type: "setFramePeriod",
                frameId: contextFrame.id,
                period: columnId
                  ? { columnId, partitionColumnIds: [] }
                  : null,
              });
            }}
          >
            <option value="">None</option>
            {contextFrame.columns
              .filter((column) => column.dataType === "date")
              .map((column) => (
                <option key={column.id} value={column.id}>
                  {column.name}
                </option>
              ))}
          </select>
        </label>
      )}
      {contextFrame.derivation && (
        <button
          onClick={() => {
            setContextMenu(null);
            setSelection({ objectId: contextFrame.id });
            setInspectorSection("wrangle");
          }}
        >
          <FunctionSquare size={14} />
          <span>
            {contextColumn
              ? `Filter or sort by ${contextColumn.name}`
              : "Edit transformations"}
          </span>
        </button>
      )}
    </>
  );
}

export type ContextMenuFramePlotItemsProps = {
  contextMenu: ContextMenuState;
  contextFrame: FrameObject;
  updateOriginalFile: (frameId: string) => Promise<void>;
  exportFrameFile: (frameId: string) => Promise<void>;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
  setError: Dispatch<SetStateAction<string | null>>;
};

/** Taking the frame elsewhere: drawn as a plot, or written out as CSV. */
export function ContextMenuFramePlotItems({
  contextMenu,
  contextFrame,
  updateOriginalFile,
  exportFrameFile,
  run,
  setContextMenu,
}: ContextMenuFramePlotItemsProps) {
  return (
    <>
      {/* A plot of this frame can live beside it as a tab or stand
          on its own; both are one operation apart, so offer both
          rather than making the user move it afterwards. */}
      {contextMenu.viewId && (
        <button
          onClick={() => {
            setContextMenu(null);
            run({
              type: "addPlot",
              name: `${contextFrame.name} plot`,
              sourceFrameId: contextFrame.id,
              spec: defaultPlotSpec(contextFrame),
              x: 0,
              y: 0,
              viewId: contextMenu.viewId,
            });
          }}
        >
          <BarChart3 size={14} />
          <span>Plot in this card</span>
        </button>
      )}
      <button
        onClick={() => {
          setContextMenu(null);
          run({
            type: "addPlot",
            name: `${contextFrame.name} plot`,
            sourceFrameId: contextFrame.id,
            spec: defaultPlotSpec(contextFrame),
            x: contextMenu.canvasX + 28,
            y: contextMenu.canvasY + 28,
          });
        }}
      >
        <BarChart3 size={14} />
        <span>Plot in a new window</span>
      </button>
      <button
        onClick={() => {
          setContextMenu(null);
          void exportFrameFile(contextFrame.id);
        }}
      >
        <ArrowDownToLine size={14} />
        <span>Export frame as…</span>
      </button>
      {contextFrame.fileOrigin && (
        <button
          onClick={() => {
            setContextMenu(null);
            void updateOriginalFile(contextFrame.id);
          }}
        >
          <Save size={14} />
          {/* An update writes the source's own format, so the label reads that
              format off the path rather than choosing between two names. */}
          <span>Update original {sourceFormatName(contextFrame.fileOrigin.path)}…</span>
        </button>
      )}
    </>
  );
}

export type ContextMenuFrameEditItemsProps = {
  contextMenu: ContextMenuState;
  contextFrame: FrameObject;
  contextColumn: Column | null;
  document: DocumentView;
  deleteContextColumn: () => void;
  deleteFromContext: (operation: Operation) => void;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
  /** With these, Delete row acts on every row the selection spans. */
  contextGrid?: GridContext | null;
  gridFocus?: GridFocus | null;
};

/**
 * Structural edits, which an owned grid has and a computed one does not:
 * rows and columns added or deleted, a column's stored type changed.
 */
export function ContextMenuFrameEditItems({
  contextMenu,
  contextFrame,
  contextColumn,
  document,
  deleteContextColumn,
  deleteFromContext,
  run,
  setContextMenu,
  contextGrid = null,
  gridFocus = null,
}: ContextMenuFrameEditItemsProps) {
  const computed = document.computedFrames[contextFrame.id];
  const editsRows = Boolean(computed?.editing.rows);
  const editsColumns = framePermitsColumnEdits(document, contextFrame.id);
  const positionable = gridPositionIsMeaningful(contextFrame, computed, contextGrid);
  const rowIds = selectedRowIds(contextGrid, gridFocus, contextMenu.rowId);
  const insertRow = (where: "above" | "below") => {
    if (!contextMenu.rowId) return;
    setContextMenu(null);
    run(insertRowOperation(contextFrame, contextMenu.rowId, where));
  };
  const insertColumn = (side: "left" | "right") => {
    if (!contextColumn) return;
    setContextMenu(null);
    run(
      insertColumnOperation(contextFrame, contextColumn.id, side, nextColumnName(contextFrame))
    );
  };
  return (
    <>
      {/* A grid whose columns the engine will not edit can still choose
          which outputs it shows. This is exactly unchecking the column in a
          final Select. The gate is `editing.columns`, not `editing.rows`: a
          frame can own its rows and still have a chain that fixes its
          schema, and offering "Delete column" there is offering a refusal.
          "Hide" here and "Delete" below each name what the click writes. */}
      {!editsColumns && contextColumn && (
        <button className="destructive" onClick={deleteContextColumn}>
          <Trash2 size={14} />
          <span>Hide column</span>
        </button>
      )}
      {editsRows && (
        <>
          {/* A row with a position gets above/below; the append is only for
              a click that is not on a row, a frame read from a file, where
              the engine refuses a position, or a sorted or filtered view,
              where a position on screen is not one the engine can keep. */}
          {contextMenu.rowId && positionable ? (
            <>
              <button onClick={() => insertRow("above")}>
                <Plus size={14} />
                <span>Insert row above</span>
              </button>
              <button onClick={() => insertRow("below")}>
                <Plus size={14} />
                <span>Insert row below</span>
              </button>
            </>
          ) : (
            <button
              onClick={() => {
                setContextMenu(null);
                run(appendRowOperation(contextFrame.id));
              }}
            >
              <Plus size={14} />
              <span>Add empty row</span>
            </button>
          )}
        </>
      )}
      {editsColumns && (
        <>
          {contextColumn ? (
            <>
              <button onClick={() => insertColumn("left")}>
                <Plus size={14} />
                <span>Insert column left</span>
              </button>
              <button onClick={() => insertColumn("right")}>
                <Plus size={14} />
                <span>Insert column right</span>
              </button>
            </>
          ) : (
            <button
              onClick={() => {
                setContextMenu(null);
                run(appendColumnOperation(contextFrame, nextColumnName(contextFrame)));
              }}
            >
              <Plus size={14} />
              <span>Add column</span>
            </button>
          )}
          {contextColumn && (
            <label className="context-menu-field">
              <span>Column type</span>
              <select
                value={contextColumn.dataType}
                onChange={(event) => {
                  setContextMenu(null);
                  run({
                    type: "setColumnType",
                    frameId: contextFrame.id,
                    columnId: contextColumn.id,
                    dataType: event.target.value as DataType,
                  });
                }}
              >
                <option value="string">Text</option>
                <option value="categorical">Categorical</option>
                <option value="integer">Integer</option>
                <option value="number">Number</option>
                <option value="currency">Currency</option>
                <option value="accounting">Accounting</option>
                <option value="percentage">Percentage</option>
                <option value="boolean">Boolean</option>
                <option value="date">Date</option>
              </select>
            </label>
          )}
        </>
      )}
      {((editsRows && contextMenu.rowId) || (editsColumns && contextColumn)) && (
        <span className="menu-separator" />
      )}
      {editsRows && contextMenu.rowId && (
        <button
          className="destructive"
          onClick={() =>
            deleteFromContext(deleteRowsOperation(contextFrame.id, rowIds))
          }
        >
          <Trash2 size={14} />
          <span>{deleteRowsLabel(rowIds.length)}</span>
        </button>
      )}
      {editsColumns && contextColumn && (
        <button className="destructive" onClick={deleteContextColumn}>
          <Trash2 size={14} />
          <span>Delete column</span>
        </button>
      )}
    </>
  );
}

export type ContextMenuFrameShapeItemsProps = {
  contextMenu: ContextMenuState;
  contextFrame: FrameObject;
  document: DocumentView;
  deleteFromContext: (operation: Operation) => void;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
};

/** Where a derived frame lives — a tab on its source or a card of its own —
 * and the end of it. */
export function ContextMenuFrameShapeItems({
  contextMenu,
  contextFrame,
  document,
  deleteFromContext,
  run,
  setContextMenu,
}: ContextMenuFrameShapeItemsProps) {
  return (
    <>
      {/* The one control for a relationship that has two shapes. A
          derived frame is either a tab on the card it reads from or
          a card of its own with a cord back to it — never both, and
          never neither — so it is a switch between them rather than
          two commands that each only apply half the time. There is
          exactly one card it would sensibly join, so nothing has to
          be aimed at and nothing can be missed. */}
      {(() => {
        const own = viewHolding(document, contextFrame.id);
        const parentId = contextFrame.derivation?.sourceFrameId;
        const parent = parentId
          ? document.objects.find((object) => object.id === parentId)
          : undefined;
        const parentView = parentId ? viewHolding(document, parentId) : undefined;
        if (!own || !parent || !parentView) return null;
        const tabbed = parentView.id === own.id;
        return (
          <label className="context-menu-check">
            <input
              type="checkbox"
              checked={tabbed}
              onChange={(event) => {
                setContextMenu(null);
                void run(
                  event.target.checked
                    ? {
                        type: "moveTab",
                        sourceViewId: own.id,
                        targetViewId: parentView.id,
                        objectId: contextFrame.id,
                        targetIndex: parentView.tabObjectIds?.length ?? 1,
                      }
                    : {
                        type: "detachTab",
                        viewId: own.id,
                        objectId: contextFrame.id,
                        x: contextMenu.canvasX,
                        y: contextMenu.canvasY,
                      }
                );
              }}
            />
            <span>Show as a tab on {parent.name}</span>
          </label>
        );
      })()}
      <button
        className="destructive"
        onClick={() =>
          deleteFromContext({
            type: "deleteObject",
            objectId: contextFrame.id,
          })
        }
      >
        <Trash2 size={14} />
        <span>Delete frame</span>
      </button>
    </>
  );
}

import {
  ArrowDownToLine,
  BarChart3,
  FolderOpen,
  FunctionSquare,
  GitBranch,
  GitMerge,
  Plus,
  Trash2,
} from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import { nextColumnName, type ContextMenuState } from "./FrameGrid";
import { defaultPlotSpec, viewHolding } from "./lib/canvasCards";
import { exportFrameCsv } from "./lib/api";
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
        <span>Join another frame</span>
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
  run,
  setContextMenu,
  setError,
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
          void exportFrameCsv(contextFrame.id).catch((reason) =>
            setError(String(reason).replace(/^Error:\s*/, ""))
          );
        }}
      >
        <ArrowDownToLine size={14} />
        <span>Export CSV</span>
      </button>
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
}: ContextMenuFrameEditItemsProps) {
  return (
    <>
      {/* A read-only grid can still choose which outputs it shows.
          This is exactly unchecking the column in a final Select;
          owned rows keep the structural delete offered below. */}
      {!document.computedFrames[contextFrame.id]?.editing.rows &&
        contextColumn && (
          <button
            className="destructive"
            onClick={deleteContextColumn}
          >
            <Trash2 size={14} />
            <span>Delete column</span>
          </button>
        )}
      {document.computedFrames[contextFrame.id]?.editing.rows && (
        <>
            <button
              onClick={() => {
                setContextMenu(null);
                run({ type: "addRow", frameId: contextFrame.id, values: {} });
              }}
            >
              <Plus size={14} />
              <span>Add empty row</span>
            </button>
            <button
              onClick={() => {
                setContextMenu(null);
                run({
                  type: "addColumn",
                  frameId: contextFrame.id,
                  name: nextColumnName(contextFrame),
                  dataType: "string",
                  afterColumnId:
                    contextColumn?.id ??
                    contextFrame.columns.at(-1)?.id ??
                    null,
                });
              }}
            >
              <Plus size={14} />
              <span>
                {contextColumn ? "Insert column here" : "Add column"}
              </span>
            </button>
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
                  <option value="percentage">Percentage</option>
                  <option value="boolean">Boolean</option>
                  <option value="date">Date</option>
                </select>
              </label>
            )}
            {contextMenu.rowId && (
              <button
                className="destructive"
                onClick={() =>
                  deleteFromContext({
                    type: "deleteRow",
                    frameId: contextFrame.id,
                    rowId: contextMenu.rowId!,
                  })
                }
              >
                <Trash2 size={14} />
                <span>Delete row</span>
              </button>
            )}
            {contextColumn && (
              <button
                className="destructive"
                onClick={deleteContextColumn}
              >
                <Trash2 size={14} />
                <span>Delete column</span>
              </button>
            )}
        </>
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
        const parentView = parentId
          ? viewHolding(document, parentId)
          : undefined;
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

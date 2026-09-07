import { DictionaryMenuItems } from "./DictionaryMenuItems";
import { Frame, Trash2 } from "lucide-react";
import { ContextMenuGroup, ContextMenuSurface } from "./ContextMenuSurface";
import type { ContextMenuState } from "./FrameGrid";
import {
  ContextMenuColumnDisplayItems,
  ContextMenuColumnItems,
  type ContextMenuColumnDisplayItemsProps,
  type ContextMenuColumnItemsProps,
} from "./ContextMenuColumnItems";
import {
  ContextMenuFrameActions,
  ContextMenuFrameEditItems,
  ContextMenuFramePlotItems,
  ContextMenuFrameShapeItems,
  type ContextMenuFrameActionsProps,
  type ContextMenuFrameEditItemsProps,
  type ContextMenuFramePlotItemsProps,
  type ContextMenuFrameShapeItemsProps,
} from "./ContextMenuFrameItems";
import {
  ContextMenuGridItems,
  type ContextMenuGridItemsProps,
} from "./ContextMenuGridItems";
import {
  ContextMenuContainerItems,
  ContextMenuCreateItems,
  type ContextMenuContainerItemsProps,
  type ContextMenuCreateItemsProps,
} from "./ContextMenuObjectItems";
import type { DataObject, FrameObject, Operation } from "./lib/types";

/**
 * Everything the right-click menu can offer, which is everything a frame,
 * a column, a cell, a card and the bare canvas each know how to do. The
 * parts are grouped by what they act on; this component is the dispatch
 * that decides which of them the click was asking for.
 *
 * The frame branch's members take `contextFrame` narrowed to a frame, which
 * is why they are handed it explicitly rather than only through the spread.
 */
export type CanvasContextMenuProps = ContextMenuGridItemsProps &
  ContextMenuContainerItemsProps &
  ContextMenuCreateItemsProps &
  Omit<ContextMenuColumnItemsProps, "contextFrame"> &
  Omit<ContextMenuColumnDisplayItemsProps, "contextFrame"> &
  Omit<ContextMenuFrameActionsProps, "contextFrame"> &
  Omit<ContextMenuFramePlotItemsProps, "contextFrame"> &
  Omit<ContextMenuFrameEditItemsProps, "contextFrame"> &
  Omit<ContextMenuFrameShapeItemsProps, "contextFrame"> & {
    contextMenu: ContextMenuState;
    contextObject: DataObject | null;
    contextFrame: FrameObject | null;
    contextKind: string;
    deleteFromContext: (operation: Operation) => void;
  };

export function CanvasContextMenu(props: CanvasContextMenuProps) {
  const {
    contextMenu,
    contextKind,
    contextObject,
    contextFrame,
    contextColumn,
    deleteFromContext,
  } = props;
  return (
    <ContextMenuSurface x={contextMenu.screenX} y={contextMenu.screenY}>
      <div className="context-menu-heading">
        <span>{contextKind}</span>
        {(contextObject || contextFrame) && (
          <strong>
            {contextColumn
              ? `${contextFrame?.name} / ${contextColumn.name}`
              : (contextFrame ?? contextObject)?.name}
          </strong>
        )}
      </div>
      <ContextMenuGridItems {...props} />
      <ContextMenuContainerItems {...props} />
      {!contextObject ? (
        <ContextMenuCreateItems {...props} />
      ) : contextFrame ? (
        <>
          <DictionaryMenuItems document={props.document} frame={contextFrame} column={contextColumn}
            run={props.run} close={() => props.setContextMenu(null)}
            onMap={(formula) => contextColumn && props.requestColumnTransformation(contextFrame, contextColumn, formula, true, contextMenu.viewId)} />
          <ContextMenuColumnItems {...props} contextFrame={contextFrame} />
          <ContextMenuColumnDisplayItems {...props} contextFrame={contextFrame} />
          {/* Everything below reads as "what this frame does with other
              frames and views", "row and column structure", then "undo
              this by deleting it" — three separators, not one undivided
              list, so the eye can skip a whole group at a glance. */}
          <span className="menu-separator" />
          <ContextMenuGroup
            collapsed={contextMenu.rowId !== undefined}
            label="Frame actions"
            Icon={Frame}
          >
            <ContextMenuFrameActions {...props} contextFrame={contextFrame} />
            <span className="menu-separator" />
            <ContextMenuFramePlotItems {...props} contextFrame={contextFrame} />
            <span className="menu-separator" />
            <ContextMenuFrameEditItems {...props} contextFrame={contextFrame} />
            <ContextMenuFrameShapeItems {...props} contextFrame={contextFrame} />
          </ContextMenuGroup>
        </>
      ) : (
        <button
          className="destructive"
          onClick={() =>
            deleteFromContext({
              type: "deleteObject",
              objectId: contextObject.id,
            })
          }
        >
          <Trash2 size={14} />
          <span>Delete {contextObject.kind}</span>
        </button>
      )}
    </ContextMenuSurface>
  );
}

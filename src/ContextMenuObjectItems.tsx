import {
  BookOpen,
  FolderInput,
  FolderOutput,
  FolderPlus,
  Grid3X3,
  ListOrdered,
  SquareFunction,
  Table2 as FrameIcon,
  Type,
} from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import type { ContextMenuState } from "./FrameGrid";
import type {
  ContainerObject,
  DataObject,
  Operation,
} from "./lib/types";

export type ContextMenuContainerItemsProps = {
  contextObject: DataObject | null;
  containers: ContainerObject[];
  containedIds: Set<string>;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
};

/**
 * Filing a card under a container, or taking it back out. Offered for the
 * kinds that a container can hold, and only for the containers that are not
 * already holding this one.
 */
export function ContextMenuContainerItems({
  contextObject,
  containers,
  containedIds,
  run,
  setContextMenu,
}: ContextMenuContainerItemsProps) {
  return (
    <>
      {contextObject &&
        (contextObject.kind === "value" ||
          contextObject.kind === "series" ||
          contextObject.kind === "container") && (
          <>
            {containers
              .filter(
                (candidate) =>
                  candidate.id !== contextObject.id &&
                  !candidate.memberIds.includes(contextObject.id)
              )
              .map((candidate) => (
                <button
                  key={candidate.id}
                  onClick={() => {
                    setContextMenu(null);
                    run({
                      type: "moveIntoContainer",
                      objectId: contextObject.id,
                      containerId: candidate.id,
                    });
                  }}
                >
                  <FolderInput size={14} />
                  <span>Keep under {candidate.name}</span>
                </button>
              ))}
            {containedIds.has(contextObject.id) && (
              <button
                onClick={() => {
                  setContextMenu(null);
                  run({
                    type: "moveIntoContainer",
                    objectId: contextObject.id,
                    containerId: null,
                  });
                }}
              >
                <FolderOutput size={14} />
                <span>Take out onto the canvas</span>
              </button>
            )}
            <span className="menu-separator" />
          </>
        )}
    </>
  );
}

export type ContextMenuCreateItemsProps = {
  contextMenu: ContextMenuState;
  addBlock: (position?: { x: number; y: number }) => Promise<string | null>;
  addText: (position?: { x: number; y: number }) => Promise<string | null>;
  addCalculationMatrix: (
    position?: { x: number; y: number }
  ) => Promise<string | null>;
  addEmptyFrame: (position?: { x: number; y: number }) => Promise<string | null>;
  addContainer: (position?: { x: number; y: number }) => Promise<string | null>;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
};

/** Right-clicking bare canvas: every card that can be made where the
 * pointer is. */
export function ContextMenuCreateItems({
  contextMenu,
  addBlock,
  addText,
  addCalculationMatrix,
  addEmptyFrame,
  addContainer,
  run,
  setContextMenu,
}: ContextMenuCreateItemsProps) {
  return (
    <>
        <button onClick={() => {
          setContextMenu(null);
          void run({ type: "addDictionary", name: "Mapping frame", x: contextMenu.canvasX, y: contextMenu.canvasY });
        }}><BookOpen size={14} /><span>Add mapping frame here</span></button>
        {/* A block, a frame, a container. There used to be three more
            — a value, a result, a list — and every one of them made a
            card that held one number. Those are lines of a block now,
            which is the same three things in a tenth of the room. */}
        <button
          onClick={() => {
            setContextMenu(null);
            void addBlock({
              x: contextMenu.canvasX,
              y: contextMenu.canvasY,
            });
          }}
        >
          <SquareFunction size={14} />
          <span>Add formula block here</span>
        </button>
        <button
          onClick={() => {
            setContextMenu(null);
            void addText({
              x: contextMenu.canvasX,
              y: contextMenu.canvasY,
            });
          }}
        >
          <Type size={14} />
          <span>Add text here</span>
        </button>
        <button
          onClick={() => {
            setContextMenu(null);
            void addCalculationMatrix({
              x: contextMenu.canvasX,
              y: contextMenu.canvasY,
            });
          }}
        >
          <Grid3X3 size={14} />
          <span>Add calculation matrix here</span>
        </button>
        <button
          onClick={() => {
            setContextMenu(null);
            void addEmptyFrame({
              x: contextMenu.canvasX,
              y: contextMenu.canvasY,
            });
          }}
        >
          <FrameIcon size={14} />
          <span>Add frame here</span>
        </button>
        <button
          onClick={() => {
            setContextMenu(null);
            void addContainer({
              x: contextMenu.canvasX,
              y: contextMenu.canvasY,
            });
          }}
        >
          <FolderPlus size={14} />
          <span>Add container here</span>
        </button>
        <button
          onClick={() => {
            setContextMenu(null);
            void run({
              type: "addGeneratorFrame",
              name: "Generator",
              formula: "sequence(1, 11)",
              columnName: null,
              x: contextMenu.canvasX,
              y: contextMenu.canvasY,
            });
          }}
        >
          <ListOrdered size={14} />
          <span>Add generator here</span>
        </button>
    </>
  );
}

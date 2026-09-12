import { lineageSourceIds } from "./lib/lineageSources";
import { CanvasCardResizeControls, type ResizeEdge } from "./CanvasCardResizeControls";
import { ModelCard } from "./models/ModelCard";
import { useCardGeometry } from "./hooks/useCardGeometry";
import { CardOutline } from "./CardOutline";
import { ContainerCard } from "./ContainerCard";
import {
  useEffect,
  useSyncExternalStore,
  type Dispatch,
  type SetStateAction,
} from "react";
import { BlockCard, BlockCardPreview } from "./BlockCard";
import { CanvasCardWindowControls } from "./CanvasCardWindowControls";
import type { TakeOwnershipHandler } from "./FrameGrid";
import { FrameCard } from "./FrameCard";
import { FrameViewTabs } from "./FrameViewTabs";
import { PlotCard } from "./PlotCard";
import {
  ResultCard,
  SeriesCard,
  ValueCard,
  VariableCard,
  objectFormulaToken,
  scalarFormulaReferences,
} from "./ScalarCards";
import { TextCard } from "./TextCard";
import { CalculationMatrixCanvasCard } from "./CalculationMatrixCard";
import {
  CANVAS_OUTLINE_ZOOM,
} from "./lib/canvasZoom";
import { dataSourceKind, type FrameOutline } from "./lib/dataSources";
import { defaultPlotSpec, viewHolding } from "./lib/canvasCards";
import type { GridDirection } from "./lib/gridNavigation";
import type { OperationHandler } from "./lib/handlers";
import type {
  CanvasView,
  Column,
  ComputedBlock,
  ComputedCalculationMatrix,
  ComputedFrame,
  ComputedResult,
  ComputedText,
  ComputedValue,
  DataObject,
  DocumentView,
  FormulaFunction,
  FrameObject,
  Selection,
  TabObject,
} from "./lib/types";
import {
  chainFilterCount,
  type GridFocus,
  type RenderedGrid,
} from "./FrameGrid";

/** Where a card is right now, which is not always where the document says. */
type ViewGeometry = Pick<CanvasView, "x" | "y" | "width" | "height" | "collapsed">;

/**
 * Card geometry as the pointer is moving it, published straight from the drag
 * and resize gestures.
 *
 * The document only learns a card's new geometry when the gesture ends and the
 * operation round-trips, so anything that has to follow a card *while* it moves
 * — the lineage cords — reads here instead, and falls back to the document for
 * every card nobody is touching.
 */
const liveViewGeometry = (() => {
  const geometry = new Map<string, ViewGeometry>();
  const listeners = new Set<() => void>();
  let version = 0;
  let frame: number | null = null;
  // Pointer moves arrive far more often than the screen repaints, so
  // subscribers are woken once a frame rather than once an event.
  const notify = () => {
    if (frame !== null) return;
    frame = requestAnimationFrame(() => {
      frame = null;
      version += 1;
      for (const listener of listeners) listener();
    });
  };
  return {
    publish(viewId: string, next: ViewGeometry) {
      geometry.set(viewId, next);
      notify();
    },
    forget(viewId: string) {
      if (geometry.delete(viewId)) notify();
    },
    read(view: CanvasView): ViewGeometry {
      return geometry.get(view.id) ?? view;
    },
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    version: () => version,
  };
})();

/**
 * How heavily a card's funnel should be drawn, and what it means.
 *
 * There are two kinds of filter here and they are not the same news. A view
 * filter narrows what this card shows and stops there — the tab's own lens,
 * which nothing computed from the frame ever sees. A wrangle filter is part
 * of what the frame *is*, so every frame derived from it inherits the
 * narrowing whether or not anyone remembers setting it.
 *
 * So the weight follows the consequence: silent when nothing is filtered,
 * the display orange when the narrowing is local, and full ink when it
 * travels. Two greys would be the honest ordering and the wrong encoding —
 * at eleven pixels they are the same mark twice.
 */
export function LineageCords({
  document,
  selection,
  width,
  height,
}: {
  document: DocumentView;
  selection: Selection | null;
  width: number;
  height: number;
}) {
  // Redraw whenever a card reports a new position, so a cord stays attached
  // to the card being dragged rather than snapping to it on release.
  useSyncExternalStore(liveViewGeometry.subscribe, liveViewGeometry.version);
  const edges = document.objects.flatMap((object) => {
    const sourceFrameIds = lineageSourceIds(object);
    return sourceFrameIds.flatMap((sourceFrameId, sourceIndex) => {
      const source = viewHolding(document, sourceFrameId);
      const targetView = viewHolding(document, object.id);
      // Both on one card is the one case with nothing to draw: a cord from a
      // card to itself says nothing, and the tab strip already says they
      // belong together.
      if (!source || !targetView || source.id === targetView.id) return [];
      const from = liveViewGeometry.read(source);
      const target = liveViewGeometry.read(targetView);
      const sourceY = from.y + (from.collapsed ? 14 : from.height / 2);
      const targetY = target.y + (target.collapsed ? 14 : target.height / 2);
      const startX = from.x + from.width;
      const endX = target.x;
      const belowSource = target.y >= from.y + (from.collapsed ? 29 : from.height);
      const path = belowSource
        ? (() => {
            const verticalStartX = from.x + from.width / 2;
            const verticalEndX = target.x + target.width / 2;
            const verticalStartY = from.y + (from.collapsed ? 29 : from.height);
            const verticalEndY = target.y;
            const bendY = Math.max(45, (verticalEndY - verticalStartY) * 0.5);
            return `M ${verticalStartX} ${verticalStartY} C ${verticalStartX} ${
              verticalStartY + bendY
            }, ${verticalEndX} ${
              verticalEndY - bendY
            }, ${verticalEndX} ${verticalEndY}`;
          })()
        : (() => {
            const bend = Math.max(55, Math.abs(endX - startX) * 0.45);
            return `M ${startX} ${sourceY} C ${startX + bend} ${sourceY}, ${
              endX - bend
            } ${targetY}, ${endX} ${targetY}`;
          })();
      return [
        {
          id: `${object.id}:${sourceIndex}`,
          active:
            selection?.objectId === object.id || selection?.objectId === sourceFrameId,
          path,
        },
      ];
    });
  });
  return (
    <svg
      className="lineage-layer"
      width={width}
      height={height}
      aria-label="Frame lineage"
    >
      {edges.map((edge) => (
        <path key={edge.id} className={edge.active ? "active" : ""} d={edge.path} />
      ))}
    </svg>
  );
}

// The same floor the card's own CSS keeps, so a drag cannot shrink a card
// past the size its contents are laid out for.
const MIN_CARD_WIDTH = 360;
const MIN_CARD_HEIGHT = 210;
const MIN_VARIABLE_WIDTH = 250;
const MIN_VARIABLE_HEIGHT = 46;

export function CanvasObject({
  view,
  object,
  objects,
  computed,
  tabs,
  computedFrames,
  computedResults,
  computedValues,
  computedBlocks,
  computedTexts,
  computedCalculationMatrices,
  scratchFocusToken,
  scratchworkElsewhere,
  formulaFunctions,
  sourceFrame,
  sourceComputed,
  closableTabIds,
  zoom,
  outline,
  selection,
  gridFocus,
  onSelect,
  onFitToWindow,
  onGridFocus,
  onGridStep,
  onRenderedRows,
  onOperation,
  onRearrangeColumns,
  onApplyVector,
  onPairVector,
  onJoinColumns,
  onFilterColumn,
  onTransformColumn,
  onEditCalculatedColumn,
  onTakeOwnership,
  onFreeze,
  onAddList,
  dataRefreshRevision,
}: {
  view: CanvasView;
  object: DataObject;
  /** Every object in the document — a container's card draws its members. */
  objects: DataObject[];
  computed?: ComputedFrame;
  tabs: TabObject[];
  computedFrames: Record<string, ComputedFrame>;
  computedResults: Record<string, ComputedResult>;
  /** What each value holds under the active scenario, keyed by value id. */
  computedValues: Record<string, ComputedValue>;
  computedBlocks: Record<string, ComputedBlock>;
  computedTexts: Record<string, ComputedText>;
  computedCalculationMatrices: Record<string, ComputedCalculationMatrix>;
  /** Set on the block ⌘J is pointing at, and bumped on every press. */
  scratchFocusToken?: number;
  /** The canonical editor is mounted in the drawer or its own window. */
  scratchworkElsewhere?: boolean;
  formulaFunctions: FormulaFunction[];
  sourceFrame?: FrameObject;
  sourceComputed?: ComputedFrame;
  closableTabIds: Set<string>;
  /** How far out the canvas is, so a pointer move can be read in canvas units. */
  zoom: number;
  /** Present only when the canvas is too far out for this card to be read. */
  outline?: FrameOutline;
  selection: Selection | null;
  gridFocus: GridFocus | null;
  onSelect: (selection: Selection) => void;
  onFitToWindow: (view: CanvasView) => void;
  onGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
  onGridStep: (direction: GridDirection) => void;
  onRenderedRows: (frameId: string, grid: RenderedGrid | null) => void;
  onOperation: OperationHandler;
  onRearrangeColumns: (frameId: string, columnIds: string[]) => void;
  onApplyVector: (
    frame: FrameObject,
    columnIds: string[],
    vector: string,
    expectedLength: number
  ) => void;
  onPairVector: (
    frame: FrameObject,
    name: string,
    vector: string,
    expectedLength: number
  ) => void;
  onJoinColumns: (
    primaryFrameId: string,
    primaryColumnId: string,
    lookupFrameId: string,
    lookupOutputColumnIds: string[]
  ) => void;
  onFilterColumn: (frame: FrameObject, column: Column) => void;
  onTransformColumn: (frame: FrameObject, column: Column, formula: string) => void;
  onEditCalculatedColumn: (
    frame: FrameObject,
    column: Column,
    rowIndex: number
  ) => void;
  /** Writes a value's answer down, or refreshes the one written. */
  onTakeOwnership?: TakeOwnershipHandler;
  onFreeze: (objectId: string) => Promise<void>;
  /** Opens the stored-vector dialog for a container. */
  onAddList: (containerId: string) => void;
  dataRefreshRevision: number;
}) {
  const standaloneVariable = object.kind === "result" && object.variable;
  const kindLabel = object.kind === "calculationMatrix" ? "calculation matrix" : object.kind;
  const { position, setPosition, size, setSize } = useCardGeometry(view);

  // Telling the cords where this card is. They are drawn outside the card, so
  // they cannot read its local state — and the document does not hear about a
  // gesture until it ends, which is a whole drag too late.
  const publishGeometry = (next: Omit<ViewGeometry, "collapsed">) =>
    liveViewGeometry.publish(view.id, { ...next, collapsed: view.collapsed });
  useEffect(() => {
    liveViewGeometry.publish(view.id, {
      x: view.x,
      y: view.y,
      width: view.width,
      height: view.height,
      collapsed: view.collapsed,
    });
  }, [view.id, view.x, view.y, view.width, view.height, view.collapsed]);
  useEffect(() => () => liveViewGeometry.forget(view.id), [view.id]);

  const beginDrag = (event: React.PointerEvent) => {
    if ((event.target as HTMLElement).closest("button,input,textarea")) return;
    event.preventDefault();
    const start = {
      x: event.clientX,
      y: event.clientY,
      left: position.x,
      top: position.y,
    };
    // The pointer moves in screen pixels and the card lives in canvas units,
    // so at 50% a hand that travels 100px has moved the card 200.
    const at = (pointer: PointerEvent) => ({
      x: Math.max(0, Math.round(start.left + (pointer.clientX - start.x) / zoom)),
      y: Math.max(0, Math.round(start.top + (pointer.clientY - start.y) / zoom)),
    });
    const move = (moveEvent: PointerEvent) => {
      const next = at(moveEvent);
      setPosition(next);
      publishGeometry({ ...next, ...size });
    };
    const end = (upEvent: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      onOperation({ type: "moveView", viewId: view.id, ...at(upEvent) });
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
  };

  /**
   * Resizing from any edge, the way a window does it: an edge moves one
   * dimension, a corner moves both. Pulling the top or left edge moves the
   * card's origin as well as its size — the opposite edge is what stays put.
   */
  const beginResize = (edge: ResizeEdge) => (event: React.PointerEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const start = {
      pointerX: event.clientX,
      pointerY: event.clientY,
      x: position.x,
      y: position.y,
      width: size.width,
      height: size.height,
    };
    const right = start.x + start.width;
    const bottom = start.y + start.height;
    const minWidth = standaloneVariable ? MIN_VARIABLE_WIDTH : MIN_CARD_WIDTH;
    const minHeight = standaloneVariable ? MIN_VARIABLE_HEIGHT : object.kind === "model" ? 100 : MIN_CARD_HEIGHT;
    let next = { x: start.x, y: start.y, width: start.width, height: start.height };
    const move = (moveEvent: PointerEvent) => {
      // Screen pixels into canvas units, as with a drag.
      const dx = (moveEvent.clientX - start.pointerX) / zoom;
      const dy = (moveEvent.clientY - start.pointerY) / zoom;
      next = { x: start.x, y: start.y, width: start.width, height: start.height };
      if (edge.includes("e")) {
        next.width = Math.max(minWidth, Math.round(start.width + dx));
      }
      if (edge.includes("s")) {
        next.height = Math.max(minHeight, Math.round(start.height + dy));
      }
      // A west or north drag is bounded twice: by the card's own minimum, and
      // by the top-left corner of the canvas, which nothing may cross.
      if (edge.includes("w")) {
        next.x = Math.min(right - minWidth, Math.max(0, Math.round(start.x + dx)));
        next.width = right - next.x;
      }
      if (edge.includes("n")) {
        next.y = Math.min(
          bottom - minHeight,
          Math.max(0, Math.round(start.y + dy))
        );
        next.height = bottom - next.y;
      }
      setSize({ width: next.width, height: next.height });
      setPosition({ x: next.x, y: next.y });
      publishGeometry(next);
    };
    const end = async () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      // The move lands first: both operations rewrite the same view, so they
      // are applied one after the other rather than raced.
      if (next.x !== view.x || next.y !== view.y) {
        await onOperation({ type: "moveView", viewId: view.id, x: next.x, y: next.y });
      }
      if (next.width !== view.width || next.height !== view.height) {
        await onOperation({
          type: "resizeView",
          viewId: view.id,
          width: next.width,
          height: next.height,
        });
      }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  };

  const isSelected = selection?.viewId
    ? selection.viewId === view.id
    : selection?.objectId === object.id;
  const hasFrame = object.kind === "frame" || object.kind === "plot";
  const isCollapsed = view.collapsed;
  // New tabs read from a frame the card already shows; a card whose own
  // source frame sits elsewhere has none to offer.
  const canAddTabs = tabs.some((tab) => tab.id === sourceFrame?.id);
  /**
   * Too far out to read the card, so it says what it is instead.
   *
   * The card keeps its size and place — the point of zooming out is to see
   * the shape of the document, and that shape is where the cards are. Only
   * what is drawn inside changes. Not rendering the grid is the other half of
   * the bargain: a hundred cards' worth of cells nobody can read is exactly
   * the work a zoomed-out canvas should not be doing.
   */
  const showOutline = hasFrame && !isCollapsed && zoom < CANVAS_OUTLINE_ZOOM;
  return (
    <section
      // The kind rides the whole card, so the colour is legible from across
      // the canvas and at any zoom -- including the outlined view, where
      // there is no title row for a mark to sit in.
      className={`canvas-object ${isSelected ? "selected" : ""} ${object.kind}-object ${
        standaloneVariable ? "variable-object" : ""
      } ${
        isCollapsed ? "collapsed" : ""
      } ${showOutline ? "outlined" : ""} ${
        object.kind === "frame" ? `kind-${dataSourceKind(object, computed)}` : ""
      }`}
      data-object-id={object.id}
      data-view-id={view.id}
      style={{
        left: position.x,
        top: position.y,
        width: size.width,
        height: isCollapsed ? 29 : size.height,
      }}
      // A press that landed on a grid cell has already chosen a more
      // specific selection, and that selection carries this view. Widening
      // it back to the whole card here would drop the cell — and with it
      // the range anchor a shift-click had just set.
      onPointerDown={(event) => {
        // A column header selects its column the same way a cell selects
        // itself; widening either back to the card dropped the column.
        if (
          (event.target as HTMLElement).closest(
            "td[data-column-id], td[data-row-id], th[data-column-id]"
          )
        )
          return;
        onSelect({ objectId: object.id, viewId: view.id });
      }}
    >
      {/* An outlined card has no title bar: at this zoom it is a few pixels
          of grey holding buttons too small to hit, and the outline wants the
          whole card — which is also what makes the space it is measured
          against the true one. The body drags in its place. */}
      {!showOutline && !standaloneVariable && (
        <div className="object-drag-handle" onPointerDown={beginDrag}>
          <span className="object-type">{isCollapsed ? object.name : kindLabel}</span>
          <span className="object-handle-actions">
            <CanvasCardWindowControls
              name={object.name}
              view={view}
              onFit={onFitToWindow}
              onOperation={onOperation}
            />
            <span className="drag-dots">•••</span>
          </span>
        </div>
      )}
      {/* The strip belongs to the card, not to whatever is inside it, so it
          is drawn here — that is what lets a plot tab and a frame tab share
          one strip instead of each card kind growing its own.

          A card with one tab and nothing it could add draws no strip: that
          is a plot in a window of its own, and a strip repeating its title
          with a dead "+" beside it is worse than no strip. */}
      {showOutline && (
        <CardOutline
          object={object}
          outline={outline}
          zoom={zoom}
          width={size.width}
          height={size.height}
          onPointerDown={beginDrag}
        />
      )}
      {!showOutline && !isCollapsed && hasFrame && (tabs.length > 1 || canAddTabs) && (
        <FrameViewTabs
          view={view}
          tabs={tabs}
          activeObject={object}
          sourceFrame={sourceFrame}
          computedFrames={computedFrames}
          onOperation={onOperation}
          onActivate={(next) => onSelect({ objectId: next.id, viewId: view.id })}
          closableTabIds={closableTabIds}
          zoom={zoom}
          filterCount={chainFilterCount}
          defaultPlotSpec={defaultPlotSpec}
        />
      )}
      {!isCollapsed && object.kind === "value" && (
        <ValueCard
          value={object}
          computed={computedValues[object.id]}
          formula={objectFormulaToken(objects, object.id)}
          onOperation={onOperation}
        />
      )}
      {!isCollapsed && standaloneVariable && (
        <VariableCard
          result={object}
          formula={objectFormulaToken(objects, object.id)}
          computed={computedResults[object.id]}
          objects={objects}
          computedFrames={computedFrames}
          formulaFunctions={formulaFunctions}
          onOperation={onOperation}
          onMovePointerDown={beginDrag}
        />
      )}
      {!isCollapsed && object.kind === "result" && !standaloneVariable && (
        <ResultCard
          onFreeze={onFreeze}
          result={object}
          formula={objectFormulaToken(objects, object.id)}
          computed={computedResults[object.id]}
          objects={objects}
          computedFrames={computedFrames}
          formulaFunctions={formulaFunctions}
          onOperation={onOperation}
        />
      )}
      {!isCollapsed && object.kind === "block" && (
        scratchworkElsewhere ? (
          <BlockCardPreview block={object} computed={computedBlocks[object.id]} />
        ) : (
          <BlockCard
            block={object}
            computed={computedBlocks[object.id]}
            focusToken={scratchFocusToken}
            objects={objects}
            computedFrames={computedFrames}
            formulaFunctions={formulaFunctions}
            onOperation={onOperation}
            onFreeze={onFreeze}
          />
        )
      )}
      {!isCollapsed && object.kind === "series" && (
        <SeriesCard
          series={object}
          formula={
            scalarFormulaReferences(objects, formulaFunctions, computedFrames).find(
              (reference) => reference.id === object.id
            )?.token ?? `\`${object.name}\``
          }
          onOperation={onOperation}
        />
      )}
      {!isCollapsed && object.kind === "container" && (
        <ContainerCard
          onFreeze={onFreeze}
          container={object}
          objects={objects}
          computedFrames={computedFrames}
          computedResults={computedResults}
          computedValues={computedValues}
          formulaFunctions={formulaFunctions}
          onOperation={onOperation}
          onAddList={onAddList}
          onSelectMember={(memberId) => onSelect({ objectId: memberId, viewId: view.id })}
        />
      )}
      {!showOutline && !isCollapsed && object.kind === "frame" && computed && (
        <FrameCard
          view={view}
          frame={object}
          computed={computed}
          selection={selection}
          gridFocus={gridFocus}
          onSelect={(next) => onSelect({ ...next, viewId: view.id })}
          onGridFocus={onGridFocus}
          onGridStep={onGridStep}
          onRenderedRows={onRenderedRows}
          onOperation={onOperation}
          onRearrangeColumns={onRearrangeColumns}
          onApplyVector={onApplyVector}
          onPairVector={onPairVector}
          onJoinColumns={onJoinColumns}
          onFilterColumn={onFilterColumn}
          onTransformColumn={onTransformColumn}
          onEditCalculatedColumn={onEditCalculatedColumn}
          dataRefreshRevision={dataRefreshRevision}
          onTakeOwnership={onTakeOwnership}
        />
      )}
      {!isCollapsed && object.kind === "model" && <ModelCard model={object} />}
      {!isCollapsed && object.kind === "text" && (
        <TextCard
          text={object}
          computed={computedTexts[object.id]}
          references={scalarFormulaReferences(objects, formulaFunctions, computedFrames)}
          onOperation={onOperation}
        />
      )}
      {!isCollapsed && object.kind === "calculationMatrix" && (
        <CalculationMatrixCanvasCard
          matrix={object}
          computed={computedCalculationMatrices[object.id]}
          objects={objects}
          computedFrames={computedFrames}
          formulaFunctions={formulaFunctions}
          onOperation={onOperation}
        />
      )}
      {!showOutline && !isCollapsed && object.kind === "plot" && sourceFrame && sourceComputed && (
        <PlotCard
          plot={object}
          frame={sourceFrame}
          computed={sourceComputed}
          dataRefreshRevision={dataRefreshRevision}
          onOperation={onOperation}
        />
      )}
      {/* Resizing works the way a window's does: the edges are invisible
          strips just inside the border, each moving the one dimension it
          owns, and the corners move both. The south-east corner is the only
          one drawn — the grow box says the card is resizable at all, and
          having found it you can grab any other side. */}
      {!showOutline && !isCollapsed && (
        <CanvasCardResizeControls name={object.name} kind={kindLabel} variable={Boolean(standaloneVariable)} beginResize={beginResize} />
      )}
    </section>
  );
}

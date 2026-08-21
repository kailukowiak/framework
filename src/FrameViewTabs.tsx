import { BarChart3, Filter, Plus, Table2 as FrameIcon, X } from "lucide-react";
import { createPortal } from "react-dom";
import {
  useEffect,
  useRef,
  useState,
  type DragEvent,
  type PointerEvent,
} from "react";
import { canvasPoint } from "./lib/canvasZoom";
import type { OperationHandler } from "./lib/handlers";
import type {
  CanvasView,
  ComputedFrame,
  TabObject,
  FrameObject,
} from "./lib/types";

type FrameTabDragPayload = {
  sourceViewId: string;
  objectId: string;
};
const FRAME_TAB_DRAG_TYPE = "application/x-framework-frame-tab";
const activeFrameTabDrag: FrameTabDragPayload | null = null;
/** Kept with `.tab-add-menu`'s width, which the menu is held on screen by. */
const TAB_ADD_MENU_WIDTH = 264;

export function readFrameTabDrag(event: DragEvent): FrameTabDragPayload | null {
  try {
    const raw = event.dataTransfer.getData(FRAME_TAB_DRAG_TYPE);
    if (!raw) return activeFrameTabDrag;
    const payload = JSON.parse(raw) as Partial<FrameTabDragPayload>;
    return typeof payload.sourceViewId === "string" &&
      typeof payload.objectId === "string"
      ? (payload as FrameTabDragPayload)
      : null;
  } catch {
    return activeFrameTabDrag;
  }
}

export function hasFrameTabDrag(event: DragEvent): boolean {
  return (
    activeFrameTabDrag !== null ||
    event.dataTransfer.types.includes(FRAME_TAB_DRAG_TYPE)
  );
}

/**
 * A card's tab strip.
 *
 * The strip is a property of the card, not of the frame inside it, so it
 * lives here rather than in `FrameCard` — that is what lets a plot tab sit
 * next to a frame tab and draw the same strip.
 */
export function FrameViewTabs({
  view,
  tabs,
  activeObject,
  sourceFrame,
  computedFrames,
  onOperation,
  onActivate,
  closableTabIds,
  zoom,
  filterCount,
  defaultPlotSpec,
}: {
  view: CanvasView;
  tabs: TabObject[];
  activeObject: TabObject;
  /** The frame the card's edits apply to — a plot tab's own source. */
  sourceFrame?: FrameObject;
  computedFrames: Record<string, ComputedFrame>;
  onOperation: OperationHandler;
  onActivate: (object: TabObject) => void;
  closableTabIds: Set<string>;
  zoom: number;
  filterCount: (computed?: ComputedFrame) => number;
  defaultPlotSpec: (frame: FrameObject) => Record<string, unknown>;
}) {
  const [dropIndex, setDropIndex] = useState<number | null>(null);
  const [addMenuOpen, setAddMenuOpen] = useState(false);
  // Where the portalled menu goes, measured from the button when it opens.
  const [menuAnchor, setMenuAnchor] = useState<{ left: number; top: number } | null>(
    null
  );
  const addButtonRef = useRef<HTMLButtonElement>(null);
  const addMenuRef = useRef<HTMLDivElement>(null);

  const toggleAddMenu = () => {
    if (addMenuOpen) {
      setAddMenuOpen(false);
      return;
    }
    const bounds = addButtonRef.current?.getBoundingClientRect();
    setMenuAnchor(
      bounds
        ? {
            // Kept on screen: the button can sit near the right edge of the
            // window, and a menu that opens off it is no menu.
            left: Math.max(
              8,
              Math.min(bounds.left, window.innerWidth - TAB_ADD_MENU_WIDTH - 8)
            ),
            top: bounds.bottom + 6,
          }
        : null
    );
    setAddMenuOpen(true);
  };

  // A menu anchored in screen pixels has to close when the thing it is
  // anchored to moves, and the canvas moves on a scroll or a zoom.
  useEffect(() => {
    if (!addMenuOpen) return;
    const closeOnOutside = (event: Event) => {
      const target = event.target as Node | null;
      if (addButtonRef.current?.contains(target)) return;
      if (addMenuRef.current?.contains(target)) return;
      setAddMenuOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setAddMenuOpen(false);
    };
    const close = () => setAddMenuOpen(false);
    window.addEventListener("pointerdown", closeOnOutside);
    window.addEventListener("keydown", closeOnEscape);
    window.addEventListener("wheel", close, { passive: true });
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("pointerdown", closeOnOutside);
      window.removeEventListener("keydown", closeOnEscape);
      window.removeEventListener("wheel", close);
      window.removeEventListener("resize", close);
    };
  }, [addMenuOpen]);
  // New tabs read from a frame the card already shows, so a card whose own
  // source is elsewhere — a plot in a window of its own — offers none. The
  // core would refuse them anyway; the point is not to offer them.
  const addableSource =
    sourceFrame && tabs.some((tab) => tab.id === sourceFrame.id)
      ? sourceFrame
      : undefined;
  const moveTab = (event: DragEvent, targetIndex: number) => {
    const payload = readFrameTabDrag(event);
    if (!payload) return;
    event.preventDefault();
    event.stopPropagation();
    setDropIndex(null);
    void onOperation({
      type: "moveTab",
      sourceViewId: payload.sourceViewId,
      targetViewId: view.id,
      objectId: payload.objectId,
      targetIndex,
    });
  };
  const suppressTabClick = useRef(false);
  const beginTabPointerDrag = (event: PointerEvent, tab: TabObject) => {
    if (event.button !== 0) return;
    const start = { x: event.clientX, y: event.clientY };
    let moved = false;
    let target: { viewId: string; index: number } | null = null;
    let marked: HTMLElement | null = null;
    const clearMark = () => {
      marked?.classList.remove("drag-target", "pointer-drop-at-end");
      marked = null;
    };
    const move = (moveEvent: globalThis.PointerEvent) => {
      if (
        !moved &&
        Math.hypot(
          moveEvent.clientX - start.x,
          moveEvent.clientY - start.y
        ) < 3
      )
        return;
      moved = true;
      moveEvent.preventDefault();
      clearMark();
      const under = document.elementFromPoint(
        moveEvent.clientX,
        moveEvent.clientY
      );
      const shell = under?.closest<HTMLElement>(".frame-view-tab-shell");
      if (shell?.dataset.tabViewId && shell.dataset.tabIndex !== undefined) {
        target = {
          viewId: shell.dataset.tabViewId,
          index: Number(shell.dataset.tabIndex),
        };
        shell.classList.add("drag-target");
        marked = shell;
        return;
      }
      const strip = under?.closest<HTMLElement>(
        ".frame-view-tabs, .frame-view-tab-add-menu"
      );
      if (strip?.dataset.tabViewId && strip.dataset.tabEndIndex !== undefined) {
        target = {
          viewId: strip.dataset.tabViewId,
          index: Number(strip.dataset.tabEndIndex),
        };
        const bar = strip.closest<HTMLElement>(".frame-view-tab-bar");
        bar?.classList.add("pointer-drop-at-end");
        marked = bar ?? strip;
        return;
      }
      target = null;
    };
    const end = (upEvent: globalThis.PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      clearMark();
      if (moved) suppressTabClick.current = true;
      if (moved && target) {
        void onOperation({
          type: "moveTab",
          sourceViewId: view.id,
          targetViewId: target.viewId,
          objectId: tab.id,
          targetIndex: target.index,
        });
      } else if (moved) {
        const under = document.elementFromPoint(upEvent.clientX, upEvent.clientY);
        const viewport = under?.closest<HTMLElement>(".canvas-viewport");
        if (viewport && !under?.closest(".canvas-object")) {
          const bounds = viewport.getBoundingClientRect();
          const dropped = canvasPoint(
            { x: upEvent.clientX, y: upEvent.clientY },
            {
              left: bounds.left,
              top: bounds.top,
              scrollLeft: viewport.scrollLeft,
              scrollTop: viewport.scrollTop,
            },
            zoom
          );
          void onOperation({
            type: "detachTab",
            viewId: view.id,
            objectId: tab.id,
            x: Math.max(0, dropped.x - 100),
            y: Math.max(0, dropped.y - 16),
          });
        }
      }
    };
    window.addEventListener("pointermove", move, { passive: false });
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  };
  const getFilterCount = (tab: TabObject) =>
    tab.kind === "frame" ? filterCount(computedFrames[tab.id]) : 0;
  return (
    // The strip scrolls when the tabs outgrow the card, so the add button
    // sits outside it: an absolutely positioned menu inside a scroll
    // container is clipped by it.
    <div
      className={`frame-view-tab-bar ${dropIndex === tabs.length ? "drop-at-end" : ""}`}
    >
      <div
        className="frame-view-tabs"
        data-tab-view-id={view.id}
        data-tab-end-index={tabs.length}
        role="tablist"
        aria-label="Card views"
        onDragOver={(event) => {
          if (!hasFrameTabDrag(event)) return;
          event.preventDefault();
          event.stopPropagation();
          event.dataTransfer.dropEffect = "move";
          setDropIndex(tabs.length);
        }}
        onDragLeave={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node | null))
            setDropIndex(null);
        }}
        onDrop={(event) => moveTab(event, tabs.length)}
      >
        {tabs.map((tab, index) => (
        <div
          key={tab.id}
          data-tab-view-id={view.id}
          data-tab-index={index}
          className={`frame-view-tab-shell ${
            tab.id === activeObject.id ? "active" : ""
          } ${dropIndex === index ? "drag-target" : ""}`}
          onDragOver={(event) => {
            if (!hasFrameTabDrag(event)) return;
            event.preventDefault();
            event.stopPropagation();
            event.dataTransfer.dropEffect = "move";
            setDropIndex(index);
          }}
          onDrop={(event) => moveTab(event, index)}
        >
          <button
            className="frame-view-tab"
            role="tab"
            aria-selected={tab.id === activeObject.id}
            title={
              getFilterCount(tab)
                ? `${getFilterCount(tab)} filter${
                    getFilterCount(tab) === 1 ? "" : "s"
                  }`
                : tab.name
            }
            onClick={() => {
              if (suppressTabClick.current) {
                suppressTabClick.current = false;
                return;
              }
              onActivate(tab);
              void onOperation({
                type: "setActiveTab",
                viewId: view.id,
                objectId: tab.id,
              });
            }}
            onPointerDown={(event) => beginTabPointerDrag(event, tab)}
          >
            {tab.kind === "plot" && <BarChart3 size={9} />}
            <span>{tab.name}</span>
            {/* A funnel, not a dot. A coloured dot on a tab is a puzzle —
                it says "something is true of this one" and leaves you to
                find out what. The count rides along when there is more than
                one, since "filtered" and "filtered three ways" are different
                things to know. */}
            {getFilterCount(tab) > 0 && (
              <span
                className="frame-view-filter-mark"
                aria-label={`${getFilterCount(tab)} filter${
                  getFilterCount(tab) === 1 ? "" : "s"
                }`}
              >
                <Filter size={9} />
                {getFilterCount(tab) > 1 && <i>{getFilterCount(tab)}</i>}
              </span>
            )}
          </button>
          {closableTabIds.has(tab.id) && (
            <button
              className="frame-view-tab-close"
              aria-label={`Close ${tab.name} tab`}
              title={`Close this tab and delete the ${tab.kind} it shows`}
              onClick={(event) => {
                event.stopPropagation();
                if (tab.id === activeObject.id) {
                  const next = tabs[index === tabs.length - 1 ? index - 1 : index + 1];
                  if (next) onActivate(next);
                }
                // A tab is an object, so closing one is deleting it. The
                // core refuses when something downstream still reads it.
                void onOperation({ type: "deleteObject", objectId: tab.id });
              }}
            >
              <X size={10} />
            </button>
          )}
        </div>
        ))}
      </div>
      {addableSource && (
        // The strip draws its "drop at the end" mark on this button, so the
        // button has to accept the drop. Without these the mark points at a
        // dead zone: dragging towards it leaves the tab list, the list's
        // dragleave clears the mark, and letting go does nothing.
        <div
          className="frame-view-tab-add-menu"
          data-tab-view-id={view.id}
          data-tab-end-index={tabs.length}
          onPointerDown={(event) => event.stopPropagation()}
          onDragOver={(event) => {
            if (!hasFrameTabDrag(event)) return;
            event.preventDefault();
            event.stopPropagation();
            event.dataTransfer.dropEffect = "move";
            setDropIndex(tabs.length);
          }}
          onDrop={(event) => moveTab(event, tabs.length)}
        >
          <button
            ref={addButtonRef}
            className="frame-view-tab-add"
            title="Add a tab showing the same data"
            aria-label="Add a tab showing the same data"
            aria-expanded={addMenuOpen}
            onClick={toggleAddMenu}
          >
            <Plus size={13} />
          </button>
          {/* Portalled out to the body, and positioned in screen pixels.
              The card clips its own overflow — it has to, for its rounded
              corners and its frame — so a menu drawn inside it is cut off at
              the card's edge, which is where this one used to lose half of
              itself. Escaping the card also escapes the canvas transform, so
              the menu stays legible at any zoom. */}
          {addMenuOpen &&
            menuAnchor &&
            createPortal(
              <div
                ref={addMenuRef}
                className="topbar-menu tab-add-menu"
                role="menu"
                style={{ position: "fixed", left: menuAnchor.left, top: menuAnchor.top }}
              >
                <button
                  onClick={() => {
                    setAddMenuOpen(false);
                    void onOperation({
                      type: "branchFrame",
                      viewId: view.id,
                      frameId: addableSource.id,
                    });
                  }}
                >
                  <FrameIcon size={15} />
                  <span>
                    <strong>Frame view</strong>
                    <small>Same data, its own transformation chain</small>
                  </span>
                </button>
                <button
                  onClick={() => {
                    setAddMenuOpen(false);
                    void onOperation({
                      type: "addPlot",
                      name: `${addableSource.name} plot`,
                      sourceFrameId: addableSource.id,
                      spec: defaultPlotSpec(addableSource),
                      x: 0,
                      y: 0,
                      viewId: view.id,
                    });
                  }}
                >
                  <BarChart3 size={15} />
                  <span>
                    <strong>Plot</strong>
                    <small>Chart {addableSource.name} in this card</small>
                  </span>
                </button>
              </div>,
              window.document.body
            )}
        </div>
      )}
    </div>
  );
}

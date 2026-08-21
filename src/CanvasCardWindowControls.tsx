import { ChevronDown, ChevronRight, Maximize2 } from "lucide-react";
import type { CanvasView, Operation } from "./lib/types";

export function CanvasCardWindowControls({
  name,
  view,
  onFit,
  onOperation,
}: {
  name: string;
  view: CanvasView;
  onFit: (view: CanvasView) => void;
  onOperation: (operation: Operation) => void;
}) {
  return <>
    <button
      className="card-window-action"
      title="Fit card to window (⇧⌘F)"
      aria-label={`Fit ${name} to window`}
      onClick={(event) => { event.stopPropagation(); onFit(view); }}
    >
      <Maximize2 size={13} />
    </button>
    <button
      className="card-window-action"
      title={`${view.collapsed ? "Expand" : "Collapse"} card (⇧⌘M)`}
      aria-label={view.collapsed ? `Expand ${name}` : `Collapse ${name}`}
      onClick={(event) => {
        event.stopPropagation();
        onOperation({ type: "setViewCollapsed", viewId: view.id, collapsed: !view.collapsed });
      }}
    >
      {view.collapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
    </button>
  </>;
}

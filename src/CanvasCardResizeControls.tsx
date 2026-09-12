import type { PointerEventHandler } from "react";

export type ResizeEdge = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";
const EDGES: ResizeEdge[] = ["n", "s", "e", "w", "ne", "nw", "sw"];
const NAMES: Record<ResizeEdge, string> = {
  n: "top edge", s: "bottom edge", e: "right edge", w: "left edge",
  ne: "top-right corner", nw: "top-left corner", se: "bottom-right corner", sw: "bottom-left corner",
};

/** Invisible edge targets and the one visible grow box share one gesture. */
export function CanvasCardResizeControls({ name, kind, variable, beginResize }: {
  name: string; kind: string; variable: boolean; beginResize: (edge: ResizeEdge) => PointerEventHandler;
}) {
  return <>
    {EDGES.map(edge => <button key={edge} className={`card-resize-edge card-resize-${edge}`}
      aria-label={`Resize ${name} by its ${NAMES[edge]}`} tabIndex={-1} onPointerDown={beginResize(edge)} />)}
    <button className={variable ? "variable-resize-handle" : "frame-resize-handle"}
      aria-label={`Resize ${name}`} title={variable ? "Drag to resize variable" : `Drag to resize ${kind}`}
      onPointerDown={beginResize("se")} />
  </>;
}

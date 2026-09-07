import { useEffect, useState } from "react";
import type { CanvasView } from "../lib/types";

/** Local drag geometry follows persisted view changes, including undo. */
export function useCardGeometry(view: CanvasView) {
  const [position, setPosition] = useState({ x: view.x, y: view.y });
  const [size, setSize] = useState({ width: view.width, height: view.height });
  useEffect(() => setPosition({ x: view.x, y: view.y }), [view.x, view.y]);
  useEffect(
    () => setSize({ width: view.width, height: view.height }),
    [view.width, view.height]
  );

  return { position, setPosition, size, setSize };
}

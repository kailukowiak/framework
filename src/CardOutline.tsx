import { Filter } from "lucide-react";
import type { CSSProperties } from "react";
import { outlineDetail } from "./lib/canvasZoom";
import { natureWords, type FrameOutline } from "./lib/dataSources";
import type { DataObject } from "./lib/types";

export function CardOutline({
  object,
  outline,
  zoom,
  width,
  height,
  onPointerDown,
}: {
  object: DataObject;
  outline?: FrameOutline;
  zoom: number;
  width: number;
  height: number;
  onPointerDown: (event: React.PointerEvent) => void;
}) {
  const detail = outlineDetail(width * zoom, height * zoom);
  // Type set in canvas units that cancel the zoom, so it lands on the same
  // number of screen pixels however far out the canvas is. The card shrinks
  // underneath it, the writing does not, and it stays the size it is meant
  // to be read at — rather than being stretched to whatever shape the card
  // happens to be, which made every card's name a different size.
  const onScreen = (pixels: number) => `${pixels / zoom}px`;
  const nameSize = onScreen(19);
  return (
    // The whole body is the drag handle at this zoom. The title bar is a few
    // pixels tall on screen out here, and asking anyone to hit it would be
    // asking them to zoom in first — which is the thing they just left.
    <div
      className="card-outline"
      data-detail={detail}
      onPointerDown={onPointerDown}
      style={
        {
          "--outline-kind": onScreen(11),
          "--outline-name": nameSize,
          "--outline-counts": onScreen(13),
          "--outline-source": onScreen(12),
        } as CSSProperties
      }
    >
      <strong>{object.name}</strong>
      {/* Everything else travels together in one box beside the name, so a
          card reads as a name and then its particulars, rather than four
          lines of equal weight. The box empties from the bottom as the card
          runs out of room; the name never goes. */}
      {detail !== "name" && (
        <div className="card-outline-info">
          <span className="card-outline-kind">
            {outline ? natureWords(outline.nature) : object.kind}
          </span>
          {outline && (
            <p>
              {outline.rows}
              <i>·</i>
              {outline.columns}
              {outline.filters > 0 && (
                <span className="card-outline-filtered">
                  <Filter size={11} />
                  {outline.filters > 1 && outline.filters}
                </span>
              )}
            </p>
          )}
          {detail === "full" && outline && <small>{outline.source}</small>}
        </div>
      )}
    </div>
  );
}


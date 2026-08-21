import {
  useCallback,
  useLayoutEffect,
  useState,
  type CSSProperties,
  type ReactNode,
  type RefObject,
} from "react";
import { createPortal } from "react-dom";

type AnchorBox = Pick<DOMRect, "left" | "right" | "top" | "bottom" | "width">;

export type FloatingMenuPlacement = {
  left: number;
  top?: number;
  bottom?: number;
  width: number;
  maxHeight: number;
};

/**
 * Place completion beside its editor in viewport coordinates.
 *
 * Canvas cards scroll, resize, overlap, and live inside a zoomed surface. A
 * menu inside one of them inherits all of those clipping boundaries. The
 * editor's screen rectangle is the only stable anchor: prefer below it, flip
 * above when that is the larger useful space, and keep the menu on screen.
 */
export function floatingMenuPlacement(
  anchor: AnchorBox,
  viewportWidth: number,
  viewportHeight: number
): FloatingMenuPlacement {
  const edge = 8;
  const gap = 4;
  const availableWidth = Math.max(0, viewportWidth - edge * 2);
  const width = Math.min(Math.max(anchor.width, 360), 680, availableWidth);
  const left = Math.max(edge, Math.min(anchor.left, viewportWidth - edge - width));
  const below = Math.max(0, viewportHeight - anchor.bottom - gap - edge);
  const above = Math.max(0, anchor.top - gap - edge);
  const placeAbove = below < 180 && above > below;
  const maxHeight = Math.max(40, placeAbove ? above : below);
  return placeAbove
    ? { left, bottom: viewportHeight - anchor.top + gap, width, maxHeight }
    : { left, top: anchor.bottom + gap, width, maxHeight };
}

export function FloatingMenu({
  anchorRef,
  className,
  children,
}: {
  anchorRef: RefObject<HTMLElement | null>;
  className: string;
  children: ReactNode;
}) {
  const [style, setStyle] = useState<CSSProperties | null>(null);
  const place = useCallback(() => {
    const anchor = anchorRef.current;
    if (!anchor) return setStyle(null);
    setStyle(
      floatingMenuPlacement(
        anchor.getBoundingClientRect(),
        window.innerWidth,
        window.innerHeight
      )
    );
  }, [anchorRef]);

  useLayoutEffect(() => {
    place();
    window.addEventListener("resize", place);
    // Capture scrolls from the canvas viewport and card interiors as well as
    // the document. Their events do not bubble to window.
    window.addEventListener("scroll", place, true);
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(place);
    if (anchorRef.current) observer?.observe(anchorRef.current);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
      observer?.disconnect();
    };
  }, [anchorRef, place]);

  if (!style) return null;
  return createPortal(
    <div className={`${className} floating-formula-menu`} style={style}>
      {children}
    </div>,
    document.body
  );
}

import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { ChevronRight, type LucideIcon } from "lucide-react";

export type ContextMenuPosition = { left: number; top: number };

/**
 * Keep a context menu inside the viewport after its real height is known.
 *
 * The menu can contain a frame's entire operation vocabulary, so guessing a
 * nominal height from the pointer position is precisely what makes its lower
 * actions unreachable. A short menu stays beside the click. When a menu will
 * not fit below it, centring the measured menu around the click preserves the
 * relationship to the thing that opened it; the viewport edges remain the
 * final authority and the menu itself scrolls.
 */
export function contextMenuPosition(
  point: { x: number; y: number },
  menu: { width: number; height: number },
  viewport: { width: number; height: number }
): ContextMenuPosition {
  const edge = 8;
  const usableHeight = Math.max(0, viewport.height - edge * 2);
  const height = Math.min(menu.height, usableHeight);
  const maxLeft = Math.max(edge, viewport.width - edge - menu.width);
  const maxTop = Math.max(edge, viewport.height - edge - height);
  const left = Math.max(edge, Math.min(point.x, maxLeft));
  const preferredTop =
    point.y + height <= viewport.height - edge
      ? point.y
      : point.y - height / 2;
  return { left, top: Math.max(edge, Math.min(preferredTop, maxTop)) };
}

export function ContextMenuSurface({
  x,
  y,
  children,
}: {
  x: number;
  y: number;
  children: ReactNode;
}) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<ContextMenuPosition | null>(null);
  const place = useCallback(() => {
    const menu = menuRef.current;
    if (!menu) return;
    const bounds = menu.getBoundingClientRect();
    setPosition(
      contextMenuPosition(
        { x, y },
        { width: bounds.width, height: bounds.height },
        { width: window.innerWidth, height: window.innerHeight }
      )
    );
  }, [x, y]);

  useLayoutEffect(() => {
    place();
    window.addEventListener("resize", place);
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(place);
    if (menuRef.current) observer?.observe(menuRef.current);
    return () => {
      window.removeEventListener("resize", place);
      observer?.disconnect();
    };
  }, [place]);

  const style: CSSProperties = {
    left: position?.left ?? x,
    top: position?.top ?? y,
    visibility: position ? "visible" : "hidden",
  };
  return (
    <div
      ref={menuRef}
      className="framework-context-menu"
      style={style}
      onPointerDown={(event) => event.stopPropagation()}
    >
      {children}
    </div>
  );
}

/** Collapses broad object operations when the menu was opened on a cell. */
export function ContextMenuGroup({
  collapsed,
  label,
  Icon,
  children,
}: {
  collapsed: boolean;
  label: string;
  Icon: LucideIcon;
  children: ReactNode;
}) {
  if (!collapsed) return <>{children}</>;
  return (
    <details className="context-menu-submenu">
      <summary>
        <Icon size={14} />
        <span>{label}</span>
        <ChevronRight className="submenu-chevron" size={14} />
      </summary>
      <div>{children}</div>
    </details>
  );
}

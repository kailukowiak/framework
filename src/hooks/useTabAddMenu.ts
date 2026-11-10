import { useEffect, useRef, useState } from "react";

/** Kept with `.tab-add-menu`'s width, which the menu is held on screen by. */
const TAB_ADD_MENU_WIDTH = 264;

/**
 * The tab strip's "+" menu: whether it is open, where it is anchored, and
 * what closes it. The menu is portalled and placed in screen pixels, so it
 * has to close when the thing it is anchored to moves, and the canvas moves
 * on a scroll or a zoom.
 */
export function useTabAddMenu() {
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

  return {
    addMenuOpen,
    setAddMenuOpen,
    menuAnchor,
    addButtonRef,
    addMenuRef,
    toggleAddMenu,
  };
}

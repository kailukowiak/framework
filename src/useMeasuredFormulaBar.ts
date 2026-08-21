import { useLayoutEffect, useRef } from "react";

/**
 * Keep the canvas edge attached to the formula bar's real lower edge. The
 * editor grows with deliberately broken-up formulas until its scroll limit;
 * a fixed guessed offset lets later lines paint over the canvas and leaves
 * the context row sitting on top of the expression.
 */
export function useMeasuredFormulaBar() {
  const bar = useRef<HTMLFormElement>(null);
  useLayoutEffect(() => {
    const node = bar.current;
    if (!node || typeof ResizeObserver === "undefined") return;
    const update = () =>
      document.documentElement.style.setProperty(
        "--formula-bar-h",
        `${node.getBoundingClientRect().height}px`
      );
    update();
    const observer = new ResizeObserver(update);
    observer.observe(node);
    return () => {
      observer.disconnect();
      document.documentElement.style.removeProperty("--formula-bar-h");
    };
  }, []);
  return bar;
}

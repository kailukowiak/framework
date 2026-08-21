import { describe, expect, it } from "vitest";
import { canvasNavigationTarget } from "./canvasNavigation";
import type { CanvasView } from "./types";

const view = (id: string, x: number, y: number): CanvasView => ({
  id,
  objectId: id,
  x,
  y,
  width: 200,
  height: 120,
  collapsed: false,
});

describe("canvasNavigationTarget", () => {
  const views = [
    view("source-a", 0, 0),
    view("source-b", 0, 240),
    view("derived-a", 360, 0),
    view("derived-b", 360, 240),
  ];

  it("moves spatially while preferring the aligned card", () => {
    expect(canvasNavigationTarget(views, "source-a", "right")?.id).toBe("derived-a");
    expect(canvasNavigationTarget(views, "derived-b", "left")?.id).toBe("source-b");
    expect(canvasNavigationTarget(views, "source-a", "down")?.id).toBe("source-b");
    expect(canvasNavigationTarget(views, "derived-b", "up")?.id).toBe("derived-a");
  });

  it("cycles in left-to-right dependency reading order and wraps", () => {
    expect(canvasNavigationTarget(views, "source-a", "next")?.id).toBe("source-b");
    expect(canvasNavigationTarget(views, "source-b", "next")?.id).toBe("derived-a");
    expect(canvasNavigationTarget(views, "source-a", "previous")?.id).toBe("derived-b");
  });

  it("returns no target when the requested half-plane is empty", () => {
    expect(canvasNavigationTarget(views, "source-a", "left")).toBeNull();
    expect(canvasNavigationTarget([], "missing", "next")).toBeNull();
  });

  it("does not mistake a wider card in the same column for one to the right", () => {
    const narrow = view("narrow", 0, 0);
    const wider = { ...view("wider", 0, 240), width: 500 };
    expect(canvasNavigationTarget([narrow, wider], "narrow", "right")).toBeNull();
    expect(canvasNavigationTarget([narrow, wider], "narrow", "down")?.id).toBe("wider");
  });
});

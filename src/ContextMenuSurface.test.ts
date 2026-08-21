import { describe, expect, it } from "vitest";
import { contextMenuPosition } from "./ContextMenuSurface";

describe("contextMenuPosition", () => {
  it("keeps a short menu beside the click", () => {
    expect(
      contextMenuPosition(
        { x: 200, y: 150 },
        { width: 224, height: 240 },
        { width: 1200, height: 800 }
      )
    ).toEqual({ left: 200, top: 150 });
  });

  it("centres a menu around a low click when it cannot fit below", () => {
    expect(
      contextMenuPosition(
        { x: 200, y: 500 },
        { width: 224, height: 400 },
        { width: 1200, height: 800 }
      )
    ).toEqual({ left: 200, top: 300 });
  });

  it("pins a viewport-height menu to the safe top edge", () => {
    expect(
      contextMenuPosition(
        { x: 1100, y: 300 },
        { width: 224, height: 1200 },
        { width: 1200, height: 800 }
      )
    ).toEqual({ left: 968, top: 8 });
  });
});

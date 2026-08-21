import { describe, expect, it } from "vitest";
import { floatingMenuPlacement } from "./FloatingMenu";

describe("floatingMenuPlacement", () => {
  it("opens below an editor when there is useful room", () => {
    expect(
      floatingMenuPlacement(
        { left: 100, right: 500, top: 100, bottom: 140, width: 400 },
        1200,
        800
      )
    ).toMatchObject({ left: 100, top: 144, width: 400 });
  });

  it("flips above a low editor instead of covering its text", () => {
    const placement = floatingMenuPlacement(
      { left: 100, right: 500, top: 650, bottom: 690, width: 400 },
      1200,
      720
    );
    expect(placement.top).toBeUndefined();
    expect(placement.bottom).toBe(74);
    expect(placement.maxHeight).toBe(638);
  });

  it("stays inside a narrow viewport", () => {
    expect(
      floatingMenuPlacement(
        { left: 260, right: 320, top: 40, bottom: 70, width: 60 },
        320,
        500
      )
    ).toMatchObject({ left: 8, width: 304 });
  });
});

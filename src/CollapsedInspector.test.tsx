// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CollapsedInspector } from "./Inspector";

afterEach(cleanup);

describe("CollapsedInspector", () => {
  it("stays visible at the inspector edge and opens the panel", () => {
    const onShow = vi.fn();
    render(<CollapsedInspector onShow={onShow} />);

    expect(
      screen.getByRole("complementary", { name: "Collapsed inspector" })
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Show inspector" }));
    expect(onShow).toHaveBeenCalledOnce();
  });
});

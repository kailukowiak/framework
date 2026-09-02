// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PinColumnsMenuItem } from "./PinColumnsMenuItem";
import type { Column, FrameObject } from "./lib/types";

afterEach(cleanup);

const columns = [{ id: "a" }, { id: "b" }, { id: "c" }] as Column[];
const frame = (pinnedColumns: number) =>
  ({ columns, display: { pinnedColumns } }) as unknown as FrameObject;

describe("PinColumnsMenuItem", () => {
  it("freezes through the column it was opened on", async () => {
    const onPin = vi.fn();
    render(<PinColumnsMenuItem frame={frame(0)} column={columns[1]} onPin={onPin} />);
    await userEvent.click(screen.getByText("Pin columns through here"));
    expect(onPin).toHaveBeenCalledWith(2);
  });

  it("offers to unfreeze from inside the frozen region", async () => {
    const onPin = vi.fn();
    render(<PinColumnsMenuItem frame={frame(2)} column={columns[0]} onPin={onPin} />);
    await userEvent.click(screen.getByText("Unpin columns"));
    expect(onPin).toHaveBeenCalledWith(0);
  });

  it("shows nothing when the menu was not opened on a column", () => {
    const { container } = render(
      <PinColumnsMenuItem frame={frame(0)} column={null} onPin={vi.fn()} />
    );
    expect(container.innerHTML).toBe("");
  });
});

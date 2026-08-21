// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SequenceFillDialog } from "./SequenceFillDialog";

afterEach(cleanup);

describe("SequenceFillDialog", () => {
  it("starts from an inferred run and emits a frame-length-bound fill", () => {
    const onApply = vi.fn();
    render(
      <SequenceFillDialog
        columnName="Month"
        orderColumns={[{ id: "month", name: "Month" }]}
        alreadyOrdered
        initialStart={10}
        initialStep={-2}
        onApply={onApply}
        onCancel={() => undefined}
      />
    );

    expect(screen.getByLabelText("Starting number")).toHaveProperty("value", "10");
    expect(screen.getByLabelText("Change each row")).toHaveProperty("value", "-2");
    fireEvent.click(screen.getByRole("button", { name: "Fill column" }));
    expect(onApply).toHaveBeenCalledWith(
      "sequence(10, 10 - 2 * frame.len(), step=-2)",
      undefined
    );
  });

  it("keeps an inferred calendar-month step in the authored formula", () => {
    const onApply = vi.fn();
    render(
      <SequenceFillDialog
        columnName="Month"
        orderColumns={[{ id: "month", name: "Month" }]}
        alreadyOrdered
        initialStart="2026-01-31"
        initialStep={1}
        kind="date"
        dateUnit="mo"
        onApply={onApply}
        onCancel={() => undefined}
      />
    );

    expect(screen.getByLabelText("Starting date")).toHaveProperty(
      "value",
      "2026-01-31"
    );
    fireEvent.click(screen.getByRole("button", { name: "Fill column" }));
    expect(onApply).toHaveBeenCalledWith(
      "sequence(2026-01-31, periods=frame.len(), step=1mo)",
      undefined
    );
  });
});

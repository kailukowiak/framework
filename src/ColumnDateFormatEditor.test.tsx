// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ColumnFormatEditor } from "./ColumnFormatEditor";
import type { Column, FrameObject } from "./lib/types";

const frame = { id: "frame-1", name: "Schedule" } as unknown as FrameObject;

function dateColumn(format?: Column["format"]): Column {
  return {
    id: "start",
    name: "Start",
    dataType: "date",
    formula: null,
    format: format ?? null,
  };
}

afterEach(() => {
  cleanup();
});

describe("ColumnFormatEditor for a date column", () => {
  it("shows the date pattern select, labelled by example, instead of number controls", () => {
    render(
      <ColumnFormatEditor
        frame={frame}
        column={dateColumn()}
        onOperation={vi.fn().mockResolvedValue(null)}
      />
    );
    const select = screen.getByRole("combobox", { name: "Date format" }) as HTMLSelectElement;
    expect(select).toBeTruthy();
    expect(screen.queryByText("Number format")).toBeNull();
    const labels = Array.from(select.options).map((option) => option.textContent);
    expect(labels).toEqual([
      "2026-09-01",
      "1 Sep 2026",
      "Sep 1, 2026",
      "Sep 2026",
      "Q3 2026",
    ]);
    // Defaults to ISO when the column carries no format yet.
    expect(select.value).toBe("iso");
  });

  it("reflects the column's stored pattern", () => {
    render(
      <ColumnFormatEditor
        frame={frame}
        column={dateColumn({ style: "plain", datePattern: "quarter" })}
        onOperation={vi.fn().mockResolvedValue(null)}
      />
    );
    const select = screen.getByRole("combobox", { name: "Date format" }) as HTMLSelectElement;
    expect(select.value).toBe("quarter");
  });

  it("dispatches setColumnFormat with the chosen pattern", async () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    render(
      <ColumnFormatEditor frame={frame} column={dateColumn()} onOperation={onOperation} />
    );
    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "Date format" }),
      "Sep 1, 2026"
    );
    expect(onOperation).toHaveBeenCalledWith({
      type: "setColumnFormat",
      frameId: "frame-1",
      columnId: "start",
      format: { style: "plain", datePattern: "monthDayYear" },
    });
  });

  it("merges the new pattern into an existing format rather than replacing it", async () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    render(
      <ColumnFormatEditor
        frame={frame}
        column={dateColumn({ style: "plain", datePattern: "monthYear" })}
        onOperation={onOperation}
      />
    );
    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "Date format" }),
      "2026-09-01"
    );
    expect(onOperation).toHaveBeenCalledWith({
      type: "setColumnFormat",
      frameId: "frame-1",
      columnId: "start",
      format: { style: "plain", datePattern: "iso" },
    });
  });
});

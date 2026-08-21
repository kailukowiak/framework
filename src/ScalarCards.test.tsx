// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ValueCard } from "./ScalarCards";
import type { ValueObject } from "./lib/types";

describe("ValueCard", () => {
  it("uses the native date picker for a date input", () => {
    render(
      <ValueCard
        value={
          {
            id: "date",
            kind: "value",
            name: "Timesheet date",
            raw: "2026-08-17",
            value: "2026-08-17",
            dataType: "date",
          } as ValueObject
        }
        onOperation={vi.fn()}
      />
    );

    expect(screen.getByDisplayValue("2026-08-17").getAttribute("type")).toBe("date");
  });
});

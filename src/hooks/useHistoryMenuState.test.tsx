// @vitest-environment jsdom

import { render, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useHistoryMenuState } from "./useHistoryMenuState";

const { setHistory } = vi.hoisted(() => ({
  setHistory: vi.fn(() => Promise.resolve()),
}));

vi.mock("../lib/api", () => ({ setHistoryMenuState: setHistory }));
vi.mock("../lib/applicationShortcuts", () => ({ hasNativeMenu: () => true }));

function HistorySurface() {
  useHistoryMenuState(false, false);
  return <textarea aria-label="Formula" />;
}

describe("native history menu state", () => {
  it("enables local undo while a text draft owns focus", async () => {
    const surface = render(<HistorySurface />);
    await waitFor(() => expect(setHistory).toHaveBeenCalledWith(false, false));

    surface.getByRole("textbox", { name: "Formula" }).focus();

    await waitFor(() => expect(setHistory).toHaveBeenCalledWith(true, true));
  });
});

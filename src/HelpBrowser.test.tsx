// @vitest-environment jsdom
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HelpBrowser } from "./HelpBrowser";
import type { HelpRequest } from "./lib/helpPanel";
import { fixtures } from "./test/support";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function help(
  request: Partial<HelpRequest> = {},
  props: Partial<ComponentProps<typeof HelpBrowser>> = {}
) {
  return (
    <HelpBrowser
      request={{ query: "", serial: 1, ...request }}
      formulaFunctions={fixtures.blank.formulaFunctions}
      canInsert={false}
      onInsert={vi.fn()}
      onClose={vi.fn()}
      {...props}
    />
  );
}

describe("HelpBrowser", () => {
  it("opens on a query handed over by Quick Commands", () => {
    render(help({ query: "pivot" }));

    expect((screen.getByLabelText("Search help") as HTMLInputElement).value).toBe("pivot");
    expect(screen.getByRole("heading", { name: "Pivot" })).toBeTruthy();
  });

  it("searches functions and how-to answers together, with no section to pick first", async () => {
    render(help());

    expect(screen.queryByRole("button", { name: "Formulas" })).toBeNull();
    const search = screen.getByRole("textbox", { name: "Search help" });
    await userEvent.type(search, "stack frame");
    expect(screen.getByRole("heading", { name: "Stack frame" })).toBeTruthy();

    await userEvent.clear(search);
    await userEvent.type(search, "sequence(start");
    expect(screen.getByRole("option", { name: /sequence\(start/ })).toBeTruthy();
  });

  it("opens the exact entry a formula cursor is on, and retargets when asked again", () => {
    const { rerender } = render(help({ query: "sequence", entryId: "root.sequence" }));
    expect(screen.getByRole("heading", { name: /sequence\(start/ })).toBeTruthy();

    rerender(help({ query: "filter", entryId: "expr.filter", serial: 2 }));
    expect((screen.getByLabelText("Search help") as HTMLInputElement).value).toBe("filter");
    expect(screen.getByRole("heading", { name: /\.filter\(/ })).toBeTruthy();
  });

  it("leaves focus where it was for a request made by pointing mid-formula", () => {
    const formula = document.createElement("textarea");
    document.body.append(formula);
    formula.focus();
    render(help({ query: "sequence", keepFocus: true }));

    expect(document.activeElement).toBe(formula);
    formula.remove();
  });

  it("follows a Related link without discarding the search being read", async () => {
    render(help({ query: "unpivot" }));

    await userEvent.click(screen.getByRole("button", { name: /^Choose several columns$/ }));
    expect(screen.getByRole("heading", { name: "Choose several columns" })).toBeTruthy();
    expect((screen.getByLabelText("Search help") as HTMLInputElement).value).toBe("unpivot");
  });

  it("finds an iterator through the variable-series guide", async () => {
    render(help());

    await userEvent.type(screen.getByRole("textbox", { name: "Search help" }), "iterator in a variable");

    expect(screen.getByRole("option", { name: /Generate a series in a variable/ })).toBeTruthy();
    expect(screen.getByText("Months 1–12")).toBeTruthy();
  });

  it("copies the selected function's useful syntax", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    render(help());

    const search = screen.getByRole("textbox", { name: "Search help" });
    await userEvent.type(search, "sequence(start");
    await userEvent.click(screen.getByRole("option", { name: /sequence\(start/ }));
    await userEvent.click(within(screen.getByRole("article")).getByRole("button", { name: "Copy" }));

    expect(writeText).toHaveBeenCalledWith("sequence(1, 13)");
  });

  it("inserts only when a formula cursor is available, and stays open afterwards", async () => {
    const onInsert = vi.fn();
    const onClose = vi.fn();
    const { rerender } = render(help({}, { onInsert, onClose }));

    const search = screen.getByRole("textbox", { name: "Search help" });
    await userEvent.type(search, "sequence(start");
    await userEvent.click(screen.getByRole("option", { name: /sequence\(start/ }));
    expect(within(screen.getByRole("article")).queryByRole("button", { name: "Insert" })).toBeNull();

    rerender(help({}, { onInsert, onClose, canInsert: true }));
    await userEvent.click(within(screen.getByRole("article")).getByRole("button", { name: "Insert" }));

    expect(onInsert).toHaveBeenCalledWith("sequence(1, 13)");
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByLabelText("FrameWork Reference")).toBeTruthy();
  });

  it("shows exact Wrangle and formatting references", async () => {
    render(help());

    const search = screen.getByRole("textbox", { name: "Search help" });
    await userEvent.type(search, "join");

    await userEvent.click(
      // The accessible name is the row's title plus its kind; the summary is
      // only the row's tooltip in the docked list.
      screen.getByRole("option", { name: /^JoinWrangle$/ })
    );
    expect(screen.getByText("Wrangle · Reference")).toBeTruthy();
    expect(screen.getByText("Anti")).toBeTruthy();

    await userEvent.clear(search);
    await userEvent.type(search, "conditional formatting");
    expect(
      screen.getByRole("option", { name: /Conditional formatting rules/ })
    ).toBeTruthy();
  });

  it("closes on Escape from the search field and from a result", async () => {
    const onClose = vi.fn();
    render(help({}, { onClose }));

    const search = screen.getByRole("textbox", { name: "Search help" });
    await userEvent.type(search, "join{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);

    const result = screen.getAllByRole("option")[0];
    result.focus();
    await userEvent.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  // Docked help stays beside the work. Escape pressed on the canvas already
  // means "stop editing this cell" and must not also dismiss the panel.
  it("ignores an Escape pressed outside the panel", async () => {
    const onClose = vi.fn();
    render(help({}, { onClose }));
    const outside = document.createElement("input");
    document.body.append(outside);
    outside.focus();

    await userEvent.keyboard("{Escape}");
    expect(onClose).not.toHaveBeenCalled();
    outside.remove();
  });
});

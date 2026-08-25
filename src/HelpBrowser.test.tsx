// @vitest-environment jsdom
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HelpBrowser } from "./HelpBrowser";
import { fixtures } from "./test/support";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("HelpBrowser", () => {
  it("finds an iterator through the variable-series guide", async () => {
    render(
      <HelpBrowser
        scope="formulas"
        formulaFunctions={fixtures.blank.formulaFunctions}
        canInsert={false}
        onScopeChange={vi.fn()}
        onInsert={vi.fn()}
        onClose={vi.fn()}
      />
    );

    await userEvent.type(screen.getByRole("textbox", { name: "Search formulas" }), "iterator in a variable");

    expect(screen.getByRole("option", { name: /Generate a series in a variable/ })).toBeTruthy();
    expect(screen.getByText("Months 1–12")).toBeTruthy();
  });

  it("copies the selected function's useful syntax", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    render(
      <HelpBrowser
        scope="formulas"
        formulaFunctions={fixtures.blank.formulaFunctions}
        canInsert={false}
        onScopeChange={vi.fn()}
        onInsert={vi.fn()}
        onClose={vi.fn()}
      />
    );

    const search = screen.getByRole("textbox", { name: "Search formulas" });
    await userEvent.type(search, "sequence(start");
    const result = screen.getByRole("option", { name: /sequence\(start/ });
    await userEvent.click(result);
    await userEvent.click(within(screen.getByRole("article")).getByRole("button", { name: "Copy" }));

    expect(writeText).toHaveBeenCalledWith("sequence(1, 13)");
  });

  it("inserts from the detail surface only when a formula cursor is available", async () => {
    const onInsert = vi.fn();
    render(
      <HelpBrowser
        scope="formulas"
        formulaFunctions={fixtures.blank.formulaFunctions}
        canInsert
        onScopeChange={vi.fn()}
        onInsert={onInsert}
        onClose={vi.fn()}
      />
    );

    const search = screen.getByRole("textbox", { name: "Search formulas" });
    await userEvent.type(search, "sequence(start");
    await userEvent.click(screen.getByRole("option", { name: /sequence\(start/ }));
    await userEvent.click(screen.getByRole("button", { name: "Insert at formula cursor" }));

    expect(onInsert).toHaveBeenCalledWith("sequence(1, 13)");
  });

  it("shows exact Wrangle and formatting references in general help", async () => {
    render(
      <HelpBrowser
        scope="guide"
        formulaFunctions={fixtures.blank.formulaFunctions}
        canInsert={false}
        onScopeChange={vi.fn()}
        onInsert={vi.fn()}
        onClose={vi.fn()}
      />
    );

    const search = screen.getByRole("textbox", { name: "Search help" });
    await userEvent.type(search, "join");

    await userEvent.click(
      // The accessible name concatenates the option's inline spans without
      // separators ("WrangleJoinMatch two frames…"), so a phrase that spans
      // the title/summary boundary can never match. Match within the summary.
      screen.getByRole("option", { name: /Match two frames by keys/ })
    );
    expect(screen.getByText("Wrangle · Reference")).toBeTruthy();
    expect(screen.getByText("Anti")).toBeTruthy();

    await userEvent.clear(search);
    await userEvent.type(search, "conditional formatting");
    expect(
      screen.getByRole("option", { name: /Conditional formatting rules/ })
    ).toBeTruthy();
  });
});

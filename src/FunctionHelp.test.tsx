// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef, type ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { BlockCompletionMenu } from "./BlockCompletionMenu";
import { FunctionHelp } from "./FunctionHelp";
import type { FormulaReference } from "./lib/formulaReferences";
import { HelpPanelContext, type HelpPanel } from "./lib/helpPanel";

afterEach(cleanup);

const filter: FormulaReference = {
  id: "expr.filter",
  label: ".filter",
  token: ".filter(",
  kind: "function",
  detail: ".filter(predicate) → expression · Keep the values where a condition holds.",
  signature: ".filter(predicate)",
  description: "Keep the values where a condition holds.",
};

const sum: FormulaReference = {
  ...filter,
  id: "expr.sum",
  label: ".sum",
  token: ".sum(",
  signature: ".sum()",
  description: "Add the values.",
};

function panel(): HelpPanel {
  return {
    request: null,
    open: vi.fn(),
    toggle: vi.fn(),
    close: vi.fn(),
    formulaTarget: { current: undefined },
  };
}

function withHelp(help: HelpPanel, children: ReactNode) {
  return <HelpPanelContext.Provider value={help}>{children}</HelpPanelContext.Provider>;
}

describe("FunctionHelp", () => {
  it("opens the full entry beside the canvas without taking the formula's focus", async () => {
    const help = panel();
    const formula = document.createElement("textarea");
    document.body.append(formula);
    formula.focus();
    render(withHelp(help, <FunctionHelp reference={filter} />));

    const line = screen.getByRole("button", { name: /\.filter\(predicate\)/ });
    expect(fireEvent.mouseDown(line)).toBe(false);
    await userEvent.click(line);

    expect(help.open).toHaveBeenCalledWith({
      query: ".filter",
      entryId: "expr.filter",
      keepFocus: true,
    });
    formula.remove();
  });

  it("adds a worked call only where the signature does not show the shape", () => {
    const { rerender } = render(withHelp(panel(), <FunctionHelp reference={filter} />));
    expect(screen.getByText('`Amount`.filter(`Region` == "West").sum()')).toBeTruthy();

    rerender(withHelp(panel(), <FunctionHelp reference={sum} />));
    expect(screen.queryByText(/`Amount`/)).toBeNull();
    expect(screen.getByText("Add the values.")).toBeTruthy();
  });

  it("is plain text where no help panel exists to open", () => {
    render(<FunctionHelp reference={filter} />);
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("Keep the values where a condition holds.")).toBeTruthy();
  });
});

function BlockMenu({ activeIndex }: { activeIndex: number }) {
  const anchor = useRef<HTMLTextAreaElement>(null);
  return (
    <>
      <textarea ref={anchor} />
      <BlockCompletionMenu
        anchorRef={anchor}
        suggestions={[sum, filter]}
        activeIndex={activeIndex}
        onTake={vi.fn()}
      />
    </>
  );
}

describe("BlockCompletionMenu", () => {
  // Scratchwork lines used to list each function's whole description on its
  // row and explain nothing about the highlighted one.
  it("explains the highlighted function the way the other formula menus do", () => {
    const { rerender } = render(withHelp(panel(), <BlockMenu activeIndex={0} />));
    expect(screen.getByRole("button", { name: /\.sum\(\)Add the values\./ })).toBeTruthy();

    rerender(withHelp(panel(), <BlockMenu activeIndex={1} />));
    expect(
      screen.getByRole("button", { name: /Keep the values where a condition holds\./ })
    ).toBeTruthy();
  });
});

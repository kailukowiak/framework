// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ActiveFormulaEditorProvider,
  useActiveFormulaEditor,
} from "./ActiveFormulaEditor";
import {
  CalculationMatrixCard,
  canonicalMatrixSource,
  type CalculationMatrixCardObject,
} from "./CalculationMatrixCard";
import type { FormulaReference } from "./lib/formulaReferences";
import { writeVectorDrag } from "./lib/vectorDrag";

afterEach(cleanup);

const references: FormulaReference[] = [
  { id: "scenario", label: "MyVar", token: "`MyVar`", kind: "value", detail: "Variable" },
  { id: "table", label: "MyTable", token: "`MyTable`.", kind: "frame", detail: "Table" },
  { id: "period", frameId: "table", label: "MyTable.Column1", token: "`MyTable`.`Column1`", kind: "column", detail: "Column" },
];

const matrix = (validBody = true): CalculationMatrixCardObject => ({
  kind: "calculationMatrix",
  id: "matrix",
  name: "Plan",
  rows: [{ id: "row-axis", name: "Scenario", source: "`MyVar`", formula: { expression: {} }, dataType: "string" }],
  columns: [{ id: "column-axis", name: "Period", source: "`MyTable`.`Column1`", formula: { expression: {} }, dataType: "date" }],
  body: { source: "Scenario * Period", formula: validBody ? { expression: {} } : null, error: validBody ? undefined : "Unknown name" },
});

const cell = (display: string) => ({
  value: Number(display),
  typedValue: { type: "number" as const, value: Number(display) },
  display,
  error: null,
  isOverride: false,
});

const computed = {
  rowTuples: [{ values: ["Base"] }, { values: ["Upside"] }],
  columnTuples: [{ values: ["Jan"] }, { values: ["Feb"] }],
  cells: [
    [cell("10"), cell("12")],
    [cell("14"), cell("16")],
  ],
};

function FormulaBarMirror() {
  const { active, setDraft } = useActiveFormulaEditor();
  return (
    <button type="button" onClick={() => setDraft("Revenue.growth(2)", { start: 17, end: 17 })}>
      Top formula: {active?.draft ?? "none"}
    </button>
  );
}

function transfer() {
  const values = new Map<string, string>();
  return {
    effectAllowed: "none",
    dropEffect: "none",
    get types() { return Array.from(values.keys()); },
    setData(type: string, value: string) { values.set(type, value); },
    getData(type: string) { return values.get(type) ?? ""; },
  } as unknown as DataTransfer;
}

describe("CalculationMatrixCard", () => {
  it("canonicalizes plain variable and table-column sources", () => {
    expect(canonicalMatrixSource("MyVar", references)).toBe("`MyVar`");
    expect(canonicalMatrixSource("MyTable.Column1", references)).toBe("`MyTable`.`Column1`");
    expect(canonicalMatrixSource("sequence(1, 4)", references)).toBe("sequence(1, 4)");
  });

  it("renders dense axis previews and the computed rectangle", () => {
    render(
      <ActiveFormulaEditorProvider>
        <CalculationMatrixCard matrix={matrix()} computed={computed} references={references} onRename={vi.fn()} onCommit={vi.fn(async () => null)} />
      </ActiveFormulaEditorProvider>
    );

    expect(screen.getByText("Scenario: Base, Upside")).toBeTruthy();
    expect(screen.getByText("Period: Jan, Feb")).toBeTruthy();
    expect(screen.getByRole("cell", { name: "16" })).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: "Feb" })).toBeTruthy();
  });

  it("keeps output blank until the body formula is valid", () => {
    render(
      <ActiveFormulaEditorProvider>
        <CalculationMatrixCard matrix={matrix(false)} computed={computed} references={references} onRename={vi.fn()} onCommit={vi.fn(async () => null)} />
      </ActiveFormulaEditorProvider>
    );

    expect(screen.getByLabelText("Calculation Matrix output").classList.contains("empty")).toBe(true);
    expect(screen.queryByRole("cell", { name: "16" })).toBeNull();
    expect(screen.getByText("Unknown name")).toBeTruthy();
  });

  it("shares and autoformats its body formula through the active editor", async () => {
    const onCommit = vi.fn(async () => null);
    render(
      <ActiveFormulaEditorProvider>
        <CalculationMatrixCard matrix={matrix()} computed={computed} references={references} onRename={vi.fn()} onCommit={onCommit} />
        <FormulaBarMirror />
      </ActiveFormulaEditorProvider>
    );

    const body = screen.getByRole("textbox", { name: "Formula =" });
    await userEvent.click(body);
    expect(screen.getByRole("button", { name: "Top formula: Scenario * Period" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Top formula: Scenario * Period" }));
    fireEvent.blur(body);

    expect(onCommit).toHaveBeenCalledWith({
      rows: [{ id: "row-axis", name: "Scenario", formula: "`MyVar`" }],
      columns: [{ id: "column-axis", name: "Period", formula: "`MyTable`.`Column1`" }],
      body: "Revenue\n  .growth(2)",
    });
  });

  it("appends a dragged vector to the chosen axis", () => {
    const onCommit = vi.fn(async () => null);
    render(
      <ActiveFormulaEditorProvider>
        <CalculationMatrixCard matrix={matrix()} computed={computed} references={references} onRename={vi.fn()} onCommit={onCommit} />
      </ActiveFormulaEditorProvider>
    );
    const dataTransfer = transfer();
    writeVectorDrag(dataTransfer, { objectId: "rates", name: "Rate", formula: "`Rates`", length: 3 });
    fireEvent.drop(screen.getByRole("region", { name: "Rows formulas" }), { dataTransfer });

    expect(onCommit).toHaveBeenCalledWith({
      rows: [
        { id: "row-axis", name: "Scenario", formula: "`MyVar`" },
        { name: "Rate", formula: "`Rates`" },
      ],
      columns: [{ id: "column-axis", name: "Period", formula: "`MyTable`.`Column1`" }],
      body: "Scenario * Period",
    });
  });
});

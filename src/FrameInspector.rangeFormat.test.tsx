// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { FrameInspector } from "./FrameInspector";
import { emptyStyle } from "./lib/conditionalFormatting";
import { fixtures, objectNamed } from "./test/support";
import type { FrameObject, FrameStyle } from "./lib/types";

afterEach(cleanup);

beforeAll(() => {
  // jsdom has no matchMedia; the colour fields ask which scheme is active.
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  })) as unknown as typeof window.matchMedia;
});

// The interaction claim: with a rectangle selected, the Bold button emits ONE
// range-targeted setFrameStyle carrying only { bold }, and reads its pressed
// state from the active cell's effective style, ranges included. What the
// engine does with it is a Rust test.
function mount(options: {
  styles?: FrameStyle[];
  styleSelection?: Parameters<typeof FrameInspector>[0]["styleSelection"];
  selection?: { rowId?: string; columnId?: string };
}) {
  const base = objectNamed(fixtures.salesWithMargin, "frame", "Monthly sales");
  const frame = { ...base, display: { ...base.display, styles: options.styles ?? [] } } as FrameObject;
  const onOperation = vi.fn(async () => null);
  render(
    <FrameInspector
      documentId="doc"
      frame={frame}
      objects={[frame]}
      formulaFunctions={[]}
      selection={(options.selection ?? { rowId: frame.rows[0].id, columnId: frame.columns[0].id }) as never}
      styleSelection={options.styleSelection}
      computed={fixtures.salesWithMargin.computedFrames[frame.id]}
      suggestedPosition={{ x: 0, y: 0 }}
      section="format"
      onAddCalculatedColumnRequestHandled={() => {}}
      onTransformColumnRequestHandled={() => {}}
      onFilterColumnRequestHandled={() => {}}
      onHidePipelineColumnRequestHandled={() => {}}
      onRearrangeColumnsRequestHandled={() => {}}
      onApplyVectorRequestHandled={() => {}}
      onPairVectorRequestHandled={() => {}}
      onOperation={onOperation}
      onSourceChanged={vi.fn(async () => null)}
      onSetCached={vi.fn(async () => null)}
      onTakeOwnership={vi.fn(async () => null)}
      onFreezeCopy={vi.fn(async () => null)}
      onJoin={() => {}}
      onTransformColumn={() => {}}
    />
  );
  return { frame, onOperation };
}

describe("FrameInspector range formatting", () => {
  it("bolds a dragged rectangle with one range op carrying only bold", async () => {
    const base = objectNamed(fixtures.salesWithMargin, "frame", "Monthly sales");
    const rowIds = base.rows.slice(0, 2).map((row) => row.id);
    const columnIds = base.columns.slice(0, 2).map((column) => column.id);
    const { frame, onOperation } = mount({
      styleSelection: { kind: "range", rowIds, columnIds },
    });
    await userEvent.click(screen.getByRole("button", { name: "Bold" }));
    expect(onOperation).toHaveBeenCalledTimes(1);
    expect(onOperation).toHaveBeenCalledWith({
      type: "setFrameStyle",
      frameId: frame.id,
      target: { kind: "range", rowIds, columnIds },
      style: { ...emptyStyle(), bold: true },
    });
  });

  it("toggles off when the active cell is already bold through a range", async () => {
    const base = objectNamed(fixtures.salesWithMargin, "frame", "Monthly sales");
    const rowIds = base.rows.slice(0, 2).map((row) => row.id);
    const columnIds = base.columns.slice(0, 2).map((column) => column.id);
    const { onOperation } = mount({
      styles: [
        { target: { kind: "range", rowIds, columnIds }, style: { ...emptyStyle(), bold: true } },
      ],
      styleSelection: { kind: "range", rowIds, columnIds },
    });
    const bold = screen.getByRole("button", { name: "Bold" });
    expect(bold.getAttribute("aria-pressed")).toBe("true");
    await userEvent.click(bold);
    expect(onOperation).toHaveBeenCalledWith(
      expect.objectContaining({ style: { ...emptyStyle(), bold: false } })
    );
  });

  it("keeps the cell target for a single cell", async () => {
    const { frame, onOperation } = mount({ styleSelection: null });
    await userEvent.click(screen.getByRole("button", { name: "Bold" }));
    expect(onOperation).toHaveBeenCalledWith(
      expect.objectContaining({
        target: { kind: "cell", rowId: frame.rows[0].id, columnId: frame.columns[0].id },
      })
    );
  });

  it("keeps the column target for a whole column", async () => {
    const { frame, onOperation } = mount({
      styleSelection: null,
      selection: { columnId: objectNamed(fixtures.salesWithMargin, "frame", "Monthly sales").columns[1].id },
    });
    await userEvent.click(screen.getByRole("button", { name: "Bold" }));
    expect(onOperation).toHaveBeenCalledTimes(1);
    expect(onOperation).toHaveBeenCalledWith(
      expect.objectContaining({ target: { kind: "column", columnId: frame.columns[1].id } })
    );
  });
});

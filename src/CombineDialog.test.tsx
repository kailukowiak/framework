// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CombineDialog } from "./CombineDialog";
import type {
  Column,
  ComputedFrame,
  DocumentView,
  FrameObject,
  Operation,
} from "./lib/types";
import { clearMocks, serveInvoke } from "./test/support";

// Four relationships, one dialog: what each of them emits, and what each of
// them refuses. The engine's own meaning for those operations is proved in
// Rust; what is proved here is that the form turns a person's answer into
// the right operation, and says why when it cannot.

const column = (id: string, name: string, dataType: Column["dataType"] = "string"): Column => ({
  id,
  name,
  dataType,
  formula: null,
});

const frame = (
  id: string,
  name: string,
  columns: Column[],
  rowCount: number,
  uniqueKeys: FrameObject["uniqueKeys"] = []
): FrameObject =>
  ({
    kind: "frame",
    id,
    name,
    columns,
    rows: Array.from({ length: rowCount }, (_, index) => ({ id: `${id}-${index}` })),
    uniqueKeys,
    summaries: [],
  }) as unknown as FrameObject;

const months = frame(
  "months",
  "Months",
  [column("month", "Month"), column("actual", "Actual", "integer")],
  12
);

const rates = frame(
  "rates",
  "Rates",
  [column("rate-month", "Month"), column("rate", "Rate", "number")],
  4
);

const view = (frames: FrameObject[], computed: Record<string, Partial<ComputedFrame>> = {}) =>
  ({
    objects: frames,
    computedFrames: computed,
  }) as unknown as DocumentView;

function open(
  frames: FrameObject[],
  onOperation: (operation: Operation) => Promise<string | null>,
  computed: Record<string, Partial<ComputedFrame>> = {}
) {
  return render(
    <CombineDialog
      state={{ primaryFrameId: frames[0].id, x: 10, y: 20 }}
      document={view(frames, computed)}
      onClose={() => undefined}
      onOperation={onOperation}
      onCreated={() => undefined}
      onChained={() => undefined}
    />
  );
}

const chooseRelationship = (value: string) =>
  fireEvent.change(screen.getByLabelText("Relationship"), { target: { value } });

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("combine relationships", () => {
  it("matches on parallel key arrays once a second key is added", async () => {
    serveInvoke({
      get_join_diagnostics: () => ({
        primaryRows: 12,
        matchedRows: 12,
        unmatchedRows: 0,
        lookupRows: 4,
        lookupNullKeyRows: 0,
        lookupDuplicateKeyValues: 0,
      }),
    });
    const entity = column("entity", "Entity");
    const rateEntity = column("rate-entity", "Entity");
    const left = frame("months", "Months", [...months.columns, entity], 12);
    const right = frame(
      "rates",
      "Rates",
      [...rates.columns, rateEntity],
      4,
      [{ id: "composite", columnIds: ["rate-month", "rate-entity"] }] as FrameObject["uniqueKeys"]
    );
    const onOperation = vi.fn().mockResolvedValue(null);
    open([left, right], onOperation);

    fireEvent.click(screen.getByRole("button", { name: "another key" }));
    fireEvent.change(screen.getByLabelText("Destination key 2"), {
      target: { value: "entity" },
    });
    fireEvent.change(screen.getByLabelText("Lookup key 2"), {
      target: { value: "rate-entity" },
    });

    const create = screen.getByRole("button", { name: /Create joined frame/ });
    await waitFor(() => expect(create.hasAttribute("disabled")).toBe(false));
    fireEvent.click(create);
    expect(onOperation.mock.calls[0][0]).toMatchObject({
      type: "addJoinFrame",
      primaryFrameId: "months",
      lookupFrameId: "rates",
      primaryKeyColumnIds: ["month", "entity"],
      lookupKeyColumnIds: ["rate-month", "rate-entity"],
      joinType: "left",
    });
  });

  it("pairs each chosen column as its own zipVector step carrying the fill", () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    open([months, rates], onOperation);
    chooseRelationship("pair");
    fireEvent.change(screen.getByLabelText("Fill"), { target: { value: "repeat" } });
    // Bring only Rate across; the other table's Month would collide by name.
    fireEvent.click(screen.getByRole("checkbox", { name: /Month/ }));
    fireEvent.click(screen.getByRole("button", { name: "Add columns" }));

    const operation = onOperation.mock.calls[0][0];
    expect(operation.type).toBe("setFramePipeline");
    expect(operation.frameId).toBe("months");
    expect(operation.steps).toHaveLength(1);
    expect(operation.steps[0]).toMatchObject({
      kind: "zipVector",
      name: "Rate",
      vector: "`Rates`.`Rate`",
      fill: "repeat",
    });
  });

  it("refuses a pair whose counts do not fit the chosen fill, and says which", () => {
    open([months, rates], vi.fn().mockResolvedValue(null));
    chooseRelationship("pair");
    expect(screen.getByText(/Exact needs equal counts: 12 rows here, 4 there/)).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Add columns" }).hasAttribute("disabled")
    ).toBe(true);

    // 12 is a whole multiple of 4, so repeating tiles cleanly.
    fireEvent.change(screen.getByLabelText("Fill"), { target: { value: "repeat" } });
    expect(
      screen.getByRole("button", { name: "Add columns" }).hasAttribute("disabled")
    ).toBe(false);
  });

  it("names the repeat refusal with both counts when the tiling is uneven", () => {
    const odd = frame("odd", "Odd", [column("value", "Value", "number")], 5);
    open([months, odd], vi.fn().mockResolvedValue(null));
    chooseRelationship("pair");
    fireEvent.change(screen.getByLabelText("Fill"), { target: { value: "repeat" } });
    expect(
      screen.getByText(/Repeat needs 12 to be a whole multiple of 5/)
    ).toBeTruthy();
  });

  it("previews the stack mapping, including a blank and a mismatch", () => {
    const other = frame(
      "prior",
      "Prior year",
      [
        column("m", "Month"),
        // Same name, different type: the union will line these up and the
        // preview has to say so before the mapping is frozen.
        column("a", "Actual", "number"),
        column("note", "Note"),
      ],
      12
    );
    const extended = frame(
      "months",
      "Months",
      [...months.columns, column("budget", "Budget", "integer")],
      12
    );
    const onOperation = vi.fn().mockResolvedValue(null);
    open([extended, other], onOperation);
    chooseRelationship("stack");

    expect(screen.getByText("— (blank)")).toBeTruthy();
    expect(screen.getByText("mismatch")).toBeTruthy();
    expect(screen.getByText(/not carried: Note/)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Stack rows" }));
    expect(onOperation.mock.calls[0][0]).toMatchObject({
      type: "setFramePipeline",
      frameId: "months",
      steps: [{ kind: "union", frameId: "prior" }],
    });
  });

  it("states the product before expanding every row with every row", () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    open([months, rates], onOperation);
    chooseRelationship("expand");
    expect(screen.getByText("12 × 4 = 48 rows")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Expand rows" }));
    expect(onOperation.mock.calls[0][0]).toMatchObject({
      type: "setFramePipeline",
      frameId: "months",
      steps: [{ kind: "expand", frameId: "rates" }],
    });
  });

  it("resends the chain already on the frame ahead of the appended step", () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    open([months, rates], onOperation, {
      months: {
        steps: [{ kind: "filter", predicates: ["`Actual` > 0"], matchAll: true }],
      },
    });
    chooseRelationship("expand");
    fireEvent.click(screen.getByRole("button", { name: "Expand rows" }));
    expect(onOperation.mock.calls[0][0].steps).toMatchObject([
      { kind: "filter", predicates: ["`Actual` > 0"], matchAll: true },
      { kind: "expand", frameId: "rates" },
    ]);
  });
});

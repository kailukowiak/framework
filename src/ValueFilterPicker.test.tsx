// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ValueFilterPicker } from "./ValueFilterPicker";
import type { StepDraft } from "./lib/pipelineSteps";
import type { DistinctValues } from "./lib/valueFilter";

afterEach(cleanup);

const column = { id: "c-region", name: "Region" };
const regions: DistinctValues = {
  total: 3,
  values: [
    { value: "East", count: 5 },
    { value: "West", count: 3 },
    { value: null, count: 1 },
  ],
};

function filterStep(...formulas: string[]): StepDraft {
  return {
    id: "s1",
    kind: "filter",
    predicates: formulas.map((formula, index) => ({ id: `p${index}`, formula })),
    matchAll: true,
  };
}

function mount(steps: StepDraft[], values: DistinctValues = regions) {
  const onChangeSteps = vi.fn();
  const onEditInWrangle = vi.fn();
  const onClose = vi.fn();
  const load = vi.fn(async () => values);
  render(
    <ValueFilterPicker
      frameId="f1"
      column={column}
      steps={steps}
      loadDistinctValues={load}
      onChangeSteps={onChangeSteps}
      onEditInWrangle={onEditInWrangle}
      onClose={onClose}
    />
  );
  return { onChangeSteps, onEditInWrangle, onClose, load };
}

const formulasOf = (steps: StepDraft[]) =>
  steps.flatMap((step) => (step.kind === "filter" ? step.predicates.map((p) => p.formula) : []));

describe("ValueFilterPicker", () => {
  it("lists the handed-in values with counts, all checked", async () => {
    const { load } = mount([]);
    expect((await screen.findByRole("checkbox", { name: /East/ }) as HTMLInputElement).checked).toBe(true);
    expect(load).toHaveBeenCalledWith("f1", "c-region");
    expect((screen.getByRole("checkbox", { name: /\(blank\)/ }) as HTMLInputElement).checked).toBe(true);
    expect(screen.getByText("5")).toBeTruthy();
  });

  it("commits unchecked values as a canonical is_in filter step", async () => {
    const user = userEvent.setup();
    const { onChangeSteps, onClose } = mount([]);
    await user.click(await screen.findByRole("checkbox", { name: /West/ }));
    await user.click(screen.getByRole("button", { name: "Apply" }));
    const next = onChangeSteps.mock.calls[0][0] as StepDraft[];
    expect(next).toHaveLength(1);
    expect(formulasOf(next)).toEqual([
      '`Region`.is_in(["East"]) | `Region`.is_null()',
    ]);
    expect(onClose).toHaveBeenCalled();
  });

  it("narrows by typing, then None and a check keep only that value", async () => {
    const user = userEvent.setup();
    const { onChangeSteps } = mount([]);
    await screen.findByRole("checkbox", { name: /East/ });
    await user.type(screen.getByRole("searchbox"), "we");
    expect(screen.queryByRole("checkbox", { name: /East/ })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Select none" }));
    await user.click(screen.getByRole("button", { name: "Select all" }));
    await user.clear(screen.getByRole("searchbox"));
    await user.keyboard("{Enter}");
    // None then All over the narrowed list leaves the others as they were.
    expect(onChangeSteps).not.toHaveBeenCalled();
  });

  it("will not apply a checklist with nothing checked", async () => {
    const user = userEvent.setup();
    const { onChangeSteps, onClose } = mount([]);
    await screen.findByRole("checkbox", { name: /East/ });
    await user.click(screen.getByRole("button", { name: "Select none" }));
    // `is_in([])` is not a formula, and "keep nothing" is never the intent.
    expect((screen.getByRole("button", { name: "Apply" }) as HTMLButtonElement).disabled).toBe(true);
    await user.keyboard("{Enter}");
    expect(onChangeSteps).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("offers Edit in Wrangle when the list cannot be loaded", async () => {
    const user = userEvent.setup();
    const onEditInWrangle = vi.fn();
    render(
      <ValueFilterPicker
        frameId="f1"
        column={column}
        steps={[]}
        loadDistinctValues={async () => {
          throw new Error("nope");
        }}
        onChangeSteps={vi.fn()}
        onEditInWrangle={onEditInWrangle}
        onClose={vi.fn()}
      />
    );
    expect((await screen.findByRole("alert")).textContent).toBe("nope");
    await user.click(screen.getByRole("button", { name: "Edit in Wrangle" }));
    expect(onEditInWrangle).toHaveBeenCalled();
  });

  it("moves with arrows, toggles with Space, commits with Return, cancels with Escape", async () => {
    const user = userEvent.setup();
    const { onChangeSteps, onClose } = mount([]);
    await screen.findByRole("checkbox", { name: /East/ });
    await user.keyboard("{ArrowDown} ");
    expect((screen.getByRole("checkbox", { name: /West/ }) as HTMLInputElement).checked).toBe(false);
    await user.keyboard("{Enter}");
    expect(formulasOf(onChangeSteps.mock.calls[0][0])).toEqual([
      '`Region`.is_in(["East"]) | `Region`.is_null()',
    ]);
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("pre-checks from a picker-written filter and rewrites that step", async () => {
    const user = userEvent.setup();
    const steps = [filterStep('`Region`.is_in(["East"])')];
    const { onChangeSteps } = mount(steps);
    expect((await screen.findByRole("checkbox", { name: /East/ }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole("checkbox", { name: /West/ }) as HTMLInputElement).checked).toBe(false);
    await user.click(screen.getByRole("checkbox", { name: /West/ }));
    await user.click(screen.getByRole("button", { name: "Apply" }));
    const next = onChangeSteps.mock.calls[0][0] as StepDraft[];
    expect(next).toHaveLength(1);
    expect(formulasOf(next)).toEqual(['`Region`.is_in(["East", "West"])']);
    expect(next[0]).toMatchObject({ id: "s1" });
  });

  it("checking everything removes the picker's predicate and an emptied step", async () => {
    const user = userEvent.setup();
    const { onChangeSteps } = mount([filterStep('`Region`.is_in(["East"])')]);
    await screen.findByRole("checkbox", { name: /East/ });
    await user.click(screen.getByRole("button", { name: "Select all" }));
    await user.click(screen.getByRole("button", { name: "Apply" }));
    expect(onChangeSteps).toHaveBeenCalledWith([]);
  });

  it("adds beside another column's predicate in the trailing filter", async () => {
    const user = userEvent.setup();
    const { onChangeSteps } = mount([filterStep("`Units` > 3")]);
    await user.click(await screen.findByRole("checkbox", { name: /West/ }));
    await user.click(screen.getByRole("button", { name: "Apply" }));
    expect(formulasOf(onChangeSteps.mock.calls[0][0])).toEqual([
      "`Units` > 3",
      '`Region`.is_in(["East"]) | `Region`.is_null()',
    ]);
  });

  it("leaves a hand-written predicate alone and offers Wrangle instead", async () => {
    const user = userEvent.setup();
    const { onChangeSteps, onEditInWrangle } = mount([
      filterStep('`Region` == "East"'),
    ]);
    expect(await screen.findByText(/already has a filter/)).toBeTruthy();
    expect(screen.queryByRole("checkbox")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Edit in Wrangle" }));
    expect(onEditInWrangle).toHaveBeenCalled();
    expect(onChangeSteps).not.toHaveBeenCalled();
  });

  it("starts unchecked when values are truncated and says how many are hidden", async () => {
    const user = userEvent.setup();
    const { onChangeSteps } = mount([], {
      total: 500,
      values: [
        { value: "East", count: 5 },
        { value: "West", count: 3 },
      ],
    });
    expect((await screen.findByRole("checkbox", { name: /East/ }) as HTMLInputElement).checked).toBe(false);
    expect(screen.getByText("2 of 500")).toBeTruthy();
    await user.keyboard("{Enter}");
    expect(onChangeSteps).not.toHaveBeenCalled();
    await waitFor(() => expect(screen.getByRole("searchbox")).toBeTruthy());
  });
});

describe("ValueFilterPicker over a truncated list", () => {
  // The first page of a long column, and the engine's answers to the two
  // queries the tests type. Replayed by query, never computed.
  const page: DistinctValues = {
    total: 5000,
    values: [
      { value: "East", count: 50 },
      { value: "West", count: 30 },
    ],
  };
  const answers: Record<string, DistinctValues> = {
    Zelda: { total: 1, values: [{ value: "Zelda", count: 1 }] },
    Yan: { total: 1, values: [{ value: "Yanni", count: 2 }] },
  };

  function mountTruncated() {
    const onChangeSteps = vi.fn();
    const load = vi.fn(async (_frameId: string, _columnId: string, query?: string) =>
      query === undefined ? page : (answers[query] ?? { total: 0, values: [] })
    );
    render(
      <ValueFilterPicker
        frameId="f1"
        column={column}
        steps={[]}
        loadDistinctValues={load}
        onChangeSteps={onChangeSteps}
        onEditInWrangle={vi.fn()}
        onClose={vi.fn()}
      />
    );
    return { onChangeSteps, load };
  }

  it("asks the engine for what is typed and lists its answer", async () => {
    const user = userEvent.setup();
    const { load } = mountTruncated();
    await screen.findByRole("checkbox", { name: /East/ });
    expect(screen.getByText("2 of 5,000")).toBeTruthy();
    await user.type(screen.getByRole("searchbox"), "Zelda");
    expect(await screen.findByRole("checkbox", { name: /Zelda/ })).toBeTruthy();
    expect(load).toHaveBeenCalledWith("f1", "c-region", "Zelda");
    expect(screen.queryByRole("checkbox", { name: /East/ })).toBeNull();
    // Every match is listed, so there is no count to give.
    expect(screen.queryByText(/ of /)).toBeNull();
  });

  it("keeps a value checked under one query and writes it after another", async () => {
    const user = userEvent.setup();
    const { onChangeSteps } = mountTruncated();
    await screen.findByRole("checkbox", { name: /East/ });
    const search = screen.getByRole("searchbox");
    await user.type(search, "Zelda");
    await user.click(await screen.findByRole("checkbox", { name: /Zelda/ }));
    await user.clear(search);
    await user.type(search, "Yan");
    await user.click(await screen.findByRole("checkbox", { name: /Yanni/ }));
    expect(screen.queryByRole("checkbox", { name: /Zelda/ })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Apply" }));
    expect(formulasOf(onChangeSteps.mock.calls[0][0])).toEqual([
      '`Region`.is_in(["Zelda", "Yanni"])',
    ]);
  });

  it("filters a complete list locally, without asking again", async () => {
    const user = userEvent.setup();
    const { load } = mount([]);
    await screen.findByRole("checkbox", { name: /East/ });
    await user.type(screen.getByRole("searchbox"), "we");
    expect(screen.queryByRole("checkbox", { name: /East/ })).toBeNull();
    expect(screen.getByRole("checkbox", { name: /West/ })).toBeTruthy();
    // Past the search delay: a call would have been made by now.
    await new Promise((resolve) => setTimeout(resolve, 250));
    expect(load).toHaveBeenCalledTimes(1);
  });
});

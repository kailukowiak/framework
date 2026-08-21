// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { FormulaReference } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import type { TextObject } from "./lib/types";
import type { ComputedText } from "./lib/bindings/ComputedText";
import { formulaHoleAt, TextCard } from "./TextCard";

afterEach(cleanup);

const targetReference: FormulaReference = {
  id: "target",
  label: "Target",
  token: "`Target`",
  kind: "value",
  detail: "Scratchwork line",
};

const salesReferences: FormulaReference[] = [
  {
    id: "sales",
    label: "Monthly sales",
    token: "`Monthly sales`.",
    kind: "frame",
    detail: "2 columns",
  },
  {
    id: "cost",
    frameId: "sales",
    label: "Monthly sales.Cost",
    token: "`Monthly sales`.`Cost`",
    kind: "column",
    detail: "Number column in Monthly sales",
  },
];

const text = (source: string): TextObject => ({
  kind: "text",
  id: "narrative",
  name: "Narrative",
  text: source,
  segments: [],
});

describe("formulaHoleAt", () => {
  it("isolates only the formula containing the caret", () => {
    expect(formulaHoleAt("Before {{one}} after {{tar}}.", 26)).toEqual({
      source: "tar",
      cursor: 3,
      contentStart: 23,
      contentEnd: 26,
    });
    expect(formulaHoleAt("Before {{one}} after", 20)).toBeNull();
    expect(formulaHoleAt("Before {{unfinished", 19)?.source).toBe("unfinished");
  });
});

describe("TextCard", () => {
  it("completes formulas inside prose and commits the whole markdown source", async () => {
    const user = userEvent.setup();
    const onOperation = vi.fn<OperationHandler>(async () => null);
    render(
      <TextCard
        text={text("Summary {{Tar")}
        computed={undefined}
        references={[targetReference]}
        onOperation={onOperation}
      />
    );

    await user.click(screen.getByTitle("Edit text"));
    const editor = screen.getByLabelText("Narrative markdown");
    const suggestion = await screen.findByRole("button", { name: /Target/ });
    expect(suggestion.closest(".reference-menu")?.parentElement).toBe(document.body);
    await user.click(suggestion);
    expect((editor as HTMLTextAreaElement).value).toBe("Summary {{`Target`");

    fireEvent.blur(editor);
    expect(onOperation).toHaveBeenCalledWith({
      type: "setTextSource",
      objectId: "narrative",
      source: "Summary {{`Target`",
    });
  });

  it("does not offer formula completion in ordinary markdown", async () => {
    const user = userEvent.setup();
    render(
      <TextCard
        text={text("Target")}
        computed={undefined}
        references={[targetReference]}
        onOperation={vi.fn(async () => null)}
      />
    );

    await user.click(screen.getByTitle("Edit text"));
    expect(screen.queryByRole("button", { name: /Target/ })).toBeNull();
  });

  it("advances from a completed frame to its columns", async () => {
    const user = userEvent.setup();
    render(
      <TextCard
        text={text("Total {{Mon")}
        computed={undefined}
        references={salesReferences}
        onOperation={vi.fn(async () => null)}
      />
    );

    await user.click(screen.getByTitle("Edit text"));
    const editor = screen.getByLabelText("Narrative markdown");
    expect(await screen.findAllByRole("button", { name: /Monthly sales/ })).toHaveLength(2);

    await user.keyboard("{Tab}");
    expect((editor as HTMLTextAreaElement).value).toBe("Total {{`Monthly sales`.");
    expect(screen.queryByText("`Monthly sales`.")).toBeNull();
    expect(screen.getByRole("button", { name: /Cost/ })).not.toBeNull();

    await user.keyboard("{Enter}");
    expect((editor as HTMLTextAreaElement).value).toBe(
      "Total {{`Monthly sales`.`Cost`"
    );
    expect(screen.queryByRole("button", { name: /Cost/ })).toBeNull();
  });

  it("shows parse and evaluation errors where their answers belong", () => {
    const computed: ComputedText = {
      source: "Parse {{nope}}; evaluate {{1 / 0}}.",
      segments: [
        { kind: "literal", text: "Parse " },
        { kind: "broken", source: "nope", error: "Formula error: Unknown name nope" },
        { kind: "literal", text: "; evaluate " },
        {
          kind: "value",
          formula: "1 / 0",
          dataType: "number",
          value: null,
          typedValue: { type: "null" },
          display: "",
          error: "Division by zero",
          isOverride: false,
        },
        { kind: "literal", text: "." },
      ],
    };
    render(
      <TextCard
        text={text(computed.source)}
        computed={computed}
        references={[]}
        onOperation={vi.fn(async () => null)}
      />
    );

    expect(screen.getByText("Unknown name nope")).not.toBeNull();
    expect(screen.getByText("Division by zero")).not.toBeNull();
    expect(screen.getAllByText("Formula error:")).toHaveLength(2);
    expect(screen.queryByText("{{nope}}")).toBeNull();
    expect(screen.queryByText("1 / 0")).toBeNull();
  });

  it("renders numeric holes with the shared default number format", () => {
    const computed: ComputedText = {
      source: "Total {{1234.5}}",
      segments: [
        { kind: "literal", text: "Total " },
        {
          kind: "value",
          formula: "1234.5",
          dataType: "number",
          value: 1234.5,
          typedValue: { type: "number", value: 1234.5 },
          display: "1234.5",
          error: null,
          isOverride: false,
        },
      ],
    };
    render(
      <TextCard
        text={text(computed.source)}
        computed={computed}
        references={[]}
        onOperation={vi.fn(async () => null)}
      />
    );

    expect(screen.getByText("1,234.50")).not.toBeNull();
  });
});

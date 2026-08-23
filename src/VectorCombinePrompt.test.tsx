// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { VectorCombinePrompt } from "./VectorCombinePrompt";

const state = {
  frameId: "frame-1",
  viewId: "view-1",
  vector: { name: "Scenario", formula: "`Scenario vectors`.`Scenario`", length: 3 },
};

describe("VectorCombinePrompt", () => {
  it("offers the two frame stacking directions", () => {
    const choose = vi.fn();
    render(
      <VectorCombinePrompt state={state} onChoose={choose} onClose={vi.fn()} />
    );

    expect(screen.getByRole("dialog", { name: "Place Scenario" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Beside · HStack/ }));
    fireEvent.click(screen.getByRole("button", { name: /Below · VStack/ }));

    expect(choose.mock.calls).toEqual([["hstack"], ["vstack"]]);
  });
});

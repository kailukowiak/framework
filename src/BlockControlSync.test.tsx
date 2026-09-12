// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { BlockCard } from "./BlockCard";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

it.each([null, "Cannot commit"])("settles a text commit without losing source (error: %s)", async (failure) => {
  let finish!: (error: string | null) => void;
  const onOperation = vi.fn(() => new Promise<string | null>((resolve) => { finish = resolve; }));
  const block = objectNamed(fixtures.parameters, "block", "Inputs");
  const content = (view: typeof fixtures.parameters) => <ActiveFormulaEditorProvider><BlockCard
    block={objectNamed(view, "block", "Inputs")} computed={view.computedBlocks[block.id]}
    objects={view.objects} computedFrames={view.computedFrames} formulaFunctions={view.formulaFunctions}
    onOperation={onOperation} onFreeze={async () => {}}
  /></ActiveFormulaEditorProvider>;
  const mounted = render(content(fixtures.parameters));
  const source = screen.getByLabelText("Inputs lines");
  const edited = fixtures.parameters.computedBlocks[block.id].source.replace("value=0.2", "value=0.20");
  fireEvent.focus(source);
  fireEvent.change(source, { target: { value: edited } });
  fireEvent.blur(source);
  expect(onOperation).toHaveBeenCalledTimes(1);
  mounted.rerender(content(fixtures.parametersSelected));
  expect(source).toHaveProperty("value", edited);
  await act(async () => finish(failure));
  expect(source).toHaveProperty("value", failure ? edited : fixtures.parametersSelected.computedBlocks[block.id].source);
});

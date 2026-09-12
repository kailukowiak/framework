// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { FormulaField } from "./FormulaField";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);
it("shows a changed stored constructor even when its formula-bar session survives blur", () => {
  const onCommit = vi.fn().mockResolvedValue(null);
  const initial = objectNamed(fixtures.parameters, "block", "Inputs").lines[0].source;
  const changed = objectNamed(fixtures.parametersSelected, "block", "Inputs").lines[0].source;
  const content = (source: string) => <ActiveFormulaEditorProvider><FormulaField editorId="growth" label="Growth formula" initial={source} references={[]} onCommit={onCommit} compact commitOnBlur /></ActiveFormulaEditorProvider>;
  const mounted = render(content(initial));
  const editor = screen.getByRole("textbox", { name: /Growth formula/ });
  fireEvent.focus(editor);
  fireEvent.blur(editor);
  mounted.rerender(content(changed));
  expect(editor).toHaveProperty("value", changed);
  mounted.rerender(content(initial));
  expect(editor).toHaveProperty("value", initial);
  expect(onCommit).not.toHaveBeenCalled();
});

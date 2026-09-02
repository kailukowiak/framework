// @vitest-environment jsdom

import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { isTextEntryTarget } from "../FrameGrid";
import { focusGridClipboardTarget } from "../lib/gridClipboardTarget";
import { useGridClipboardTarget } from "./useGridClipboardTarget";

function TargetHarness({ active }: { active: boolean }) {
  useGridClipboardTarget(active);
  return null;
}

describe("grid clipboard target", () => {
  afterEach(() => {
    cleanup();
    document.querySelector("[data-framework-grid-clipboard]")?.remove();
  });

  it("gives native clipboard commands a selected first responder", () => {
    focusGridClipboardTarget();

    const target = document.activeElement as HTMLTextAreaElement;
    expect(target.hasAttribute("data-framework-grid-clipboard")).toBe(true);
    expect(target.selectionStart).toBe(0);
    expect(target.selectionEnd).toBe(1);
    expect(isTextEntryTarget(target)).toBe(false);
  });

  it("re-arms when a cell editor returns to navigate mode", async () => {
    const mounted = render(<TargetHarness active={false} />);
    mounted.rerender(<TargetHarness active />);

    await waitFor(() =>
      expect(document.activeElement?.hasAttribute("data-framework-grid-clipboard")).toBe(
        true
      )
    );
  });
});

// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProjectPanel } from "./ProjectPanel";
import { fixtures } from "./test/support";

describe("ProjectPanel", () => {
  afterEach(cleanup);

  it("owns the document-level Excel export action", async () => {
    render(
      <ProjectPanel
        document={fixtures.salesWithFormula}
        path="/tmp/sales.fw"
        onClose={vi.fn()}
        onOperation={vi.fn(async () => null)}
        onSaveAs={vi.fn(async () => {})}
        onPackage={vi.fn(async () => {})}
        onCompact={vi.fn(async () => {})}
      />
    );

    await userEvent.click(screen.getByRole("button", { name: "Export to Excel…" }));

    expect(screen.getByRole("heading", { name: "Choose worksheets" })).toBeTruthy();
    expect(screen.getByRole("checkbox", { name: /Monthly sales/ })).toBeTruthy();
  });
});

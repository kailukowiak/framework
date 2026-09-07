// @vitest-environment jsdom
import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { clearMocks, fixtures, objectNamed, serveInvoke } from "../test/support";
import { usePlotRows } from "./usePlotRows";

afterEach(clearMocks);

it("invalidates paged chart rows on lineage or refresh changes, without rereading on unrelated renders", async () => {
  const frame = objectNamed(fixtures.importedSales, "frame", "Imported sales");
  const computed = fixtures.importedSales.computedFrames[frame.id];
  // Explicit page answers only: the mock does not calculate anything.
  const page = { frameId: frame.id, totalRows: 1, rows: [["East", "10"]], columns: frame.columns, rowIds: ["row"], offset: 0, limit: 10000, styleMatches: [] };
  const fetch = vi.fn().mockResolvedValue(page);
  serveInvoke({ get_frame_page: fetch });
  const { result, rerender } = renderHook(({ refresh, fingerprint }) => usePlotRows(frame, { ...computed, fingerprint }, false, refresh), {
    initialProps: { refresh: 0, fingerprint: computed.fingerprint },
  });
  await waitFor(() => expect(result.current.loading).toBe(false));
  expect(fetch).toHaveBeenCalledTimes(1);
  rerender({ refresh: 0, fingerprint: computed.fingerprint });
  expect(fetch).toHaveBeenCalledTimes(1);
  rerender({ refresh: 0, fingerprint: "changed-lineage" });
  await waitFor(() => expect(fetch).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(result.current.loading).toBe(false));
  rerender({ refresh: 1, fingerprint: "changed-lineage" });
  await waitFor(() => expect(fetch).toHaveBeenCalledTimes(3));
});

it("does not publish a late page after its source was invalidated", async () => {
  const frame = objectNamed(fixtures.importedSales, "frame", "Imported sales");
  const computed = fixtures.importedSales.computedFrames[frame.id];
  let finish!: (value: unknown) => void;
  const old = new Promise((resolve) => { finish = resolve; });
  const fetch = vi.fn().mockReturnValueOnce(old).mockResolvedValue({ totalRows: 0, rows: [] });
  serveInvoke({ get_frame_page: fetch });
  const { result, rerender } = renderHook(({ refresh }) => usePlotRows(frame, computed, false, refresh), { initialProps: { refresh: 0 } });
  rerender({ refresh: 1 });
  await waitFor(() => expect(result.current.loading).toBe(false));
  await act(async () => finish({ totalRows: 1, rows: [["Old", "999"]] }));
  expect(result.current.rows).toEqual([]);
});

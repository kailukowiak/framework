import { describe, expect, it, vi } from "vitest";
import { acceptCompletionOnce } from "./completionAcceptance";

describe("acceptCompletionOnce", () => {
  it("ignores a repeated accept until the source or cursor advances", () => {
    const acceptedAt = { current: null as string | null };
    const accept = vi.fn();

    acceptCompletionOnce(acceptedAt, "`Mon\u00004", accept);
    acceptCompletionOnce(acceptedAt, "`Mon\u00004", accept);
    expect(accept).toHaveBeenCalledTimes(1);

    acceptCompletionOnce(acceptedAt, "`Monthly sales`.\u000016", accept);
    expect(accept).toHaveBeenCalledTimes(2);
  });
});

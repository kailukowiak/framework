import { describe, expect, it } from "vitest";
import { libraryEntryState } from "./datasetLibraryEntries";

describe("libraryEntryState", () => {
  it("is enabled with no suffix once the file exists and reads fine", () => {
    expect(libraryEntryState({ exists: true, readable: true })).toEqual({
      disabled: false,
      suffix: null,
    });
  });

  it("disables and labels an entry that exists but cannot be read", () => {
    expect(libraryEntryState({ exists: true, readable: false })).toEqual({
      disabled: true,
      suffix: "can't be read",
    });
  });

  it("stays enabled when readable is omitted, so old cached entries and test fixtures do not go dark", () => {
    expect(libraryEntryState({ exists: true })).toEqual({
      disabled: false,
      suffix: null,
    });
  });

  it("never disables an entry that does not exist — that is a different, prior state", () => {
    expect(libraryEntryState({ exists: false, readable: false })).toEqual({
      disabled: false,
      suffix: null,
    });
  });
});

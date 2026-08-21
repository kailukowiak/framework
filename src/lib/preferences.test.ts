import { describe, expect, it } from "vitest";
import {
  DEFAULT_IMPORT_MODE,
  DEFAULT_INTERFACE_SCALE,
  parseAskOnImport,
  parseImportMode,
  MAX_INTERFACE_SCALE,
  MIN_INTERFACE_SCALE,
  clampInterfaceScale,
  formatInterfaceScale,
  mcpSetupText,
  nudgeInterfaceScale,
  parseInterfaceScale,
  parseUseThousandsSeparators,
} from "./preferences";

describe("clampInterfaceScale", () => {
  it("keeps a scale inside the range the window can lay out in", () => {
    expect(clampInterfaceScale(3)).toBe(MAX_INTERFACE_SCALE);
    expect(clampInterfaceScale(0.1)).toBe(MIN_INTERFACE_SCALE);
    expect(clampInterfaceScale(1.2)).toBe(1.2);
  });

  it("lands on a step rather than between two", () => {
    expect(clampInterfaceScale(1.23)).toBe(1.25);
    expect(clampInterfaceScale(1.11)).toBe(1.1);
  });

  it("falls back to full size for a scale that is not a number", () => {
    expect(clampInterfaceScale(Number.NaN)).toBe(DEFAULT_INTERFACE_SCALE);
    expect(clampInterfaceScale(Number.POSITIVE_INFINITY)).toBe(
      DEFAULT_INTERFACE_SCALE
    );
  });
});

describe("MCP setup instructions", () => {
  it("writes current Codex and Claude Code stdio commands", () => {
    expect(mcpSetupText("codex", "/bin/framework-mcp", "/work/model.fw")).toBe(
      "codex mcp add framework -- '/bin/framework-mcp' --document '/work/model.fw'"
    );
    expect(mcpSetupText("claude", "/bin/framework-mcp", "/work/model.fw")).toBe(
      "claude mcp add framework -- '/bin/framework-mcp' --document '/work/model.fw'"
    );
  });

  it("quotes shell paths and emits the generic stdio configuration", () => {
    expect(mcpSetupText("codex", "/Kai's tools/mcp", "/work/my model.fw")).toContain(
      "'/Kai'\\''s tools/mcp'"
    );
    expect(JSON.parse(mcpSetupText("generic", "/bin/mcp", "/work/model.fw")))
      .toEqual({
        mcpServers: {
          framework: {
            command: "/bin/mcp",
            args: ["--document", "/work/model.fw"],
          },
        },
      });
  });
});

describe("nudgeInterfaceScale", () => {
  it("moves by a step you can see", () => {
    expect(nudgeInterfaceScale(1, 1)).toBe(1.1);
    expect(nudgeInterfaceScale(1, -1)).toBe(0.9);
  });

  // Holding the shortcut down should stop at the end of the range, not walk
  // off it and leave the next press with ground to make up.
  it("stops at the ends", () => {
    expect(nudgeInterfaceScale(MAX_INTERFACE_SCALE, 1)).toBe(MAX_INTERFACE_SCALE);
    expect(nudgeInterfaceScale(MIN_INTERFACE_SCALE, -1)).toBe(MIN_INTERFACE_SCALE);
  });

  it("does not accumulate float dust", () => {
    let scale = MIN_INTERFACE_SCALE;
    for (let press = 0; press < 5; press += 1) scale = nudgeInterfaceScale(scale, 1);
    expect(scale).toBe(1.3);
  });
});

describe("formatInterfaceScale", () => {
  it("reads as a percentage", () => {
    expect(formatInterfaceScale(1)).toBe("100%");
    expect(formatInterfaceScale(1.25)).toBe("125%");
    expect(formatInterfaceScale(0.8)).toBe("80%");
  });
});

describe("parseInterfaceScale", () => {
  it("reads back what was stored", () => {
    expect(parseInterfaceScale("1.25")).toBe(1.25);
  });

  it("treats a missing or unreadable preference as full size", () => {
    expect(parseInterfaceScale(null)).toBe(DEFAULT_INTERFACE_SCALE);
    expect(parseInterfaceScale("")).toBe(DEFAULT_INTERFACE_SCALE);
    expect(parseInterfaceScale("large")).toBe(DEFAULT_INTERFACE_SCALE);
  });

  it("brings a scale from an older build into range", () => {
    expect(parseInterfaceScale("4")).toBe(MAX_INTERFACE_SCALE);
  });
});

describe("import preferences", () => {
  it("reads back a stored choice", () => {
    expect(parseImportMode("linked")).toBe("linked");
    expect(parseImportMode("stored")).toBe("stored");
  });

  // Storing the data is the answer that cannot surprise anyone: the values
  // stay as they were read and the document depends on nothing outside it.
  it("keeps the data in the document when nothing says otherwise", () => {
    expect(parseImportMode(null)).toBe(DEFAULT_IMPORT_MODE);
    expect(parseImportMode("")).toBe("stored");
    expect(parseImportMode("something an older build wrote")).toBe("stored");
  });

  it("asks unless told not to", () => {
    expect(parseAskOnImport(null)).toBe(true);
    expect(parseAskOnImport("true")).toBe(true);
    expect(parseAskOnImport("false")).toBe(false);
  });
});

describe("number preferences", () => {
  it("groups by default and only disables grouping explicitly", () => {
    expect(parseUseThousandsSeparators(null)).toBe(true);
    expect(parseUseThousandsSeparators("true")).toBe(true);
    expect(parseUseThousandsSeparators("false")).toBe(false);
  });
});

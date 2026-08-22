import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
// @ts-expect-error — a plain .mjs script; see changelog-section.test.ts.
import { closeUnreleased } from "./set-version.mjs";
// @ts-expect-error — as above.
import { changelogSection } from "./changelog-section.mjs";

const CHANGELOG = `# Changelog

Prose about how this file works.

## Unreleased

- Something shipping now

## 0.1.3

- Something that already shipped
`;

describe("closeUnreleased", () => {
  it("renames Unreleased to the version and opens a fresh one", () => {
    const after = closeUnreleased(CHANGELOG, "0.1.4");
    expect(after).toContain("## Unreleased\n\n## 0.1.4\n\n- Something shipping now");
  });

  // The point of the rename: the release workflow looks the version up by
  // heading, and finds nothing until Unreleased has been closed under it.
  it("makes the entry findable by version, leaving Unreleased empty", () => {
    const after = closeUnreleased(CHANGELOG, "0.1.4");
    expect(changelogSection(after, "0.1.4")).toBe("- Something shipping now");
    expect(changelogSection(after, "Unreleased")).toBeNull();
    expect(changelogSection(after, "0.1.3")).toBe("- Something that already shipped");
  });

  it("reports a changelog with no Unreleased section rather than guessing", () => {
    expect(closeUnreleased("# Changelog\n\n## 0.1.3\n\n- Shipped\n", "0.1.4")).toBeNull();
  });
});

// Every place the version is spelled has to agree, or a release ships with a
// stale number in a corner of the app that nothing compares against the tag.
describe("the version this working tree carries", () => {
  const root = (path: string) =>
    readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

  it("is the same in every file that spells it out", () => {
    const version = JSON.parse(root("package.json")).version;
    expect(version).toMatch(/^\d+\.\d+\.\d+$/);
    expect(JSON.parse(root("src-tauri/tauri.conf.json")).version).toBe(version);
    expect(root("Cargo.toml")).toContain(`version = "${version}"`);
    expect(root("crates/framework-mcp/src/main.rs")).toContain(
      `version = "${version}"`
    );
  });
});

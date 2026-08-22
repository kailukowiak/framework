import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
// @ts-expect-error — a plain .mjs script, run by the release workflow as a
// command; there is nothing worth a .d.ts here.
import { changelogSection } from "./changelog-section.mjs";

const CHANGELOG = `# Changelog

Prose about how this file works.

## Unreleased

- Something not shipped yet

## 0.1.3

- Linux desktops recognise FrameWork

## 0.1.2

- Updates arrive in the app
- Package-manager installs say so

## 0.1.1
`;

describe("changelogSection", () => {
  it("returns one version's entry, stopping at the next heading", () => {
    expect(changelogSection(CHANGELOG, "0.1.2")).toBe(
      "- Updates arrive in the app\n- Package-manager installs say so"
    );
  });

  // Tags carry the v; tauri.conf.json and the changelog do not. Both spellings
  // reach this, and neither is worth a release failing over.
  it("accepts the tag spelling of a version", () => {
    expect(changelogSection(CHANGELOG, "v0.1.3")).toBe(
      "- Linux desktops recognise FrameWork"
    );
  });

  it("reads Unreleased like any other section", () => {
    expect(changelogSection(CHANGELOG, "Unreleased")).toBe(
      "- Something not shipped yet"
    );
  });

  // Both are the same failure to the workflow: nothing to tell anyone about.
  it("reports a missing version and an empty section alike", () => {
    expect(changelogSection(CHANGELOG, "9.9.9")).toBeNull();
    expect(changelogSection(CHANGELOG, "0.1.1")).toBeNull();
  });

  // The file this repo actually ships, at the version it currently carries:
  // a release cannot describe itself if the heading shape drifts away from
  // what the workflow greps for, or if the bump was made without one.
  it("has an entry for the version this working tree is at", () => {
    const real = readFileSync(
      new URL("../CHANGELOG.md", import.meta.url),
      "utf8"
    );
    const { version } = JSON.parse(
      readFileSync(new URL("../package.json", import.meta.url), "utf8")
    );
    expect(changelogSection(real, version)).toBeTruthy();
    expect(changelogSection(real, "0.1.3")).toBeTruthy();
  });
});

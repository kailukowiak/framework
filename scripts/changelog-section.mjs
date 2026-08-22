/**
 * Lifts one version's section out of CHANGELOG.md.
 *
 * Release notes exist in exactly one place. The GitHub release body and the
 * in-app update offer are both generated from this file, because the
 * alternative — notes written into the release workflow, or typed into the
 * Releases page at tag time — is notes written by whoever is cutting the
 * release, at the moment they have least context about what went into it.
 * The changelog entry is written when the change is, by the person making it.
 *
 * Missing is an error, not an empty string: a release that cannot say what
 * changed should stop the build in twenty seconds rather than publish three
 * platforms' installers under a blank heading.
 *
 *     node scripts/changelog-section.mjs 0.1.3 [path/to/CHANGELOG.md]
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** The section body under `## <version>`, up to the next `##`, or null. */
export function changelogSection(markdown, version) {
  const lines = markdown.split("\n");
  const wanted = version.replace(/^v/, "").trim().toLowerCase();
  let collecting = false;
  const body = [];
  for (const line of lines) {
    const heading = line.match(/^##\s+(.*)$/);
    if (heading) {
      if (collecting) break;
      collecting = heading[1].replace(/^v/, "").trim().toLowerCase() === wanted;
      continue;
    }
    if (collecting) body.push(line);
  }
  if (!collecting && body.length === 0) return null;
  const text = body.join("\n").trim();
  return text === "" ? null : text;
}

// Node runs this file both as a module (the test imports it) and as a
// command (the release workflow runs it); only the latter should exit.
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const [version, path = "CHANGELOG.md"] = process.argv.slice(2);
  if (!version) {
    console.error("usage: node scripts/changelog-section.mjs <version> [changelog]");
    process.exit(2);
  }
  const section = changelogSection(readFileSync(path, "utf8"), version);
  if (section === null) {
    console.error(
      `No entry for ${version} in ${path}. Add a "## ${version.replace(/^v/, "")}" section describing what changed, commit it, and move the tag onto that commit.`
    );
    process.exit(1);
  }
  process.stdout.write(`${section}\n`);
}

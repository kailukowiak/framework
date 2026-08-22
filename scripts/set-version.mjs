/**
 * Sets the version everywhere it is written, and closes the changelog's
 * Unreleased section under that number.
 *
 * The version lives in five hand-edited places — the npm package, the Cargo
 * workspace, tauri.conf.json, the MCP server's advertised version, and the
 * changelog heading — plus two lockfiles that record it back. Nothing checks
 * that they agree, and only one of them is compared against the tag, so the
 * failure mode is a release that builds and publishes with a stale number in
 * the corner of an About box or in what the MCP server tells an agent it is.
 * One command writing all of them is the fix; `npm run version:set 0.1.4`.
 *
 * Deliberately does not commit, tag, or push. Read `git diff` first — this
 * script has no way to know whether the changelog section it just closed
 * actually describes the commits going into the release.
 */
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const VERSION = /^\d+\.\d+\.\d+$/;

/** `## Unreleased` becomes `## <version>`, with a fresh empty one above it. */
export function closeUnreleased(changelog, version) {
  const heading = changelog.match(/^##\s+Unreleased\s*$/m);
  if (!heading) return null;
  return changelog.replace(
    /^##\s+Unreleased\s*\n+/m,
    `## Unreleased\n\n## ${version}\n\n`
  );
}

function edit(path, rewrite) {
  const before = readFileSync(path, "utf8");
  const after = rewrite(before);
  if (after === null || after === before) {
    console.error(`::error::${path} was not changed — its version is written somewhere this script does not know about.`);
    process.exit(1);
  }
  writeFileSync(path, after);
  console.log(`  ${path}`);
}

// Run as a command, not when the test imports closeUnreleased.
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const version = process.argv[2];
  if (!version || !VERSION.test(version)) {
    console.error("usage: node scripts/set-version.mjs <major.minor.patch>");
    process.exit(2);
  }

  console.log(`Setting version ${version} in:`);
  edit("package.json", (s) => s.replace(/("version":\s*)"\d+\.\d+\.\d+"/, `$1"${version}"`));
  edit("Cargo.toml", (s) => s.replace(/^version = "\d+\.\d+\.\d+"$/m, `version = "${version}"`));
  edit("src-tauri/tauri.conf.json", (s) =>
    s.replace(/("version":\s*)"\d+\.\d+\.\d+"/, `$1"${version}"`)
  );
  // What the MCP server calls itself to an agent that connects to it.
  edit("crates/framework-mcp/src/main.rs", (s) =>
    s.replace(/(version = )"\d+\.\d+\.\d+"/, `$1"${version}"`)
  );
  edit("CHANGELOG.md", (s) => closeUnreleased(s, version));

  // The lockfiles record the workspace's own version, so they are part of the
  // bump. Both commands touch only the local packages, never dependencies.
  console.log("Refreshing lockfiles:");
  execFileSync("npm", ["install", "--package-lock-only", "--ignore-scripts"], { stdio: "inherit" });
  execFileSync("cargo", ["update", "--workspace"], { stdio: "inherit" });

  console.log(`
Now, in order:

  git diff                                  # check the changelog section says what shipped
  git commit -am "Bump to ${version}"
  git push
  git tag v${version} && git push origin v${version}

The tag is what starts the release build, and it must sit on the commit that
carries this bump — the workflow compares v${version} against
src-tauri/tauri.conf.json and refuses the build if they disagree. Tag after
the bump commit is pushed, never before.`);
}

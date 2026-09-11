/**
 * Builds and installs the separately-identified debug app, in whatever form
 * this platform installs.
 *
 * None of this can be written once in package.json, because the differences
 * are not preferences:
 *
 * - `app` is a macOS bundle and the Windows CLI rejects it before it builds
 *   anything (`--bundles [possible values: msi, nsis]`).
 * - Windows keys file associations on a registry ProgId taken from each
 *   `fileAssociations` name, and a ProgId is one global key: sharing those
 *   names with the release means whichever installed last owns `.fw`, and
 *   uninstalling it removes the association outright. macOS keys on the
 *   bundle identifier, which `tauri.dev-bundle.conf.json` already moves, and
 *   lists both apps under Open With. So only Windows needs its own names.
 * - A debug build links the Windows console subsystem, which is right when a
 *   shell is waiting for the output and wrong when Explorer launched it. The
 *   `dev-bundle` feature is how `main.rs` tells the two apart.
 *
 * Everything shared stays shared: the debug profile, and
 * `tauri.dev-bundle.conf.json`, which renames the product and moves the
 * identifier so this build installs beside the release instead of over it.
 */
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const DEV_CONFIG = "src-tauri/tauri.dev-bundle.conf.json";

const PLATFORM = {
  darwin: { bundles: "app", configs: [DEV_CONFIG], features: [] },
  win32: {
    bundles: "nsis",
    configs: [DEV_CONFIG, "src-tauri/tauri.dev-bundle.windows.conf.json"],
    features: ["dev-bundle"],
  },
};

const platform = PLATFORM[process.platform];
if (!platform) {
  // Linux installs from deb/rpm/AppImage, none of which register a second
  // copy the way this command promises. `npm run tauri dev` is the workflow
  // there until someone needs otherwise.
  throw new Error(
    `No dev-app bundle format for ${process.platform}. Use \`npm run tauri dev\`.`,
  );
}

const root = fileURLToPath(new URL("..", import.meta.url));
// Run the CLI's own entry under node rather than the `tauri` shim: the shim is
// only on PATH when a package manager put it there, and this script should
// also work when it is run directly.
const tauri = createRequire(import.meta.url).resolve("@tauri-apps/cli/tauri.js");

execFileSync(
  process.execPath,
  [
    tauri,
    "build",
    "--debug",
    "--bundles",
    platform.bundles,
    // Merged in order, so the Windows overrides land on top of the shared
    // dev identity rather than replacing it.
    ...platform.configs.flatMap((config) => ["--config", config]),
    ...(platform.features.length
      ? ["--features", platform.features.join(",")]
      : []),
  ],
  { stdio: "inherit", cwd: root },
);

execFileSync(
  process.execPath,
  [fileURLToPath(new URL("register-dev-app.mjs", import.meta.url))],
  { stdio: "inherit", cwd: root },
);

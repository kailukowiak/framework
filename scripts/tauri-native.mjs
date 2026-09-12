/** Keep developer and e2e builds on the same native libraries as releases.
 * The child environment also lets an unbundled `tauri dev` executable find
 * those libraries; packaged apps use their own Frameworks/resource directory.
 */
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { resolve, delimiter } from "node:path";

const root = resolve(import.meta.dirname, "..");
const args = process.argv.slice(2);
const platform = { darwin: "macos", win32: "windows" }[process.platform];
const env = { ...process.env };
if (platform && ["build", "dev"].includes(args[0])) {
  execFileSync(process.execPath, [resolve(root, "scripts/prepare-xgboost-native.mjs")], { stdio: "inherit" });
  const native = resolve(root, "src-tauri/native", platform);
  env.XGBOOST_LIB_DIR = native;
  if (process.platform === "darwin") {
    env.HOMEBREW_PREFIX = native;
    if (args[0] === "dev") {
      env.DYLD_FALLBACK_LIBRARY_PATH = [native, resolve(native, "opt/libomp/lib"), env.DYLD_FALLBACK_LIBRARY_PATH].filter(Boolean).join(delimiter);
    }
  } else {
    env.PATH = [native, env.PATH].filter(Boolean).join(delimiter);
  }
  args.push("--config", resolve(root, `src-tauri/tauri.${platform}-release.conf.json`));
}
const cli = createRequire(import.meta.url).resolve("@tauri-apps/cli/tauri.js");
execFileSync(process.execPath, [cli, ...args], { cwd: root, env, stdio: "inherit" });

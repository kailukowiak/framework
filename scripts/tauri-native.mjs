/** Keep developer and e2e builds on the same native libraries as releases.
 * The child environment also lets an unbundled `tauri dev` executable find
 * those libraries; packaged apps use their own Frameworks/resource directory.
 */
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { resolve, delimiter } from "node:path";

const root = resolve(import.meta.dirname, "..");
const args = process.argv.slice(2);
const platform = { darwin: "macos", win32: "windows", linux: "linux" }[process.platform];
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
  } else if (process.platform === "linux") {
    // The CLI spawns the `tauri dev` executable itself rather than through
    // `cargo run`, so Cargo's library path is not in effect and the loader
    // needs telling where the staged copy the link step used lives. `build`
    // needs it too: the AppImage bundler resolves the executable's shared
    // libraries with ldd before copying them into the image.
    env.LD_LIBRARY_PATH = [native, env.LD_LIBRARY_PATH].filter(Boolean).join(delimiter);
  } else {
    // Windows hands Node the variable as `Path`. Writing `PATH` beside it
    // makes a second variable holding only the native folder, and the
    // child picked that one -- so `cargo metadata` was "program not found"
    // and every Windows release build failed at the first step.
    const pathKey = Object.keys(env).find((key) => key.toUpperCase() === "PATH") ?? "PATH";
    env[pathKey] = [native, env[pathKey]].filter(Boolean).join(delimiter);
  }
  args.push("--config", resolve(root, `src-tauri/tauri.${platform}-release.conf.json`));
}
const cli = createRequire(import.meta.url).resolve("@tauri-apps/cli/tauri.js");
execFileSync(process.execPath, [cli, ...args], { cwd: root, env, stdio: "inherit" });

import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

// Launch Services indexes a bundle, not the unbundled hot-reload executable.
// Register a separately identified debug app without changing CSV defaults or
// replacing the release in Applications. A stable local identity is important:
// macOS records privacy grants against the designated requirement in the code
// signature, while an ad-hoc signature identifies only one exact build. Signing
// ad-hoc here therefore made every rebuild ask for Documents access again.
if (process.platform === "darwin") {
  const identity = process.env.FRAMEWORK_LOCAL_CODE_SIGNING_IDENTITY ?? "FrameWork Local Development";
  const bundle = fileURLToPath(new URL("../target/debug/bundle/macos/FrameWork Dev.app", import.meta.url));
  const identities = execFileSync("security", ["find-identity", "-v", "-p", "codesigning"], {
    encoding: "utf8",
  });
  const hasIdentity = identities
    .split("\n")
    .some((line) => line.includes(`\"${identity}\"`) && !line.includes("CSSMERR_"));
  if (!hasIdentity) {
    throw new Error(
      `Missing local code-signing identity \"${identity}\". Run npm run setup:dev-signing once, then rebuild.`,
    );
  }
  execFileSync(
    "codesign",
    ["--force", "--deep", "--timestamp=none", "--sign", identity, bundle],
    { stdio: "inherit" },
  );
  execFileSync("/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister", ["-f", bundle], { stdio: "inherit" });
  console.log(`Registered ${bundle} for Finder’s Open With menu.`);
} else if (process.platform === "win32") {
  // Nothing to mirror lsregister or the signing step with. The NSIS installer
  // writes the file associations itself, and Windows has no designated
  // requirement recording privacy grants against a signature, so a rebuilt
  // binary is not a different app to it.
  //
  // Installing is safe here only because the Windows build carries its own
  // `fileAssociations` names and its own installer hook (see
  // `tauri.dev-bundle.windows.conf.json`): the registry ProgIds it claims are
  // its own, and the hook hands every extension's default back, so it never
  // takes `.fw` from the release and uninstalling it leaves the release's
  // associations alone.
  const directory = fileURLToPath(
    new URL("../target/debug/bundle/nsis/", import.meta.url),
  );
  // Match this build's own product name rather than any installer: a plain
  // `tauri build --bundles nsis` leaves the release's setup.exe in the same
  // directory, and installing that would put a debug binary where the real
  // FrameWork lives.
  const { productName } = JSON.parse(
    readFileSync(
      new URL("../src-tauri/tauri.dev-bundle.conf.json", import.meta.url),
      "utf8",
    ),
  );
  const built = readdirSync(directory).find(
    (entry) =>
      entry.startsWith(`${productName}_`) && entry.endsWith("-setup.exe"),
  );
  if (!built) {
    throw new Error(`No ${productName} installer under ${directory}.`);
  }
  const installer = join(directory, built);
  // /S is the NSIS silent switch. The installer is per-user, so it neither
  // prompts for elevation nor needs a window to click through.
  execFileSync(installer, ["/S"], { stdio: "inherit" });
  console.log(`Installed ${built} for Explorer’s Open With menu.`);
}

import { execFileSync } from "node:child_process";
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
}

import { createHash } from "node:crypto";
import { chmod, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const root = resolve(import.meta.dirname, "..");
const nativeRoot = join(root, "src-tauri", "native");

const artifacts = {
  darwin: {
    xgboost: {
      url: "https://github.com/marcomq/rust-xgboost/raw/refs/tags/v3.0.5/xgboost-sys/lib/mac_arm64/libxgboost.dylib",
      sha256: "e438dacf4a1ec44e4f5f1e5005f291f7b66156af7fc49e543b60a5fdc079c024",
    },
    // Homebrew's LLVM 23.1.1 arm64 Tahoe bottle. The extracted library has a
    // macOS 26.0 deployment target, which is FrameWork's supported floor.
    libompBottle: {
      url: "https://ghcr.io/v2/homebrew/core/libomp/blobs/sha256:78b11c916952efc86cfebfe80ec6e877c8f0f974839d2329ef9653fc8fd6718c",
      sha256: "78b11c916952efc86cfebfe80ec6e877c8f0f974839d2329ef9653fc8fd6718c",
      librarySha256: "ec784b1b000012b16db34f4990d5c7eb86324d1ce096761a2a2706d681750d70",
    },
  },
  win32: {
    dll: {
      url: "https://github.com/marcomq/rust-xgboost/raw/refs/tags/v3.0.5/xgboost-sys/lib/win_amd64/xgboost.dll",
      sha256: "831b8fbeb97a879712e315a34ed36b26e9a297ca768317ed930b8a623bd5ccc1",
    },
    importLibrary: {
      url: "https://github.com/marcomq/rust-xgboost/raw/refs/tags/v3.0.5/xgboost-sys/lib/win_amd64/xgboost.lib",
      sha256: "0ac9f77281d584c0d9f6ec67cbc7ee143fa21f777ca9446945c619c47802a05c",
    },
  },
};

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

async function downloadPinned(artifact, destination, headers = {}) {
  await mkdir(dirname(destination), { recursive: true });
  const existing = await readFile(destination).catch(() => undefined);
  if (existing && sha256(existing) === artifact.sha256) return;

  const response = await fetch(artifact.url, { headers, redirect: "follow" });
  if (!response.ok) throw new Error(`download failed (${response.status}) for ${artifact.url}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  const actual = sha256(bytes);
  if (actual !== artifact.sha256) {
    throw new Error(`checksum mismatch for ${artifact.url}: ${actual}`);
  }
  const temporary = `${destination}.tmp-${process.pid}`;
  await writeFile(temporary, bytes);
  await rename(temporary, destination);
}

function run(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8", stdio: "inherit" });
  if (result.status !== 0) throw new Error(`${command} exited with ${result.status}`);
}

async function prepareMacos() {
  if (process.arch !== "arm64") throw new Error(`unsupported macOS architecture: ${process.arch}`);
  const destination = join(nativeRoot, "macos");
  const xgboost = join(destination, "libxgboost.dylib");
  const bottle = join(destination, "libomp-bottle.tar.gz");
  const ompRoot = join(destination, "opt", "libomp");
  const libomp = join(ompRoot, "lib", "libomp.dylib");
  await downloadPinned(artifacts.darwin.xgboost, xgboost);
  await downloadPinned(artifacts.darwin.libompBottle, bottle, {
    Accept: "application/vnd.oci.image.layer.v1.tar+gzip",
    Authorization: "Bearer QQ==",
  });
  await rm(ompRoot, { recursive: true, force: true });
  await mkdir(join(ompRoot, "lib"), { recursive: true });
  run("tar", ["-xzf", bottle, "-C", destination, "libomp/23.1.1/lib/libomp.dylib", "libomp/23.1.1/LICENSE.TXT"]);
  await rename(join(destination, "libomp", "23.1.1", "lib", "libomp.dylib"), libomp);
  await rename(join(destination, "libomp", "23.1.1", "LICENSE.TXT"), join(ompRoot, "LICENSE.TXT"));
  await rm(join(destination, "libomp"), { recursive: true, force: true });
  if (sha256(await readFile(libomp)) !== artifacts.darwin.libompBottle.librarySha256) {
    throw new Error("extracted libomp.dylib checksum mismatch");
  }
  await chmod(xgboost, 0o755);
  await chmod(libomp, 0o755);
  run("install_name_tool", ["-id", "@rpath/libxgboost.dylib", "-change", "@rpath/libomp.dylib", "@rpath/libomp.dylib", xgboost]);
  run("install_name_tool", ["-id", "@rpath/libomp.dylib", libomp]);
  // install_name_tool invalidates Mach-O signatures. Tauri signs the final
  // nested copies again with the app identity during bundling.
  run("codesign", ["--force", "--sign", "-", xgboost]);
  run("codesign", ["--force", "--sign", "-", libomp]);
  console.log(`prepared XGBoost native libraries in ${destination}`);
}

async function prepareWindows() {
  if (process.arch !== "x64") throw new Error(`unsupported Windows architecture: ${process.arch}`);
  const destination = join(nativeRoot, "windows");
  await downloadPinned(artifacts.win32.dll, join(destination, "xgboost.dll"));
  await downloadPinned(artifacts.win32.importLibrary, join(destination, "xgboost.lib"));
  console.log(`prepared XGBoost native libraries in ${destination}`);
}

if (process.platform === "darwin") await prepareMacos();
else if (process.platform === "win32") await prepareWindows();
else console.log(`XGBoost native training is not packaged for ${process.platform}`);

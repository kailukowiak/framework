import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// End-to-end harness for the real desktop app.
//
// `npm run test:e2e` builds the frontend, builds the desktop binary with the
// `e2e` cargo feature (which embeds a W3C WebDriver server — macOS has no
// external driver for WKWebView), then runs these specs against the launched
// app. The specs drive the same binary a person would use: real WKWebView,
// real Rust engine, real persistence.
//
// `npm run test:e2e:only` skips both builds for spec iteration — but the
// binary it runs is whatever was built last, so never trust a green
// `test:e2e:only` after changing app code.
// ---------------------------------------------------------------------------

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const runDirectory = mkdtempSync(join(tmpdir(), "framework-e2e-"));

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./specs/**/*.e2e.ts"],
  // One app instance at a time. Each wdio invocation gets a private tutorial
  // library below runDirectory, so even destructive UI flows cannot touch the
  // workbooks a person keeps in Documents.
  maxInstances: 1,
  capabilities: [
    // `tauri:options` is read by @wdio/tauri-service at runtime, but 1.3.0
    // ships no type augmentation for it, so the vendor key has to be
    // asserted past WDIO's standard capability type.
    // The bundled .app, not the bare cargo binary: an unbundled executable
    // has no Info.plist identity, and its WebKit helper processes proved
    // unreliable — blank webviews and crash reports. The bundle is also what
    // people actually run.
    {
      browserName: "tauri",
      "tauri:options": {
        // The inner executable, not the .app directory: the service spawns
        // the path directly and cannot exec a directory. Launched this way
        // the process still has full bundle identity — macOS resolves
        // Info.plist relative to the executable.
        application: join(
          repositoryRoot,
          "target",
          "debug",
          "bundle",
          "macos",
          "FrameWork.app",
          "Contents",
          "MacOS",
          "framework-desktop"
        ),
      },
    } as WebdriverIO.Capabilities,
  ],
  services: [
    [
      "@wdio/tauri-service",
      {
        driverProvider: "embedded",
        captureBackendLogs: true,
        env: {
          FRAMEWORK_E2E_TUTORIAL_DIRECTORY: join(runDirectory, "tutorials"),
          FRAMEWORK_E2E_CONNECTOR_PROFILE_PATH: join(runDirectory, "cli-connectors.json"),
          FRAMEWORK_E2E_DATABASE_CONNECTION_PATH: join(runDirectory, "database-connections.json"),
        },
      },
    ],
  ],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    // A spec that opens a workbook pays for document load plus recompute;
    // generous so a slow first run does not read as a product bug.
    timeout: 90_000,
  },
  waitforTimeout: 15_000,
  connectionRetryTimeout: 60_000,
  logLevel: "warn",
  onComplete: () => {
    rmSync(runDirectory, { recursive: true, force: true });
  },
};

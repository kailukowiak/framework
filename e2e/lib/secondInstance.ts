import { spawn, type ChildProcess } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// A second, independent app instance, driven over the raw W3C protocol.
//
// The persistence spec needs a process boundary: proof that an edit made in
// the harness's app comes back from disk in a process that never saw it.
// The service cannot restart its own app mid-run, but e2e builds are
// deliberately many-instance, so the spec launches its own — on a different
// WebDriver port — and walks its fresh Data library with a handful of raw
// fetch calls. Same protocol, same embedded server, no second framework.
// ---------------------------------------------------------------------------

const BUNDLE_EXECUTABLE = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
  "target",
  "debug",
  "bundle",
  "macos",
  "FrameWork.app",
  "Contents",
  "MacOS",
  "framework-desktop"
);

const PORT = 4655;
const BASE = `http://127.0.0.1:${PORT}`;

export interface SecondInstance {
  /** Clicks the first element matching the XPath, retrying while it renders. */
  click(xpath: string): Promise<void>;
  /** Resolves once an element matching the XPath exists. */
  waitForElement(xpath: string): Promise<void>;
  /** Quits the instance and its session. */
  dispose(): Promise<void>;
}

async function findElement(
  sessionId: string,
  xpath: string
): Promise<string | null> {
  const response = await fetch(`${BASE}/session/${sessionId}/element`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ using: "xpath", value: xpath }),
  });
  if (!response.ok) return null;
  const body = (await response.json()) as { value: Record<string, string> };
  return Object.values(body.value)[0] ?? null;
}

async function retrying<T>(
  attempt: () => Promise<T | null>,
  what: string
): Promise<T> {
  const deadline = Date.now() + 15_000;
  for (;;) {
    const result = await attempt();
    if (result !== null) return result;
    if (Date.now() > deadline) throw new Error(`second instance: ${what} never appeared`);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
}

export async function launchSecondInstance(): Promise<SecondInstance> {
  const child: ChildProcess = spawn(BUNDLE_EXECUTABLE, [], {
    env: { ...process.env, TAURI_WEBDRIVER_PORT: String(PORT) },
    stdio: "ignore",
  });

  await retrying(async () => {
    const ready = await fetch(`${BASE}/status`)
      .then((response) => response.ok)
      .catch(() => false);
    return ready ? true : null;
  }, "the WebDriver status endpoint");

  const session = await fetch(`${BASE}/session`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ capabilities: {} }),
  });
  const sessionId = ((await session.json()) as {
    value: { sessionId: string };
  }).value.sessionId;

  return {
    async click(xpath: string) {
      const element = await retrying(
        () => findElement(sessionId, xpath),
        xpath
      );
      const clicked = await fetch(
        `${BASE}/session/${sessionId}/element/${element}/click`,
        { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" }
      );
      if (!clicked.ok) throw new Error(`second instance: click failed for ${xpath}`);
    },
    async waitForElement(xpath: string) {
      await retrying(() => findElement(sessionId, xpath), xpath);
    },
    async dispose() {
      await fetch(`${BASE}/session/${sessionId}`, { method: "DELETE" }).catch(
        () => undefined
      );
      child.kill();
    },
  };
}

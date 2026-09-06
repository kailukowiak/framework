import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import ScratchworkWindow from "./ScratchworkWindow";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { ErrorBoundary } from "./ErrorBoundary";
import { installGlobalErrorReporting } from "./lib/errorReporting";
import "./styles.css";

// The e2e build is a separate frontend artifact embedded only by the desktop
// binary compiled with src-tauri's `e2e` feature. Loading the guest bridge at
// build time avoids a runtime marker racing the page bootstrap, while the
// dynamic import keeps the bridge out of every ordinary frontend bundle.
if (import.meta.env.VITE_FRAMEWORK_E2E === "true") {
  void import("@wdio/tauri-plugin");
}

// Catches what the error boundary below cannot: a throw or a rejected
// promise outside React's render tree, which otherwise disappears with
// nothing recorded anywhere.
installGlobalErrorReporting();

const scratchworkWindow = new URLSearchParams(window.location.search).has(
  "scratchwork"
);
if (scratchworkWindow) document.body.classList.add("scratchwork-window-body");
const Root = scratchworkWindow ? ScratchworkWindow : App;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <ActiveFormulaEditorProvider>
        <Root />
      </ActiveFormulaEditorProvider>
    </ErrorBoundary>
  </StrictMode>
);

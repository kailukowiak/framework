import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import "./styles.css";

// The e2e build is a separate frontend artifact embedded only by the desktop
// binary compiled with src-tauri's `e2e` feature. Loading the guest bridge at
// build time avoids a runtime marker racing the page bootstrap, while the
// dynamic import keeps the bridge out of every ordinary frontend bundle.
if (import.meta.env.VITE_FRAMEWORK_E2E === "true") {
  void import("@wdio/tauri-plugin");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ActiveFormulaEditorProvider>
      <App />
    </ActiveFormulaEditorProvider>
  </StrictMode>
);

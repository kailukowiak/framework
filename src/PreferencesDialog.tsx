import { CircleAlert, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { McpSettings } from "./lib/api";
import {
  DEFAULT_INTERFACE_SCALE,
  INTERFACE_SCALE_STEP,
  MAX_INTERFACE_SCALE,
  MIN_INTERFACE_SCALE,
  formatInterfaceScale,
  mcpSetupText,
  type ImportMode,
  type McpClient,
} from "./lib/preferences";

/** Settings that belong to this machine rather than to one document. */
export function PreferencesDialog({
  interfaceScale,
  interfaceScaleError,
  onInterfaceScale,
  importMode,
  onImportModeChange,
  askOnImport,
  onAskOnImportChange,
  copyIncludesHeaders,
  onCopyIncludesHeaders,
  useThousandsSeparators,
  onUseThousandsSeparators,
  mcpSettings,
  mcpSettingsError,
  documentPath,
  onMcpEnabledChange,
  onKeyboardShortcuts,
  onClose,
}: {
  interfaceScale: number;
  interfaceScaleError: string | null;
  onInterfaceScale: (scale: number) => void;
  importMode: ImportMode;
  onImportModeChange: (mode: ImportMode) => void;
  askOnImport: boolean;
  onAskOnImportChange: (ask: boolean) => void;
  copyIncludesHeaders: boolean;
  onCopyIncludesHeaders: (includeHeaders: boolean) => void;
  useThousandsSeparators: boolean;
  onUseThousandsSeparators: (useGrouping: boolean) => void;
  mcpSettings: McpSettings | null;
  mcpSettingsError: string | null;
  documentPath: string | null;
  onMcpEnabledChange: (enabled: boolean) => void;
  onKeyboardShortcuts: () => void;
  onClose: () => void;
}) {
  const [mcpClient, setMcpClient] = useState<McpClient>("codex");
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="insert-dialog preferences-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">THIS MACHINE</span>
            <h2>Preferences</h2>
          </div>
          <button className="icon-button" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <section className="preference-section">
          <div className="preference-heading">
            <strong>Interface size</strong>
            <span className="preference-readout">
              {formatInterfaceScale(interfaceScale)}
            </span>
          </div>
          <div className="preference-scale">
            <span className="scale-end">A</span>
            <input
              autoFocus
              type="range"
              min={MIN_INTERFACE_SCALE}
              max={MAX_INTERFACE_SCALE}
              step={INTERFACE_SCALE_STEP}
              value={interfaceScale}
              aria-label="Interface size"
              onChange={(event) => onInterfaceScale(Number(event.target.value))}
            />
            <span className="scale-end large">A</span>
          </div>
          <p className="preference-note">
            Scales the whole window — type, rows, panels and all — rather than
            the text alone, so a larger size stays the same layout. <kbd>⌘-</kbd>{" "}
            and <kbd>⌘+</kbd> adjust it from anywhere, and <kbd>⌘0</kbd> returns
            to 100%.
          </p>
          {interfaceScaleError && (
            <p className="preference-error">
              <CircleAlert size={12} /> This build could not resize the window:{" "}
              {interfaceScaleError}
            </p>
          )}
        </section>

        <section className="preference-section">
          <div className="preference-heading"><strong>Importing</strong></div>
          <label className="preference-select">
            <span>New imports</span>
            <select
              value={importMode}
              onChange={(event) => onImportModeChange(event.target.value as ImportMode)}
            >
              <option value="stored">Static — hold the data in the document</option>
              <option value="linked">Refreshable — keep reading the file</option>
            </select>
          </label>
          <label className="preference-check">
            <input
              type="checkbox"
              checked={askOnImport}
              onChange={(event) => onAskOnImportChange(event.target.checked)}
            />
            <span>Ask each time I import</span>
          </label>
          <p className="preference-note">
            Stored data belongs to the document: nothing refreshes over it and it
            can be edited. A linked frame follows its file instead, and a refresh
            replaces what is in it.
          </p>
        </section>

        <section className="preference-section">
          <div className="preference-heading"><strong>Numbers</strong></div>
          <label className="preference-check">
            <input
              type="checkbox"
              checked={useThousandsSeparators}
              onChange={(event) => onUseThousandsSeparators(event.target.checked)}
            />
            <span>Use thousands separators</span>
          </label>
          <p className="preference-note">
            Applies across frames on this machine. A column’s decimal, currency,
            percentage, and scale choices still belong to the workbook.
          </p>
        </section>

        <section className="preference-section">
          <div className="preference-heading"><strong>Copying</strong></div>
          <label className="preference-check">
            <input
              type="checkbox"
              checked={copyIncludesHeaders}
              onChange={(event) => onCopyIncludesHeaders(event.target.checked)}
            />
            <span>Include column names when copying cells</span>
          </label>
          <p className="preference-note">
            What <kbd>⌘C</kbd> does by default. Right-clicking a selection still
            offers both, whichever way this is set.
          </p>
        </section>

        <section className="preference-section">
          <div className="preference-heading">
            <strong>Model Context Protocol</strong>
            <span className="preference-readout">
              {mcpSettings
                ? mcpSettings.enabled
                  ? "Enabled"
                  : "Disabled"
                : "Checking…"}
            </span>
          </div>
          <label className="preference-check">
            <input
              type="checkbox"
              checked={mcpSettings?.enabled ?? false}
              disabled={!mcpSettings}
              onChange={(event) => onMcpEnabledChange(event.target.checked)}
            />
            <span>Allow MCP clients on this machine</span>
          </label>
          <p className="preference-note">
            Lets configured agents inspect and edit FrameWork documents through
            named objects and formulas. Turning it off refuses the next request
            from clients that are already connected.
          </p>
          {mcpSettingsError && (
            <p className="preference-error">
              <CircleAlert size={12} /> {mcpSettingsError}
            </p>
          )}
          <details className="mcp-setup">
            <summary>Set up a client</summary>
            <label className="preference-select">
              <span>Client</span>
              <select
                value={mcpClient}
                onChange={(event) => setMcpClient(event.target.value as McpClient)}
              >
                <option value="codex">Codex</option>
                <option value="claude">Claude Code</option>
                <option value="generic">Generic MCP client</option>
              </select>
            </label>
            <pre>
              {mcpSetupText(
                mcpClient,
                mcpSettings?.executablePath ?? null,
                documentPath
              )}
            </pre>
            {!mcpSettings?.executablePath && (
              <p className="preference-note">
                Build the server with <code>cargo build -p framework-mcp</code>,
                then replace the executable placeholder with its absolute path.
              </p>
            )}
            {!documentPath && (
              <p className="preference-note">
                Save this scratch document before connecting a client, then use
                the resulting <code>.fw</code> path.
              </p>
            )}
          </details>
        </section>

        <div className="dialog-actions">
          <button className="secondary-action" onClick={onKeyboardShortcuts}>
            Keyboard shortcuts…
          </button>
          <button
            className="secondary-action"
            disabled={interfaceScale === DEFAULT_INTERFACE_SCALE}
            onClick={() => onInterfaceScale(DEFAULT_INTERFACE_SCALE)}
          >
            Reset size
          </button>
          <button className="primary-action" onClick={onClose}>Done</button>
        </div>
      </div>
    </div>
  );
}

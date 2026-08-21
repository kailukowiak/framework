import { useCallback, useEffect, useState } from "react";
import { getMcpSettings, setMcpEnabled, type McpSettings } from "../lib/api";

/** The MCP server's current settings, and a way to toggle it on/off. */
export function useMcpSettings(): {
  mcpSettings: McpSettings | null;
  mcpSettingsError: string | null;
  changeMcpEnabled: (enabled: boolean) => Promise<void>;
} {
  const [mcpSettings, setMcpSettingsState] = useState<McpSettings | null>(null);
  const [mcpSettingsError, setMcpSettingsError] = useState<string | null>(null);

  useEffect(() => {
    void getMcpSettings()
      .then((settings) => {
        setMcpSettingsState(settings);
        setMcpSettingsError(null);
      })
      .catch((reason: unknown) =>
        setMcpSettingsError(String(reason).replace(/^Error:\s*/, ""))
      );
  }, []);

  const changeMcpEnabled = useCallback(async (enabled: boolean) => {
    try {
      const settings = await setMcpEnabled(enabled);
      setMcpSettingsState(settings);
      setMcpSettingsError(null);
    } catch (reason) {
      setMcpSettingsError(String(reason).replace(/^Error:\s*/, ""));
    }
  }, []);

  return { mcpSettings, mcpSettingsError, changeMcpEnabled };
}

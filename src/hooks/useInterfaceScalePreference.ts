import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useCallback, useEffect, useState } from "react";
import {
  DEFAULT_INTERFACE_SCALE,
  clampInterfaceScale,
  parseInterfaceScale,
} from "../lib/preferences";

/** How large this machine draws the app. See `lib/preferences.ts`. */
const INTERFACE_SCALE_PREFERENCE = "framework.interfaceScale";

function readInterfaceScalePreference(): number {
  try {
    return parseInterfaceScale(window.localStorage.getItem(INTERFACE_SCALE_PREFERENCE));
  } catch {
    return DEFAULT_INTERFACE_SCALE;
  }
}

/**
 * The interface scale preference, persisted locally, plus whatever error the
 * webview reported the last time it tried to apply one.
 */
export function useInterfaceScalePreference(): {
  interfaceScale: number;
  setInterfaceScale: (value: number) => void;
  interfaceScaleError: string | null;
} {
  const [interfaceScale, setInterfaceScaleState] = useState(
    readInterfaceScalePreference
  );
  const [interfaceScaleError, setInterfaceScaleError] = useState<string | null>(
    null
  );

  // The webview owns the zoom, so it has to be told on every launch and not
  // only when the slider moves. Failure is worth saying out loud rather than
  // swallowing: a slider that quietly does nothing reads as a broken app,
  // where the real cause is a webview that will not scale.
  useEffect(() => {
    void getCurrentWebview()
      .setZoom(interfaceScale)
      .then(() => setInterfaceScaleError(null))
      .catch((reason: unknown) => setInterfaceScaleError(String(reason)));
  }, [interfaceScale]);

  const setInterfaceScale = useCallback((value: number) => {
    const scale = clampInterfaceScale(value);
    setInterfaceScaleState(scale);
    try {
      window.localStorage.setItem(INTERFACE_SCALE_PREFERENCE, String(scale));
    } catch {
      // Storage refusing the write costs the choice its memory, not its
      // effect — the window is already at the new size.
    }
  }, []);

  return { interfaceScale, setInterfaceScale, interfaceScaleError };
}
